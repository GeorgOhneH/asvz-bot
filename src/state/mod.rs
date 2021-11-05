pub mod job;
pub mod user;

use std::cmp::max;
use std::collections::HashMap;
use teloxide::{prelude::*, utils::command::BotCommand, RequestError};

use crate::asvz::lesson::lesson_data;
use crate::asvz::login::asvz_login;
use crate::cmd::{Command, LessonID, Password, Username};
use crate::state::job::{Job, JobKind};
use crate::state::user::{LoginCredentials, UserId, UserState};
use crate::BOT_NAME;
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

static START_MSG: &str = r"Hello Welcome to the asvz bot.
This Bot allows you to get notified/enrolled when a lesson starts or a place open up.
See /help for all awailable commands";

#[derive(Debug)]
pub struct State {
    jobs: FuturesUnordered<Job>,
    users: HashMap<UserId, UserState>,
}

impl Stream for State {
    type Item = Result<Result<(), RequestError>, JoinError>;

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
            users: HashMap::new(),
        }
    }

    pub fn current_jobs(&self, user_id: UserId) -> String {
        let mut r = String::from("Current Jobs:");
        for job in self.jobs.iter().filter(|job| job.user_id == user_id) {
            match &job.kind {
                JobKind::Notify(id) => {
                    r.push_str("\nNotify ");
                    r.push_str(id.as_str());
                }
                JobKind::Enroll(id) => {
                    r.push_str("\nEnroll ");
                    r.push_str(id.as_str());
                }
                JobKind::Internal(_) => (),
            }
        }
        r
    }

    fn cancel_jobs(&self, user_id: UserId) -> usize {
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

    pub fn handle_update(&mut self, cx: UpdateWithCx<AutoSend<Bot>, Message>) {
        if let Some((msg, user_id)) = extract_id_text(&cx.update) {
            let job = match Command::parse(msg, BOT_NAME) {
                Ok(cmd) => self.handle_cmd(cmd, user_id, cx),
                Err(err) => {

                    let text = cmd_err_to_str(err);
                    Job::msg_user(user_id, cx, text)
                }
            };
            self.jobs.push(job);
        }
    }

    pub fn handle_cmd(
        &mut self,
        cmd: Command,
        user_id: UserId,
        cx: UpdateWithCx<AutoSend<Bot>, Message>,
    ) -> Job {
        let user_state = self
            .users
            .entry(user_id)
            .or_insert_with(|| UserState::new());
        match cmd {
            Command::Start => Job::msg_user(user_id, cx, START_MSG),
            Command::Help => Job::msg_user(user_id, cx, Command::descriptions()),
            Command::Notify { lesson_id } => Job::notify(user_id, cx, lesson_id),
            Command::Enroll { lesson_id } => {
                if let Some(cred) = &user_state.credentials {
                    Job::enroll(
                        user_id,
                        cx,
                        lesson_id,
                        cred.username.clone(),
                        cred.password.clone(),
                    )
                } else {
                    let text = "You need to be logged in to directly enroll\
                    \nSee /help for more info";
                    Job::msg_user(user_id, cx, text)
                }
            }
            Command::Login { username, password } => {
                let msg = if let Some(cred) = &mut user_state.credentials {
                    cred.update(username, password);
                    "Updated credentials"
                } else {
                    user_state.credentials = Some(LoginCredentials::new(username, password));
                    "Stored credentials"
                };
                Job::msg_user(user_id, cx, msg)
            }
            Command::Jobs => Job::msg_user(user_id, cx, self.current_jobs(user_id)),
            Command::CancelAll => {
                let count = self.cancel_jobs(user_id);
                let text = format!("Canceled {} Jobs", count);
                Job::msg_user(user_id, cx, text)
            }
        }
    }
}

fn cmd_err_to_str(err: ParseError) -> String {
    match err {
        ParseError::UnknownCommand(_) => "Unknown Command".into(),
        ParseError::WrongBotName(name) => panic!("Wrong bot name: {}", name),
        ParseError::IncorrectFormat(err) => {
            format!("Arguments are not correctly formatted: {}", err)
        }
        ParseError::TooFewArguments {
            expected,
            found,
            message,
        } => {
            format!(
                "Expected {} arguments (got {}). msg: {}",
                expected, found, message
            )
        }
        ParseError::TooManyArguments {
            expected,
            found,
            message,
        } => {
            format!(
                "Expected {} arguments (got {}). msg: {}",
                expected, found, message
            )
        }
        ParseError::Custom(err) => format!("{}", err),
    }
}

fn extract_id_text(msg: &Message) -> Option<(&str, UserId)> {
    match &msg.kind {
        MessageKind::Common(msg_common) => match (&msg_common.media_kind, &msg_common.from) {
            (MediaKind::Text(txt), Some(user)) if !user.is_bot => {
                Some((&txt.text, UserId(user.id)))
            }
            _ => None,
        },
        _ => None,
    }
}
