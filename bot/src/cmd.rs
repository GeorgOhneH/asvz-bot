use std::fmt;
use std::fmt::Formatter;
use std::str::FromStr;
use teloxide::utils::command::ParseError;

use asvz::lesson::LessonID;
use bot_derive::BotCommands;

use crate::user::UrlAction;

#[derive(Clone, Debug)]
pub struct Username(String);

impl Username {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl FromStr for Username {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Err("You need to supply a non empty username".to_string())
        } else {
            Ok(Self(s.to_string()))
        }
    }
}

#[derive(Clone)]
pub struct Password(String);

impl Password {
    pub fn as_str_dangerous(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Password").field(&"***").finish()
    }
}

impl FromStr for Password {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Err("You need to supply a non-empty password!".to_string())
        } else {
            Ok(Self(s.to_string()))
        }
    }
}

pub trait BotCommands: Sized {
    fn parse(s: &str, bot_username: &str) -> Result<Self, ParseError>;
    fn descriptions() -> String;
}

#[derive(Debug, bot_derive::BotCommands)]
#[command(
    rename = "lowercase",
    description = "The following commands are supported:"
)]
pub enum Command {
    #[command(description = " - Display the Start Message.")]
    Start,

    #[command(description = " - Displays this text.")]
    Help,

    #[command(
        description = " <lesson_id> - Get notified when a lesson starts or a spot becomes available.",
        parse_with = "split"
    )]
    Notify { lesson_id: LessonID },

    #[command(
        description = " <lesson_id> - Get weekly notifications when a lesson starts or a spot becomes available.",
        parse_with = "split"
    )]
    NotifyWeekly { lesson_id: LessonID },

    #[command(
        description = " <lesson_id> - Get automatically enrolled when a lesson starts or a spot becomes available.",
        parse_with = "split"
    )]
    Enroll { lesson_id: LessonID },

    #[command(
        description = " <lesson_id> - Get automatically enrolled when a lesson starts or a spot becomes available (repeats every week).",
        parse_with = "split"
    )]
    EnrollWeekly { lesson_id: LessonID },

    #[command(
        description = " <username> <password> - Stores your username and password, so you can be enrolled automatically. \
    Important: While your password is never stored in persistent memory, \
    your are still giving a random person on the internet your password. \
    I wouldn't do it, if I were you :)",
        parse_with = "split"
    )]
    Login {
        username: Username,
        password: Password,
    },

    #[command(description = " - Remove your login credentials.")]
    Logout,

    #[command(
        description = " {0, 1, 2} - Sets the behavior when a lesson url is found:\n\
        \t 0: Default - If you are logged in I will enroll you, otherwise I will only notify you\n\
        \t 1: Notify - I will always notify you\n\
        \t 2: Enroll - I will always enroll you\n",
        parse_with = "split"
    )]
    UrlAction { url_action: UrlAction },

    #[command(description = " - Show your current Jobs.")]
    Jobs,

    #[command(description = " - Cancel all Jobs.")]
    CancelAll,
}
