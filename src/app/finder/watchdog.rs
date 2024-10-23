use std::{fmt::Display, sync::Arc};

use anyhow::Result;
use tokio::{sync::Mutex, time::Instant};
use tracing::{info, info_span, warn};

use crate::context::{Context, BOT_RESEND_FREQ, BOT_RESP_TIMEOUT};

use super::engine::Engine;

pub struct Watchdog {
    engine: Engine,
    keyword: &'static str,
    last_update: Arc<Mutex<Instant>>,
}

impl Display for Watchdog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}_{}_WD", self.engine.name, self.keyword))?;
        Ok(())
    }
}

impl Watchdog {
    pub fn new(engine: Engine, keyword: &'static str, last_update: Arc<Mutex<Instant>>) -> Self {
        Watchdog {
            engine,
            keyword,
            last_update,
        }
    }
    pub async fn background_task(self, ctx: Context) -> Result<()> {
        let wd_span = info_span!("看门狗");
        let _span = wd_span.enter();

        info!("发送初始消息");
        ctx.client
            .send_message(self.engine.chat, self.keyword)
            .await?;
        let mut freq_limit = tokio::time::interval(BOT_RESEND_FREQ);
        loop {
            freq_limit.tick().await;
            let mut last = self.last_update.lock().await;
            if tokio::time::Instant::now() - *last > BOT_RESP_TIMEOUT {
                warn!("超时 > {}", self);
                info!("发送消息");
                ctx.client
                    .send_message(self.engine.chat, self.keyword)
                    .await?;
                *last = tokio::time::Instant::now();
            }
        }
    }
}
