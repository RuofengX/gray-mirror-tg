use std::fmt::{Display, Formatter};

use crate::context::Context;

use anyhow::{anyhow, Result};
use grammers_client::{
    session::PackedType,
    types::{Chat, PackedChat},
    Client,
};
use soso::SosoScraper;

use super::{App, Updater};

pub mod soso;

pub const KEYWORDS: [&str; 6] = ["KK", "世纪", "金州", "金帝", "东风", "担保"];

#[derive(Debug)]
pub struct Finder {
    client: Client,
}

impl Display for Finder {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.write_str("Finder")?;
        Ok(())
    }
}

impl App for Finder {
    async fn ignite(&mut self, context: &mut Context) -> Result<()> {
        context.add_app(SosoScraper::default()).await?;
        Ok(())
    }
}

impl Updater for Finder {}

impl Finder {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}
