use std::fmt::Display;

use crate::{
    app::{App, Updater},
    context::Context,
    types::MirrorMessage,
};
use anyhow::Result;
use async_trait::async_trait;
use grammers_client::{session::PackedType, types::PackedChat, Client};
use tracing::info_span;

pub const SOSO: PackedChat = PackedChat {
    ty: PackedType::Bot,
    id: 7048419795,
    access_hash: Some(7758671014432728719),
};

#[derive(Debug)]
pub struct SosoScraper {
    pub keyword: &'static str,
}

impl Display for SosoScraper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("SOSO爬虫-{}", self.keyword))?;
        Ok(())
    }
}
impl App for SosoScraper {
    async fn ignite(&mut self, context: &mut Context) -> Result<()> {
        // context.client.send_message(SOSO, "/start").await?;
        // tokio::time::sleep(Duration::from_secs(3)).await;
        context.client.send_message(SOSO, self.keyword).await?;
        // context.client.send_message(SOSO, "KK园区").await?;
        Ok(())
    }
}

#[async_trait]
impl Updater for SosoScraper {
    async fn message_recv(&mut self, client: &Client, msg: MirrorMessage) -> Result<()> {
        let new_span = info_span!("处理新消息");
        let _span = new_span.enter();

        msg.extract_links(&self);
        let buttons = msg.extract_inline_buttons();
        for btn in buttons {
            if btn.text.contains("下一页") || btn.text.contains("➡️") {
                msg.click_callback_buttons(client, &btn).await?;
            }
        }

        // TODO

        Ok(())
    }

    async fn message_edited(&mut self, client: &Client, msg: MirrorMessage) -> Result<()> {
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
    pub fn new(keyword: &'static str) -> Self {
        Self { keyword }
    }
}
