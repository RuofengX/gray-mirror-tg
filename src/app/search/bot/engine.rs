use std::{sync::Arc, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use grammers_client::{session::PackedType, types::PackedChat};
use tokio::{sync::Mutex, time::Instant};
use tracing::info;

use crate::{message, Context, MessageExt, Source, Updater};

#[derive(Debug, Clone, Copy)]
pub struct GenericEngine {
    pub name: &'static str,
    pub chat: PackedChat,
}

impl GenericEngine {
    pub const SOSO: GenericEngine = GenericEngine {
        name: "SOSO",
        chat: PackedChat {
            ty: PackedType::Bot,
            id: 7048419795,
            access_hash: Some(7758671014432728719),
        },
    };

    pub const JISOU: GenericEngine = GenericEngine {
        name: "jiso2bot",
        chat: PackedChat {
            ty: PackedType::Bot,
            id: 6213379764,
            access_hash: Some(7074953819817629361),
        },
    };

    pub async fn new(username: &'static str, ctx: Context) -> Result<Option<Self>> {
        if let Some(chat) = ctx.resolve_username(username).await? {
            Ok(Some(Self {
                name: username,
                chat: chat.pack(),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn start_search(
        &self,
        keyword: &'static str,
        source: Source,
        time_sync: Arc<Mutex<Instant>>,
    ) -> impl Updater {
        Scraper::new(self.clone(), keyword, source, time_sync)
    }
}

#[derive(Debug)]
pub struct Scraper {
    pub keyword: &'static str,
    pub source: Source,
    pub engine: GenericEngine,
    last_update: Arc<Mutex<Instant>>,
}

impl Scraper {
    pub fn new(
        engine: GenericEngine,
        keyword: &'static str,
        source: Source,
        last_update: Arc<Mutex<Instant>>,
    ) -> Self {
        Scraper {
            keyword,
            source,
            engine,
            last_update,
        }
    }
}

#[async_trait]
impl Updater for Scraper {
    fn name(&self) -> &'static str {
        self.engine.name
    }
    async fn message_recv(&mut self, context: Context, msg: MessageExt) -> Result<()> {
        let msg_id = context
            .persist
            .put_message(message::ActiveModel::from_inner_msg(
                &msg.inner,
                self.source,
            ))
            .await?
            .msg_id;

        let link_source = Source::from_message(msg_id);
        for link in msg.links() {
            info!(desc=link.desc, "接收链接");
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

        msg.inner.mark_as_read().await.ok();

        Ok(())
    }

    async fn message_edited(&mut self, client: Context, msg: MessageExt) -> Result<()> {
        self.message_recv(client, msg).await?;
        Ok(())
    }

    fn filter_incoming(&self) -> bool {
        true
    }

    fn filter_chat_id(&self) -> Option<i64> {
        Some(self.engine.chat.id)
    }

    fn filter_word(&self) -> Option<String> {
        Some(format!("{}", self.keyword))
    }
}
