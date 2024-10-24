use std::{fmt::Display, sync::Arc, time::Duration};

use crate::{
    app::Updater,
    context::Context,
    types::{message, MessageExt, Source},
};
use anyhow::Result;
use async_trait::async_trait;
use tokio::{sync::Mutex, time::Instant};
use tracing::info_span;

use super::engine::Engine;

#[derive(Debug)]
pub struct SosoScraper {
    //TODO: 改为通用型搜索，
    pub keyword: &'static str,
    pub source: Source,
    pub engine: Engine,
    last_update: Arc<Mutex<Instant>>,
}

impl SosoScraper {
    pub const ENGINE: Engine = Engine::SOSO;
    pub fn new(
        _context: Context,
        keyword: &'static str,
        source: Source,
        last_update: Arc<Mutex<Instant>>,
    ) -> Self {
        SosoScraper {
            keyword,
            source,
            engine: Engine::SOSO,
            last_update,
        }
    }
}

impl Display for SosoScraper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}_{}", self.engine.name, self.keyword))?;
        Ok(())
    }
}

#[async_trait]
impl Updater for SosoScraper {
    async fn message_recv(&mut self, context: Context, msg: MessageExt) -> Result<()> {
        let new_span = info_span!("处理新消息");
        let _span = new_span.enter();

        let msg_id = context
            .persist
            .put_message(message::ActiveModel::from_inner_msg(&msg.inner, self.source))
            .await?
            .msg_id;

        let link_source = Source::from_message(msg_id);
        for link in msg.links() {
            context
                .persist
                .put_link(link.to_model(&link_source))
                .await?;
        }

        let buttons = msg.callback_buttons();
        for btn in buttons {
            if btn.text.contains("下一页") || btn.text.contains("➡️") {
                let _ = msg
                    .click_callback_button(&context.client, &btn, Duration::from_secs(10))
                    .await;
                // 搜搜机器人不会有返回值，而是直接修改消息内容，直接忽略
            }
        }

        let mut last = self.last_update.lock().await;
        *last = Instant::now();

        Ok(())
    }

    async fn message_edited(&mut self, client: Context, msg: MessageExt) -> Result<()> {
        self.message_recv(client, msg).await?;
        Ok(())
    }

    fn filter_incoming(&self) -> bool {
        true
    }

    fn filter_chat_id(&self) -> Option<&[i64]> {
        Some(&[Self::ENGINE.chat.id])
    }

    fn filter_word(&self) -> Option<String> {
        Some(format!("关键词：{}", self.keyword))
    }
}
