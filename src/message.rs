use strum::Display;
use teloxide::{payloads::SetMessageReaction, prelude::Requester, requests::JsonRequest, types::{ChatId, Message, MessageId, ReactionType}, Bot};
use crate::variants;

#[derive(Display, PartialEq)]
pub enum Reaction {
    Pending,
    Accepted,
    Declined,
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
                _ => unreachable!(),
            }.to_string();

            action.reaction = Some(vec![ReactionType::Emoji { emoji }]);
        }

        action
    }
}

pub trait PlainMessage {
    fn is_plain_text(&self) -> bool;
}

impl PlainMessage for Message {
    fn is_plain_text(&self) -> bool {
        self.text().is_some() 
            && self.photo().is_none()
            && self.video().is_none()
            && self.audio().is_none()
            && self.document().is_none()
            && self.sticker().is_none()
            && self.voice().is_none()
            && self.story().is_none()
            // Probably not needed
            && self.caption().is_none()
            && self.animation().is_none()
            && self.video_note().is_none()
    }
}
