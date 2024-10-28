use std::future::Future;

use anyhow::Result;

use crate::{context::Context, Runable};

/// 更新相关
pub mod update;

/// 自动添加群组、频道
pub mod fetch_chat;
/// 利用soso等机器人挖掘关联群组
pub mod finder;

/// 收集全量数据
pub mod gray_mirror;

pub trait App: Runable + Send + Sync {
    /// 初始化数据
    fn ignite(&mut self, _context: Context) -> impl Future<Output = Result<()>> {
        async { Ok(()) }
    }
}
