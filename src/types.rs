use crate::{dialogue_state::DialogueState, request::RequestDecision};

use teloxide::{dispatching::dialogue::ErasedStorage, prelude::Dialogue, types::{ChatId, MessageId}};

pub type MyDialogue = Dialogue<DialogueState, ErasedStorage<DialogueState>>;
pub type MyStorage = std::sync::Arc<ErasedStorage<DialogueState>>;
pub type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub type AdminCallbackData = (RequestDecision, ChatId, MessageId);