use anyhow::{anyhow, Result};
use grammers_client::{types::Chat, Client};

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
