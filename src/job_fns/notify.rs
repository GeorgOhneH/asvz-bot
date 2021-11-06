use std::cmp::max;
use std::collections::HashMap;
use teloxide::{prelude::*, utils::command::BotCommand, RequestError};

use crate::asvz::lesson::lesson_data;
use crate::asvz::login::asvz_login;
use crate::cmd::{LessonID, Password, Username};
use crate::job_fns::ExistStatus;
use crate::utils::current_timestamp;
use crate::utils::reply;
use crate::utils::ret_on_err;
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

#[instrument(skip(cx))]
pub async fn notify(
    cx: &UpdateWithCx<AutoSend<Bot>, Message>,
    id: LessonID,
) -> Result<ExistStatus, RequestError> {
    trace!("new notify job");
    let client = reqwest::Client::new();

    let data = ret_on_err!(lesson_data(&client, &id).await);
    let current_ts = current_timestamp();

    let until_ts = ret_on_err!(data.enroll_until_timestamp());

    let from_ts = ret_on_err!(data.enroll_from_timestamp());

    if from_ts > current_ts {
        // We still need to wait to enroll
        let wait_time = max(from_ts - current_ts - 60, 0) as u64;
        reply!(cx, "I will remind you to enroll in {} seconds", wait_time).await?;
        tokio::time::sleep(Duration::from_secs(wait_time)).await;
        let current_time = current_timestamp();
        let msg = format!("enrolling starts in {} seconds", from_ts - current_time);
        return Ok(ExistStatus::success(msg));
    }

    for count in 0.. {
        if current_ts > until_ts {
            return Ok(ExistStatus::failure("You can no longer enroll"));
        }

        let fresh_data = ret_on_err!(lesson_data(&client, &id).await);
        let free_places = fresh_data.data.participants_max - fresh_data.data.participant_count;
        if free_places > 0 {
            let msg = format!("There are currently {} free places", free_places);
            return Ok(ExistStatus::Success(msg));
        }
        if count == 0 {
            reply!(
                cx,
                "It's already full. I will notify you, when something opens up"
            )
            .await?;
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
    unreachable!()
}
