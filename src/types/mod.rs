use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

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
    // TODO: 添加群组爬虫的来源
}

#[derive(Debug, Clone)]
pub struct Source {
    ty: SourceType,
    id: i32,
}

impl Source {
    pub fn from_search(search: &search::Model) -> Self {
        Self {
            ty: SourceType::Search,
            id: search.id,
        }
    }

    pub fn from_link(link: &link::Model) -> Self {
        Self {
            ty: SourceType::Link,
            id: link.id,
        }
    }

    pub fn from_message(msg_id: i32) -> Self {
        Self {
            ty: SourceType::Message,
            id: msg_id,
        }
    }
}
impl Into<(SourceType, i32)> for Source {
    fn into(self) -> (SourceType, i32) {
        (self.ty, self.id)
    }
}
