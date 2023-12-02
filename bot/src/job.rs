use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;

use futures::FutureExt;
use teloxide::adaptors::AutoSend;
use teloxide::{prelude::*, RequestError};
use tokio::task::{JoinError, JoinHandle};

use asvz::lesson::LessonID;

use crate::cmd::{Password, Username};
use crate::job_err::JobError;
use crate::job_fns;
use crate::job_update_cx::JobUpdateCx;
use crate::user::{BotCtx, UserId};

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
        bot: BotCtx,
    ) -> Self {
        JobBuilder::new(kind, user_id, bot).build()
    }
    pub fn builder(
        kind: JobKind,
        user_id: UserId,
        bot: BotCtx,
    ) -> JobBuilder {
        JobBuilder::new(kind, user_id, bot)
    }
}

impl Future for Job {
    type Output = Result<Result<(), JobError>, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
        Pin::new(&mut self.handle).poll(cx)
    }
}

pub struct JobBuilder {
    kind: JobKind,
    retry_count: usize,
    user_id: UserId,
    bot: BotCtx,
    pre_msg: Option<String>,
}

impl JobBuilder {
    pub fn new(
        kind: JobKind,
        user_id: UserId,
        bot: BotCtx,
    ) -> Self {
        Self {
            kind,
            user_id,
            bot,
            retry_count: 0,
            pre_msg: None,
        }
    }

    pub fn pre_msg(mut self, msg: impl Into<String>) -> Self {
        self.pre_msg = Some(msg.into());
        self
    }

    pub fn retry_count(mut self, retry_count: usize) -> Self {
        self.retry_count = retry_count;
        self
    }

    pub fn build(self) -> Job {
        let fut = self.kind.clone().to_fut(self.bot.clone());
        let handle = if let Some(pre_msg) = self.pre_msg {
            let bot_clone = self.bot.clone();
            let fut = async move {
                job_fns::msg_user(&bot_clone, pre_msg).await?;
                fut.await
            };
            tokio::spawn(job_fns::utils::attach_ctx(
                fut,
                self.user_id,
                self.kind.clone(),
                self.bot,
                self.retry_count,
            ))
        } else {
            tokio::spawn(job_fns::utils::attach_ctx(
                fut,
                self.user_id,
                self.kind.clone(),
                self.bot,
                self.retry_count,
            ))
        };
        Job {
            kind: self.kind,
            user_id: self.user_id,
            handle,
        }
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
        bot: BotCtx,
    ) -> impl Future<Output = Result<(), RequestError>> {
        match self {
            Self::Notify(id) => {
                let job_cx = JobUpdateCx::new(bot, id.clone());
                async move {
                    job_fns::utils::wrap_exit_status(&job_cx, job_fns::notify(&job_cx, id)).await
                }
                .boxed()
            }
            Self::NotifyWeekly(id) => {
                let job_cx = JobUpdateCx::new(bot, id.clone());
                async move {
                    job_fns::utils::wrap_exit_status(&job_cx, job_fns::notify_weekly(&job_cx, id))
                        .await
                }
                .boxed()
            }
            Self::Enroll(id, username, password) => {
                let job_cx = JobUpdateCx::new(bot, id.clone());
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
                let job_cx = JobUpdateCx::new(bot, id.clone());
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
                    async move { job_fns::msg_user(&bot, msg.clone()).await }.boxed()
                }
                InternalJob::DeleteMsgUser(msg) => {
                    async move { job_fns::reply_and_del(&bot, msg.clone()).await }.boxed()
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
