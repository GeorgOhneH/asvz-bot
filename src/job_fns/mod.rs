pub use crate::job_fns::enroll::enroll;
pub use crate::job_fns::msg_user::msg_user;
pub use crate::job_fns::notify::notify;

mod enroll;
mod msg_user;
mod notify;
pub mod utils;

pub enum ExistStatus {
    Success(String),
    Failure(String),
}

impl ExistStatus {
    pub fn success<T: Into<String>>(msg: T) -> Self {
        Self::Success(msg.into())
    }
    pub fn failure<T: Into<String>>(msg: T) -> Self {
        Self::Failure(msg.into())
    }
}
