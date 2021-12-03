use thiserror::Error;
use url::ParseError;

use crate::asvz::api::lesson::LessonError;

#[derive(Error, Debug)]
pub enum AsvzError {
    #[error("Http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Http error: {0}")]
    HttpMiddleware(#[from] reqwest_middleware::Error),
    #[error("Unable to connect to Lesson: {0:?}")]
    Lesson(LessonError),
    #[error("Unexpected Response from the Server")]
    UnexpectedFormat,
}

impl From<url::ParseError> for AsvzError {
    fn from(_: ParseError) -> Self {
        Self::UnexpectedFormat
    }
}
