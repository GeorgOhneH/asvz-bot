use std::sync::Arc;

use futures::Future;
use teloxide::prelude::*;
use teloxide::RequestError;

use crate::job::JobKind;
use crate::job_err::JobError;
use crate::job_fns::ExistStatus;
use crate::job_update_cx::JobUpdateCx;
use crate::user::UserId;

pub async fn wrap_exit_status(
    cx: &JobUpdateCx,
    fut: impl Future<Output = Result<ExistStatus, RequestError>>,
) -> Result<(), RequestError> {
    match fut.await? {
        ExistStatus::Success(msg) => {
            cx.answer(format!("{}\nJob existed successfully", msg))
                .await?
        }
        ExistStatus::Failure(msg) => cx.answer(format!("{}\nJob failed", msg)).await?,
        ExistStatus::Error(msg) => cx.answer(format!("{}\nJob canceled", msg)).await?,
    };
    Ok(())
}

pub async fn attach_ctx<T>(
    fut: impl Future<Output = Result<T, RequestError>>,
    user_id: UserId,
    job_kind: JobKind,
    cx: Arc<UpdateWithCx<AutoSend<Bot>, Message>>,
    retry_count: usize,
) -> Result<T, JobError> {
    fut.await
        .map_err(|err| JobError::new(err, user_id, job_kind, cx, retry_count))
}
