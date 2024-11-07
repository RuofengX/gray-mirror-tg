use anyhow::Result;
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use grammers_client::types::PackedChat;
use tracing::{info, warn};

use crate::{message, Context, PrintError, Runable, Source};

pub struct History {
    packed_chat: PackedChat,
    limit: usize,
    since: NaiveDateTime,
}
impl History {
    pub fn new(packed_chat: PackedChat, limit: usize, since: NaiveDateTime) -> Self {
        Self {
            packed_chat,
            limit,
            since,
        }
    }
}
#[async_trait]
impl Runable for History {
    fn name(&self) -> &'static str {
        "历史消息镜像"
    }
    async fn run(&mut self, ctx: Context) -> Result<()> {
        let limit = self.limit;
        let last_update = self.since.and_utc().timestamp() as i64;
        let now = Utc::now().timestamp() as i64;
        let delta_time = now - last_update;

        // 获取历史迭代器
        let mut history = ctx
            .client
            .iter_messages(self.packed_chat)
            .limit(limit)
            .max_date(delta_time as i32);

        // 循环前的准备
        let chat_id = self.packed_chat.id;
        let source = Source::from_chat(chat_id);
        let mut count = 0;
        warn!(chat_id, limit, delta_time, "获取聊天记录-开始");

        while let Some(Some(msg)) = history.next().await.ok_or_warn() {
            ctx.interval.find_msg.tick().await;
            count += 1;
            info!(chat_id, count, limit, delta_time, "获取聊天记录");
            ctx.persist
                .put_message(message::ActiveModel::from_inner_msg(&msg, source))
                .await?;
        }

        if count <= limit {
            info!(chat_id, count, limit, delta_time, "聊天记录提前结束");
        }
        warn!(chat_id, count, limit, delta_time, "获取聊天记录-结束");
        Ok(())
    }
}
