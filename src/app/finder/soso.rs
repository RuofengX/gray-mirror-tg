use std::{fmt::Display, sync::Arc, time::Duration};

use crate::{
    app::Updater,
    context::Context,
    types::{message, MessageExt, Source},
};
use anyhow::Result;
use async_trait::async_trait;
use tokio::{sync::RwLock, time::Instant};
use tracing::info_span;

use super::engine::Engine;

#[derive(Debug)]
pub struct SosoScraper {  //TODO: 改为通用型搜索，
    pub keyword: &'static str,
    pub source: Source,
    last_update: Arc<RwLock<Instant>>,
}

impl SosoScraper {
    pub const ENGINE: Engine = Engine::SOSO;
    pub fn new(
        _context: Context,
        keyword: &'static str,
        source: Source,
        last_update: Arc<RwLock<Instant>>,
    ) -> Self {
        SosoScraper {
            keyword,
            source,
            last_update,
        }
    }
}

impl Display for SosoScraper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("SOSO_{}", self.keyword))?;
        Ok(())
    }
}

#[async_trait]
impl Updater for SosoScraper {
    async fn message_recv(&mut self, context: Context, msg: MessageExt) -> Result<()> {
        let new_span = info_span!("处理新消息");
        let _span = new_span.enter();

        context
            .persist
            .put_message(message::ActiveModel::from_msg(&msg, &self.source))
            .await?;
        let links = msg.links();
        context.persist.put_link_vec(links).await?;

        let buttons = msg.callback_buttons();
        for btn in buttons {
            if btn.text.contains("下一页") || btn.text.contains("➡️") {
                let _ = msg
                    .click_callback_button(&context.client, &btn, Duration::from_secs(10))
                    .await;
                // 搜搜机器人不会有返回值，而是直接修改消息内容，直接忽略
            }
        }

        let mut last = self.last_update.write().await;
        *last = Instant::now();

        // TODO

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
