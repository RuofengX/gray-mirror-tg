//! [`link::Model`] -> [`LinkIter`] -> [`link::Model`] -> [`ChatMessage`]
//! -> Updater get [`chat::ActiveModel`], persist
//! -> Updater get [`message::ActiveModel`], persist
//! -> Updater join group

use std::collections::VecDeque;

use anyhow::{anyhow, bail, Result};
use sea_orm::{DbConn, EntityTrait, Paginator, PaginatorTrait, SelectModel};
use url::Url;

use crate::types::{link, Source};

pub struct LinkIter<'db> {
    pub cursor: Paginator<'db, DbConn, SelectModel<link::Model>>,
    pub buf: VecDeque<link::Model>,
}
impl<'db> LinkIter<'db> {
    pub fn from_db(db: &'db DbConn) -> Self {
        Self {
            cursor: link::Entity::find().paginate(db, 1024),
            buf: VecDeque::new(),
        }
    }

    pub async fn next(&mut self) -> Result<Option<link::Model>> {
        if self.buf.is_empty() {
            while let Some(links) = self.cursor.fetch_and_next().await? {
                if links.is_empty() {
                    return Ok(None);
                }
                self.buf.extend(links);
            }
        }
        Ok(self.buf.pop_front())
    }
}

#[derive(Debug)]
pub struct ChatMessage {
    pub name: String,
    pub msg_id: i32,
    pub source: Source,
}
impl TryFrom<link::Model> for ChatMessage {
    type Error = anyhow::Error;

    fn try_from(value: link::Model) -> Result<Self> {
        let url = Url::parse(&value.link)?;
        let mut path = url.path_segments().ok_or(anyhow!("[0]未找到路径"))?;
        let name = path.next().ok_or(anyhow!("[1]未找到聊天名"))?.to_string(); // TODO: 处理这些路径
        if name.starts_with("+") {
            bail!("[1]是邀请链接")
        }
        let msg_id = path
            .next()
            .ok_or(anyhow!("[2]未找到消息号"))?
            .parse::<i32>()
            .map_err(|_| anyhow!("[2]不是消息号码"))?;
        let rtn = ChatMessage {
            name,
            msg_id,
            source: Source::from_link(&value),
        };
        Ok(rtn)
    }
}
