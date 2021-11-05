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

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Action {
    pub kind: ActionKind,
    pub user_id: UserId,
    #[derivative(Debug = "ignore")]
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
            user_id: UserId(user_id),
            cx,
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub enum ActionKind {
    Notify(LessonID),
    Enroll(LessonID),
    Login(Username, #[derivative(Debug = "ignore")] Password),
    ListJobs,
    CancelAll,
}
