use teloxide::{prelude::*, utils::command::BotCommand, RequestError};

use crate::state::user::UserId;
use crate::BOT_NAME;
use futures::stream::FuturesUnordered;
use futures::stream::{self, StreamExt};
use futures::{FutureExt, TryFutureExt};
use std::error::Error;
use std::fmt;
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
use tokio::sync::mpsc::Sender;
use tokio::task::{JoinError, JoinHandle};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::trace;

#[derive(Debug, Clone)]
pub struct LessonID(String);

#[derive(Clone, Debug)]
pub struct Username(String);

impl Username {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl FromStr for Username {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Err("You need to supply a non empty username".to_string())
        } else {
            Ok(Self(s.to_string()))
        }
    }
}

#[derive(Clone)]
pub struct Password(String);

impl Password {
    pub fn as_str_dangerous(&self) -> &str {
        self.0.as_str()
    }
}

impl FromStr for Password {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Err("You need to supply a non empty password".to_string())
        } else {
            Ok(Self(s.to_string()))
        }
    }
}

impl LessonID {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl FromStr for LessonID {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Err("You need to supply a non empty id".to_string())
        } else {
            Ok(Self(s.to_string()))
        }
    }
}

#[derive(BotCommand)]
#[command(rename = "lowercase", description = "These commands are supported:")]
pub enum Command {
    #[command(description = "Show the Start Message")]
    Start,
    #[command(description = "Displays this text")]
    Help,
    #[command(
        description = "You get notified when a lesson starts or a place becomes available",
        parse_with = "split"
    )]
    Notify { lesson_id: LessonID },
    #[command(
        description = "You get enrolled when a lesson starts or a place becomes available",
        parse_with = "split"
    )]
    Enroll { lesson_id: LessonID },
    #[command(
        description = "Stores your username and password, so I can directly enroll you. \
    Note your password is never stored on persistent memory, \
    but your are still giving a random person on the internet your password",
        parse_with = "split"
    )]
    Login {
        username: Username,
        password: Password,
    },
    #[command(description = "Show your current Jobs.")]
    Jobs,
    #[command(description = "Cancel all Jobs.")]
    CancelAll,
}
