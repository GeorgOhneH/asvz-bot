use futures::FutureExt;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;

use teloxide::adaptors::AutoSend;
use teloxide::{prelude::*, RequestError};
use tokio::task::{JoinError, JoinHandle};

use crate::cmd::{LessonID, Password, Username};
use crate::job_err::JobError;
use crate::job_fns;
use crate::job_update_cx::JobUpdateCx;
use crate::user::UserId;
use std::sync::Arc;

#[derive(Debug)]
pub struct Job {
    pub kind: JobKind,
    pub user_id: UserId,
    pub handle: JoinHandle<Result<(), JobError>>,
}

impl Job {
    pub fn new(
        kind: JobKind,
        user_id: UserId,
        cx: Arc<UpdateWithCx<AutoSend<Bot>, Message>>,
    ) -> Self {
        let fut = kind.clone().to_fut(cx.clone());
        Self {
            kind: kind.clone(),
            user_id,
            handle: tokio::spawn(job_fns::utils::attach_ctx(fut, user_id, kind, cx)),
        }
    }

    pub fn new_with_msg(
        kind: JobKind,
        msg: impl Into<String>,
        user_id: UserId,
        cx: Arc<UpdateWithCx<AutoSend<Bot>, Message>>,
    ) -> Self {
        let msg = msg.into();
        let fut = kind.clone().to_fut(cx.clone());
        let cx_clone = cx.clone();
        let fut_with_msg = async move {
            job_fns::msg_user(&cx_clone, msg).await?;
            fut.await
        };
        Self {
            kind: kind.clone(),
            user_id,
            handle: tokio::spawn(job_fns::utils::attach_ctx(fut_with_msg, user_id, kind, cx)),
        }
    }
}

impl Future for Job {
    type Output = Result<Result<(), JobError>, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
        Pin::new(&mut self.handle).poll(cx)
    }
}

#[derive(Debug, Clone)]
pub enum JobKind {
    Notify(LessonID),
    NotifyWeekly(LessonID),
    Enroll(LessonID, Username, Password),
    EnrollWeekly(LessonID, Username, Password),
    Internal(InternalJob),
}

impl JobKind {
    pub fn is_internal(&self) -> bool {
        matches!(self, Self::Internal(_))
    }

    pub fn to_fut(
        self,
        cx: Arc<UpdateWithCx<AutoSend<Bot>, Message>>,
    ) -> impl Future<Output = Result<(), RequestError>> {
        match self {
            Self::Notify(id) => {
                let job_cx = JobUpdateCx::new(cx, id.clone());
                async move {
                    job_fns::utils::wrap_exit_status(&job_cx, job_fns::notify(&job_cx, id)).await
                }
                .boxed()
            }
            Self::NotifyWeekly(id) => {
                let job_cx = JobUpdateCx::new(cx, id.clone());
                async move {
                    job_fns::utils::wrap_exit_status(&job_cx, job_fns::notify_weekly(&job_cx, id))
                        .await
                }
                .boxed()
            }
            Self::Enroll(id, username, password) => {
                let job_cx = JobUpdateCx::new(cx, id.clone());
                async move {
                    job_fns::utils::wrap_exit_status(
                        &job_cx,
                        job_fns::enroll(&job_cx, id.clone(), username, password),
                    )
                    .await
                }
                .boxed()
            }
            Self::EnrollWeekly(id, username, password) => {
                let job_cx = JobUpdateCx::new(cx, id.clone());
                async move {
                    job_fns::utils::wrap_exit_status(
                        &job_cx,
                        job_fns::enroll_weekly(&job_cx, id.clone(), username, password),
                    )
                    .await
                }
                .boxed()
            }
            Self::Internal(internal) => match internal {
                InternalJob::MsgUser(msg) => {
                    async move { job_fns::msg_user(&cx, msg.clone()).await }.boxed()
                }
                InternalJob::DeleteMsgUser(msg) => {
                    async move { job_fns::reply_and_del(&cx, msg.clone()).await }.boxed()
                }
            },
        }
    }
}

impl From<InternalJob> for JobKind {
    fn from(internal_job: InternalJob) -> Self {
        Self::Internal(internal_job)
    }
}

#[derive(Clone, Debug)]
pub enum InternalJob {
    MsgUser(String),
    DeleteMsgUser(String),
}
