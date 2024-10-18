use std::collections::HashSet;

use crate::context::Context;

use anyhow::{anyhow, Result};
use grammers_client::{
    session::PackedType,
    types::{Chat, PackedChat},
    Client,
};
use serde::{Deserialize, Serialize};
use soso::SosoScraper;
use url::Url;

use super::{App, Updater};

pub mod soso;

pub const KEYWORDS: [&str; 6] = ["KK", "世纪", "金州", "金帝", "东风", "担保"];

pub struct Finder {
    client: Client,
}

impl App for Finder {
    async fn ignite(&mut self, context: &mut Context) -> Result<()> {
        println!("Finder启动");
        println!("@soso");
        context.add_app(SosoScraper::default()).await?;
        Ok(())
    }
}

impl Updater for Finder {}

impl Finder {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn find_chat(&self, username: &str) -> Result<Chat> {
        let rtn = self
            .client
            .resolve_username(username)
            .await?
            .ok_or(anyhow!("未能找到用户: {}", username))?;
        Ok(rtn)
    }

    pub async fn join_bot(&self, id: i64) -> Result<PackedChat> {
        let chat = PackedChat {
            ty: PackedType::Bot,
            id,
            access_hash: None,
        };
        self.client.send_message(chat, "/start").await?;
        Ok(chat)
    }

    pub async fn get_soso(&mut self) -> Result<PackedChat> {
        Ok(self.find_chat("soso").await?.pack())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RelatedLink {
    pub link: String,
    pub desc: String,
}
impl PartialEq for RelatedLink {
    fn eq(&self, other: &Self) -> bool {
        self.link == other.link
    }
}
impl RelatedLink {
    pub fn new(link: String, desc: String) -> Self {
        Self { link, desc }
    }
}

pub struct TgMsg{
    // todo
}
impl TryFrom<RelatedLink> for TgMsg{
    type Error = anyhow::Error;

    fn try_from(value: RelatedLink) -> std::result::Result<Self, Self::Error> {
        todo!()
    }
}

