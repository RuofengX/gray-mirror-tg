use std::{fmt::Display, future::Future};

use anyhow::Result;
use async_trait::async_trait;
use grammers_client::{
    types::{CallbackQuery, InlineQuery, InlineSend, Message, MessageDeletion},
    Client, Update,
};
use tokio::sync::broadcast::{self, Receiver};
use tracing::{error, info_span, trace};

use crate::{context::Context, types::MirrorMessage};

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
    async fn message_recv(&mut self, client: &Client, msg: MirrorMessage) -> Result<()> {
        let _ = (client, msg);
        Ok(())
    }

    /// Occurs when a message is updated.
    async fn message_edited(&mut self, client: &Client, msg: MirrorMessage) -> Result<()> {
        let _ = (client, msg);
        Ok(())
    }

    /// Occurs when a message is deleted.
    ///
    /// * return Some(Result<()>) if parsed;
    /// * return None if not parsed
    async fn message_deletion_raw(
        &mut self,
        _client: &Client,
        _msg_del: MessageDeletion,
    ) -> Option<Result<()>> {
        None
    }

    /// Occurs when Telegram calls back into your bot because an inline callback
    /// button was pressed.
    ///
    /// * return Some(Result<()>) if parsed;
    /// * return None if not parsed
    async fn callback_query_raw(
        &mut self,
        _client: &Client,
        _callback_query: CallbackQuery,
    ) -> Option<Result<()>> {
        None
    }

    /// Occurs whenever you sign in as a bot and a user sends an inline query
    /// such as `@bot query`.
    ///
    /// * return Some(Result<()>) if parsed;
    /// * return None if not parsed
    async fn inline_query_raw(
        &mut self,
        _client: &Client,
        _inline_query: InlineQuery,
    ) -> Option<Result<()>> {
        None
    }

    /// Represents an update of user choosing the result of inline query and sending it to their chat partner.
    ///
    /// * return Some(Result<()>) if parsed;
    /// * return None if not parsed
    async fn inline_send_raw(
        &mut self,
        _client: &Client,
        _inline_send: InlineSend,
    ) -> Option<Result<()>> {
        None
    }

    /// DO NOT RELOAD THIS FUNCTION
    /// UNLESS YOU KNOW WHAT YOU DO
    ///
    /// * return Some(()) if parsed;
    /// * return None if not parsed
    ///
    /// Every error should be parsed inside this function.
    async fn parse_update(&mut self, client: &Client, update: Update) -> Option<()> {
        let parse_span = info_span!("更新分配器");
        let _span = parse_span.enter();

        let result = {
            match update {
                Update::NewMessage(ref raw_msg) => {
                    if self.raw_msg_filter(raw_msg) {
                        Some(self.message_recv(client, raw_msg.into()).await)
                    } else {
                        None
                    }
                }
                Update::MessageEdited(ref raw_msg) => {
                    if self.raw_msg_filter(raw_msg) {
                        Some(self.message_edited(client, raw_msg.into()).await)
                    } else {
                        None
                    }
                }
                Update::MessageDeleted(msg_del) => self.message_deletion_raw(client, msg_del).await,
                Update::CallbackQuery(callback_query) => {
                    self.callback_query_raw(client, callback_query).await
                }
                Update::InlineQuery(inline_query) => {
                    self.inline_query_raw(client, inline_query).await
                }
                Update::InlineSend(inline_send) => self.inline_send_raw(client, inline_send).await,
                Update::Raw(_) => None,
                _ => None,
            }
        };
        if let Some(result) = result {
            match result {
                Ok(_) => trace!("更新器处理了消息"),
                Err(e) => error!("更新器返回了错误 > {} >> {e}", &self as &dyn Display),
            }
            Some(())
        } else {
            None
        }
    }

    /// default implement will fliter all message that incoming
    /// using `!raw_msg.outgoing()`
    ///
    /// * return true this message will get parsed later;
    /// * return false will ignore this message
    fn raw_msg_filter(&self, raw_msg: &Message) -> bool {
        let mut flag = true;

        if self.filter_incoming() {
            raw_msg.outgoing().then(|| flag = false);
        }

        if let Some(ids) = self.filter_chat_id() {
            if !ids.contains(&raw_msg.chat().id()) {
                flag = false;
            }
        }

        if let Some(word) = self.filter_word() {
            if !raw_msg.text().contains(word) {
                flag = false
            }
        }

        flag
    }

    fn filter_incoming(&self) -> bool {
        true
    }

    fn filter_chat_id(&self) -> Option<&[i64]> {
        None
    }

    fn filter_word(&self) -> Option<&str> {
        None
    }

    // fn filter_text(&self)
}

pub struct UpdateRuntime {
    client: Client,
    recv: broadcast::Receiver<Update>,
    method: Box<dyn Updater>,
}

impl Display for UpdateRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.method.fmt(f)?;
        write!(f, " >> 运行时")?;
        Ok(())
    }
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

    pub async fn run(mut self) -> Result<()> {
        while let Ok(update) = self.recv.recv().await {
            self.method.parse_update(&self.client, update).await;
        }
        Ok(())
    }
}

pub trait BackgroundTask: Display + Send + Sync {
    fn name() -> &'static str;
    fn start(&mut self, client: Client) -> impl Future<Output = Result<()>> + Send; // No error handling
}
