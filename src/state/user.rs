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
            settings: Settings {},
        }
    }

    pub fn with_credentials(credentials: LoginCredentials) -> Self {
        Self {
            credentials: Some(credentials),
            settings: Settings {},
        }
    }
}

#[derive(Debug)]
pub struct Settings {}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct LoginCredentials {
    pub username: Username,
    #[derivative(Debug = "ignore")]
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
