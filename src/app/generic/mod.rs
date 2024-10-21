use std::fmt::Display;

use anyhow::Result;
use async_trait::async_trait;

use crate::{context::Context, types::MessageExt};

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
    async fn message_recv(&mut self, _context: Context, msg: MessageExt) -> Result<()> {
        println!("接收到新消息: {}", msg.text());
        Ok(())
    }
    async fn message_edited(&mut self, _context: Context, msg: MessageExt) -> Result<()> {
        println!("消息修改: {}", msg.text());
        Ok(())
    }
}
