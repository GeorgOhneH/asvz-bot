use teloxide::{prelude::*, utils::command::BotCommand, RequestError};

use crate::state::{Action, ActionKind, JobKind};
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
use tokio::task::{JoinError, JoinHandle};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{trace};

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
    #[command(description = "same as help.")]
    Start,
    #[command(description = "display this text.")]
    Help,
    #[command(description = "You get notified when a lesson starts or a place becomes available.")]
    Notify(LessonID),
    #[command(description = "You get enrolled when a lesson starts or a place becomes available.")]
    Enroll(LessonID),
    #[command(description = "login.", parse_with = "split")]
    Login {
        username: Username,
        password: Password,
    },
    #[command(description = "Show your current Jobs.")]
    ListJobs,
    #[command(description = "Cancel all Jobs.")]
    CancelAllJobs,
}

impl Command {
    async fn answer(
        self,
        cx: UpdateWithCx<AutoSend<Bot>, Message>,
        user_id: i64,
    ) -> Result<Option<Action>, RequestError> {
        match self {
            Command::Start => {
                cx.answer(Command::descriptions()).await?;
                Ok(None)
            }
            Command::Help => {
                cx.answer(Command::descriptions()).await?;
                Ok(None)
            }
            Command::Notify(id) => Ok(Some(Action::new(ActionKind::Notify(id), user_id, cx))),
            Command::Enroll(id) => Ok(Some(Action::new(ActionKind::Enroll(id), user_id, cx))),
            Command::Login { username, password } => {
                cx.delete_message().await?;
                Ok(Some(Action::new(
                    ActionKind::Login(username, password),
                    user_id,
                    cx,
                )))
            }
            Command::ListJobs => Ok(Some(Action::new(ActionKind::ListJobs, user_id, cx))),
            Command::CancelAllJobs => Ok(Some(Action::new(ActionKind::CancelAll, user_id, cx))),
        }
    }
}

async fn handle_cmd_err(
    cx: UpdateWithCx<AutoSend<Bot>, Message>,
    err: ParseError,
) -> Result<Option<Action>, RequestError> {
    match err {
        ParseError::UnknownCommand(_) => cx.answer("Unknown Command").await?,
        ParseError::WrongBotName(name) => panic!("Wrong bot name: {}", name),
        ParseError::IncorrectFormat(err) => {
            cx.answer(format!("Arguments are not correctly formatted: {}", err))
                .await?
        }
        ParseError::TooFewArguments {
            expected,
            found,
            message,
        } => {
            cx.answer(format!(
                "Expected {} arguments (got {}). msg: {}",
                expected, found, message
            ))
            .await?
        }
        ParseError::TooManyArguments {
            expected,
            found,
            message,
        } => {
            cx.answer(format!(
                "Expected {} arguments (got {}). msg: {}",
                expected, found, message
            ))
            .await?
        }
        ParseError::Custom(err) => cx.answer(format!("{}", err)).await?,
    };
    Ok(None)
}

fn extract_cmd_id(
    cx: &UpdateWithCx<AutoSend<Bot>, Message>,
) -> Option<Result<(Command, i64), ParseError>> {
    match &cx.update.kind {
        MessageKind::Common(msg_common) => match (&msg_common.media_kind, &msg_common.from) {
            (MediaKind::Text(txt), Some(user)) if !user.is_bot => {
                trace!("Got new telegram msg from {}: {}", user.id, &txt.text);
                match Command::parse(&txt.text, BOT_NAME.to_string()) {
                    Ok(cmd) => Some(Ok((cmd, user.id))),
                    Err(err) => Some(Err(err)),
                }
            }
            _ => None,
        },
        _ => None,
    }
}

async fn _handle_update(
    cx: UpdateWithCx<AutoSend<Bot>, Message>,
) -> Result<Option<Action>, RequestError> {
    match extract_cmd_id(&cx) {
        Some(Ok((cmd, id))) => cmd.answer(cx, id).await,
        Some(Err(err)) => handle_cmd_err(cx, err).await,
        None => Ok(None),
    }
}

pub async fn handle_update(
    update: Update,
    bot: AutoSend<Bot>,
) -> Result<Option<Action>, RequestError> {
    match update.kind {
        UpdateKind::Message(msg) => {
            _handle_update(UpdateWithCx {
                requester: bot,
                update: msg,
            })
            .await
        }
        _ => Ok(None),
    }
}
