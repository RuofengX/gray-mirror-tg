use anyhow::Result;
use grammers_client::types::{Dialog, Message as RawMessage};
use tracing::info;

use crate::{
    app::App,
    context::Context,
    types::{chat, message, MessageExt, Source},
};

use super::update::Updater;

pub struct GrayMirror;
impl GrayMirror {
    const NAME: &str = "灰镜";
    pub fn new() -> Self {
        Self {}
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
        info!(source_id = source.id, "接收更新");
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
