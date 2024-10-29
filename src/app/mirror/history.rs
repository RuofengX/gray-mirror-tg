use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use async_trait::async_trait;
use grammers_client::types::PackedChat;
use sea_orm::{EntityTrait, PaginatorTrait};
use tracing::{info, warn};

use crate::{
    chat::{self, PackedChatOnly},
    message, App, Context, PrintError, Runable, Source,
};

pub struct History {
    packed_chat: PackedChat,
    limit: usize,
}
impl History {
    pub fn new(packed_chat: PackedChat, limit: usize) -> Self {
        Self { packed_chat, limit }
    }
}
#[async_trait]
impl Runable for History {
    fn name(&self) -> &'static str {
        "历史消息镜像"
    }
    async fn run(&mut self, ctx: Context) -> Result<()> {
        let limit = self.limit;

        // 获取历史迭代器
        let mut history = ctx
            .client
            .iter_messages(self.packed_chat)
            .limit(limit)
            .max_date(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("时间不够倒退")
                    .as_secs() as i32,
            );

        // 循环前的准备
        let chat_id = self.packed_chat.id;
        let source = Source::from_chat(chat_id);
        let mut count = 0;
        warn!(chat_id, limit, "获取聊天记录-开始");

        while let Some(Some(msg)) = history.next().await.unwrap_or_warn() {
            ctx.interval.find_msg.tick().await;
            count += 1;
            info!(chat_id, count, limit, "获取聊天记录");
            ctx.persist
                .put_message(message::ActiveModel::from_inner_msg(&msg, source))
                .await?;
        }

        if count <= limit {
            info!(chat_id, count, limit, "聊天记录提前结束");
        }
        warn!(chat_id, count, limit, "获取聊天记录-结束");
        Ok(())
    }
}

pub struct PassiveHistory {
    limit: usize,
}
impl PassiveHistory {
    pub fn new(limit: usize) -> Self {
        PassiveHistory { limit }
    }
}

#[async_trait]
impl Runable for PassiveHistory {
    fn name(&self) -> &'static str {
        "增量历史消息镜像"
    }
    async fn run(&mut self, ctx: Context) -> Result<()> {
        let limit = self.limit;
        let mut rx = ctx.channel.fetch_history.subscribe();

        loop {
            let packed_chat = rx.recv().await?;
            ctx.add_runable(History::new(packed_chat, limit)).await;
        }
    }
}

pub struct FullHistory {
    limit: usize,
}
impl FullHistory {
    pub fn new(limit: usize) -> Self {
        FullHistory { limit }
    }
}

impl App for FullHistory {
    fn name(&self) -> &'static str {
        "全量历史消息镜像"
    }
    async fn ignite(&mut self, ctx: Context) -> Option<()> {
        let db = &ctx.persist.db;
        let mut chat_iter = chat::Entity::find()
            .into_partial_model::<PackedChatOnly>()
            .paginate(db, 16);

        while let Some(chats) = chat_iter.fetch_and_next().await.unwrap_or_log().flatten() {
            for packed_chat in chats
                .iter()
                .map(|m| m.packed().unwrap_or_warn())
                .filter(|m| m.is_some())
                .map(|m| m.unwrap())
                .into_iter()
            {
                ctx.add_runable(History::new(packed_chat, self.limit)).await;
            }
        }
        Some(())
    }
}
