use std::{fmt::Display, time::Duration};

use crate::{
    app::{App, Updater},
    context::Context,
    types::{MirrorMessage, Source},
};
use anyhow::Result;
use async_trait::async_trait;
use grammers_client::{
    grammers_tl_types::{
        enums::InputPeer, functions::messages::GetBotCallbackAnswer, types::InputPeerChat,
    },
    session::PackedType,
    types::{Message, PackedChat},
    Client,
};
use tracing::{debug, info_span};

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
    async fn message_recv(&mut self, _client: &Client, msg: MirrorMessage) -> Result<()> {
        let new_span = info_span!("处理新消息");
        let _span = new_span.enter();
        // TODO

        msg.extract_links(Source::search("KK园区"));
        msg.extract_inline_buttoms();

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

    fn filter_word(&self) -> Option<&str> {
        Some("关键词：")
    }
}

impl SosoScraper {
    pub async fn press_next_page_buttom(&self, client: &Client, msg: &Message) -> Result<()> {
        tokio::time::sleep(Duration::from_secs(8)).await;
        let a = client
            .invoke(&GetBotCallbackAnswer {
                game: false,
                peer: InputPeer::Chat(InputPeerChat {
                    chat_id: msg.chat().id(),
                }),
                msg_id: msg.id(),
                data: None, // TODO
                password: None,
            })
            .await?;
        debug!("{:?}", a);

        Ok(())
    }
}
