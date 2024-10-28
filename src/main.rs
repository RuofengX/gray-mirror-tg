use anyhow::Result;
use context::Context;
use tokio;
pub mod app;
pub mod context;
pub mod error;
pub mod login;
pub mod persist;
pub mod types;
pub mod abstruct;
pub mod update;

pub use error::PrintError;
pub use abstruct::*;
pub use types::*;
pub use app::App;
pub use update::Updater;

#[tokio::main(flavor = "multi_thread", worker_threads = 32)]
async fn main() -> Result<()> {
    println!("你好世界!");

    let ctx = Context::new().await?;

    ctx.add_update_parser().await?;
    // ctx.fetch_all_chat_history(100000).await?;
    ctx.fetch_all_chat_history(100).await?; // 防止错过消息

    ctx.add_app(app::gray_mirror::GrayMirror::new()).await?;
    ctx.add_app(app::finder::Search::default()).await?;
    ctx.add_app(app::fetch_chat::AddChat::new()).await?;
    ctx.run().await?;

    Ok(())
}
