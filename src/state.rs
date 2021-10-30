use teloxide::{prelude::*, utils::command::BotCommand, RequestError};

use crate::cmd::LessonID;
use futures::stream::FuturesUnordered;
use futures::stream::{self, StreamExt};
use futures::{FutureExt, Stream, TryFutureExt};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::task::Context;
use std::time::Duration;
use teloxide::adaptors::AutoSend;
use teloxide::dispatching::update_listeners;
use teloxide::dispatching::update_listeners::AsUpdateStream;
use teloxide::types::{MediaKind, MessageKind, Update, UpdateKind, User};
use teloxide::utils::command::ParseError;
use tokio::task::{JoinError, JoinHandle};
use tokio_stream::wrappers::UnboundedReceiverStream;
use crate::asvz::lesson::{lesson_data};

pub struct State {
    jobs: FuturesUnordered<Job>,
}

impl Stream for State {
    type Item = Result<Result<Option<Action>, RequestError>, JoinError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        Pin::new(&mut self.jobs).poll_next(cx)
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            jobs: FuturesUnordered::new(),
        }
    }

    pub fn current_jobs(&self, user_id: i64) -> String {
        let mut r = String::from("Current Jobs:");
        for job in self.jobs.iter().filter(|job| job.user_id == user_id) {
            match &job.kind {
                JobKind::Notify(id) => {
                    r.push_str("\n");
                    r.push_str("Notify ");
                    r.push_str(id.as_str());
                }
                JobKind::Internal(_) => (),
            }
        }
        r
    }

    fn cancel_jobs(&self, user_id: i64) -> usize {
        let mut count = 0;
        for job in self
            .jobs
            .iter()
            .filter(|job| job.user_id == user_id && !job.kind.is_internal())
        {
            job.handle.abort();
            count += 1;
        }
        count
    }

    pub fn handle_action(&mut self, action: Action) {
        match action.kind {
            ActionKind::AddJob(kind) => {
                self.jobs
                    .push(Job::new(kind, action.user_id, action.cx, &self))
            }
            ActionKind::ListJobs => {
                let text = self.current_jobs(action.user_id);
                self.jobs
                    .push(Job::msg_user(action.user_id, action.cx, text))
            }
            ActionKind::CancelAll => {
                let count = self.cancel_jobs(action.user_id);
                let text = format!("Canceled {} Jobs", count);
                self.jobs
                    .push(Job::msg_user(action.user_id, action.cx, text))
            }
        }
    }
}

struct Job {
    kind: JobKind,
    user_id: i64,
    handle: JoinHandle<Result<Option<Action>, RequestError>>,
}

impl Job {
    pub fn new(
        kind: JobKind,
        user_id: i64,
        cx: UpdateWithCx<AutoSend<Bot>, Message>,
        state: &State,
    ) -> Self {
        let handle = match &kind {
            JobKind::Notify(id) => tokio::spawn(JobKind::notify(cx, id.clone())),
            JobKind::Internal(internal) => match internal {
                InternalJob::MsgUser(msg) => tokio::spawn(JobKind::msg_user(cx, msg.clone())),
            },
        };
        Self {
            kind,
            user_id,
            handle,
        }
    }

    pub fn msg_user(user_id: i64, cx: UpdateWithCx<AutoSend<Bot>, Message>, text: String) -> Self {
        let handle = tokio::spawn(JobKind::msg_user(cx, text.clone()));
        Self {
            kind: JobKind::Internal(InternalJob::MsgUser(text)),
            user_id,
            handle,
        }
    }
}

impl Future for Job {
    type Output = Result<Result<Option<Action>, RequestError>, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
        Pin::new(&mut self.handle).poll(cx)
    }
}

pub struct Action {
    pub kind: ActionKind,
    pub user_id: i64,
    pub cx: UpdateWithCx<AutoSend<Bot>, Message>,
}

impl Action {
    pub fn new<T: Into<ActionKind>>(
        kind: T,
        user_id: i64,
        cx: UpdateWithCx<AutoSend<Bot>, Message>,
    ) -> Self {
        Self {
            kind: kind.into(),
            user_id,
            cx,
        }
    }
}

pub enum ActionKind {
    AddJob(JobKind),
    ListJobs,
    CancelAll,
}

impl From<JobKind> for ActionKind {
    fn from(jk: JobKind) -> Self {
        Self::AddJob(jk)
    }
}

#[derive(Debug, Clone)]
pub enum JobKind {
    Notify(LessonID),
    Internal(InternalJob),
}

#[derive(Clone, Debug)]
pub enum InternalJob {
    MsgUser(String),
}

impl JobKind {
    fn is_internal(&self) -> bool {
        match self {
            Self::Internal(_) => true,
            _ => false,
        }
    }
    async fn notify(
        cx: UpdateWithCx<AutoSend<Bot>, Message>,
        id: LessonID,
    ) -> Result<Option<Action>, RequestError> {
        cx.answer("starting notif").await?;
        let client = reqwest::Client::new();
        loop {
            match lesson_data(&client, &id).await {
                Ok(data) => {

                }
                Err(err) => {
                    cx.answer(format!("got error shuting down this job: {}", err)).await?;
                    return Ok(None)
                }
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
            log::info!("notify");
            cx.answer(format!("notify {:?}", &id)).await?;
        }
    }

    async fn msg_user(
        cx: UpdateWithCx<AutoSend<Bot>, Message>,
        text: String,
    ) -> Result<Option<Action>, RequestError> {
        cx.answer(text).await?;
        Ok(None)
    }
}
