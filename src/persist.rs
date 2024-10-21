use anyhow::Result;
use async_trait::async_trait;
use reqwest::StatusCode;
use tracing::{error, info_span};
use url::Url;

use crate::types::MirrorData;

#[async_trait]
pub trait Persist: Sync + Send {
    async fn push(&self, collection: &str, data: MirrorData) -> ();
    async fn contain(&self, collection: &str, data: &MirrorData) -> Result<bool>;
}

pub struct HTTP {
    client: reqwest::Client,
}
impl HTTP {
    const BASE: &'static str = dotenv_codegen::dotenv!("PERSIST_URL");
    const UA: &'static str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
    pub fn new() -> Self {
        HTTP {
            client: reqwest::Client::new(),
        }
    }
    pub fn base_url() -> Url {
        Url::parse(Self::BASE)
            .expect("PERSIST_URL 解析错误")
            .join("tg")
            .unwrap()
    }
}

#[async_trait]
impl Persist for HTTP {
    /// `/tg/{collection}/push`
    async fn push(&self, collection: &str, data: MirrorData) -> () {
        let push_span = info_span!("投递");
        let _span = push_span.enter();

        let url = HTTP::base_url()
            .join(collection)
            .unwrap()
            .join("push")
            .unwrap();

        let req = self.client.post(url);
        let resp = req
            .header("User-Agent", HTTP::UA)
            .body(serde_json::to_string(&data).unwrap())
            .send()
            .await;

        match resp {
            Ok(resp) => {
                if resp.status() != StatusCode::OK {
                    error!("未成功 >> {}", resp.status());
                }
            }
            Err(e) => {
                error!("未成功 >> {}", e);
            }
        }
    }

    /// `/tg/{collection}/contain`
    async fn contain(&self, collection: &str, data: &MirrorData) -> Result<bool> {
        let contain_span = info_span!("查重");
        let _span = contain_span.enter();
        let url = HTTP::base_url().join(collection)?.join("push")?;

        let req = self.client.post(url);
        let resp = req
            .header("User-Agent", HTTP::UA)
            .body(serde_json::to_string(&data)?)
            .send()
            .await?;

        if resp.status() != StatusCode::OK {
            error!("未成功 >> {}", resp.status());
        }

        let text = resp.text().await?;
        if text == "true".to_string() {
            return Ok(true);
        }

        if text == "false".to_string() {
            return Ok(false);
        }

        error!("服务器返回错误值 >> {}", text);
        Ok(false)
    }
}
