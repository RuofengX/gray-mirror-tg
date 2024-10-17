use std::pin::Pin;

use anyhow::Result;
use async_trait::async_trait;
use grammers_client::{
    types::{CallbackQuery, InlineQuery, InlineSend, Message, MessageDeletion},
    Client, Update,
};
use tokio::sync::broadcast::{self, Sender};

pub mod simple;

#[async_trait]
pub trait UpdateHandle: Send + Sync {
    /// Occurs whenever a new text message or a message with media is produced.
    async fn new_message(&mut self, client: &Client, msg: Message) -> Result<()>;

    /// Occurs when a message is updated.
    async fn message_edited(&mut self, client: &Client, msg: Message) -> Result<()>;

    /// Occurs when a message is deleted.
    async fn message_deletion(&mut self, client: &Client, msg_del: MessageDeletion) -> Result<()>;

    /// Occurs when Telegram calls back into your bot because an inline callback
    /// button was pressed.
    async fn callback_query(
        &mut self,
        client: &Client,
        callback_query: CallbackQuery,
    ) -> Result<()>;

    /// Occurs whenever you sign in as a bot and a user sends an inline query
    /// such as `@bot query`.
    async fn inline_query(&mut self, client: &Client, inline_query: InlineQuery) -> Result<()>;

    /// Represents an update of user choosing the result of inline query and sending it to their chat partner.
    async fn inline_send(&mut self, client: &Client, inline_send: InlineSend) -> Result<()>;
}

pub struct UpdateParser {
    client: Client,
    r: broadcast::Receiver<Update>,
    config: Box<dyn UpdateHandle>,
}

impl AsRef<Client> for UpdateParser {
    fn as_ref(&self) -> &Client {
        &self.client
    }
}

impl UpdateParser {
    pub fn new(client: Client, handler: Box<dyn UpdateHandle>) -> (Self, Sender<Update>) {
        let (s, r) = broadcast::channel(1024);
        (Self { client, r, config: handler }, s)
    }
    pub fn subscribe(
        client: Client,
        handler: Box<dyn UpdateHandle>,
        sender: Sender<Update>,
    ) -> Self {
        Self {
            client,
            r: sender.subscribe(),
            config: handler,
        }
    }

    pub async fn start_daemon(&mut self) -> Result<()> {
        while let Ok(update) = self.r.recv().await {
            self.handle_update(update).await?;
        }
        Ok(())
    }

    async fn handle_update(&mut self, update: Update) -> Result<Option<()>> {
        match update {
            Update::NewMessage(msg) => self.config.new_message(&self.client, msg).await?,
            Update::MessageEdited(msg) => self.config.message_edited(&self.client, msg).await?,
            Update::MessageDeleted(msg_del) => {
                self.config.message_deletion(&self.client, msg_del).await?
            }
            Update::CallbackQuery(callback_query) => {
                self.config
                    .callback_query(&self.client, callback_query)
                    .await?
            }
            Update::InlineQuery(inline_query) => {
                self.config
                    .inline_query(&self.client, inline_query)
                    .await?
            }
            Update::InlineSend(inline_send) => {
                self.config.inline_send(&self.client, inline_send).await?
            }
            Update::Raw(_) => return Ok(None),
            _ => return Ok(None),
        };
        Ok(Some(()))
    }
}
