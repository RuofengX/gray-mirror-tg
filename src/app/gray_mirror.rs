use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use async_trait::async_trait;
use grammers_client::types::{Chat, Dialog, Message as RawMessage};
use tracing::{info, info_span};

use crate::{
    app::{App, Updater},
    context::Context,
    types::{chat, message, MessageExt, Source},
    PrintError,
};

pub struct GrayMirror;
impl GrayMirror {
    const NAME: &str = "灰镜";
    pub fn new() -> Self {
        Self {}
    }
}

impl std::fmt::Display for GrayMirror {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Self::NAME.fmt(f)
    }
}

impl App for GrayMirror {
    async fn ignite(&mut self, context: crate::context::Context) -> anyhow::Result<()> {
        // fetch_all_joined_group(context.clone()).await?;
        context.add_updater(GrayMirror::new()).await?;
        Ok(())
    }
}

#[async_trait]
impl Updater for GrayMirror {
    async fn message_recv(&mut self, context: Context, msg: MessageExt) -> Result<()> {
        let source = Source::from_chat(msg.inner.chat().id());
        info!("接收更新 >> {:?}", source);
        context
            .persist
            .put_message(message::ActiveModel::from_inner_msg(&msg.inner, source))
            .await?;
        msg.inner.mark_as_read().await.ok();
        Ok(())
    }

    /// Occurs when a message is updated.
    async fn message_edited(&mut self, context: Context, msg: MessageExt) -> Result<()> {
        self.message_recv(context.clone(), msg).await?;
        Ok(())
    }

    fn raw_msg_filter(&self, raw_msg: &RawMessage) -> bool {
        let mut flag = true;

        if self.filter_incoming() {
            raw_msg.outgoing().then(|| flag = false);
        }

        match raw_msg.chat() {
            grammers_client::types::Chat::User(u) => {
                if u.is_bot() {
                    flag = false;
                }
            }
            _ => (),
        }

        flag
    }
}

pub async fn fetch_all_joined_group(context: Context) -> Result<()> {
    let mut dialogs = context.client.iter_dialogs();
    while let Some(Dialog { chat, .. }) = dialogs.next().await? {
        if context.persist.find_chat(chat.username()).await?.is_some() {
            continue;
        }
        context
            .persist
            .put_chat(chat::ActiveModel::from_chat(&chat, Source::from_manual()))
            .await?;
    }
    Ok(())
}

pub async fn fetch_chat_history(context: Context, chat: Chat, limit: usize) -> Result<()> {
    let history_span = info_span!("获取群组历史");
    let _span = history_span.enter();
    let source = Source::from_chat(chat.id());
    info!("{:?}", source);

    let mut history = context.client.iter_messages(chat).limit(limit).max_date(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("时间不够倒退")
            .as_secs() as i32,
    );

    while let Some(Some(msg)) = history.next().await.log_error() {
        context
            .persist
            .put_message(message::ActiveModel::from_inner_msg(&msg, source))
            .await?;
    }
    Ok(())
}
