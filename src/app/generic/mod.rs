use std::fmt::Display;

use anyhow::Result;
use async_trait::async_trait;

use crate::{context::Context, types::MirrorMessage};

use super::{App, Updater};

#[derive(Debug, Default)]
pub struct PrintAll {}

impl Display for PrintAll {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Print All")?;
        Ok(())
    }
}

impl App for PrintAll {}

#[async_trait]
impl Updater for PrintAll {
    async fn message_recv(&mut self, _context: Context, msg: MirrorMessage) -> Result<()> {
        let info = ron::to_string(&msg)?;
        println!("接收到新消息: {}", info);
        Ok(())
    }
    async fn message_edited(&mut self, _context:Context, msg: MirrorMessage) -> Result<()> {
        println!("msg edit: {}", ron::to_string(&msg)?);
        Ok(())
    }
}
