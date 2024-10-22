use std::{fmt::Display, sync::Arc, time::Duration};

use anyhow::Result;
use tokio::{sync::RwLock, time::Instant};
use tracing::{info, info_span, warn};

use crate::context::Context;

use super::engine::Engine;

const TIMEOUT: Duration = Duration::from_secs(30);

pub struct Watchdog {
    engine: Engine,
    keyword: &'static str,
    last_update: Arc<RwLock<Instant>>,
}

impl Display for Watchdog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}_{}_WD", self.engine.name, self.keyword))?;
        Ok(())
    }
}

impl Watchdog {
    pub fn new(engine: Engine, keyword: &'static str, last_update: Arc<RwLock<Instant>>) -> Self {
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
        loop {
            tokio::time::sleep(TIMEOUT).await;
            let last = self.last_update.read().await;
            if tokio::time::Instant::now() - *last > TIMEOUT {
                warn!("超时 > {}", self);
                info!("发送消息");
                ctx.client
                    .send_message(self.engine.chat, self.keyword)
                    .await?;

            }
        }
    }
}
