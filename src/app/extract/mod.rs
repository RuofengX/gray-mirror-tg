use anyhow::Result;
use async_trait::async_trait;
use grammers_client::grammers_tl_types as tl;
use grammers_client::types::Chat;
use sea_orm::{EntityTrait, PaginatorTrait};
use tracing::{info, warn};

use crate::Runable;
use crate::{
    context::Context,
    types::{chat, link},
    PrintError,
};

use url_parse::{ChatMessage, Invite, LinkParse, MaybeChannel};

pub mod url_parse;

pub struct ScanLink {}

#[async_trait]
impl Runable for ScanLink {
    fn name(&self) -> &'static str {
        "链接扫描"
    }
    async fn run(&mut self, ctx: Context) -> Result<()> {
        let db = &ctx.persist.db;
        loop {
            // 从数据库获取一批链接
            warn!("开始扫描全部链接");
            let mut pages = link::Entity::find().paginate(db, 60);
            while let Some(links) = pages.fetch_and_next().await? {
                // 将链接尝试转换为 (群组名-消息id) 结构
                let msg_link_vec: Vec<LinkParse> = links
                    .into_iter()
                    .map(|link| LinkParse::try_from(link))
                    .filter_map(|x| {
                        if let Err(e) = &x {
                            // 忽略转换错误
                            warn!("链接转换失败 > {}", e);
                        }
                        x.ok()
                    })
                    .collect();

                for link_parse in msg_link_vec {
                    let source = link_parse.source();
                    let chat = match link_parse {
                        LinkParse::ChatMessage(chat_msg) => {
                            Self::parse_chat_msg(chat_msg, ctx.clone())
                                .await
                                .unwrap_or_warn()
                                .flatten()
                        }
                        LinkParse::Invite(invite) => Self::parse_invite(invite, ctx.clone())
                            .await
                            .unwrap_or_warn()
                            .flatten(),
                        LinkParse::MaybeChannel(channel) => {
                            Self::parse_channel(channel, ctx.clone())
                                .await
                                .unwrap_or_warn()
                                .flatten()
                        }
                    };
                    if let Some(chat) = chat {
                        // 加入chat
                        ctx.interval.join_chat.tick().await;
                        ctx.client.join_chat(&chat).await?;
                        // 保存chat
                        ctx.persist
                            .put_chat(chat::ActiveModel::from_chat(&chat, source))
                            .await?;
                        // 告诉后台进程获取历史
                        ctx.channel.fetch_history.send(chat.pack())?;
                    }
                }
            }
            warn!("扫描全部链接完成");
        }
    }
}

// ---以下为私有方法---
impl ScanLink {
    pub fn new() -> Self {
        Self {}
    }

    async fn parse_chat_msg(chat_msg: ChatMessage, ctx: Context) -> Result<Option<Chat>> {
        let chat_name = chat_msg.username.as_str(); // 群组名

        // 判断是否已采集，避免频繁调用resolve_username
        // 获取chat::Model
        let chat = ctx.persist.find_chat(Some(chat_name)).await?;

        let chat = if chat.is_some() {
            // 已采集
            info!(chat_name, "已采集过群组名");
            None
        } else {
            // 未采集
            warn!(chat_name, "新采集群组名");
            // 限制resolve频率
            ctx.interval.resolve_username.tick().await;
            let chat = ctx.client.resolve_username(chat_name).await;

            chat.unwrap_or_warn().flatten()
        };

        Ok(chat)
    }

    async fn parse_invite(invite: Invite, ctx: Context) -> Result<Option<Chat>> {
        let link = invite.invite_link.as_str();

        // 限制加入聊天频率
        ctx.interval.join_chat.tick().await;
        // 取回chat实例
        let chats = match ctx.client.accept_invite_link(link).await? {
            tl::enums::Updates::Combined(updates) => Some(updates.chats),
            tl::enums::Updates::Updates(updates) => Some(updates.chats),
            _ => None,
        };
        let chat = match chats {
            Some(chats) if !chats.is_empty() => {
                Some(grammers_client::types::Chat::from_raw(chats[0].clone()))
            }
            Some(chats) if chats.is_empty() => None,
            None => None,
            Some(_) => None,
        };

        if let Some(chat) = chat {
            ctx.persist
                .put_chat(chat::ActiveModel::from_chat(&chat, invite.source))
                .await?;
            Ok(Some(chat))
        } else {
            Ok(None)
        }
    }

    async fn parse_channel(may_channel: MaybeChannel, ctx: Context) -> Result<Option<Chat>> {
        let chat_username = may_channel.username.as_str();

        if ctx.persist.find_chat(Some(chat_username)).await?.is_some() {
            // 已采集
            info!(chat_name = chat_username, "已采集过群组名");
            return Ok(None);
        }

        ctx.interval.resolve_username.tick().await;

        if let Some(chat) = ctx.client.resolve_username(&may_channel.username).await? {
            warn!(chat_name = chat_username, "新采集群组名");

            ctx.persist
                .put_chat(chat::ActiveModel::from_chat(&chat, may_channel.source))
                .await?;
            Ok(Some(chat))
        } else {
            info!(chat_name = chat_username, "未找到群组名");
            Ok(None)
        }
    }
}
