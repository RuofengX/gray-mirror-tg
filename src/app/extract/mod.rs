use anyhow::Result;
use async_trait::async_trait;
use grammers_client::types::Chat;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tracing::{info, warn};

use crate::{context::Context, types::link, PrintError};
use crate::{Runable, Source};

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
            let links = link::Entity::find()
                .filter(link::Column::Parsed.eq(false))
                // .order_by_desc(link::Column::Id)
                .all(db)
                .await?;
            let mut count = 0;
            for link_model in links {
                let id = link_model.id;
                count += 1;
                info!(count, "处理链接");

                // 将链接尝试转换为 (群组名-消息id) 结构
                let link = LinkParse::try_from(link_model).ok_or_warn();
                if link.is_none() {
                    continue;
                }

                // 转换为link
                let link = link.unwrap();

                let source = link.source();

                let chat = match link {
                    LinkParse::ChatMessage(chat_msg) => {
                        Self::parse_chat_msg(id, chat_msg, ctx.clone(), source)
                            .await
                            .ok_or_log()
                            .flatten()
                    }
                    LinkParse::Invite(invite) => Self::parse_invite(id, invite, ctx.clone())
                        .await
                        .ok_or_log()
                        .flatten(),
                    LinkParse::MaybeChannel(channel) => {
                        Self::parse_channel(id, channel, ctx.clone())
                            .await
                            .ok_or_log()
                            .flatten()
                    }
                };
                if let Some(chat) = chat {
                    let username = chat.username();
                    info!(count, username, "成功解析链接并加入");
                } else {
                    info!(count, "未能解析链接");
                }
            }
            warn!(count, "扫描全部链接完成");
        }
    }
}

// ---以下为私有方法---
impl ScanLink {
    pub fn new() -> Self {
        Self {}
    }

    async fn parse_chat_msg(
        link_id: i32,
        chat_msg: ChatMessage,
        ctx: Context,
        source: Source,
    ) -> Result<Option<Chat>> {
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
            ctx.resolve_username(chat_name).await.ok_or_warn().flatten()
        };

        if let Some(chat) = &chat {
            // 加入chat
            ctx.join_chat(chat, source).await?;
            // 将链接标记为已提取
            ctx.persist
                .set_link_extracted(link_id, Some(chat.pack()))
                .await?;
        } else {
            // 将链接标记为已提取
            ctx.persist.set_link_extracted(link_id, None).await?;
        }

        Ok(chat)
    }

    async fn parse_invite(link_id: i32, invite: Invite, ctx: Context) -> Result<Option<Chat>> {
        let link = invite.invite_link.as_str();
        let chat = ctx.join_invite_link(link, invite.source).await?;

        if let Some(chat) = chat {
            warn!(link, "加入邀请链接");
            // 将链接标记为已提取
            ctx.persist
                .set_link_extracted(link_id, Some(chat.pack()))
                .await?;
            Ok(Some(chat))
        } else {
            warn!(link, "未能加入邀请链接");
            // 将链接标记为已提取，无pack
            ctx.persist.set_link_extracted(link_id, None).await?;
            Ok(None)
        }
    }

    async fn parse_channel(
        link_id: i32,
        may_channel: MaybeChannel,
        ctx: Context,
    ) -> Result<Option<Chat>> {
        let chat_username = may_channel.username.as_str();

        if ctx.persist.find_chat(Some(chat_username)).await?.is_some() {
            // 已采集
            info!(chat_name = chat_username, "已采集过群组名");
            // 将链接标记为已提取，无pack
            ctx.persist.set_link_extracted(link_id, None).await?;
            return Ok(None);
        }

        if let Some(chat) = ctx.resolve_username(&may_channel.username).await? {
            warn!(chat_name = chat_username, "新采集群组名");
            ctx.join_chat(&chat, may_channel.source).await?;
            // 将链接标记为已提取，无pack
            ctx.persist
                .set_link_extracted(link_id, Some(chat.pack()))
                .await?;
            Ok(Some(chat))
        } else {
            info!(chat_name = chat_username, "未找到群组名");
            // 将链接标记为已提取，无pack
            ctx.persist.set_link_extracted(link_id, None).await?;
            Ok(None)
        }
    }
}
