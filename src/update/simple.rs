use anyhow::Result;
use grammers_client::{types::{Message, MessageDeletion}, Client};

use super::UpdateHandle;

pub struct JustPrint {
    client: Client
}
impl AsRef<Client> for JustPrint{
    fn as_ref(&self) -> &Client {
        &self.client
    }
}

impl UpdateHandle for JustPrint {
    async fn new_message(&mut self, msg: Message) -> Result<()> {
        println!("reveice message: {:#?}", msg);
        Ok(())
    }

    async fn message_edited(&mut self, msg: Message) -> Result<()> {
        println!("message edited: {:#?}", msg);
        Ok(())
    }

    async fn message_deletion(&mut self, msg_del: MessageDeletion) -> Result<()> {
        println!("message deletion: {:#?}", msg_del);
        Ok(())
    }
}
