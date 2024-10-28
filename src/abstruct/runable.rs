use std::future::Future;

use anyhow::Result;

use crate::context::Context;

pub trait Runable: Send + 'static {
    fn name(&self) -> &'static str;
    fn run(self, ctx: Context) -> impl Future<Output = Result<()>> + Send + 'static;
}
