use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, IntoActiveModel, Set};
use tracing::warn;

use crate::{app::History, Context, Runable};

/// # Process Model
/// 
///   Latest < ---Joined ---|----Quit  --------------------- > Oldest
///
///   0. sync chat join status
///   1. join oldest-quit chat
///   2. fetch all history
///   3. set update time
///   4. quit oldest-joined chat
///
pub struct Sentence {}
impl Sentence {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Runable for Sentence {
    fn name(&self) -> &'static str {
        "周期更新冷历史记录"
    }
    async fn run(&mut self, ctx: Context) -> Result<()> {
        let mut ticker = tokio::time::interval(Duration::from_secs(10));

        loop {
            ticker.tick().await;
            // 0. sync chat join status
            ctx.persist.sync_chat_joined(ctx.clone()).await?;

            // 1.
            let oldest_quit = ctx.persist.find_oldest_chat(Some(false)).await?;
            if oldest_quit.is_none() {
                return Ok(());
            }
            let chat = oldest_quit.unwrap();

            // 2.
            let mut task = History::new(chat.packed()?, 1000000, chat.last_update);
            task.run(ctx.clone()).await?;

            // 3.
            let mut chat = chat.into_active_model();
            chat.last_update = Set(Utc::now().naive_utc());
            chat.update(&ctx.persist.db).await?;

            // 4. 
            if let Some(oldest_joined) = ctx.persist.find_oldest_chat(Some(true)).await? {
                ctx.quit_chat(oldest_joined.packed()?).await?;
            } else {
                warn!("未能找到最老的已加入聊天");
            }
        }
    }
}

pub struct SyncChat {}
impl SyncChat {
    pub fn new() -> Self {
        Self {}
    }
}
#[async_trait]
impl Runable for SyncChat {
    fn name(&self) -> &'static str {
        "同步聊天状态"
    }
    async fn run(&mut self, ctx: Context) -> Result<()> {
        ctx.persist.sync_chat_joined(ctx.clone()).await?;
        Ok(())
    }
}
