use std::time::Duration;

use crate::user::BotCtx;
use teloxide::{prelude::*, RequestError};
use tracing::{instrument, trace};

#[instrument(skip(bot, text))]
pub async fn msg_user(bot: &BotCtx, text: String) -> Result<(), RequestError> {
    trace!("new msg job");
    bot.answer(text).await?;
    Ok(())
}

#[instrument(skip(bot, text))]
pub async fn reply_and_del(bot: &BotCtx, text: String) -> Result<(), RequestError> {
    trace!("reply_and_del");
    bot.answer(text).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;
    bot.delete_message().await?;
    Ok(())
}
