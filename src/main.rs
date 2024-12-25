pub mod types;
pub mod variants;
pub mod message;
pub mod dialogue_state;
pub mod request;
pub mod config;
pub mod admin_storage;

use dptree::case;
use log::{error, info, warn};

use std::sync::Arc;

use message::{MessageReaction, PlainMessage, Reaction};
use teloxide::{
    prelude::*,
    dispatching::dialogue::{
        self, serializer::Json, ErasedStorage, SqliteStorage, Storage
    },
    types::{MessageReactionUpdated, ReplyParameters},
    utils::command::BotCommands
};
use chrono::Utc;
use admin_storage::AdminStorage;
use dialogue_state::DialogueState;
use request::{RequestDecision, RequestType};
use config::Config;
use types::{AdminCallbackData, HandlerResult, MyDialogue, MyStorage};

#[derive(Clone, BotCommands)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    Start,
    Cancel,
}

const LOG4RS_CONFIG: &str = if cfg!(debug_assertions) {
    "log4rs.debug.yaml"
} else {
    "log4rs.yaml"
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    log4rs::init_file(LOG4RS_CONFIG, Default::default())?;
    info!("{}", "-".repeat(50));
    info!("Booting up");

    dotenvy::dotenv_override()?;

    let bot = Bot::from_env();
    let bot_name = bot.get_my_name().await?.name;
    info!("{bot_name} is running");

    let config = Config::from_env()?;
    if config.admins.is_empty() {
        return Err(anyhow::anyhow!("No admins found. Do you have ADMIN_CHAT_ID in environment variables?"));
    }

    let storage: MyStorage = SqliteStorage::open(&config.db_filename, Json).await.unwrap().erase();
    info!("Bot admins are: {:?}, database loaded from: {}", config.admins, config.db_filename);

    let admin_storage = AdminStorage::open(&config.admin_db_filename).await?;
    info!("Admin database loaded from: {}", config.admin_db_filename);

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![Command::Start].endpoint(start))
        .branch(case![Command::Cancel].endpoint(cancel));

    let handler = dialogue::enter::<Update, ErasedStorage<DialogueState>, DialogueState, _>()
        .branch(
            Update::filter_message()
                .branch(command_handler)
                .branch(dptree::endpoint(on_message))
        )
        .branch(Update::filter_callback_query().endpoint(callback_handler))
        .branch(Update::filter_edited_message().endpoint(on_edit_message))
        .branch(Update::filter_my_chat_member().endpoint(on_my_chat_member))
        .branch(Update::filter_message_reaction_updated().endpoint(on_reaction_updated));

    info!("Initializing update dispatcher");
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![storage, admin_storage, config.admins])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    info!("Shutting down");
    Ok(())
}

async fn start(
    bot: Bot,
    dialogue: MyDialogue,
    state: DialogueState,
    msg: Message,
) -> HandlerResult {
    let Some(user) = msg.from else {
        error!("Start command received, but user is missing for some reason. Message: {:?}", msg);
        return Ok(());
    };

    if state.has_sent_request() {
        bot.send_message(msg.chat.id, variants::START_HAS_PENDING_REQUEST_MESSAGE()).await?;
        info!("User {} has pending request but tried to start again", user.id);
        return Ok(())
    }

    let was_banned = state.is_banned;
    if state.current_request.is_some() || was_banned {
        let mut new_state = state.clone();
        new_state.current_request = None;
        new_state.is_banned = false;
        dialogue.update(new_state).await?;
        info!("User {} request cleared, bot was banned: {}", user.id, was_banned);
    }

    let text = if was_banned {
        variants::START_INTRO_MESSAGE_AFTER_BAN()
    } else {
        variants::START_INTRO_MESSAGE()
    };

    bot.send_message(msg.chat.id, text).reply_markup(RequestType::get_keyboard()).await?;
    info!("{:?} has started the bot", user);
    Ok(())
}

async fn cancel(
    bot: Bot,
    dialogue: MyDialogue,
    state: DialogueState,
    msg: Message,
) -> HandlerResult {
    let user_id = msg.from.map_or(0, |user| user.id.0);

    if state.current_request.is_none() {
        bot.send_message(msg.chat.id, variants::CANCEL_NO_REQUEST_MESSAGE()).await?;
        info!("User {user_id} tried to cancel request but there was no request to cancel");
        return Ok(());
    }

    if !state.has_sent_request() {
        bot.send_message(msg.chat.id, variants::CANCEL_REQUEST_IS_NOT_SENT_MESSAGE()).await?;
        info!("User {user_id} tried to cancel request but request is still pending details from user");
        return Ok(());
    }

    let user_message_id = state.current_request.as_ref().unwrap().user_message_id.unwrap();
    if let Ok(new_state) = state.try_remove_request(&bot).await {
        dialogue.update(new_state).await?;
        info!("Request from {user_id} is cancelled, admin messages are deleted");
    }
    
    bot.send_message(msg.chat.id, variants::CANCEL_COMPLETE_MESSAGE()).await?;
    bot.react_to_message_with(msg.chat.id, user_message_id, Reaction::None).await?;
    info!("User {user_id} cancelled request");
    Ok(())
}

