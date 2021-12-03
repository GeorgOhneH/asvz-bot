use std::time::Duration;

use teloxide::adaptors::AutoSend;
use teloxide::{prelude::*, RequestError};
use tracing::{instrument, trace};

#[instrument(skip(cx, text))]
pub async fn msg_user(
    cx: &UpdateWithCx<AutoSend<Bot>, Message>,
    text: String,
) -> Result<(), RequestError> {
    trace!("new msg job");
    cx.answer(text).await?;
    Ok(())
}

#[instrument(skip(cx, text))]
pub async fn reply_and_del(
    cx: &UpdateWithCx<AutoSend<Bot>, Message>,
    text: String,
) -> Result<(), RequestError> {
    trace!("reply_and_del");
    cx.answer(text).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;
    cx.delete_message().await?;
    Ok(())
}
