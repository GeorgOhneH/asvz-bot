use std::cmp::max;
use std::time::Duration;

use crate::asvz::error::AsvzError;
use reqwest::{Client, StatusCode};
use teloxide::adaptors::AutoSend;
use teloxide::{prelude::*, RequestError};
use tracing::{instrument, trace};

use crate::asvz::lesson::{lesson_data, search_data};
use crate::asvz::login::asvz_login;
use crate::cmd::{LessonID, Password, Username};
use crate::job_fns::ExistStatus;
use crate::job_update_cx::JobUpdateCx;
use crate::utils::ret_on_err;
use crate::utils::{current_timestamp, reply};

#[instrument(skip(cx, password))]
pub async fn enroll(
    cx: &JobUpdateCx,
    id: LessonID,
    username: Username,
    password: Password,
) -> Result<ExistStatus, RequestError> {
    trace!("new enroll job");
    let client = Client::builder().cookie_store(true).build().unwrap();
    enroll_once(&client, cx, &id, &username, &password).await
}

#[instrument(skip(cx))]
pub async fn enroll_weekly(
    cx: &JobUpdateCx,
    start_id: LessonID,
    username: Username,
    password: Password,
) -> Result<ExistStatus, RequestError> {
    trace!("new enroll_weekly job");
    let client = Client::builder().cookie_store(true).build().unwrap();
    let mut current_id = start_id;
    loop {
        match enroll_once(&client, cx, &current_id, &username, &password).await? {
            ExistStatus::Success(msg) | ExistStatus::Failure(msg) => {
                cx.answer(msg).await?;
            }
            ExistStatus::Error(msg) => return Ok(ExistStatus::Error(msg)),
        }
        let event_list = ret_on_err!(search_data(&client, &current_id, 1).await);
        if let Some(id) = event_list.lesson_id() {
            current_id = id;
            reply!(cx, "Found next week's lesson: {}", current_id.as_str()).await?;
        } else {
            return Ok(ExistStatus::failure("Unable to find next lesson"));
        }
    }
}

async fn enroll_once(
    client: &Client,
    cx: &JobUpdateCx,
    id: &LessonID,
    username: &Username,
    password: &Password,
) -> Result<ExistStatus, RequestError> {
    trace!("enroll once");
    let enroll_url = format!(
        "https://schalter.asvz.ch/tn-api/api/Lessons/{}/enroll",
        id.as_str()
    );
    let mut token = ret_on_err!(
        asvz_login(client, username.as_str(), password.as_str_dangerous()).await,
        "Unable to log in"
    );

    let data = ret_on_err!(lesson_data(client, id).await);
    let until_ts = ret_on_err!(data.enroll_until_timestamp());
    let from_ts = ret_on_err!(data.enroll_from_timestamp());

    let current_ts = current_timestamp();
    if from_ts > current_ts {
        // We still need to wait to enroll
        let wait_time = max(from_ts - current_ts - 30, 0) as u64;
        reply!(cx, "I will enroll you in {} seconds", from_ts - current_ts).await?;
        trace!("waiting for {} seconds before we can enroll", wait_time);
        tokio::time::sleep(Duration::from_secs(wait_time)).await;

        token = ret_on_err!(
            asvz_login(client, username.as_str(), password.as_str_dangerous()).await,
            "Unable to log in"
        );
        trace!("refreshed token");

        let current_ts = current_timestamp();
        let wait_time = max(from_ts - current_ts - 2, 0) as u64;
        trace!("waiting again for {} seconds", wait_time);
        tokio::time::sleep(Duration::from_secs(wait_time)).await;

        while current_timestamp() < from_ts + 5 {
            trace!("starting to enroll");
            let enroll_response = ret_on_err!(
                client
                    .post(enroll_url.clone())
                    .bearer_auth(&token)
                    .json(&())
                    .send()
                    .await
            );
            trace!(
                "enroll response with status code {}",
                enroll_response.status()
            );

            match enroll_response.status() {
                StatusCode::CREATED => {
                    return Ok(ExistStatus::success("I successfully enrolled you"));
                }
                StatusCode::UNPROCESSABLE_ENTITY => (),
                code => {
                    let msg = format!("Got unexpected status code: {}", code);
                    return Ok(ExistStatus::error(msg));
                }
            }
        }
    }

    for count in 0.. {
        let current_ts = current_timestamp();

        if current_ts > until_ts {
            return Ok(ExistStatus::failure("You can no longer enroll"));
        }
        let enroll_response = ret_on_err!(
            client
                .post(enroll_url.clone())
                .bearer_auth(&token)
                .json(&())
                .send()
                .await
        );

        trace!(
            "Tried to enroll with status code: {}",
            enroll_response.status()
        );

        match enroll_response.status() {
            StatusCode::CREATED => {
                return Ok(ExistStatus::success("I successfully enrolled you"));
            }
            StatusCode::UNAUTHORIZED => {
                token = ret_on_err!(
                    asvz_login(client, username.as_str(), password.as_str_dangerous()).await,
                    "Unable to log in"
                );
            }
            StatusCode::UNPROCESSABLE_ENTITY => (),
            code => {
                let msg = format!("Got unexpected status code: {}", code);
                return Ok(ExistStatus::error(msg));
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
