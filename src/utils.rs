use std::time::{SystemTime, UNIX_EPOCH};

macro_rules! ret_on_err {
    ($expression:expr) => {
        match $expression {
            Ok(val) => val,
            Err(err) => {
                tracing::warn!("Job error: {}", &err);
                let msg = format!("I got an unexpected error: {}", &err);
                return Ok(ExistStatus::error(msg));
            }
        }
    };
    ($expression:expr, $string:expr) => {
        match $expression {
            Ok(val) => val,
            Err(err) => {
                tracing::warn!("Job error: {}", &err);
                let msg = format!("{}: {}", $string, &err);
                return Ok(ExistStatus::error(msg));
            }
        }
    };
}
pub(crate) use ret_on_err;

macro_rules! reply {
    ($cx:ident, $($arg:tt)*) => {
        $cx.answer(format!($($arg)*))
    };
}
pub(crate) use reply;

pub fn current_timestamp() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System Time before UNIX EPOCH - how did we get here?")
            .as_secs(),
    )
    .expect("u64 to big")
}
