use crate::dialogue_state::DialogueState;

use teloxide::{dispatching::dialogue::ErasedStorage, prelude::Dialogue};

pub type MyDialogue = Dialogue<DialogueState, ErasedStorage<DialogueState>>;
pub type MyStorage = std::sync::Arc<ErasedStorage<DialogueState>>;
pub type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;