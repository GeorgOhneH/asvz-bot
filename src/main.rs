#![allow(unused_imports)]
#![allow(dead_code)]
#![feature(let_else)]

mod cmd;
mod state;
mod asvz;

use teloxide::{prelude::*, utils::command::BotCommand, RequestError};

use crate::state::State;
use cmd::handle_update;
use futures::stream::FuturesUnordered;
use futures::stream::{self, StreamExt};
use futures::{FutureExt, TryFutureExt};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::task::Context;
use std::time::Duration;
use teloxide::adaptors::AutoSend;
use teloxide::dispatching::update_listeners;
use teloxide::dispatching::update_listeners::AsUpdateStream;
use teloxide::types::{MediaKind, MessageKind, Update, UpdateKind, User};
use teloxide::utils::command::ParseError;
use tokio::task::{JoinError, JoinHandle};
use tokio_stream::wrappers::UnboundedReceiverStream;

static BOT_NAME: &str = "asvz_bot";

#[tokio::main]
async fn main() {
    run().await;
}

async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting simple_commands_bot...");

    let bot = Bot::from_env().auto_send();
    let mut state = State::new();

    let mut bot_update = update_listeners::polling_default(bot.clone()).await;
    let bot_stream = bot_update.as_stream();
    tokio::pin!(bot_stream);

    let mut update_handles = FuturesUnordered::new();

    loop {
        tokio::select! {
            Some(update) = bot_stream.next() => {
                update_handles.push(handle_update(update.unwrap(), bot.clone()));
            },
            Some(action) = update_handles.next() => {
                if let Some(ac) = action.unwrap() {
                    state.handle_action(ac)
                }
            },
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
