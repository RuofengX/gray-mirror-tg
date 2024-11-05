use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;

use crate::{Context, Runable};

pub struct Sentence {}

#[async_trait]
impl Runable for Sentence {
    fn name(&self) -> &'static str {
        "周期更新冷历史记录"
    }
    async fn run(&mut self, ctx: Context) -> Result<()> {
        let mut ticker = tokio::time::interval(Duration::from_secs(10));

        loop {
            ticker.tick().await;

            let chat = ctx.persist.find_quit_candidate().await?;
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
