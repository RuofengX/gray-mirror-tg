pub mod extract;
pub mod mirror;
pub mod search;

use std::future::Future;

pub use extract::ScanLink;
pub use mirror::{history::History, update::LiveMirror};
pub use search::SearchLink;

use crate::Context;

pub trait App {
    fn name(&self) -> &'static str;
    fn ignite(&mut self, ctx: Context) -> impl Future<Output = Option<()>>;
}
