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

    let mut context = Context::new().await?;

    context.start_listen_updates().await;

    context
        .add_app(app::finder::Finder::new(Engine::SOSO))
        .await?;

    context.run().await?;

    Ok(())
}
