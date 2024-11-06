use std::{sync::Arc, time::Duration};

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
        warn!(
            engine = self.engine.name,
            keyword = self.keyword,
            "发送初始消息"
        );
        ctx.client
            .send_message(self.engine.chat, self.keyword)
            .await?;

        let engine = self.engine.name;
        let keyword = self.keyword;
        let mut count = 0;
        let mut ticker = tokio::time::interval(Duration::from_secs(7));
        loop {
            count += 1;
            ticker.tick().await;
            info!(count, engine, keyword, "WD检测");
            let mut last = self.last_update.lock().await;
            if tokio::time::Instant::now() - *last > BOT_RESP_TIMEOUT {
                info!(count, engine, keyword, "搜索超时",);
                info!(count, engine, keyword, "重发送消息");
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
