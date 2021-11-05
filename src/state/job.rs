use std::cmp::max;
use std::collections::HashMap;
use teloxide::{prelude::*, utils::command::BotCommand, RequestError};

use crate::asvz::lesson::lesson_data;
use crate::asvz::login::asvz_login;
use crate::cmd::{LessonID, Password, Username};
use chrono::DateTime;
use derivative::Derivative;
use futures::stream::FuturesUnordered;
use futures::stream::{self, StreamExt};
use futures::{FutureExt, Stream, TryFutureExt};
use reqwest::{Client, StatusCode};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::task::Context;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use teloxide::adaptors::AutoSend;
use teloxide::dispatching::update_listeners;
use teloxide::dispatching::update_listeners::AsUpdateStream;
use teloxide::types::{MediaKind, MessageKind, Update, UpdateKind, User};
use teloxide::utils::command::ParseError;
use tokio::task::{JoinError, JoinHandle};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, instrument, trace};
use crate::state::user::UserId;
use crate::job_fns;


#[derive(Debug)]
pub struct Job {
    pub kind: JobKind,
    pub user_id: UserId,
    pub handle: JoinHandle<Result<(), RequestError>>,
}

impl Job {
    pub fn notify(user_id: UserId, cx: UpdateWithCx<AutoSend<Bot>, Message>, id: LessonID) -> Self {
        let handle = tokio::spawn(job_fns::notify(cx, id.clone()));
        Self {
            kind: JobKind::Notify(id),
            user_id,
            handle,
        }
    }
    pub fn enroll(
        user_id: UserId,
        cx: UpdateWithCx<AutoSend<Bot>, Message>,
        id: LessonID,
        username: Username,
        password: Password,
    ) -> Self {
        let handle = tokio::spawn(job_fns::enroll(cx, id.clone(), username, password));
        Self {
            kind: JobKind::Enroll(id),
            user_id,
            handle,
        }
    }

    pub fn msg_user(
        user_id: UserId,
        cx: UpdateWithCx<AutoSend<Bot>, Message>,
        text: String,
    ) -> Self {
        let handle = tokio::spawn(job_fns::msg_user(cx, text.clone()));
        Self {
            kind: JobKind::Internal(InternalJob::MsgUser(text)),
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
    Enroll(LessonID),
    Internal(InternalJob),
}

impl JobKind {
    pub fn is_internal(&self) -> bool {
        match self {
            Self::Internal(_) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub enum InternalJob {
    MsgUser(String),
}
