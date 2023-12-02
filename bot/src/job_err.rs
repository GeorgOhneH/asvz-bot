use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

use serde::Serializer;
use teloxide::prelude::*;
use teloxide::{Bot, RequestError};

use crate::job::JobKind;
use crate::user::{BotCtx, UserId};

pub struct JobError {
    pub source: RequestError,
    pub user_id: UserId,
    pub job_kind: JobKind,
    pub bot: BotCtx,
    pub retry_count: usize,
}

impl Debug for JobError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobError")
            .field("source", &self.source)
            .field("user_id", &self.user_id)
            .field("job_kind", &self.job_kind)
            .finish()
    }
}

impl JobError {
    pub fn new(
        source: RequestError,
        user_id: UserId,
        job_kind: JobKind,
        bot: BotCtx,
        retry_count: usize,
    ) -> Self {
        Self {
            source,
            user_id,
            job_kind,
            bot,
            retry_count,
        }
    }
}

impl Display for JobError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "user {} job got an unexpected error: {}",
            self.user_id.0, &self.source
        ))
    }
}

impl Error for JobError {}
