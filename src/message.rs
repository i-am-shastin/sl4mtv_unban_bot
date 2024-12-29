use crate::variants;

use strum::Display;
use teloxide::{payloads::SetMessageReaction, prelude::Requester, requests::JsonRequest, types::{ChatId, MessageId, ReactionType}, Bot};

#[derive(Display, PartialEq)]
pub enum Reaction {
    Pending,
    Accepted,
    Declined,
    Okay,
    Salute,
    None
}

pub trait MessageReaction {
    fn react_to_message_with(&self, chat_id: ChatId, message_id: MessageId, reaction: Reaction) -> JsonRequest<SetMessageReaction>;
}

impl MessageReaction for Bot {
    fn react_to_message_with(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        reaction: Reaction
    ) -> JsonRequest<SetMessageReaction> {
        let mut action = self.set_message_reaction(chat_id, message_id);

        if reaction == Reaction::None {
            action.reaction = None;
        } else {
            let emoji = match reaction {
                Reaction::Pending => variants::emoji::REACTION_PENDING,
                Reaction::Accepted => variants::emoji::REACTION_ACCEPTED,
                Reaction::Declined => variants::emoji::REACTION_DECLINED,
                Reaction::Okay => variants::emoji::REACTION_OKAY,
                Reaction::Salute => variants::emoji::REACTION_SALUTE,
                _ => unreachable!(),
            }.to_string();

            action.reaction = Some(vec![ReactionType::Emoji { emoji }]);
        }

        action
    }
}
