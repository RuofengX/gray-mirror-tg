use std::fmt::{Display, Formatter};

use crate::context::Context;

use anyhow::Result;
use soso::SosoScraper;

use super::{App, Updater};

pub mod soso;

pub const KEYWORDS: [&str; 6] = ["KK", "世纪", "金州", "金帝", "东风", "担保"];

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
        context.add_app(SosoScraper::new("KK园区")).await?;
        Ok(())
    }
}

impl Updater for Finder {}

impl Finder {
    pub fn new() -> Self {
        Self {}
    }
}
