use anyhow::Result;
use grammers_client::{
    types::{CallbackQuery, InlineQuery, InlineSend, Message, MessageDeletion},
    Client, Update,
};

pub mod simple;

trait UpdateHandle: AsRef<Client> {
    /// Occurs whenever a new text message or a message with media is produced.
    async fn new_message(&mut self, msg: Message) -> Result<()>;

    /// Occurs when a message is updated.
    async fn message_edited(&mut self, msg: Message) -> Result<()>;

    /// Occurs when a message is deleted.
    async fn message_deletion(&mut self, msg_del: MessageDeletion) -> Result<()>;

    /// Occurs when Telegram calls back into your bot because an inline callback
    /// button was pressed.
    async fn callback(&mut self, _callback_query: CallbackQuery) -> Result<()> {
        Ok(())
    }

    /// Occurs whenever you sign in as a bot and a user sends an inline query
    /// such as `@bot query`.
    async fn inline_query(&mut self, _inline_query: InlineQuery) -> Result<()> {
        Ok(())
    }

    /// Represents an update of user choosing the result of inline query and sending it to their chat partner.
    async fn inline_send(&mut self, _inline_send: InlineSend) -> Result<()> {
        Ok(())
    }

    async fn handle_update(&mut self, update: Update) -> Result<Option<()>> {
        match update {
            Update::NewMessage(msg) => self.new_message(msg).await?,
            Update::MessageEdited(msg) => self.message_edited(msg).await?,
            Update::MessageDeleted(msg_del) => self.message_deletion(msg_del).await?,
            Update::CallbackQuery(callback_query) => self.callback(callback_query).await?,
            Update::InlineQuery(inline_query) => self.inline_query(inline_query).await?,
            Update::InlineSend(inline_send) => self.inline_send(inline_send).await?,
            Update::Raw(_) => return Ok(None),
            _ => return Ok(None),
        };
        Ok(Some(()))
    }
}
