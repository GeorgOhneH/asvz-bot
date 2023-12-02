use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::RequestError;

use asvz::lesson::LessonID;
use crate::user::BotCtx;

pub struct JobUpdateCx {
    bot: BotCtx,
    id: LessonID,
}

impl JobUpdateCx {
    pub fn new(bot: BotCtx, id: LessonID) -> Self {
        Self { bot, id }
    }

    fn transform_msg(&self, text: &str) -> String {
        format!("[{}] {}", self.id.as_str(), text)
    }

    pub async fn answer<T: Into<String>>(&self, text: T) -> Result<(), RequestError> {
        self.bot.answer(self.transform_msg(&text.into())).await
    }
}
