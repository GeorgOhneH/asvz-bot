use crate::asvz::api::lesson::{LessonData, LessonError};
use crate::cmd::LessonID;
use reqwest::Client;
use crate::asvz::error::AsvzError;

pub async fn lesson_data(client: &Client, id: &LessonID) -> Result<LessonData, AsvzError> {
    let url = format!(
        "https://schalter.asvz.ch/tn-api/api/Lessons/{}",
        id.as_str()
    );
    let response = client.get(url).send().await?;
    let full = response.bytes().await?;
    if let Ok(data) = serde_json::from_slice::<LessonData>(&full) {
        Ok(data)
    } else if let Ok(err) = serde_json::from_slice::<LessonError>(&full) {
        Err(AsvzError::Lesson(err))
    } else {
        Err(AsvzError::UnexpectedFormat)
    }
}
