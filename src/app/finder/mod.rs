use std::{
    fmt::{Display, Formatter},
    sync::Arc,
    time::Duration,
};

use crate::{
    context::Context,
    types::{search, Source},
};

use anyhow::Result;
use engine::Engine;
use sea_orm::Set;
use soso::SosoScraper;
use tokio::sync::Mutex;

use super::{App, Updater};

pub mod engine;
pub mod soso;
pub mod watchdog;

pub const KEYWORDS: [&str; 3] = [
    "柏盛",
    "园区",
    "担保",
    // "世纪园区",
    // "金州园区",
    // "金帝园区",
    // "东风园区",
];
pub const SEARCH_ENGINE: &str = "SOSO";
// TODO: Add bot list

#[derive(Debug)]
pub struct Finder {
    engine: Engine,
}

impl Display for Finder {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.write_str("关键词搜索")?;
        Ok(())
    }
}

impl App for Finder {
    async fn ignite(&mut self, context: Context) -> Result<()> {
        for key in KEYWORDS {
            // 新建搜索
            let search = search::ActiveModel {
                bot: Set(SEARCH_ENGINE.to_string()),
                start_time: Set(chrono::Local::now().naive_local()),
                keyword: Set(key.to_string()),
                ..Default::default()
            };
            let search = context.persist.put_search(search).await?;
            let source = Source::from_search(&search);

            // 时间同步量
            let time_sync = Arc::new(Mutex::new(tokio::time::Instant::now()));

            // 启动WD
            let watchdog = watchdog::Watchdog::new(self.engine, key, time_sync.clone());
            context
                .add_background_task(
                    &format!("{}", &watchdog),
                    watchdog.background_task(context.clone()),
                )
                .await;

            // 启动更新器
            context
                .add_updater(SosoScraper::new(
                    context.clone(),
                    key,
                    source,
                    time_sync.clone(),
                ))
                .await?;

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
        Ok(())
    }
}

impl Updater for Finder {}

impl Finder {
    pub fn new(engine: Engine) -> Self {
        Self { engine }
    }
}
