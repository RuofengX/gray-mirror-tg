//! [`link::Model`] -> [`LinkIter`] -> [`link::Model`] -> [`ChatMessage`]
//! -> Updater get [`types::chat::ActiveModel`], persist
//! -> Updater get [`types::message::ActiveModel`], persist
//! -> Updater join group

use std::collections::VecDeque;

use anyhow::{anyhow, bail, Result};
use sea_orm::{DbConn, EntityTrait, Paginator, PaginatorTrait, SelectModel};
use tracing::debug;
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
pub enum LinkParse {
    ChatMessage(ChatMessage),
    Invite(Invite),
    MaybeChannel(MaybeChannel),
}
impl LinkParse {
    pub fn source(&self) -> Source {
        match self {
            LinkParse::ChatMessage(cm) => cm.source,
            LinkParse::Invite(i) => i.source,
            LinkParse::MaybeChannel(mc) => mc.source,
        }
    }
}
impl TryFrom<link::Model> for LinkParse {
    type Error = anyhow::Error;

    fn try_from(value: link::Model) -> Result<Self> {
        //TODO: 添加t.me开头的判断

        let url = Url::parse(&value.link)?;
        let source = Source::from_link(&value);
        let mut path = url
            .path_segments()
            .ok_or(anyhow!("[0]未找到路径 >> {}", value.link))?;
        let part1 = path
            .next()
            .ok_or(anyhow!("[1]未找到聊天名 >> {}", value.link))?
            .to_string();
        if part1.starts_with("+") {
            debug!("[1]是邀请链接 >> {}", value.link);
            return Ok(Self::Invite(Invite {
                invite_link: url.to_string(),
                invite_code: part1,
                source,
            }));
        };
        if let Some(part2) = path.next() {
            if let Ok(part2_num) = part2.parse::<i32>() {
                debug!("[2]是消息编号 >> {}", value.link);
                let rtn = Self::ChatMessage(ChatMessage {
                    username: part1,
                    msg_id: part2_num,
                    source,
                });
                return Ok(rtn);
            } else {
                bail!("[2]不是消息编号 >> {}", value.link);
            }
        } else {
            debug!("[1]可能是群组链接 >> {}", value.link);
            let rtn = Self::MaybeChannel(MaybeChannel {
                username: part1,
                source,
            });
            return Ok(rtn);
        };
    }
}

#[derive(Debug)]
pub struct ChatMessage {
    pub username: String,
    pub msg_id: i32,
    pub source: Source,
}
#[derive(Debug)]
pub struct Invite {
    pub invite_link: String,
    pub invite_code: String,
    pub source: Source,
}

#[derive(Debug)]
pub struct MaybeChannel {
    pub username: String,
    pub source: Source,
}
pub struct ChatMessageExt {
    pub chat: grammers_client::types::Chat,
    pub msg_id: i32,
    pub source: Source,
}
impl ChatMessageExt {
    pub fn new(chat: grammers_client::types::Chat, msg_id: i32, source: Source) -> Self {
        Self {
            chat,
            msg_id,
            source,
        }
    }
}
