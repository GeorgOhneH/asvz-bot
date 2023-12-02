use std::cmp::max;
use std::time::Duration;

use asvz::api::enrollment::EnrollmentData;
use reqwest::{Client, StatusCode};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, Error};
use reqwest_retry::{
    default_on_request_failure, default_on_request_success, policies::ExponentialBackoff,
    RetryTransientMiddleware, Retryable, RetryableStrategy,
};
use reqwest_tracing::{DefaultSpanBackend, TracingMiddleware};
use teloxide::{prelude::*, RequestError};
use tracing::{instrument, trace};

use asvz::error::AsvzError;
use asvz::lesson::LessonID;
use asvz::lesson::{lesson_data, search_data};
use asvz::login::asvz_login;

use crate::cmd::{Password, Username};
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
    let client = build_client();
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
    let client = build_client();
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
    client: &ClientWithMiddleware,
    cx: &JobUpdateCx,
    id: &LessonID,
    username: &Username,
    password: &Password,
) -> Result<ExistStatus, RequestError> {
    trace!("enroll once");
    let enroll_url = format!(
        "https://schalter.asvz.ch/tn-api/api/Lessons/{}/Enrollment",
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
        let wait_time = max(from_ts - current_ts, 0) as u64;
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
                    if let Ok(enrollment_data) = enroll_response.json::<EnrollmentData>().await {
                        trace!("enrollment_data: {:?}", enrollment_data);
                    }
                    return Ok(ExistStatus::success("I successfully enrolled you"));
                }
                StatusCode::UNPROCESSABLE_ENTITY => (),
                StatusCode::TOO_MANY_REQUESTS => {
                    tokio::time::sleep(Duration::from_millis(300)).await;
                }
                code => {
                    let msg = format!("Got unexpected status code: {}", code);
                    return Ok(ExistStatus::error(msg));
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    trace!("trying normal enrollment");
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
                if let Ok(enrollment_data) = enroll_response.json::<EnrollmentData>().await {
                    trace!("enrollment_data: {:?}", enrollment_data);
                }
                return Ok(ExistStatus::success("I successfully enrolled you"));
            }
            StatusCode::UNPROCESSABLE_ENTITY => (),
            StatusCode::TOO_MANY_REQUESTS => {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
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

pub struct EnrollRetryableStrategy;

impl RetryableStrategy for EnrollRetryableStrategy {
    fn handle(&self, res: &Result<reqwest::Response, Error>) -> Option<Retryable> {
        match res {
            Ok(success) => enroll_on_request_success(success),
            Err(error) => default_on_request_failure(error),
        }
    }
}

pub fn enroll_on_request_success(success: &reqwest::Response) -> Option<Retryable> {
    let status = success.status();
    if status.is_server_error() {
        Some(Retryable::Transient)
    } else if status.is_client_error() && status != StatusCode::REQUEST_TIMEOUT {
        Some(Retryable::Fatal)
    } else if status.is_success() {
        None
    } else if status == StatusCode::REQUEST_TIMEOUT {
        Some(Retryable::Transient)
    } else {
        Some(Retryable::Fatal)
    }
}

fn build_client() -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    ClientBuilder::new(Client::builder().cookie_store(true).build().unwrap())
        .with(TracingMiddleware::<DefaultSpanBackend>::new())
        .with(RetryTransientMiddleware::new_with_policy_and_strategy(
            retry_policy,
            EnrollRetryableStrategy,
        ))
        .build()
}
