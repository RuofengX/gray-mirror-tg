use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use grammers_client::{
    session::PackedType,
    types::{Message, PackedChat},
    Client,
};

use crate::{
    app::{App, PrintAll, Updater},
    context::Context,
};

pub const SOSO: PackedChat = PackedChat {
    ty: PackedType::Bot,
    id: 7048419795,
    access_hash: Some(7758671014432728719),
};

#[derive(Debug, Default)]
pub struct ParseSOSO;

impl App for ParseSOSO {
    async fn ignite(&mut self, context: &mut Context) -> Result<()> {
        context.add_app(PrintAll::default()).await?;
        context.client.send_message(SOSO, "/start").await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        context.client.send_message(SOSO, "KK园区").await?;
        // context.client.send_message(SOSO, "KK园区").await?;
        Ok(())
    }

}

#[async_trait]
impl Updater for ParseSOSO {
    async fn new_message(&mut self, client: &Client, msg: Message) -> Result<()> {
        if !Self::filter(&msg) {
            return Ok(());
        }
        // TODO
        let _ = client;
        Ok(())
    }
}

impl ParseSOSO {
    fn filter(msg: &Message) -> bool {
        msg.chat().id() == SOSO.id
    }

    async fn press_next_page_buttom(&self, client: &Client, msg: &Message) -> Result<()>{
        Ok(())

    }
}