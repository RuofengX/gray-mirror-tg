use anyhow::Result;
use async_trait::async_trait;
use grammers_client::{types::Message as RawMessage, Update};
use tokio::{
    sync::broadcast::{self, Receiver},
    task::JoinSet,
};
use tracing::warn;

use crate::{context::Context, types::MessageExt, PrintError, Runable};

/// 匹配器，以供部分实现
#[async_trait]
pub trait Updater: Send + Sync + 'static {
    fn name(&self) -> &'static str;

    /// Occurs whenever a new text message or a message with media is produced.
    async fn message_recv(&mut self, _context: Context, _msg: MessageExt) -> Result<()> {
        Ok(())
    }

    /// Occurs when a message is updated.
    async fn message_edited(&mut self, _context: Context, _msg: MessageExt) -> Result<()> {
        Ok(())
    }

    /// DO NOT RELOAD THIS FUNCTION
    /// UNLESS YOU KNOW WHAT YOU DO
    ///
    /// * return Some(()) if parsed;
    /// * return None if not parsed
    ///
    /// Every error should be parsed inside this function.
    async fn parse_update(&mut self, context: Context, update: Update) -> Option<()> {
        let result = {
            match update {
                Update::NewMessage(ref raw_msg) => {
                    if self.raw_msg_filter(raw_msg) {
                        Some(self.message_recv(context, raw_msg.into()).await)
                    } else {
                        None
                    }
                }
                Update::MessageEdited(ref raw_msg) => {
                    if self.raw_msg_filter(raw_msg) {
                        Some(self.message_edited(context, raw_msg.into()).await)
                    } else {
                        None
                    }
                }
                Update::Raw(_) => None,
                _ => None,
            }
        };
        result.and_then(|some| some.ok_or_log())
    }

    /// default implement will fliter all message that incoming
    /// using `!raw_msg.outgoing()`
    ///
    /// * return true this message will get parsed later;
    /// * return false will ignore this message
    fn raw_msg_filter(&self, raw_msg: &RawMessage) -> bool {
        let mut flag = true;

        if self.filter_incoming() {
            raw_msg.outgoing().then(|| flag = false);
        }

        if let Some(id) = self.filter_chat_id() {
            if raw_msg.chat().id() != id {
                flag = false;
            }
        }

        if let Some(word) = self.filter_word() {
            if !raw_msg.text().contains(&word) {
                flag = false
            }
        }

        flag
    }

    fn filter_incoming(&self) -> bool {
        true
    }

    fn filter_chat_id(&self) -> Option<i64> {
        None
    }

    fn filter_word(&self) -> Option<String> {
        None
    }

    // fn filter_text(&self)
}

pub struct UpdateApp {
    parser: Vec<UpdateParser>,
    tx: broadcast::Sender<Update>,
}
impl UpdateApp {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(2048);
        Self {
            parser: Vec::new(),
            tx,
        }
    }
    pub fn add_parser(&mut self, parser: impl Updater) -> () {
        let rt = UpdateParser::new(self.tx.subscribe(), parser);
        self.parser.push(rt);
    }
}
impl Default for UpdateApp {
    fn default() -> Self {
        Self::new()
    }
}
#[async_trait]
impl Runable for UpdateApp {
    fn name(&self) -> &'static str {
        "更新"
    }
    async fn run(&mut self, ctx: Context) -> Result<()> {
        let mut count = 0;
        loop {
            let mut tasks = JoinSet::new();
            while let Some(mut i) = self.parser.pop() {
                let ctxx = ctx.clone();
                tasks.spawn(async move { i.run(ctxx).await });
            }
            let mut listener = UpdateListener::new(self.tx.clone());
            let ctxx = ctx.clone();
            tasks.spawn(async move { listener.run(ctxx).await });
            while let Some(task) = tasks.join_next().await {
                task?.into_log();
            }
            count += 1;
            warn!(count, "Update守护进程过早退出，自动重启")
        }
    }
}

pub struct UpdateParser {
    inner: Box<dyn Updater>,
    rx: broadcast::Receiver<Update>,
}
impl UpdateParser {
    pub fn new(update_receiver: Receiver<Update>, parser: impl Updater) -> Self {
        Self {
            inner: Box::new(parser),
            rx: update_receiver,
        }
    }
}
#[async_trait]
impl Runable for UpdateParser {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    async fn run(&mut self, ctx: Context) -> Result<()> {
        while let Ok(update) = self.rx.recv().await {
            self.inner.parse_update(ctx.clone(), update).await;
        }
        Ok(())
    }
}

pub struct UpdateListener {
    tx: broadcast::Sender<Update>,
}
impl UpdateListener {
    pub fn new(update_sender: broadcast::Sender<Update>) -> Self {
        Self { tx: update_sender }
    }
}
#[async_trait]
impl Runable for UpdateListener {
    fn name(&self) -> &'static str {
        "更新监听器"
    }

    async fn run(&mut self, ctx: Context) -> Result<()> {
        while let Ok(update) = ctx.client.next_update().await {
            self.tx.send(update)?;
        }
        Ok(())
    }
}
