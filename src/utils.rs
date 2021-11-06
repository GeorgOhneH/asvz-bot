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

macro_rules! ret_on_err {
    ($expression:expr) => {
        match $expression {
            Ok(val) => val,
            Err(err) => {
                let msg = format!("I got an unexpected error: {}", err);
                return Ok(ExistStatus::failure(msg));
            }
        }
    };
    ($expression:expr, $string:expr) => {
        match $expression {
            Ok(val) => val,
            Err(err) => {
                let msg = format!("{}: {}", $string, err);
                return Ok(ExistStatus::failure(msg));
            }
        }
    };
}
pub(crate) use ret_on_err;

macro_rules! reply {
    ($cx:ident, $($arg:tt)*) => {
        $cx.answer(format!($($arg)*))
    };
}
pub(crate) use reply;

pub fn current_timestamp() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System Time before UNIX EPOCH")
            .as_secs(),
    )
    .expect("u64 to big")
}
