use crate::request::{AdminRequest, Request, RequestDecision, RequestType};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use teloxide::{
    payloads::SendMessageSetters,
    prelude::Requester,
    types::{ChatId, Message, ReplyParameters},
    Bot,
    RequestError
};
use anyhow::{anyhow, Result};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct DialogueState {
    pub total_requests: u8,
    pub last_request_date: Option<DateTime<Utc>>,
    pub current_request: Option<Request>,
    pub is_banned: bool,
}

impl DialogueState {
    fn format_last_request_date(&self) -> String {
        if let Some(date) = self.last_request_date {
            let duration = Utc::now().signed_duration_since(date);
            let days = duration.num_days();
            let hours = duration.num_hours() % 24;
            let minutes = duration.num_minutes() % 60;
            
            let mut text = String::with_capacity(16);
            if days > 0 {
                text.push_str(&format!("{}д ", days));
            }
            if hours > 0 {
                text.push_str(&format!("{}ч ", hours));
            }
            if minutes > 0 {
                text.push_str(&format!("{}м", minutes));
            }
            
            let duration_text = text.trim();
            let duration_text = if duration_text.is_empty() {
                "меньше минуты"
            } else {
                duration_text
            };

            format!(" С момента последнего обращения прошло {}.", duration_text)
        } else {
            String::new()
        }
    }

    pub fn format_request_info(&self) -> String {
        format!(
            "Пользователю нужна помощь с {}.\nПредыдущих запросов: {}.{}\n\nКаков вердикт?", 
            self.current_request.as_ref().map(|r| r.request_type.to_string()).unwrap_or("WTF_SHOULDNT_HAPPEN".to_string()), 
            self.total_requests, 
            self.format_last_request_date()
        )
    }

    pub fn has_sent_request(&self) -> bool {
        self.current_request.as_ref().is_some_and(|req| !req.sent_to.is_empty())
    }

    pub async fn try_remove_request(&self, bot: &Bot) -> Result<DialogueState, RequestError> {
        let mut new_state = self.clone();
        if let Some(request) = new_state.current_request.take() {
            for admin_request in request.sent_to {
                bot.delete_messages(admin_request.admin_chat_id, [admin_request.forwarded_message_id, admin_request.message_id]).await?;
            }
        }
        Ok(new_state)
    }

    pub async fn try_edit_request(&self, bot: &Bot, request_type: &RequestType, chat_id: ChatId) -> Result<DialogueState> {
        if let Some(request) = &self.current_request {
            if bot.edit_message_text(chat_id, request.bot_message_id, request_type.get_text()).await.is_ok() {
                let mut new_state = self.clone();
                new_state.current_request.as_mut().unwrap().request_type = request_type.clone();
                Ok(new_state)
            } else {
                Err(anyhow!("Failed to edit message"))
            }
        } else {
            Err(anyhow!("Current request not found"))
        }
    }

    pub async fn try_create_request(&self, bot: &Bot, request_type: &RequestType, chat_id: ChatId) -> Result<DialogueState, RequestError> {
        let mut new_state = self.clone();
        let message = bot.send_message(chat_id, request_type.get_text()).await?;
        new_state.current_request = Some(Request {
            bot_message_id: message.id,
            user_message_id: None,
            request_type: request_type.clone(),
            sent_to: vec![],
            date: Utc::now().timestamp(),
        });
        Ok(new_state)
    }

    pub async fn try_send_request(&self, bot: &Bot, msg: &Message, admins: Vec<ChatId>) -> Result<(DialogueState, String)> {
        let mut new_state = self.clone();
        let request_info_text = new_state.format_request_info();
        let request = new_state.current_request.as_mut().unwrap();

        for admin_chat_id in admins {
            let Ok(forwarded_message) = bot.forward_message(admin_chat_id, msg.chat.id, msg.id).await else {
                continue;
            };
            let Ok(message) = bot.send_message(admin_chat_id, &request_info_text)
                .reply_parameters(ReplyParameters::new(forwarded_message.id))
                .reply_markup(RequestDecision::get_keyboard_for_message(msg))
                .await else {
                    continue;
                };
    
            request.sent_to.push(AdminRequest {
                forwarded_message_id: forwarded_message.id,
                message_id: message.id,
                admin_chat_id,
            });
        }

        request.user_message_id = Some(msg.id);
        let sent_to = request.sent_to.iter().map(|r| r.admin_chat_id.to_string()).collect::<Vec<String>>().join(", ");

        Ok((new_state, sent_to))
    }
}
