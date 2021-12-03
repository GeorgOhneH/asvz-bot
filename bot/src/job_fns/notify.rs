use std::cmp::max;
use std::time::Duration;

use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use reqwest_tracing::TracingMiddleware;
use teloxide::adaptors::AutoSend;
use teloxide::{prelude::*, RequestError};
use tracing::{instrument, trace};

use asvz::error::AsvzError;
use asvz::lesson::LessonID;
use asvz::lesson::{lesson_data, search_data};

use crate::job_fns::ExistStatus;
use crate::job_update_cx::JobUpdateCx;
use crate::utils::current_timestamp;
use crate::utils::reply;
use crate::utils::ret_on_err;

#[instrument(skip(cx))]
pub async fn notify(cx: &JobUpdateCx, id: LessonID) -> Result<ExistStatus, RequestError> {
    trace!("new notify job");
    let client = build_client();
    notify_once(&client, cx, &id).await
}

#[instrument(skip(cx))]
pub async fn notify_weekly(
    cx: &JobUpdateCx,
    start_id: LessonID,
) -> Result<ExistStatus, RequestError> {
    trace!("new notify_weekly job");
    let client = build_client();
    let mut current_id = start_id;
    loop {
        match notify_once(&client, cx, &current_id).await? {
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

async fn notify_once(
    client: &ClientWithMiddleware,
    cx: &JobUpdateCx,
    id: &LessonID,
) -> Result<ExistStatus, RequestError> {
    trace!("notify_once");
    let data = ret_on_err!(lesson_data(client, id).await);
    let current_ts = current_timestamp();

    let until_ts = ret_on_err!(data.enroll_until_timestamp());

    let from_ts = ret_on_err!(data.enroll_from_timestamp());

    if from_ts > current_ts {
        // We still need to wait to enroll
        let wait_time = max(from_ts - current_ts - 60, 0) as u64;
        reply!(cx, "I will remind you to enroll in {} seconds.", wait_time).await?;
        tokio::time::sleep(Duration::from_secs(wait_time)).await;
        let current_time = current_timestamp();
        let msg = format!("Enrollment starts in {} seconds!", from_ts - current_time);
        return Ok(ExistStatus::success(msg));
    }

    for count in 0.. {
        if current_ts > until_ts {
            return Ok(ExistStatus::failure("You can no longer enroll."));
        }

        let fresh_data = ret_on_err!(lesson_data(client, id).await);
        let free_places = fresh_data.data.participants_max - fresh_data.data.participant_count;
        if free_places > 0 {
            let msg = format!("There are currently {} free spots.", free_places);
            return Ok(ExistStatus::Success(msg));
        }
        if count == 0 {
            reply!(
                cx,
                "This lesson is already full. I will notify you, when a spot opens up."
            )
            .await?;
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
    unreachable!()
}

fn build_client() -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    ClientBuilder::new(Client::builder().cookie_store(true).build().unwrap())
        .with(TracingMiddleware)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}
