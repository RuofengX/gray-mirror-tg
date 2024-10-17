use anyhow::{anyhow, Result};
use grammers_client::{types::Chat, Client};

pub const SEARCH_ENGINE: [i64; 1] = [
    7048419795, // SOSO机器人
];

pub struct Finder {
    client: Client,
}
impl From<&Client> for Finder {
    fn from(value: &Client) -> Self {
        let client = value.clone();
        Finder { client }
    }
}
impl Finder {
    pub async fn find_chat(&self, username: &str) -> Result<Chat> {
        let rtn = self
            .client
            .resolve_username(username)
            .await?
            .ok_or(anyhow!("user: {} not found", username))?;
        Ok(rtn)
    }
}
