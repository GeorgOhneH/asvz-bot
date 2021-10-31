#![allow(unused_imports)]
#![allow(dead_code)]
#![feature(let_else)]

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

    let mut bot_update = setup_listener(bot.clone()).await;
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

pub async fn setup_listener(
    bot: AutoSend<Bot>,
) -> impl update_listeners::UpdateListener<Infallible> {
    if true {
        update_listeners::polling_default(bot).await
    } else {
        webhook(bot).await
    }
}

pub async fn webhook(bot: AutoSend<Bot>) -> impl update_listeners::UpdateListener<Infallible> {
    let url = Url::parse("Your HTTPS ngrok URL here. Get it by `ngrok http 80`").unwrap();

    // You might want to specify a self-signed certificate via .certificate
    // method on SetWebhook.
    bot.set_webhook(url).await.expect("Cannot setup a webhook");

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    let server = warp::post()
        .and(warp::body::json())
        .map(move |json: serde_json::Value| {
            if let Ok(update) = Update::try_parse(&json) {
                tx.send(Ok(update))
                    .expect("Cannot send an incoming update from the webhook")
            }

            StatusCode::OK
        })
        .recover(handle_rejection);

    let (stop_token, stop_flag) = AsyncStopToken::new_pair();

    let addr = "127.0.0.1:80".parse::<SocketAddr>().unwrap();
    let server = warp::serve(server);
    let (_addr, fut) = server.bind_with_graceful_shutdown(addr, stop_flag);

    // You might want to use serve.key_path/serve.cert_path methods here to
    // setup a self-signed TLS certificate.

    tokio::spawn(fut);
    let stream = UnboundedReceiverStream::new(rx);

    fn streamf<S, T>(state: &mut (S, T)) -> &mut S {
        &mut state.0
    }

    StatefulListener::new(
        (stream, stop_token),
        streamf,
        |state: &mut (_, AsyncStopToken)| state.1.clone(),
    )
}
