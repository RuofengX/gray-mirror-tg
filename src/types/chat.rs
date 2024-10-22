use sea_orm::{entity::prelude::*, Set};
use serde::{Deserialize, Serialize};

use super::{MessageExt, Source, SourceType};

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
    pub id: i64,
    pub ty: ChatType,
    pub name: String,
    #[sea_orm(unique)]
    pub chat_id: i64,
    pub source: SourceType,
    pub source_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl ActiveModel {
    pub fn from_msg(msg: &MessageExt, source: &Source) -> Self {
        let chat = msg.inner.chat();
        Self {
            ty: Set((&chat).into()),
            name: Set(chat.name().to_string()),
            chat_id: Set(chat.id()),
            source: Set(source.ty),
            source_id: Set(source.id),
            ..Default::default()
        }
    }
}
