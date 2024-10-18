use std::fmt::Display;

use anyhow::Result;
use async_trait::async_trait;
use grammers_client::{types::Message, Client};

use super::{App, Updater};

#[derive(Debug, Default)]
pub struct PrintAll {}
impl PrintAll {
    fn filter(msg: &Message) -> bool {
        !msg.outgoing()
    }
}

impl Display for PrintAll{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Print All")?;
        Ok(())
    }
}

impl App for PrintAll {}

#[async_trait]
impl Updater for PrintAll {
    async fn new_message(&mut self, client: &Client, msg: Message) -> Result<()> {
        if !Self::filter(&msg) {
            return Ok(());
        }
        println!("{}", msg.raw.out);
        let info = serde_json::to_string_pretty(&msg.raw.message)?;
        let _ = client;
        println!("new msg: {}", info);
        Ok(())
    }
    async fn message_edited(&mut self, client: &Client, msg: Message) -> Result<()> {
        if !Self::filter(&msg) {
            return Ok(());
        }
        let _ = client;
        println!("msg edit: {}", msg.raw.message);
        Ok(())
    }
}
