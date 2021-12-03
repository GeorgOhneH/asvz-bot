use std::collections::HashMap;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::task::Context;
use std::thread::sleep;
use std::time::Duration;

use futures::stream::FuturesUnordered;
use futures::Stream;
use lazy_static::lazy_static;
use regex::Regex;
use teloxide::adaptors::AutoSend;
use teloxide::types::{MediaKind, MessageKind};
use teloxide::utils::command::ParseError;
use teloxide::{prelude::*, utils::command::BotCommand, RequestError};
use tokio::task::JoinError;
use tracing::{error, instrument, trace};

use asvz::lesson::LessonID;

use crate::cmd::Command;
use crate::job::{InternalJob, Job, JobKind};
use crate::job_err::JobError;
use crate::user::{LoginCredentials, UrlAction, UserId, UserState};
use crate::BOT_NAME;

static START_MSG: &str = r"Welcome to the ASVZ telegram bot.
This bot allows you to get notified/enroll when a lesson starts or as soon as a spot opens up.
See /help for all available commands.
The source code is available online: (https://github.com/GeorgOhneH/asvz-bot)";

lazy_static! {
    static ref LESSON_URL_RE: Regex =
        Regex::new("https://schalter.asvz.ch/tn/lessons/([0-9]+)").unwrap();
}

#[derive(Debug)]
pub struct State {
    jobs: FuturesUnordered<Job>,
    users: HashMap<UserId, UserState>,
}

impl Stream for State {
    type Item = Result<Result<(), JobError>, JoinError>;

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
                JobKind::NotifyWeekly(id) => {
                    r.push_str("\nNotifyWeekly ");
                    r.push_str(id.as_str());
                }
                JobKind::Enroll(id, _, _) => {
                    r.push_str("\nEnroll ");
                    r.push_str(id.as_str());
                }
                JobKind::EnrollWeekly(id, _, _) => {
                    r.push_str("\nEnrollWeekly ");
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

    pub fn handle_update(&mut self, cx: Arc<UpdateWithCx<AutoSend<Bot>, Message>>) {
        if let Some((msg, user_id)) = extract_id_text(&cx.update) {
            let job = match Command::parse(msg, BOT_NAME) {
                Ok(cmd) => self.handle_cmd(cmd, user_id, cx),
                Err(err) => {
                    if let Some(caps) = LESSON_URL_RE.captures(msg) {
                        let lesson_id = LessonID::from_str(&caps[1]).expect("Captures non number");
                        self.handle_url(lesson_id, user_id, cx)
                    } else {
                        self.handle_cmd_err(err, user_id, cx)
                    }
                }
            };
            self.jobs.push(job);
        }
    }

    #[instrument(skip(self))]
    pub fn handle_err(&mut self, err: JobError) {
        error!("Got JobError");
        let JobError {
            source,
            user_id,
            job_kind,
            cx,
            retry_count,
        } = err;
        match source {
            RequestError::RetryAfter(wait) => sleep(Duration::from_secs(wait as u64 + 5)),
            _ => (),
        };
        let job = Job::builder(job_kind, user_id, cx)
            .pre_msg("An unexpected error occurred. Restarting your Job")
            .retry_count(retry_count + 1)
            .build();
        self.jobs.push(job)
    }

    #[instrument(skip(self, cx), fields(user_state = ?self.users.get(&user_id)))]
    pub fn handle_cmd(
        &mut self,
        cmd: Command,
        user_id: UserId,
        cx: Arc<UpdateWithCx<AutoSend<Bot>, Message>>,
    ) -> Job {
        trace!("new cmd");
        let user_state = self.users.entry(user_id).or_insert_with(UserState::new);
        let job_kind = match cmd {
            Command::Start => InternalJob::MsgUser(START_MSG.to_string()).into(),
            Command::Help => InternalJob::MsgUser(Command::descriptions()).into(),
            Command::Notify { lesson_id } => JobKind::Notify(lesson_id),
            Command::NotifyWeekly { lesson_id } => JobKind::NotifyWeekly(lesson_id),
            Command::Enroll { lesson_id } => {
                if let Some(cred) = &user_state.credentials {
                    JobKind::Enroll(lesson_id, cred.username.clone(), cred.password.clone())
                } else {
                    let text = "You need to be logged in to directly enroll\
                    \nSee /help for more info.";
                    InternalJob::MsgUser(text.to_string()).into()
                }
            }
            Command::EnrollWeekly { lesson_id } => {
                if let Some(cred) = &user_state.credentials {
                    JobKind::EnrollWeekly(lesson_id, cred.username.clone(), cred.password.clone())
                } else {
                    let text = "You need to be logged in to directly enroll\
                    \nSee /help for more info.";
                    InternalJob::MsgUser(text.to_string()).into()
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
                InternalJob::DeleteMsgUser(msg.to_string()).into()
            }
            Command::Logout => {
                let msg = if user_state.credentials.is_some() {
                    "Deleted your credentials"
                } else {
                    "You have no credentials stored"
                };
                user_state.credentials = None;
                InternalJob::MsgUser(msg.to_string()).into()
            }
            Command::UrlAction { url_action } => {
                InternalJob::MsgUser(format!("Changed your url_action to {:?}.", url_action)).into()
            }
            Command::Jobs => InternalJob::MsgUser(self.current_jobs(user_id)).into(),
            Command::CancelAll => {
                let count = self.cancel_jobs(user_id);
                let text = format!("Canceled {} Jobs.", count);
                InternalJob::MsgUser(text).into()
            }
        };

        Job::new(job_kind, user_id, cx)
    }

    #[instrument(skip(self, cx), fields(user_state = ?self.users.get(&user_id)))]
    fn handle_cmd_err(
        &mut self,
        err: ParseError,
        user_id: UserId,
        cx: Arc<UpdateWithCx<AutoSend<Bot>, Message>>,
    ) -> Job {
        trace!("new cmd err");
        let msg = match err {
            ParseError::UnknownCommand(_) => {
                "Unknown Command. See /help for available commands.".into()
            }
            ParseError::WrongBotName(name) => panic!("Wrong bot name: {}", name),
            ParseError::IncorrectFormat(err) => {
                format!("Arguments are not formatted correctly: {}!", err)
            }
            ParseError::TooFewArguments {
                expected,
                found,
                message: _,
            }
            | ParseError::TooManyArguments {
                expected,
                found,
                message: _,
            } => {
                format!(
                    "Expected {} arguments but got {}. See /help for more info.",
                    expected, found
                )
            }
            ParseError::Custom(err) => format!("{}. See /help for more info.", err),
        };

        let kind = InternalJob::MsgUser(msg).into();
        Job::new(kind, user_id, cx)
    }

    #[instrument(skip(self, cx), fields(user_state = ?self.users.get(&user_id)))]
    pub fn handle_url(
        &mut self,
        lesson_id: LessonID,
        user_id: UserId,
        cx: Arc<UpdateWithCx<AutoSend<Bot>, Message>>,
    ) -> Job {
        trace!("new lesson url");
        let user_state = self.users.entry(user_id).or_insert_with(UserState::new);

        match (user_state.settings.url_action, &user_state.credentials) {
            (UrlAction::Default | UrlAction::Enroll, Some(cred)) => {
                let kind = JobKind::Enroll(lesson_id, cred.username.clone(), cred.password.clone());
                let msg = "Found lesson url. Starting an enrollment job. \
                If you wanted to get notified you can change \
                the default behavior. See /help.";
                Job::builder(kind, user_id, cx).pre_msg(msg).build()
            }
            (UrlAction::Default | UrlAction::Notify, None) | (UrlAction::Notify, Some(_)) => {
                let kind = JobKind::Notify(lesson_id);
                let msg = "Found lesson url. Starting a notification job. \
                    If you wanted to enroll you can change \
                    the default behavior. See /help.";
                Job::builder(kind, user_id, cx).pre_msg(msg).build()
            }
            (UrlAction::Enroll, None) => {
                let msg =
                    "I can't enroll you without you being logged in. See /help for more info.";
                let kind = InternalJob::MsgUser(msg.to_string());
                Job::builder(kind.into(), user_id, cx).pre_msg(msg).build()
            }
        }
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
