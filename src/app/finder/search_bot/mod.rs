use std::{sync::Arc, time::Duration};

use crate::{
    context::Context,
    types::{search, Source},
    Runable,
};

use anyhow::Result;
use async_trait::async_trait;
use engine::Engine;
use sea_orm::Set;
use soso::SosoScraper;
use tokio::sync::Mutex;

use crate::app::App;

pub mod engine;
pub mod soso;
pub mod watchdog;

pub const KEYWORDS: [&str; 3] = [
    "柏盛", "园区",
    "担保",
    // "世纪园区",
    // "金州园区",
    // "金帝园区",
    // "东风园区",
];
pub const SEARCH_ENGINE: &str = "SOSO";
// TODO: Add bot list

#[derive(Debug)]
pub struct Search {
    engine: Engine,
}

#[async_trait]
impl Runable for Search {
    fn name(&self) -> &'static str {
        "关键词搜索"
    }
    async fn run(&mut self, ctx: Context) -> Result<()> {
        Ok(())
    }
}

impl App for Search {
    async fn ignite(&mut self, ctx: Context) -> Result<()> {
        for key in KEYWORDS {
            // 新建搜索
            let search = search::ActiveModel {
                bot: Set(SEARCH_ENGINE.to_string()),
                start_time: Set(chrono::Local::now().naive_local()),
                keyword: Set(key.to_string()),
                ..Default::default()
            };
            let search = ctx.persist.put_search(search).await?;
            let source = Source::from_search(&search);

            // 时间同步量
            let time_sync = Arc::new(Mutex::new(tokio::time::Instant::now()));
            // 启动WD
            let watchdog = watchdog::Watchdog::new(self.engine, key, time_sync.clone());
            ctx.add_runable(watchdog).await;

            // 启动更新处理器
            ctx.add_update_parser(SosoScraper::new(&key, source, time_sync)).await;

            tokio::time::sleep(Duration::from_secs(15)).await;
        }
        Ok(())
    }
}

impl Search {
    pub fn new(engine: Engine) -> Self {
        Self { engine }
    }
}
impl Default for Search {
    fn default() -> Self {
        Self {
            engine: Engine::SOSO,
        }
    }
}
