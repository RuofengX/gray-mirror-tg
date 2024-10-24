use anyhow::Result;
use async_trait::async_trait;
use tracing::info;

use crate::{
    app::{App, Updater},
    context::Context,
    types::{message, MessageExt, Source},
};

pub struct GrayMirror;
impl GrayMirror {
    const NAME: &str = "灰镜";
}

impl std::fmt::Display for GrayMirror {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Self::NAME.fmt(f)
    }
}

impl App for GrayMirror {
    async fn ignite(&mut self, _context: crate::context::Context) -> anyhow::Result<()> {
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
        Ok(())
    }

    /// Occurs when a message is updated.
    async fn message_edited(&mut self, context: Context, msg: MessageExt) -> Result<()> {
        self.message_recv(context.clone(), msg).await?;
        Ok(())
    }
}
