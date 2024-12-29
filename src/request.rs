use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, IntoEnumIterator};
use teloxide::types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, Message, MessageId};

use crate::{variants, types::AdminCallbackData};

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

#[derive(Debug, Clone, PartialEq,EnumIter, Serialize, Deserialize)]
pub enum RequestDecision {
    Yes,
    No,
}

impl std::fmt::Display for RequestDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestDecision::Yes => write!(f, "{}", variants::BUTTON_ACCEPT_TEXT),
            RequestDecision::No => write!(f, "{}", variants::BUTTON_DECLINE_TEXT),
        }
    }
}

impl RequestDecision {
    pub fn get_keyboard_for_message(msg: &Message) -> InlineKeyboardMarkup {
        InlineKeyboardMarkup::new(
            vec![
                RequestDecision::iter()
                    .filter_map(|request_decision| {
                        let data: AdminCallbackData = (request_decision.clone(), msg.chat.id, msg.id);
                        serde_json::to_string(&data)
                            .map(|callback_data| {
                                InlineKeyboardButton::callback(request_decision.to_string(), callback_data)
                            })
                            .ok()
                    })
                    .collect::<Vec<_>>()
            ]
        )
    }
}
