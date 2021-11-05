use std::cmp::max;
use std::collections::HashMap;
use teloxide::{prelude::*, utils::command::BotCommand, RequestError};

use crate::asvz::lesson::lesson_data;
use crate::asvz::login::asvz_login;
use crate::cmd::{LessonID, Password, Username};
use crate::utils::ret_on_err;
use crate::utils::{current_timestamp, reply, CountLoop};
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

#[instrument(skip(cx, password))]
pub async fn enroll(
    cx: UpdateWithCx<AutoSend<Bot>, Message>,
    id: LessonID,
    username: Username,
    password: Password,
) -> Result<(), RequestError> {
    let client = Client::builder().cookie_store(true).build().unwrap();
    let enroll_url = format!(
        "https://schalter.asvz.ch/tn-api/api/Lessons/{}/enroll",
        id.as_str()
    );
    let mut token = ret_on_err!(
        asvz_login(&client, username.as_str(), password.as_str_dangerous()).await,
        cx,
        "Unable to log in"
    );

    let data = ret_on_err!(lesson_data(&client, &id).await, cx);
    let until_ts = ret_on_err!(data.enroll_until_timestamp(), cx);
    let from_ts = ret_on_err!(data.enroll_from_timestamp(), cx);

    let current_ts = current_timestamp();
    if from_ts > current_ts {
        // We still need to wait to enroll
        let wait_time = max(from_ts - current_ts - 30, 0) as u64;
        reply!(cx, "I will enroll you in {} seconds", from_ts - current_ts).await?;
        trace!("waiting for {} seconds before we can enroll", wait_time);
        tokio::time::sleep(Duration::from_secs(wait_time)).await;

        token = ret_on_err!(
            asvz_login(&client, username.as_str(), password.as_str_dangerous()).await,
            cx,
            "Unable to log in"
        );
        trace!("refreshed token");

        let current_ts = current_timestamp();
        let wait_time = max(from_ts - current_ts - 2, 0) as u64;
        trace!("waiting again {} seconds", wait_time);
        tokio::time::sleep(Duration::from_secs(wait_time)).await;

        while current_timestamp() < from_ts + 5 {
            trace!("starting to enroll");
            let enroll_response = ret_on_err!(
                client
                    .post(enroll_url.clone())
                    .bearer_auth(&token)
                    .json(&())
                    .send()
                    .await,
                cx
            );
            trace!(
                "enroll response with status code {}",
                enroll_response.status()
            );

            match enroll_response.status() {
                StatusCode::CREATED => {
                    reply!(cx, "I successfully enrolled you\nStopping Job").await?;
                    return Ok(());
                }
                StatusCode::UNPROCESSABLE_ENTITY => (),
                code => {
                    reply!(cx, "Got unexpected status code: {}\nStopping Job", code).await?;
                    return Ok(());
                }
            }
        }
    }

    for count in CountLoop::new() {
        let current_ts = current_timestamp();

        if current_ts > until_ts {
            reply!(cx, "You can no longer enroll\nStopping this Job").await?;
            return Ok(());
        }
        let enroll_response = ret_on_err!(
            client
                .post(enroll_url.clone())
                .bearer_auth(&token)
                .json(&())
                .send()
                .await,
            cx
        );

        trace!(
            "Tried to enroll with status code: {}",
            enroll_response.status()
        );

        match enroll_response.status() {
            StatusCode::CREATED => {
                cx.answer("Successfully enrolled you\nClosing Job").await?;
                return Ok(());
            }
            StatusCode::UNAUTHORIZED => {
                token = ret_on_err!(
                    asvz_login(&client, username.as_str(), password.as_str_dangerous()).await,
                    cx,
                    "Unable to log in"
                );
            }
            StatusCode::UNPROCESSABLE_ENTITY => (),
            code => {
                reply!(cx, "Got unexpected status code: {}\nStopping Job", code).await?;
                return Ok(());
            }
        }

        if count == 0 {
            reply!(
                cx,
                "It's already full. I will try to enroll you, when something opens up"
            )
            .await?;
        }

        tokio::time::sleep(Duration::from_secs(10)).await;
    }
    unreachable!()
}
