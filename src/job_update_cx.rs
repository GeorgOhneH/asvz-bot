use crate::cmd::LessonID;
use teloxide::prelude::*;
use teloxide::RequestError;

pub struct JobUpdateCx {
    cx: UpdateWithCx<AutoSend<Bot>, Message>,
    id: LessonID,
}

impl JobUpdateCx {
    pub fn new(cx: UpdateWithCx<AutoSend<Bot>, Message>, id: LessonID) -> Self {
        Self { cx, id }
    }

    fn transform_msg(&self, text: &str) -> String {
        format!("[{}] {}", self.id.as_str(), text)
    }

    pub async fn answer<T: Into<String>>(&self, text: T) -> Result<Message, RequestError> {
        self.cx.answer(self.transform_msg(&text.into())).await
    }
}
