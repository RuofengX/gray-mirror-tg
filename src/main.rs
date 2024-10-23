use anyhow::Result;
use app::finder::engine::Engine;
use context::Context;
use tokio;

pub mod login;

pub mod app;
pub mod context;
pub mod persist;
pub mod types;

#[tokio::main(flavor = "multi_thread", worker_threads = 32)]
async fn main() -> Result<()> {
    println!("你好世界!");

    let ctx = Context::new().await?.enable_update().await?;

    ctx.add_app(app::finder::Finder::new(Engine::SOSO)).await?;
    ctx.add_app(app::addchat::AddChat::new()).await?;
    ctx.run().await?;

    Ok(())
}
