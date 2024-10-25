use anyhow::Result;
use app::finder::engine::Engine;
use context::Context;
use tokio;
use tracing::{info, warn};

pub mod login;

pub mod app;
pub mod context;
pub mod persist;
pub mod types;

#[tokio::main(flavor = "multi_thread", worker_threads = 32)]
async fn main() -> Result<()> {
    println!("你好世界!");

    let ctx = Context::new().await?;

    ctx.enable_update().await?;
    ctx.fetch_all_chat_history(100000).await?;

    ctx.add_app(app::gray_mirror::GrayMirror::new()).await?;
    ctx.add_app(app::finder::Finder::new(Engine::SOSO)).await?;
    ctx.add_app(app::addchat::AddChat::new()).await?;
    ctx.run().await?;

    Ok(())
}

pub trait PrintError<T, E> {
    fn unwrap_or_warn(self) -> Option<T>;
    fn into_log(self) -> ();
}
impl<T: std::fmt::Debug, E: std::fmt::Display> PrintError<T, E> for Result<T, E> {
    fn unwrap_or_warn(self) -> Option<T> {
        match self {
            Ok(t) => Some(t),
            Err(e) => {
                warn!("{}", e);
                None
            }
        }
    }
    fn into_log(self) -> () {
        match self {
            Ok(t) => info!("{:?}", t),
            Err(e) => warn!("{}", e),
        }
    }
}
