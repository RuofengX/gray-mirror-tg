use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub mod chat;
pub mod link;
pub mod message;
pub mod search;

pub use link::Model;
pub use message::MessageExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum SourceType {
    #[sea_orm(string_value = "-")]
    None,
    #[sea_orm(string_value = "search")]
    Search,
    #[sea_orm(string_value = "link")]
    Link,
    #[sea_orm(string_value = "message")]
    Message,
    #[sea_orm(string_value = "chat")]
    Chat,
    #[sea_orm(string_value = "")]
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Source {
    pub ty: SourceType,
    pub id: i64,
}

impl Source {
    pub fn from_search(search: &search::Model) -> Self {
        Self {
            ty: SourceType::Search,
            id: search.id.into(),
        }
    }

    pub fn from_link(link: &link::Model) -> Self {
        Self {
            ty: SourceType::Link,
            id: link.id.into(),
        }
    }

    pub fn from_message(msg_id: i32) -> Self {
        Self {
            ty: SourceType::Message,
            id: msg_id.into(),
        }
    }

    pub fn from_chat(chat_id: i64) -> Self {
        Self {
            ty: SourceType::Chat,
            id: chat_id,
        }
    }

    pub fn from_manual() -> Self {
        Self {
            ty: SourceType::Manual,
            id: -1,
        }
    }
}
