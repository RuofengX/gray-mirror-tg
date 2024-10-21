use anyhow::Result;
use context::Context;
use tokio;

pub mod login;

pub mod app;
pub mod context;
pub mod types;
pub mod persist;

#[tokio::main]
async fn main() -> Result<()> {
    println!("你好世界!");

    let mut context = Context::new().await?;

    context.start_listen_updates().await;

    context.add_app(app::finder::Finder::new()).await?;

    context.run().await?;

    Ok(())
}
