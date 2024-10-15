use anyhow::Result;
use grammers_client::types::{Message, MessageDeletion};

use super::UpdateHandle;

pub struct JustPrint {}

impl UpdateHandle for JustPrint {
    async fn new_message(&mut self, msg: Message) -> Result<()> {
        println!("reveice message: {:x}", msg);
    }

    async fn message_edited(&mut self, msg: Message) -> Result<()> {
        println!("message edited: {:x}", msg);
    }

    async fn message_deletion(&mut self, msg_del: MessageDeletion) -> Result<()> {
        println!("message deletion: {:x}", msg);
    }
}
