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

macro_rules! return_on_err {
    ($expression:expr, $cx:ident) => {
        match $expression {
            Ok(val) => val,
            Err(err) => {
                $cx.answer(format!("I got an unexpected error: {}", err))
                    .await?;
                return Ok(None);
            }
        }
    };
    ($expression:expr, $cx:ident, $string:expr) => {
        match $expression {
            Ok(val) => val,
            Err(err) => {
                $cx.answer(format!("{}: {}", $string, err)).await?;
                return Ok(None);
            }
        }
    };
}

macro_rules! reply {
    ($cx:ident, $($arg:tt)*) => {
        $cx.answer(format!($($arg)*))
    };
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct UserId(i64);

struct CountLoop {
    count: usize,
}

impl CountLoop {
    pub fn new() -> Self {
        Self { count: 0 }
    }
}

impl Iterator for CountLoop {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let c_count = self.count;
        self.count += 1;
        Some(c_count)
    }
}

#[derive(Debug)]
pub struct State {
    jobs: FuturesUnordered<Job>,
    users: HashMap<UserId, UserState>,
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

#[derive(Debug)]
struct UserState {
    credentials: Option<LoginCredentials>,
}

impl UserState {
    pub fn new() -> Self {
        Self { credentials: None }
    }

    pub fn with_credentials(credentials: LoginCredentials) -> Self {
        Self {
            credentials: Some(credentials),
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
struct LoginCredentials {
    username: Username,
    #[derivative(Debug = "ignore")]
    password: Password,
}

impl LoginCredentials {
    pub fn new(username: Username, password: Password) -> Self {
        Self { username, password }
    }
}

struct Job {
    kind: JobKind,
    user_id: UserId,
    handle: JoinHandle<Result<Option<Action>, RequestError>>,
}

impl Job {
    pub fn notify(user_id: UserId, cx: UpdateWithCx<AutoSend<Bot>, Message>, id: LessonID) -> Self {
        let handle = tokio::spawn(JobKind::notify(cx, id.clone()));
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
        let handle = tokio::spawn(JobKind::enroll(cx, id.clone(), username, password));
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

#[derive(Debug, Clone)]
pub enum JobKind {
    Notify(LessonID),
    Enroll(LessonID),
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

    fn current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System Time before UNIX EPOCH")
            .as_secs() as i64
    }

    #[instrument(skip(cx))]
    async fn notify(
        cx: UpdateWithCx<AutoSend<Bot>, Message>,
        id: LessonID,
    ) -> Result<Option<Action>, RequestError> {
        let client = reqwest::Client::new();
        for count in CountLoop::new() {
            let data = return_on_err!(lesson_data(&client, &id).await, cx);
            let current_ts = Self::current_timestamp();

            let until_ts = return_on_err!(data.enroll_until_timestamp(), cx);

            if current_ts > until_ts {
                reply!(cx, "You can no longer enroll\nStopping this Job").await?;
                return Ok(None);
            }

            let from_ts = return_on_err!(data.enroll_from_timestamp(), cx);

            if from_ts > current_ts {
                // We still need to wait to enroll
                let wait_time = max(from_ts - current_ts - 60, 0) as u64;
                reply!(cx, "I will remind you to enroll in {} seconds", wait_time).await?;
                tokio::time::sleep(Duration::from_secs(wait_time)).await;
                let current_time = Self::current_timestamp();
                reply!(cx, "enrolling starts in {} seconds", from_ts - current_time).await?;
                return Ok(None);
            } else {
                let free_places = data.data.participants_max - data.data.participant_count;
                if free_places > 0 {
                    reply!(
                        cx,
                        "There are currently {} free places\nStopping this job",
                        free_places
                    )
                    .await?;
                    return Ok(None);
                } else {
                    if count == 0 {
                        reply!(
                            cx,
                            "It's already full. I will notify you, when something opens up"
                        )
                        .await?;
                    }
                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            }
        }
        unreachable!()
    }

    #[instrument(skip(cx, password))]
    async fn enroll(
        cx: UpdateWithCx<AutoSend<Bot>, Message>,
        id: LessonID,
        username: Username,
        password: Password,
    ) -> Result<Option<Action>, RequestError> {
        let client = Client::builder().cookie_store(true).build().unwrap();
        let enroll_url = format!(
            "https://schalter.asvz.ch/tn-api/api/Lessons/{}/enroll",
            id.as_str()
        );
        let mut token = return_on_err!(
            asvz_login(&client, username.as_str(), password.as_str_dangerous()).await,
            cx,
            "Unable to log in"
        );

        let data = return_on_err!(lesson_data(&client, &id).await, cx);
        let until_ts = return_on_err!(data.enroll_until_timestamp(), cx);
        let from_ts = return_on_err!(data.enroll_from_timestamp(), cx);

        for count in CountLoop::new() {
            let current_ts = Self::current_timestamp();

            if current_ts > until_ts {
                reply!(cx, "You can no longer enroll\nStopping this Job").await?;
                return Ok(None);
            }

            if from_ts > current_ts {
                // We still need to wait to enroll
                let wait_time = max(from_ts - current_ts - 30, 0) as u64;
                reply!(cx, "I will enroll you in {} seconds", from_ts - current_ts).await?;
                trace!("waiting for {} seconds before we can enroll", wait_time);
                tokio::time::sleep(Duration::from_secs(wait_time)).await;

                token = return_on_err!(
                    asvz_login(&client, username.as_str(), password.as_str_dangerous()).await,
                    cx,
                    "Unable to log in"
                );
                trace!("refreshed token");

                let current_ts = Self::current_timestamp();
                let wait_time = max(from_ts - current_ts - 2, 0) as u64;
                trace!("waiting again {} seconds", wait_time);
                tokio::time::sleep(Duration::from_secs(wait_time)).await;

                while Self::current_timestamp() < from_ts + 5 {
                    trace!("starting to enroll");
                    let enroll_response = return_on_err!(
                        client
                            .post(enroll_url.clone())
                            .bearer_auth(&token)
                            .json(&())
                            .send()
                            .await,
                        cx
                    );
                    trace!(
                        "enroll response with status code {}",
                        enroll_response.status()
                    );

                    match enroll_response.status() {
                        StatusCode::CREATED => {
                            reply!(cx, "I successfully enrolled you\nStopping Job").await?;
                            return Ok(None);
                        }
                        StatusCode::UNPROCESSABLE_ENTITY => (),
                        code => {
                            reply!(cx, "Got unexpected status code: {}\nStopping Job", code)
                                .await?;
                            return Ok(None);
                        }
                    }
                }
            } else {
                let enroll_response = return_on_err!(
                    client
                        .post(enroll_url.clone())
                        .bearer_auth(&token)
                        .json(&())
                        .send()
                        .await,
                    cx
                );

                trace!(
                    "Tried to enroll with status code: {}",
                    enroll_response.status()
                );

                match enroll_response.status() {
                    StatusCode::CREATED => {
                        cx.answer("Successfully enrolled you\nClosing Job").await?;
                        return Ok(None);
                    }
                    StatusCode::UNAUTHORIZED => {
                        token = return_on_err!(
                            asvz_login(&client, username.as_str(), password.as_str_dangerous())
                                .await,
                            cx,
                            "Unable to log in"
                        );
                    }
                    StatusCode::UNPROCESSABLE_ENTITY => (),
                    code => {
                        reply!(cx, "Got unexpected status code: {}\nStopping Job", code).await?;
                        return Ok(None);
                    }
                }

                if count == 0 {
                    reply!(
                        cx,
                        "It's already full. I will try to enroll you, when something opens up"
                    )
                    .await?;
                }

                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
        unreachable!()
    }

    #[instrument(skip(cx), level = "trace")]
    async fn msg_user(
        cx: UpdateWithCx<AutoSend<Bot>, Message>,
        text: String,
    ) -> Result<Option<Action>, RequestError> {
        cx.answer(text).await?;
        Ok(None)
    }
}
