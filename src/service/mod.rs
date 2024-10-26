use std::future::Future;

use anyhow::Result;
use tokio::sync::mpsc;

pub trait Service: Future<Output = Result<()>> {
    const NAME: &str;
    type Input;
    fn sender(&self) -> mpsc::Sender<Self::Input>;
}
