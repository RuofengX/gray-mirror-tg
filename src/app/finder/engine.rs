use grammers_client::{session::PackedType, types::PackedChat};


#[derive(Debug, Clone, Copy)]
pub struct Engine {
    pub name: &'static str,
    pub chat: PackedChat,
}

impl Engine {
    pub const SOSO: Engine = Engine {
        name: "soso",
        chat: PackedChat {
            ty: PackedType::Bot,
            id: 7048419795,
            access_hash: Some(7758671014432728719),
        },
    };
}
