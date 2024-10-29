use std::{sync::Arc, time::Duration};

use crate::{
    context::Context,
    types::{search, Source},
    App, PrintError,
};

use engine::GenericEngine;
use sea_orm::Set;
use tokio::sync::Mutex;
use tracing::warn;

pub mod engine;
pub mod watchdog;

pub const BOT_RESP_TIMEOUT: Duration = std::time::Duration::from_secs(60);
pub const BOT_RESEND_INTERVAL: Duration = std::time::Duration::from_secs(60);

#[derive(Debug)]
pub struct SearchLink {
    engine: GenericEngine,
    keywords: Vec<&'static str>,
}
impl SearchLink {
    pub fn new(engine: GenericEngine, keywords: impl Iterator<Item = &'static str>) -> Self {
        Self {
            engine,
            keywords: keywords.collect(),
        }
    }
}

impl App for SearchLink {
    fn name(&self) -> &'static str {
        "关键词搜索"
    }
    async fn ignite(&mut self, ctx: Context) -> Option<()> {
        // 新建计时器
        let bot_resend = Arc::new(Mutex::new(tokio::time::interval(BOT_RESEND_INTERVAL)));
        for keyword in &self.keywords {
            // 新建搜索
            warn!(keyword, "新建搜索");
            let search = search::ActiveModel {
                bot: Set(self.engine.name.to_string()),
                start_time: Set(chrono::Local::now().naive_local()),
                keyword: Set(keyword.to_string()),
                ..Default::default()
            };
            let search = ctx.persist.put_search(search).await.unwrap_or_log()?;
            let source = Source::from_search(&search);

            // 时间同步量
            let time_sync = Arc::new(Mutex::new(tokio::time::Instant::now()));
            // 启动WD
            let watchdog = watchdog::Watchdog::new(
                self.engine,
                keyword,
                time_sync.clone(),
                bot_resend.clone(),
            );
            ctx.add_runable(watchdog).await;
            // 启动更新处理器
            ctx.add_parser(
                self.engine
                    .start_search(&keyword, source, time_sync.clone()),
            )
            .await;
        }
        Some(())
    }
}