async fn on_message(
    bot: Bot,
    dialogue: MyDialogue,
    state: DialogueState,
    msg: Message,
    admins: Vec<ChatId>,
    admin_storage: Arc<AdminStorage>,
) -> HandlerResult {
    if !msg.chat.is_private() {
        return Ok(())
    }

    let Some(user) = msg.from.as_ref() else {
        return Ok(())
    };

    if admins.contains(&user.id.into()) {
        match msg.reply_to_message() {
            Some(reply) if reply.reply_markup().is_some() && msg.text().is_some() => {
                admin_storage.save_admin_message(user.id, reply.id, msg.id, msg.text().unwrap()).await?;
                bot.react_to_message_with(msg.chat.id, msg.id, Reaction::Okay).await?;
            }
            _ => {
                bot.send_message(msg.chat.id, variants::ADMIN_MUST_REPLY_TO_REQUEST_MESSAGE()).await?;
            }
        }
        return Ok(())
    }

    if state.current_request.is_none() {
        bot.send_message(msg.chat.id, variants::CURRENT_REQUEST_MISSING_MESSAGE()).await?;
        info!("User {} sent message without selecting request type", user.id);
        return Ok(())
    };

    if state.has_sent_request() {
        bot.send_message(msg.chat.id, variants::CURRENT_REQUEST_PENDING_MESSAGE()).await?;
        info!("User {} has pending request but sent another message", user.id);
        return Ok(())
    }

    if !msg.is_plain_text() {
        bot.send_message(msg.chat.id, variants::TYPE_NOT_SUPPORTED_MESSAGE()).reply_parameters(ReplyParameters::new(msg.id)).await?;
        info!("User {} sent non-plain text", user.id);
        return Ok(())
    }

    if let Ok((new_state, sent_to)) = state.try_send_request(&bot, &msg, admins).await {
        dialogue.update(new_state).await?;
        bot.react_to_message_with(msg.chat.id, msg.id, Reaction::Pending).await?;
        info!("User {} request accepted and sent to these admins: {sent_to}", user.id);
    } else {
        warn!("User {} request failed to send", user.id);
    }

    Ok(())
}

async fn callback_handler(
    bot: Bot,
    dialogue: MyDialogue,
    state: DialogueState,
    admins: Vec<ChatId>,
    storage: MyStorage,
    admin_storage: Arc<AdminStorage>,
    q: CallbackQuery,
) -> HandlerResult {
    let Some(ref query_data) = q.data else {
        return Ok(())
    };

    bot.answer_callback_query(&q.id).await?;

    if let Ok(request_type) = serde_json::from_str::<RequestType>(query_data) {
        if state.has_sent_request() {
            return Ok(())
        }

        if matches!(&state.current_request, Some(request) if request.request_type == request_type) {
            info!("User {} re-selected request type {request_type} again", q.from.id);
            return Ok(())
        }

        if let Ok(new_state) = state.try_edit_request(&bot, &request_type, dialogue.chat_id()).await {
            dialogue.update(new_state).await?;
            info!("User {} request type changed to {request_type}", q.from.id);
            return Ok(())
        }

        match state.try_create_request(&bot, &request_type, dialogue.chat_id()).await {
            Ok(new_state) => {
                dialogue.update(new_state).await?;
                info!("User {} selected request type: {request_type}", q.from.id);
            },
            Err(e) => {
                warn!("User {} selected request type: {request_type} but request creation failed: {e}", q.from.id);
            },
        }

        return Ok(())
    }

    if !admins.contains(&q.from.id.into()) {
        return Ok(())
    }

    let Ok((decision, chat_id, message_id)) = serde_json::from_str::<AdminCallbackData>(query_data) else {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid admin callback data")));
    };

    let Some(current_user_state) = storage.clone().get_dialogue(chat_id).await? else {
        warn!("User {chat_id} has no active dialogue");
        return Ok(());
    };

    if let Ok(mut user_state) = current_user_state.try_remove_request(&bot).await {
        user_state.last_request_date = Some(Utc::now());
        user_state.total_requests = user_state.total_requests.saturating_add(1);
        storage.update_dialogue(chat_id, user_state).await?;
        info!("User {chat_id} requests data updated");
    } else {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("User {chat_id} requests data failed to update"))));
    }

    let status = if decision == RequestDecision::Yes { Reaction::Accepted } else { Reaction::Declined };
    info!("Admin {} set status {status} for request from user {chat_id}", q.from.id);

    bot.react_to_message_with(chat_id, message_id, status).await?;

    let mut base_text = if decision == RequestDecision::Yes {
        variants::REQUEST_COMPLETE_ACCEPTED_MESSAGE()
    } else {
        variants::REQUEST_COMPLETE_DECLINED_MESSAGE()
    }.to_string();

    if let Some(msg) = q.regular_message() {
        if let Some((reply_msg_id, reply)) = admin_storage.get_admin_message(q.from.id, msg.id).await? {
            base_text = format!("{}\n\nКомментарий администратора: {}", base_text, reply);
            bot.delete_message(msg.chat.id, reply_msg_id).await?;
        }
    }

    bot.send_message(chat_id, base_text).await?;
    info!("User {chat_id} received response message");

    Ok(())
}

async fn on_edit_message(
    bot: Bot,
    state: DialogueState,
    msg: Message,
) -> HandlerResult {
    if !state.has_sent_request() || !msg.is_plain_text() {
        return Ok(());
    }

    if let Some(request) = state.current_request.as_ref() {
        if request.user_message_id.is_some_and(|id| id == msg.id) {
            bot.send_message(msg.chat.id, variants::MESSAGE_EDITED_WARNING_MESSAGE()).await?;
        }
    }
    Ok(())
}

async fn on_reaction_updated(
    bot: Bot,
    reaction_update: MessageReactionUpdated,
) -> HandlerResult {
    bot.set_message_reaction(reaction_update.chat.id, reaction_update.message_id)
        .reaction(reaction_update.new_reaction)
        .await?;
    Ok(())
}

async fn on_my_chat_member(
    dialogue: MyDialogue,
    state: DialogueState,
    chat_update: ChatMemberUpdated,
) -> HandlerResult {
    if chat_update.new_chat_member.is_banned() {
        let mut new_state = state.clone();
        new_state.is_banned = true;
        dialogue.update(new_state).await?;
    }
    Ok(())
}