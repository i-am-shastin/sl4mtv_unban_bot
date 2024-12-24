use std::env;
use anyhow::Result;
use teloxide::types::ChatId;

pub struct Config {
    pub admins: Vec<ChatId>,
    pub db_filename: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            admins: env::var("BOT_ADMINS")?.split(',').map(|id| id.parse::<i64>().map(|id| ChatId(id))).collect::<Result<Vec<ChatId>, _>>()?,
            db_filename: env::var("DB_FILENAME").unwrap_or("db.sqlite".to_string()),
        })
    }
}
