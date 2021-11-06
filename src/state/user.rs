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
use tracing::{debug, instrument, trace};
use tracing::field::{Field, Visit};

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct UserId(pub i64);

#[derive(Debug)]
pub struct UserState {
    pub credentials: Option<LoginCredentials>,
    pub settings: Settings,
}

impl UserState {
    pub fn new() -> Self {
        Self {
            credentials: None,
            settings: Settings::new(),
        }
    }

    pub fn with_credentials(credentials: LoginCredentials) -> Self {
        Self {
            credentials: Some(credentials),
            settings: Settings::new(),
        }
    }
}


#[derive(Debug)]
pub struct Settings {
    pub url_action: UrlAction,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            url_action: UrlAction::Default,
        }
    }
}


#[derive(Debug, Copy, Clone)]
pub enum UrlAction {
    Default,
    Notify,
    Enroll,
}

impl FromStr for UrlAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(UrlAction::Default),
            "1" => Ok(UrlAction::Notify),
            "2" => Ok(UrlAction::Enroll),
            _ => Err("Use one of these: 0: Default, 1: Notify, 2: Enroll".into()),
        }
    }
}

#[derive(Debug)]
pub struct LoginCredentials {
    pub username: Username,
    pub password: Password,
}

impl LoginCredentials {
    pub fn new(username: Username, password: Password) -> Self {
        Self { username, password }
    }
    pub fn update(&mut self, username: Username, password: Password) {
        self.username = username;
        self.password = password;
    }
}
