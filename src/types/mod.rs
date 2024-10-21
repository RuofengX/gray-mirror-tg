use serde::{Deserialize, Serialize};

pub mod message;
pub mod link;

pub use message::Model as Message;
pub use link::Model as Link;


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
