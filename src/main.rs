#![allow(unused_imports)]
#![allow(dead_code)]

pub mod asvz;
pub mod cmd;
pub mod job_fns;
pub mod state;
pub mod utils;

use std::collections::HashMap;
use std::convert::Infallible;
use teloxide::{prelude::*, utils::command::BotCommand, RequestError};

use crate::asvz::login::asvz_login;
use crate::state::State;
use futures::stream::FuturesUnordered;
use futures::stream::{self, StreamExt};
use futures::{FutureExt, TryFutureExt};
use regex::Regex;
use reqwest::{Client, StatusCode};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::task::Context;
use std::time::Duration;
use teloxide::adaptors::AutoSend;
use teloxide::dispatching::stop_token::AsyncStopToken;
use teloxide::dispatching::update_listeners;
use teloxide::dispatching::update_listeners::{AsUpdateStream, StatefulListener};
use teloxide::types::{MediaKind, MessageKind, Update, UpdateKind, User};
use teloxide::utils::command::ParseError;
use tokio::task::{JoinError, JoinHandle};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::log::LevelFilter;
use tracing::{info, Level};
use tracing_subscriber::{EnvFilter, Layer};
use url::Url;
use warp::Filter;

static BOT_NAME: &str = "asvz_bot";

#[tokio::main]
async fn main() {
    run().await;
}

async fn run() {
    let filter = EnvFilter::from_default_env()
        .add_directive(Level::TRACE.into())
        .add_directive("my_crate=trace".parse().unwrap())
        .add_directive("hyper=info".parse().unwrap());
    let subscriber = tracing_subscriber::fmt().with_env_filter(filter).finish();
    tracing::subscriber::set_global_default(subscriber).expect("Unable to make logging");

    info!("Starting Bot");

    let bot = Bot::from_env().auto_send();
    let mut state = State::new();

    let mut bot_update = update_listeners::polling_default(bot.clone()).await;
    let bot_stream = bot_update.as_stream();
    tokio::pin!(bot_stream);

    loop {
        tokio::select! {
            Some(update) = bot_stream.next() => {
                if let UpdateKind::Message(msg) = update.unwrap().kind {
                    let cx = UpdateWithCx {
                        requester: bot.clone(),
                        update: msg,
                    };
                    state.handle_update(cx);
                }
            },
            Some(result) = state.next() => {
                match result {
                    Ok(_) => (),
                    Err(err) => {
                        if let Ok(reason) = err.try_into_panic() {
                            std::panic::resume_unwind(reason);
                        }
                    }
                }
            },
            else => break,
        }
    }
}
