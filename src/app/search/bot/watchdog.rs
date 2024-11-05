use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::{
    sync::Mutex,
    time::{Instant, Interval},
};
use tracing::{info, warn};

use crate::{app::search::bot::BOT_RESP_TIMEOUT, context::Context, PrintError, Runable};

use super::engine::GenericEngine;

pub struct Watchdog {
    engine: GenericEngine,
    keyword: &'static str,
    last_update: Arc<Mutex<Instant>>,
    bot_resend_tick: Arc<Mutex<Interval>>,
}

impl Watchdog {
    pub fn new(
        engine: GenericEngine,
        keyword: &'static str,
        last_update: Arc<Mutex<Instant>>,
        bot_resend_tick: Arc<Mutex<Interval>>,
    ) -> Self {
        Watchdog {
            engine,
            keyword,
            last_update,
            bot_resend_tick,
        }
    }
}

#[async_trait]
impl Runable for Watchdog {
    fn name(&self) -> &'static str {
        "搜索看门狗"
    }
    async fn run(&mut self, ctx: Context) -> Result<()> {
        self.bot_resend_tick.lock().await.tick().await;
        warn!(engine=self.engine.name, keyword=self.keyword, "发送初始消息");
        ctx.client
            .send_message(self.engine.chat, self.keyword)
            .await?;

        loop {
            let mut last = self.last_update.lock().await;
            if tokio::time::Instant::now() - *last > BOT_RESP_TIMEOUT {
                let engine = self.engine.name;
                let keyword = self.keyword;
                info!(engine, keyword, "搜索超时",);
                info!(engine, keyword, "重发送消息");
                self.bot_resend_tick.lock().await.tick().await;
                ctx.client
                    .send_message(self.engine.chat, self.keyword)
                    .await
                    .ok_or_warn();
                *last = tokio::time::Instant::now();
            }
        }
    }
}
