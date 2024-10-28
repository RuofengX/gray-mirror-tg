use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::{sync::Mutex, time::Instant};
use tracing::{info, info_span, warn};

use crate::{
    context::{Context, BOT_RESP_TIMEOUT},
    PrintError, Runable,
};

use super::engine::Engine;

pub struct Watchdog {
    engine: Engine,
    keyword: &'static str,
    last_update: Arc<Mutex<Instant>>,
}

impl Watchdog {
    pub fn new(engine: Engine, keyword: &'static str, last_update: Arc<Mutex<Instant>>) -> Self {
        Watchdog {
            engine,
            keyword,
            last_update,
        }
    }
}

#[async_trait]
impl Runable for Watchdog {
    fn name(&self) -> &'static str {
        "搜索看门狗"
    }
    async fn run(&mut self, ctx: Context) -> Result<()> {
        info!("发送初始消息");
        ctx.client
            .send_message(self.engine.chat, self.keyword)
            .await?;

        loop {
            ctx.interval.bot_resend.tick().await;
            let mut last = self.last_update.lock().await;
            if tokio::time::Instant::now() - *last > BOT_RESP_TIMEOUT {
                info!(keyword = self.keyword, "搜索超时",);
                info!("重发送消息");
                ctx.client
                    .send_message(self.engine.chat, self.keyword)
                    .await
                    .unwrap_or_warn();
                *last = tokio::time::Instant::now();
            }
        }
    }
}
