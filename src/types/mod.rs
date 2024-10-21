use serde::{Deserialize, Serialize};
use sea_orm::entity::prelude::*;

pub mod message;
pub mod link;

pub use message::Message as Message;
pub use link::Link as Link;


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GenericData{
    Message(Message),
    Link(Link),
}
impl From<Message> for GenericData{
    fn from(value: Message) -> Self {
        GenericData::Message(value)
    }
}

impl From<Link> for GenericData{
    fn from(value: Link) -> Self {
        GenericData::Link(value)
    }
}

#[derive(EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum PeerType{
    #[sea_orm(string_value = "chat")]
    Chat,
    #[sea_orm(string_value = "channel")]
    Channel,
    #[sea_orm(string_value = "user")]
    User,
}