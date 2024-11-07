use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, IntoActiveModel, Set};
use tokio::time::Interval;
use tracing::warn;

use crate::{app::History, chat::ChatType, Context, PrintError, Runable};

/// # Process Model
///
///   Latest < ---Joined ---|----Quit  --------------------- > Oldest
///
///   0. sync chat join status
///   1. join oldest-quit chat
///   2. fetch all history
///   3. set update time
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
        let mut ticker = tokio::time::interval(Duration::from_secs(300));

        loop {
            let result = tick(&mut ticker, ctx.clone()).await;
            if result.is_err() {
                result.ok_or_warn();
                continue;
            }
        }
    }
}

async fn tick(ticker: &mut Interval, ctx: Context) -> Result<()> {
    ticker.tick().await;

    // 0. sync chat join status
    ctx.persist.sync_chat_joined(ctx.clone()).await?;

    // 1. join oldest-quit chat
    let oldest_quit = ctx.persist.find_oldest_channel().await?;
    if oldest_quit.is_none() {
        return Ok(());
    }
    let oldest_quit = oldest_quit.unwrap();
    warn!(oldest_quit.chat_id, "周期更新 >> 加入冷频道");
    if oldest_quit.ty == ChatType::Channel {
        ctx.join_quited_chat(oldest_quit.chat_id).await.into_log();
    } else {
        // TODO: group和user类型直接加入会panic导致线程挂掉无法恢复，需要上游调整
    }

    // 2. fetch all history
    warn!(oldest_quit.chat_id, "周期更新 >> 开始获取历史");
    let mut task = History::new(oldest_quit.packed()?, 1000000, oldest_quit.last_update);
    task.run(ctx.clone()).await.into_log();

    // 3. set update time
    warn!(oldest_quit.chat_id, "周期更新 >> 更新时间");
    let mut chat = oldest_quit.into_active_model();
    chat.last_update = Set(Utc::now().naive_utc());
    chat.update(&ctx.persist.db).await?;

    return Ok(());
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
