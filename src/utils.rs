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

macro_rules! ret_on_err {
    ($expression:expr, $cx:ident) => {
        match $expression {
            Ok(val) => val,
            Err(err) => {
                $cx.answer(format!("I got an unexpected error: {}", err))
                    .await?;
                return Ok(());
            }
        }
    };
    ($expression:expr, $cx:ident, $string:expr) => {
        match $expression {
            Ok(val) => val,
            Err(err) => {
                $cx.answer(format!("{}: {}", $string, err)).await?;
                return Ok(());
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
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System Time before UNIX EPOCH")
        .as_secs() as i64
}

pub struct CountLoop {
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
