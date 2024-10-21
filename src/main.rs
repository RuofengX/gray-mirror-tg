use anyhow::Result;
use context::Context;
use tokio;

pub mod login;

/// 处理存量数据
pub mod types;

pub mod app;
pub mod context;

#[tokio::main]
async fn main() -> Result<()> {
    println!("你好世界!");

    let mut context = Context::new().await?;

    context.add_app(app::finder::Finder::new()).await?;

    context.start_listen_updates();
    context.run().await?;

    Ok(())
}
