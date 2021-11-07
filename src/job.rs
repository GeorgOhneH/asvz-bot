use std::future::Future;
use std::pin::Pin;
use std::task::Context;

use teloxide::adaptors::AutoSend;
use teloxide::{prelude::*, RequestError};
use tokio::task::{JoinError, JoinHandle};

use crate::cmd::{LessonID, Password, Username};
use crate::job_fns;
use crate::job_update_cx::JobUpdateCx;
use crate::user::UserId;

#[derive(Debug)]
pub struct Job {
    pub kind: JobKind,
    pub user_id: UserId,
    pub handle: JoinHandle<Result<(), RequestError>>,
}

impl Job {
    pub fn notify(user_id: UserId, cx: UpdateWithCx<AutoSend<Bot>, Message>, id: LessonID) -> Self {
        let kind = JobKind::Notify(id.clone());
        let job_cx = JobUpdateCx::new(cx, id.clone());
        let fut = async move {
            job_fns::utils::wrap_exit_status(&job_cx, job_fns::notify(&job_cx, id)).await
        };
        Self {
            kind,
            user_id,
            handle: tokio::spawn(fut),
        }
    }
    pub fn notify_weekly(
        user_id: UserId,
        cx: UpdateWithCx<AutoSend<Bot>, Message>,
        id: LessonID,
    ) -> Self {
        let kind = JobKind::NotifyWeekly(id.clone());
        let job_cx = JobUpdateCx::new(cx, id.clone());
        let fut = async move {
            job_fns::utils::wrap_exit_status(&job_cx, job_fns::notify_weekly(&job_cx, id)).await
        };
        Self {
            kind,
            user_id,
            handle: tokio::spawn(fut),
        }
    }
    pub fn notify_with_msg(
        user_id: UserId,
        cx: UpdateWithCx<AutoSend<Bot>, Message>,
        id: LessonID,
        msg: impl Into<String>,
    ) -> Self {
        let kind = JobKind::Notify(id.clone());
        let msg = msg.into();
        let job_cx = JobUpdateCx::new(cx, id.clone());
        let fut = async move {
            job_cx.answer(msg).await?;
            job_fns::utils::wrap_exit_status(&job_cx, job_fns::notify(&job_cx, id)).await
        };
        Self {
            kind,
            user_id,
            handle: tokio::spawn(fut),
        }
    }
    pub fn enroll(
        user_id: UserId,
        cx: UpdateWithCx<AutoSend<Bot>, Message>,
        id: LessonID,
        username: Username,
        password: Password,
    ) -> Self {
        let kind = JobKind::Enroll(id.clone());
        let job_cx = JobUpdateCx::new(cx, id.clone());
        let fut = async move {
            job_fns::utils::wrap_exit_status(
                &job_cx,
                job_fns::enroll(&job_cx, id.clone(), username, password),
            )
            .await
        };
        Self {
            kind,
            user_id,
            handle: tokio::spawn(fut),
        }
    }
    pub fn enroll_weekly(
        user_id: UserId,
        cx: UpdateWithCx<AutoSend<Bot>, Message>,
        id: LessonID,
        username: Username,
        password: Password,
    ) -> Self {
        let kind = JobKind::EnrollWeekly(id.clone());
        let job_cx = JobUpdateCx::new(cx, id.clone());
        let fut = async move {
            job_fns::utils::wrap_exit_status(
                &job_cx,
                job_fns::enroll_weekly(&job_cx, id.clone(), username, password),
            )
            .await
        };
        Self {
            kind,
            user_id,
            handle: tokio::spawn(fut),
        }
    }
    pub fn enroll_with_msg(
        user_id: UserId,
        cx: UpdateWithCx<AutoSend<Bot>, Message>,
        id: LessonID,
        username: Username,
        password: Password,
        msg: impl Into<String>,
    ) -> Self {
        let kind = JobKind::Enroll(id.clone());
        let job_cx = JobUpdateCx::new(cx, id.clone());
        let msg = msg.into();
        let fut = async move {
            job_cx.answer(msg).await?;
            job_fns::utils::wrap_exit_status(
                &job_cx,
                job_fns::enroll(&job_cx, id.clone(), username, password),
            )
            .await
        };
        Self {
            kind,
            user_id,
            handle: tokio::spawn(fut),
        }
    }

    pub fn msg_user<T: Into<String>>(
        user_id: UserId,
        cx: UpdateWithCx<AutoSend<Bot>, Message>,
        text: T,
    ) -> Self {
        let msg = text.into();
        let handle = tokio::spawn(job_fns::msg_user(cx, msg.clone()));
        Self {
            kind: JobKind::Internal(InternalJob::MsgUser(msg)),
            user_id,
            handle,
        }
    }

    pub fn reply_and_delete<T: Into<String>>(
        user_id: UserId,
        cx: UpdateWithCx<AutoSend<Bot>, Message>,
        text: T,
    ) -> Self {
        let msg = text.into();
        let handle = tokio::spawn(job_fns::reply_and_del(cx, msg.clone()));
        Self {
            kind: JobKind::Internal(InternalJob::DeleteMsgUser(msg)),
            user_id,
            handle,
        }
    }
}

impl Future for Job {
    type Output = Result<Result<(), RequestError>, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
        Pin::new(&mut self.handle).poll(cx)
    }
}

#[derive(Debug, Clone)]
pub enum JobKind {
    Notify(LessonID),
    NotifyWeekly(LessonID),
    Enroll(LessonID),
    EnrollWeekly(LessonID),
    Internal(InternalJob),
}

impl JobKind {
    pub fn is_internal(&self) -> bool {
        matches!(self, Self::Internal(_))
    }
}

#[derive(Clone, Debug)]
pub enum InternalJob {
    MsgUser(String),
    DeleteMsgUser(String),
}
