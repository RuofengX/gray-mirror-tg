use anyhow::Result;
use app::search::engine::GenericEngine;

pub mod abstruct;
pub mod app;
pub mod channel;
pub mod context;
pub mod error;
pub mod login;
pub mod persist;
pub mod types;
pub mod update;

pub use abstruct::*;
pub use app::App;
pub use context::Context;
pub use error::PrintError;
pub use types::*;
pub use update::Updater;

const KEYWORDS: [&str; 5] = ["柏盛", "财神", "菩萨", "园区", "担保"];

#[tokio::main(flavor = "multi_thread", worker_threads = 32)]
async fn main() -> Result<()> {
    println!("你好世界!");

    let ctx = Context::new().await?;

    ctx.add_app(app::SearchLink::new(
        GenericEngine::JISOU,
        KEYWORDS.into_iter(),
    ))
    .await;
    // ctx.add_app(app::FullMirror::new(100)).await;
    ctx.add_runable(app::ScanLink::new()).await;
    ctx.add_runable(app::UpdateMirror::new(10_0000)).await;
    ctx.add_parser(app::LiveMirror::default()).await;

    ctx.start_update_parser().await;
    ctx.run().await?;

    Ok(())
}
