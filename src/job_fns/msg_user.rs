use teloxide::adaptors::AutoSend;
use teloxide::{prelude::*, RequestError};
use tracing::{instrument, trace};

#[instrument(skip(cx, text))]
pub async fn msg_user(
    cx: UpdateWithCx<AutoSend<Bot>, Message>,
    text: String,
) -> Result<(), RequestError> {
    trace!("new msg job");
    cx.answer(text).await?;
    Ok(())
}
