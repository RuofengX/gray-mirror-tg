use anyhow::Result;
use context::Context;
use tokio;

pub mod client;

/// 处理存量数据
pub mod history;

pub mod app;
pub mod context;

#[tokio::main]
async fn main() -> Result<()> {
    println!("你好世界!");

    // 获取客户端
    let client = client::login_with_dotenv().await?;

    let mut context = Context::new(client.clone()).await?;

    context.add_app(app::finder::Finder::new(client.clone())).await?;

    context.run().await?;

    Ok(())
}
