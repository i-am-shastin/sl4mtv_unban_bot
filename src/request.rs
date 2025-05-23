use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, IntoEnumIterator};
use teloxide::types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, Message, MessageId};

use crate::variants;
use anyhow::Result;

#[derive(Clone, Serialize, Deserialize)]
pub struct Request {
    pub bot_message_id: MessageId,
    pub user_message_id: Option<MessageId>,
    pub sent_to: Vec<AdminRequest>,
    pub date: i64,
    pub request_type: RequestType,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AdminRequest {
    pub forwarded_message_id: MessageId,
    pub message_id: MessageId,
    pub admin_chat_id: ChatId,
}

#[derive(Debug, Display, Clone, EnumIter, PartialEq, Serialize, Deserialize)]
pub enum RequestType {
    Twitch,
    YouTube,
    #[strum(to_string = "Telegram (группа)")]
    Telegram,
    #[strum(to_string = "Telegram (канал)")]
    TelegramChannel,
}

impl RequestType {
    pub fn get_text(&self) -> &str {
        match self {
            RequestType::Twitch => variants::HELP_WITH_TWITCH_TEXT(),
            RequestType::YouTube => variants::HELP_WITH_YOUTUBE_TEXT(),
            RequestType::Telegram | RequestType::TelegramChannel => variants::HELP_WITH_TELEGRAM_TEXT(),
        }
    }

    pub fn get_keyboard() -> InlineKeyboardMarkup {
        let buttons: Vec<InlineKeyboardButton> = RequestType::iter()
            .filter_map(|request_type| {
                serde_json::to_string(&request_type)
                    .map(|callback_data| {
                        InlineKeyboardButton::callback(request_type.to_string(), callback_data)
                    })
                    .ok()
            })
            .collect();

        let keyboard: Vec<Vec<InlineKeyboardButton>> = buttons
            .chunks(2)
            .map(|chunk| chunk.to_vec())
            .collect();

        InlineKeyboardMarkup::new(keyboard)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RequestDecision {
    Yes((ChatId, i32)),
    No((ChatId, i32)),
    Ban((ChatId, i64)),
}

impl RequestDecision {
    pub fn get_keyboard_for_message(msg: &Message) -> Result<InlineKeyboardMarkup> {
        Ok(
            InlineKeyboardMarkup::new(vec![
                vec![
                    InlineKeyboardButton::callback(variants::BUTTON_ACCEPT_TEXT, serde_json::to_string(&RequestDecision::Yes((msg.chat.id, msg.id.0)))?),
                    InlineKeyboardButton::callback(variants::BUTTON_DECLINE_TEXT, serde_json::to_string(&RequestDecision::No((msg.chat.id, msg.id.0)))?)
                ],
                vec![
                    InlineKeyboardButton::callback(variants::POST_BAN_BUTTON_7D_TEXT, serde_json::to_string(&RequestDecision::Ban((msg.chat.id, 7)))?),
                    InlineKeyboardButton::callback(variants::POST_BAN_BUTTON_30D_TEXT, serde_json::to_string(&RequestDecision::Ban((msg.chat.id, 30)))?),
                    InlineKeyboardButton::callback(variants::POST_BAN_BUTTON_INFINITE_TEXT, serde_json::to_string(&RequestDecision::Ban((msg.chat.id, 9999)))?),
                ]
            ])
        )
    }
}
