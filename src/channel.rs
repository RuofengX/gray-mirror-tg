use grammers_client::types::PackedChat;
use tokio::sync::broadcast;

type ChatType = PackedChat;

pub struct Channel {
    pub fetch_history: broadcast::Sender<ChatType>
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            fetch_history: broadcast::channel(1024).0,
        }
    }
}
