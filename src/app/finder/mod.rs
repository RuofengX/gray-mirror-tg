use std::{
    fmt::{Display, Formatter},
    time::Duration,
};

use crate::{
    context::Context,
    types::{search, Source},
};

use anyhow::Result;
use grammers_client::types::PackedChat;
use sea_orm::Set;
use soso::{SosoScraper, SOSO};

use super::{App, Updater};

pub mod soso;

pub const KEYWORDS: [&str; 6] = [
    "KK园区",
    "世纪园区",
    "金州园区",
    "金帝园区",
    "东风园区",
    "担保",
];
pub const BOTS: PackedChat = SOSO; // TODO: Add bot list

#[derive(Debug)]
pub struct Finder {}

impl Display for Finder {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.write_str("搜索")?;
        Ok(())
    }
}

impl App for Finder {
    async fn ignite(&mut self, context: Context) -> Result<()> {
        for key in KEYWORDS {
            let search = search::ActiveModel {
                bot: Set("SOSO".to_string()),
                start_time: Set(chrono::Local::now().naive_local()),
                keyword: Set(key.to_string()),
                ..Default::default()
            };
            let search = context.persist.put_search(search).await?;
            let source = Source::from_search(&search);
            context.add_updater(SosoScraper::new(key, source)).await;
            context.client.send_message(soso::SOSO, key).await?;
            tokio::time::sleep(Duration::from_secs(5)).await;
            // TODO: 添加逻辑，当长时间接受不到某一关键词的反馈时，watchdog重新搜索
        }
        Ok(())
    }
}

impl Updater for Finder {}

impl Finder {
    pub fn new() -> Self {
        Self {}
    }
}
