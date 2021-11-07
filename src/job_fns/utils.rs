use futures::Future;
use teloxide::prelude::*;
use teloxide::RequestError;

use crate::job_fns::ExistStatus;
use crate::job_update_cx::JobUpdateCx;

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
