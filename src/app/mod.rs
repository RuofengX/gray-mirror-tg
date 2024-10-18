use std::{fmt::Display, future::Future};

use anyhow::Result;
use async_trait::async_trait;
use grammers_client::{
    types::{CallbackQuery, InlineQuery, InlineSend, Message, MessageDeletion},
    Client, Update,
};
use tokio::sync::broadcast::{self, Receiver};
use tracing::error;

use crate::context::Context;

/// 简单的范用应用
pub mod generic;

/// 利用soso等机器人挖掘关联群组
pub mod finder;

pub trait App: Updater + Display + Send + Sync {
    /// 初始化数据
    fn ignite(&mut self, context: &mut Context) -> impl Future<Output = Result<()>> {
        let _ = context;
        async { Ok(()) }
    }
}

/// 匹配器，以供部分实现
#[async_trait]
pub trait Updater: Display + Send + Sync {
    /// Occurs whenever a new text message or a message with media is produced.
    async fn new_message(&mut self, client: &Client, msg: Message) -> Result<()> {
        let _ = (client, msg);
        Ok(())
    }

    /// Occurs when a message is updated.
    async fn message_edited(&mut self, client: &Client, msg: Message) -> Result<()> {
        let _ = (client, msg);
        Ok(())
    }

    /// Occurs when a message is deleted.
    async fn message_deletion(&mut self, client: &Client, msg_del: MessageDeletion) -> Result<()> {
        let _ = (client, msg_del);
        Ok(())
    }

    /// Occurs when Telegram calls back into your bot because an inline callback
    /// button was pressed.
    async fn callback_query(
        &mut self,
        client: &Client,
        callback_query: CallbackQuery,
    ) -> Result<()> {
        let _ = (client, callback_query);
        Ok(())
    }

    /// Occurs whenever you sign in as a bot and a user sends an inline query
    /// such as `@bot query`.
    async fn inline_query(&mut self, client: &Client, inline_query: InlineQuery) -> Result<()> {
        let _ = (client, inline_query);
        Ok(())
    }

    /// Represents an update of user choosing the result of inline query and sending it to their chat partner.
    async fn inline_send(&mut self, client: &Client, inline_send: InlineSend) -> Result<()> {
        let _ = (client, inline_send);
        Ok(())
    }

    /// Return Ok(Some(())) if parsed; return Ok(None) if not parsed
    async fn parse_update(&mut self, client: &Client, update: Update) -> () {
        let result: anyhow::Result<()> = {
            match update {
                Update::NewMessage(msg) => self.new_message(client, msg).await,
                Update::MessageEdited(msg) => self.message_edited(client, msg).await,
                Update::MessageDeleted(msg_del) => self.message_deletion(client, msg_del).await,
                Update::CallbackQuery(callback_query) => {
                    self.callback_query(client, callback_query).await
                }
                Update::InlineQuery(inline_query) => self.inline_query(client, inline_query).await,
                Update::InlineSend(inline_send) => self.inline_send(client, inline_send).await,
                Update::Raw(_) =>Ok(()),
                _ => Ok(()),
            }
        };
        if let Err(e) = result{
            error!("{} parse update error > {e}", &self as &dyn Display);
            return
        };
    }
}

pub struct UpdateRuntime {
    client: Client,
    recv: broadcast::Receiver<Update>,
    method: Box<dyn Updater>,
}

impl AsRef<Client> for UpdateRuntime {
    fn as_ref(&self) -> &Client {
        &self.client
    }
}

impl UpdateRuntime {
    pub fn new(
        update_receiver: Receiver<Update>,
        client: Client,
        method: Box<dyn Updater>,
    ) -> Self {
        Self {
            client,
            recv: update_receiver,
            method,
        }
    }

    pub async fn update_daemon(&mut self) -> () {
        while let Ok(update) = self.recv.recv().await {
            self.method.parse_update(&self.client, update).await;
        }
    }
}

pub trait BackgroundTask: Display + Send + Sync {
    fn name() -> &'static str;
    fn start(&mut self, client: Client) -> impl Future<Output = Result<()>> + Send;  // No error handling
} 
