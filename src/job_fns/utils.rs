use futures::Future;
use teloxide::prelude::*;
use teloxide::RequestError;

use crate::job_fns::ExistStatus;

pub async fn wrap_exit_status(
    cx: &UpdateWithCx<AutoSend<Bot>, Message>,
    fut: impl Future<Output = Result<ExistStatus, RequestError>>,
) -> Result<(), RequestError> {
    match fut.await? {
        ExistStatus::Success(msg) => {
            cx.answer(format!("{}\nJob existed successfully", msg))
                .await?
        }
        ExistStatus::Failure(msg) => {
            cx.answer(format!("{}\nJob existed unsuccessfully", msg))
                .await?
        }
    };
    Ok(())
}
