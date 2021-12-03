use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::RequestError;

use asvz::lesson::LessonID;

pub struct JobUpdateCx {
    cx: Arc<UpdateWithCx<AutoSend<Bot>, Message>>,
    id: LessonID,
}

impl JobUpdateCx {
    pub fn new(cx: Arc<UpdateWithCx<AutoSend<Bot>, Message>>, id: LessonID) -> Self {
        Self { cx, id }
    }

    fn transform_msg(&self, text: &str) -> String {
        format!("[{}] {}", self.id.as_str(), text)
    }

    pub async fn answer<T: Into<String>>(&self, text: T) -> Result<Message, RequestError> {
        self.cx.answer(self.transform_msg(&text.into())).await
    }
}
