use anyhow::Result;
use app::search::engine::GenericEngine;

pub mod abstruct;
pub mod app;
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

const KEYWORDS: [&str; 5] = ["园区", "东南亚", "曝光", "担保公群", "需求"];

#[tokio::main(flavor = "multi_thread", worker_threads = 32)]
async fn main() -> Result<()> {
    println!("你好世界!");

    let ctx = Context::new().await?;

    // 维护退出的聊天
    ctx.add_runable(app::mirror::eliminate::Sentence::new())
        .await;

    // 主动扫描数据库链接表
    ctx.add_runable(app::ScanLink::new()).await;

    // 主动搜索
    ctx.add_app(app::SearchLink::new(
        // GenericEngine::JISOU,
        GenericEngine::SOSO,
        KEYWORDS.into_iter(),
    ))
    .await;

    // 实时镜像更新
    ctx.add_parser(app::LiveMirror::default()).await;

    // 启动所有更新
    ctx.start_update_parser().await;
    ctx.run().await?;

    Ok(())
}
