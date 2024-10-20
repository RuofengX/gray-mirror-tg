use std::fmt::Display;

use crate::{
    app::{App, Updater},
    context::Context,
};
use anyhow::Result;
use async_trait::async_trait;
use grammers_client::{
    grammers_tl_types::enums::MessageEntity,
    session::PackedType,
    types::{Message, PackedChat},
    Client,
};
use tracing::{info_span, warn, warn_span};

use super::RelatedLink;

pub const SOSO: PackedChat = PackedChat {
    ty: PackedType::Bot,
    id: 7048419795,
    access_hash: Some(7758671014432728719),
};

#[derive(Debug, Default)]
pub struct SosoScraper;

impl Display for SosoScraper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SOSO Scraper")?;
        Ok(())
    }
}
impl App for SosoScraper {
    async fn ignite(&mut self, context: &mut Context) -> Result<()> {
        // context.client.send_message(SOSO, "/start").await?;
        // tokio::time::sleep(Duration::from_secs(3)).await;
        context.client.send_message(SOSO, "KK园区").await?;
        // context.client.send_message(SOSO, "KK园区").await?;
        Ok(())
    }
}

#[async_trait]
impl Updater for SosoScraper {
    async fn new_message(&mut self, client: &Client, msg: Message) -> Result<()> {
        let new_span = info_span!("处理新消息");
        let _span = new_span.enter();

        if !Self::fliter(&msg) {
            return Ok(());
        }
        let mut rtn = Vec::new();
        SosoScraper::extract_link(&msg, &mut rtn)?;

        for i in rtn {
            let fetch_span = warn_span!("发现关联链接");
            let _span = fetch_span.enter();
            warn!(stage = "数据发现", "{}", i);
        }
        // TODO

        let _ = client;
        Ok(())
    }

    async fn message_edited(&mut self, client: &Client, msg: Message) -> Result<()> {
        self.new_message(client, msg).await?;
        Ok(())
    }
}

impl SosoScraper {
    fn fliter(msg: &Message) -> bool {
        msg.chat().id() == SOSO.id && !msg.outgoing() && msg.text().contains("关键词：")
    }

    fn extract_link(msg: &Message, writer: &mut Vec<RelatedLink>) -> Result<()> {
        let words: Vec<u16> = msg.raw.message.encode_utf16().collect();

        if let Some(ref ents) = msg.raw.entities {
            for ent in ents {
                match ent {
                    MessageEntity::TextUrl(url) => {
                        let link = url.url.clone();

                        let offset = url.offset as usize;
                        let len = url.length as usize;

                        let desc = String::from_utf16(&words[offset..offset + len])?;
                        writer.push(RelatedLink::new(link, desc));
                    }
                    _ => (),
                }
            }
        }
        Ok(())
    }

    pub async fn press_next_page_buttom(&self, client: &Client, msg: &Message) -> Result<()> {
        let _ = client;
        let _ = msg;
        // TODO: inline query
        Ok(())
    }
}
