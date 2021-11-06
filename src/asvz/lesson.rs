use crate::asvz::api::lesson::{LessonData, LessonError};
use crate::asvz::error::AsvzError;
use crate::cmd::LessonID;
use reqwest::Client;
use tracing::{instrument, trace, warn};

#[instrument(skip(client))]
pub async fn lesson_data(client: &Client, id: &LessonID) -> Result<LessonData, AsvzError> {
    trace!("fetching lesson data");
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
        warn!(
            "Unable to decode: {}",
            String::from_utf8_lossy(full.as_ref())
        );
        Err(AsvzError::UnexpectedFormat)
    }
}
