use anyhow::Result;

use crate::context::Context;

pub trait Runable {
    fn name(&self) -> &'static str;
    fn run(&mut self, ctx: Context) -> Result<()>;
}
