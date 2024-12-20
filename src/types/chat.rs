use anyhow::Result;
use grammers_client::types::PackedChat;
use sea_orm::{entity::prelude::*, FromQueryResult, Set};
use serde::{Deserialize, Serialize};

use super::{Source, SourceType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum ChatType {
    #[sea_orm(string_value = "user")]
    User,
    #[sea_orm(string_value = "group")]
    Group,
    #[sea_orm(string_value = "channel")]
    Channel,
}

impl From<&grammers_client::types::Chat> for ChatType {
    fn from(value: &grammers_client::types::Chat) -> Self {
        match value {
            grammers_client::types::Chat::User(_) => ChatType::User,
            grammers_client::types::Chat::Group(_) => ChatType::Group,
            grammers_client::types::Chat::Channel(_) => ChatType::Channel,
        }
    }
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "chat")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub chat_id: i64,
    pub ty: ChatType,
    pub usernames: Vec<String>,
    pub name: String,
    pub packed: String,
    pub source: SourceType,
    pub source_id: i64,
    pub joined: bool,
    pub last_update: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl ActiveModel {
    pub fn from_chat(chat: &grammers_client::types::Chat, joined: bool, source: Source) -> Self {
        let usernames = chat
            .username()
            .map(|username| vec![username])
            .unwrap_or_else(|| chat.usernames())
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        Self {
            chat_id: Set(chat.id()),
            ty: Set(chat.into()),
            usernames: Set(usernames),
            name: Set(chat.name().to_string()),
            packed: Set(chat.pack().to_hex()),
            source: Set(source.ty),
            source_id: Set(source.id),
            joined: Set(joined),
            ..Default::default()
        }
    }
}

impl Model {
    pub fn packed(&self) -> Result<PackedChat> {
        Ok(PackedChat::from_hex(&self.packed)?)
    }
}

#[derive(Debug, DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
pub struct PackedChatOnly {
    packed: String,
}
impl PackedChatOnly {
    pub fn packed(&self) -> Result<PackedChat> {
        Ok(PackedChat::from_hex(&self.packed)?)
    }
}
