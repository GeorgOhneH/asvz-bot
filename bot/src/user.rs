use std::str::FromStr;
use teloxide::Bot;
use teloxide::prelude::*;
use teloxide::types::MessageId;

use crate::cmd::{Password, Username};


#[derive(Clone)]
pub struct BotCtx {
    bot: Bot,
    chat_id: ChatId,
    msg_id: MessageId,
}

impl BotCtx {
    pub fn new(bot: Bot, chat_id: ChatId, msg_id: MessageId) -> Self {
        Self {
            bot, chat_id, msg_id,
        }
    }

    pub async fn answer(&self, text: String) -> ResponseResult<()>  {
        self.bot.send_message(self.chat_id, text).await?;
        Ok(())
    }

    pub async fn delete_message(&self) -> ResponseResult<()>  {
        self.bot.delete_message(self.chat_id, self.msg_id).await?;
        Ok(())
    }
}
#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct UserId(pub u64);

#[derive(Debug)]
pub struct UserState {
    pub credentials: Option<LoginCredentials>,
    pub settings: Settings,
}

impl UserState {
    pub fn new() -> Self {
        Self {
            credentials: None,
            settings: Settings::new(),
        }
    }

    pub fn with_credentials(credentials: LoginCredentials) -> Self {
        Self {
            credentials: Some(credentials),
            settings: Settings::new(),
        }
    }
}

#[derive(Debug)]
pub struct Settings {
    pub url_action: UrlAction,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            url_action: UrlAction::Default,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum UrlAction {
    Default,
    Notify,
    Enroll,
}

impl FromStr for UrlAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(UrlAction::Default),
            "1" => Ok(UrlAction::Notify),
            "2" => Ok(UrlAction::Enroll),
            _ => Err("Use one of following: 0: Default, 1: Notify, 2: Enroll".into()),
        }
    }
}

#[derive(Debug)]
pub struct LoginCredentials {
    pub username: Username,
    pub password: Password,
}

impl LoginCredentials {
    pub fn new(username: Username, password: Password) -> Self {
        Self { username, password }
    }
    pub fn update(&mut self, username: Username, password: Password) {
        self.username = username;
        self.password = password;
    }
}
