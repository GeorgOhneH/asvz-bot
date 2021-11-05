pub mod user;
pub mod job;

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
use crate::state::user::{UserId, UserState, LoginCredentials};
use crate::action::{Action, ActionKind};
use crate::state::job::{JobKind, Job};


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

    #[instrument(skip(self))]
    pub fn handle_action(&mut self, action: Action) {
        trace!(
            "new action. user_state: {:?}",
            self.users.get(&action.user_id)
        );
        match action.kind {
            ActionKind::Notify(id) => self.jobs.push(Job::notify(action.user_id, action.cx, id)),
            ActionKind::Enroll(id) => {
                if let Some(UserState {
                                credentials: Some(cred),
                            }) = self.users.get(&action.user_id)
                {
                    self.jobs.push(Job::enroll(
                        action.user_id,
                        action.cx,
                        id,
                        cred.username.clone(),
                        cred.password.clone(),
                    ));
                } else {
                    let text = "You need to be logged in to directly enroll".to_string();
                    self.jobs
                        .push(Job::msg_user(action.user_id, action.cx, text))
                }
            }
            ActionKind::Login(username, password) => {
                let credentials = LoginCredentials::new(username, password);
                if let Some(user) = self.users.get_mut(&action.user_id) {
                    user.credentials = Some(credentials);
                    let text = "Updated credentials (I deleted your msg)".to_string();
                    self.jobs
                        .push(Job::msg_user(action.user_id, action.cx, text))
                } else {
                    let user = UserState::with_credentials(credentials);
                    self.users.insert(action.user_id, user);
                    let text = "Stored credentials (I deleted your msg)".to_string();
                    self.jobs
                        .push(Job::msg_user(action.user_id, action.cx, text))
                }
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

