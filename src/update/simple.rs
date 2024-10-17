use anyhow::Result;
use async_trait::async_trait;
use grammers_client::{
    types::{CallbackQuery, InlineQuery, InlineSend, Message, MessageDeletion},
    Client,
};

use super::UpdateHandle;

#[derive(Debug, Default)]
pub struct PrintConfig {}

#[async_trait]
impl UpdateHandle for PrintConfig {
    async fn new_message(&mut self, _client: &Client, msg: Message) -> Result<()> {
        println!("reveice message: {:#?}", msg);
        Ok(())
    }

    async fn message_edited(&mut self, _client: &Client, msg: Message) -> Result<()> {
        println!("message edited: {:#?}", msg);
        Ok(())
    }

    async fn message_deletion(&mut self, _client: &Client, msg_del: MessageDeletion) -> Result<()> {
        println!("message deletion: {:#?}", msg_del);
        Ok(())
    }
    async fn callback_query(
        &mut self,
        _client: &Client,
        callback_query: CallbackQuery,
    ) -> Result<()> {
        println!("callback query: {:#?}", callback_query);
        Ok(())
    }

    async fn inline_query(&mut self, _client: &Client, inline_query: InlineQuery) -> Result<()> {
        println!("inline query: {:#?}", inline_query);
        Ok(())
    }

    async fn inline_send(&mut self, _client: &Client, inline_send: InlineSend) -> Result<()> {
        println!("inline send: {:#?}", inline_send);
        Ok(())
    }
}
