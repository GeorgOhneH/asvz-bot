#![allow(unused_imports)]
#![allow(dead_code)]

mod asvz;
mod cmd;
mod state;

use std::collections::HashMap;
use std::convert::Infallible;
use teloxide::{prelude::*, utils::command::BotCommand, RequestError};

use crate::asvz::login::asvz_login;
use crate::state::State;
use cmd::handle_update;
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
        .add_directive("trace".parse().unwrap())
        .add_directive("hyper=info".parse().unwrap())
        .add_directive("my_crate=trace".parse().unwrap());
    let subscriber = tracing_subscriber::fmt().with_env_filter(filter).finish();
    tracing::subscriber::set_global_default(subscriber).expect("Unable to make logging");

    info!("Starting Bot");

    let bot = Bot::from_env().auto_send();
    let mut state = State::new();

    let mut bot_update = update_listeners::polling_default(bot.clone()).await;
    let bot_stream = bot_update.as_stream();
    tokio::pin!(bot_stream);

    let mut update_handles = FuturesUnordered::new();

    let (action_tx, mut action_rx) = tokio::sync::mpsc::channel(512);


    loop {
        tokio::select! {
            Some(update) = bot_stream.next() => {
                update_handles.push(handle_update(update.unwrap(), bot.clone(), action_tx.clone()));
            },
            Some(_) = update_handles.next() => (),
            Some(action) = action_rx.recv() => {
                state.handle_action(action)
            }
            Some(result) = state.next() => {
                match result {
                    Ok(action) => {
                        if let Some(ac) = action.unwrap() {
                            state.handle_action(ac)
                        }
                    }
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

