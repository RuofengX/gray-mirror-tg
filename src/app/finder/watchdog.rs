use std::{fmt::Display, sync::Arc, time::Duration};

use anyhow::Result;
use tokio::{sync::RwLock, time::Instant};

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
        ctx.client
            .send_message(self.engine.chat, self.keyword)
            .await?;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let last = self.last_update.read().await;
            if tokio::time::Instant::now() - *last > TIMEOUT {
                ctx.client
                    .send_message(self.engine.chat, self.keyword)
                    .await?;
            }
        }
    }
}
