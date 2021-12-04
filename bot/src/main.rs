#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(clippy::new_without_default)]

use std::sync::Arc;

use futures::stream::StreamExt;
use reqwest::Client;
use teloxide::dispatching::update_listeners;
use teloxide::dispatching::update_listeners::AsUpdateStream;
use teloxide::prelude::*;
use teloxide::types::UpdateKind;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

use asvz::lesson::search_data;
use asvz::lesson::LessonID;

use crate::state::State;

pub mod cmd;
pub mod job;
pub mod job_err;
pub mod job_fns;
pub mod job_update_cx;
pub mod state;
pub mod user;
pub mod utils;

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

    // let client = Client::new();
    // dbg!(search_data(&client, &LessonID("236310".to_string()), 1).await);

    info!("Starting Bot");

    let bot = Bot::from_env().auto_send();
    let mut state = State::new();

    let mut bot_update = update_listeners::polling_default(bot.clone()).await;
    let bot_stream = bot_update.as_stream();
    tokio::pin!(bot_stream);

    loop {
        tokio::select! {
            Some(update) = bot_stream.next() => {
                match update {
                    Ok(update) => {
                        if let UpdateKind::Message(msg) = update.kind {
                            let cx = Arc::new(UpdateWithCx {
                                requester: bot.clone(),
                                update: msg,
                            });
                            state.handle_update(cx);
                        }
                    }
                    Err(err) => state.handle_req_err(err),
                }
            },
            Some(handle_result) = state.next() => {
                match handle_result {
                    Ok(result) => {
                        if let Err(err) = result {
                            state.handle_job_err(err)
                        }
                    },
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
