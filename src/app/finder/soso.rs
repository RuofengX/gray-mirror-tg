use std::{fmt::Display, time::Duration};

use crate::{
    app::Updater,
    context::Context,
    types::{message, MessageExt, Source},
};
use anyhow::Result;
use async_trait::async_trait;
use grammers_client::{session::PackedType, types::PackedChat};
use tracing::info_span;

pub const SOSO: PackedChat = PackedChat {
    ty: PackedType::Bot,
    id: 7048419795,
    access_hash: Some(7758671014432728719),
};

#[derive(Debug)]
pub struct SosoScraper {
    pub keyword: &'static str,
    pub source: Source,
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
                    .click_callback_button(&context.client, &btn, Duration::from_secs(1))
                    .await;
                // 搜搜机器人不会有返回值，而是直接修改消息内容，直接忽略
            }
        }

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
        Some(&[SOSO.id])
    }

    fn filter_word(&self) -> Option<String> {
        Some(format!("关键词：{}", self.keyword))
    }
}

impl SosoScraper {
    pub fn new(keyword: &'static str, source: Source) -> Self {
        Self { keyword, source }
    }
}
