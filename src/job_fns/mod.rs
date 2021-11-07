pub use crate::job_fns::enroll::enroll;
pub use crate::job_fns::enroll::enroll_weekly;
pub use crate::job_fns::internals::msg_user;
pub use crate::job_fns::internals::reply_and_del;
pub use crate::job_fns::notify::notify;
pub use crate::job_fns::notify::notify_weekly;

mod enroll;
mod internals;
mod notify;
pub mod utils;

pub enum ExistStatus {
    Success(String),
    Failure(String),
    Error(String)
}

impl ExistStatus {
    pub fn success<T: Into<String>>(msg: T) -> Self {
        Self::Success(msg.into())
    }
    pub fn failure<T: Into<String>>(msg: T) -> Self {
        Self::Failure(msg.into())
    }
    pub fn error<T: Into<String>>(msg: T) -> Self {
        Self::Error(msg.into())
    }
}
