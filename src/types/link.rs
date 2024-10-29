use std::fmt::{Display, Formatter};

use sea_orm::{entity::prelude::*, ActiveValue::NotSet, Set};

use super::{Source, SourceType};

#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "link")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub link: String,
    pub desc: String,
    pub source: SourceType,
    pub source_id: i64,
    pub parsed: bool,
    pub packed: Option<String>,
}
// TODO: 加一个check_at字段，超过一段时间就再检查一遍

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        self.link == other.link
    }
}

impl Display for Model {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.desc.fmt(f)
    }
}

pub struct Link {
    pub link: String,
    pub desc: String,
}

impl Link {
    pub fn to_model(self, source: &Source) -> ActiveModel {
        ActiveModel {
            id: NotSet,
            link: Set(self.link),
            desc: Set(self.desc),
            source: Set(source.ty),
            source_id: Set(source.id),
            parsed: Set(false),
            packed: Set(None),
        }
    }
}
