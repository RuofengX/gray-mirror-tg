use anyhow::Result;
use async_trait::async_trait;
use grammers_client::types::Message as RawMessage;
use tracing::info;

use crate::{
    context::Context,
    types::{message, MessageExt, Source},
    update::Updater,
};

#[derive(Debug, Default)]
pub struct LiveMirror;

#[async_trait]
impl Updater for LiveMirror {
    fn name(&self) -> &'static str {
        "增量消息镜像"
    }
    async fn message_recv(&mut self, context: Context, msg: MessageExt) -> Result<()> {
        let chat = msg.inner.chat();
        info!(chat_id = chat.id(), "接收更新");
        let source = Source::from_chat(chat.id());
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
