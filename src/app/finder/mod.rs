use std::{fmt::{Display, Formatter}, time::Duration};

use crate::context::Context;

use anyhow::Result;
use soso::SosoScraper;

use super::{App, Updater};

pub mod soso;

pub const KEYWORDS: [&str; 6] = ["KK园区", "世纪园区", "金州园区", "金帝园区", "东风园区", "担保"];

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
        for i in KEYWORDS {
            context.add_updater(SosoScraper::new(i)).await;
            context.client.send_message(soso::SOSO, i).await?;
            tokio::time::sleep(Duration::from_secs(10)).await;
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
