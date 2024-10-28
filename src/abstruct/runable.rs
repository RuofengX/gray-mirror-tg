use anyhow::Result;
use async_trait::async_trait;

use crate::context::Context;

#[async_trait]
pub trait Runable: Send + 'static {
    fn name(&self) -> &'static str;
    async fn run(&mut self, ctx: Context) -> Result<()>;
}
