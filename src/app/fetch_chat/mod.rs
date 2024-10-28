use std::fmt::Display;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use grammers_client::grammers_tl_types as tl;
use grammers_client::types::PackedChat;
use sea_orm::{EntityTrait, PaginatorTrait};
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tracing::{debug, debug_span, info, info_span, warn};

use crate::types::Source;
use crate::{
    context::Context,
    types::{chat, link, message},
    PrintError,
};

use super::App;
use convert::{ChatMessage, ChatMessageExt, Invite, LinkParse, MaybeChannel};

pub mod convert;

pub struct AddChat {}

impl App for AddChat {
    async fn ignite(&mut self, context: Context) -> Result<()> {
        let (s, r) = tokio::sync::mpsc::channel(64);
        context
            .add_background_task(
                info_span!("链接消息提取"),
                Self::fetch_message(context.clone(), r),
            )
            .await;
        context
            .add_background_task(
                info_span!("群组探针"),
                Self::fetch_group(context.clone(), s),
            )
            .await;
        Ok(())
    }
}

impl AddChat {
    const NAME: &str = "添加群组";
    pub fn new() -> Self {
        Self {}
    }
    async fn fetch_group(context: Context, chat_send: Sender<ChatMessageExt>) -> Result<()> {
        let group_span = info_span!("获取群组");
        let _span = group_span.enter();

        let db = &context.persist.db;
        let (mut history_send, history_recv) = tokio::sync::mpsc::channel(32);

        context
            .add_background_task(
                info_span!("历史镜像"),
                fetch_chat_history(history_recv, context.clone(), 100000),
            )
            .await;

        loop {
            // 从数据库获取一批链接
            while let Some(links) = link::Entity::find()
                .paginate(db, 60)
                .fetch_and_next()
                .await?
            {
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

                // 遍历link
                for link_parse in msg_link_vec {
                    let span = info_span!("处理链接");
                    let _span = span.enter();

                    debug!("{:?}", link_parse);
                    match link_parse {
                        LinkParse::ChatMessage(chat_msg) => {
                            Self::parse_chat_msg(
                                chat_msg,
                                context.clone(),
                                &chat_send,
                                &mut history_send,
                            )
                            .await
                            .unwrap_or_warn();
                        }
                        LinkParse::Invite(invite) => {
                            Self::parse_invite(invite, context.clone(), &mut history_send)
                                .await
                                .unwrap_or_warn();
                        }
                        LinkParse::MaybeChannel(channel) => {
                            Self::parse_channel(channel, context.clone(), &mut history_send)
                                .await
                                .unwrap_or_warn();
                        }
                    };
                }
            }
        }
    }

    /// 后台获取具体msg
    async fn fetch_message(
        context: Context,
        mut chat_recv: tokio::sync::mpsc::Receiver<ChatMessageExt>,
    ) -> Result<()> {
        let group_span = debug_span!("获取消息");
        let _span = group_span.enter();

        // 但凡chat存在
        while let Some(chat_msg) = chat_recv.recv().await {
            let span = info_span!("处理消息链接", chat_msg.msg_id);
            let _span = span.enter();

            debug!("接受数据");
            // 解包数据
            let chat = &chat_msg.chat;
            let chat_id = chat_msg.chat.id();
            let msg_id = chat_msg.msg_id;
            let source = chat_msg.source;

            // 从chat提取消息
            context.interval.find_msg.tick().await;
            let msg = context
                .client
                .get_messages_by_id(chat, &[msg_id])
                .await?
                .pop()
                .unwrap();
            let span = info_span!("查找消息", chat_id, msg_id);
            let _span = span.enter();

            if let Some(msg) = msg {
                // 存储msg
                info!("找到消息");
                let msg = message::ActiveModel::from_inner_msg(&msg, source);
                context.persist.put_message(msg).await?;
            } else {
                info!("未找到消息");
            };
        }
        Ok(())
    }

    async fn parse_chat_msg(
        chat_msg: ChatMessage,
        context: Context,
        msg_request: &Sender<ChatMessageExt>,
        history_request: &Sender<PackedChat>,
    ) -> Result<()> {
        let chat_name = chat_msg.username.as_str(); // 群组名

        // 判断是否已采集，避免频繁调用resolve_username
        // 获取chat::Model
        let chat = context.persist.find_chat(Some(chat_name)).await?;

        let span = info_span!("处理聊天消息", chat_name);
        let _span = span.enter();

        let chat = if let Some(chat) = chat {
            // 已采集
            info!("已采集过群组名");

            // 限制unpack频率
            context.interval.unpack_chat.tick().await;
            // 从Model读取PackedChat
            let packed_chat = chat.to_packed()?;

            // 用Client解包PackedChat，并返回
            let chat = context.client.unpack_chat(packed_chat).await?;
            chat
        } else {
            // 未采集
            warn!("新采集群组名");
            // 限制resolve频率
            context.interval.resolve_username.tick().await;
            let chat = context.client.resolve_username(chat_name).await;
            if matches!(chat, Err(_)) {
                // 查不到chat出错，放弃，搞下一个link
                warn!("查询群组名出错",);
                return Ok(());
            }
            let chat = chat.expect("已检查");
            if matches!(chat, None) {
                // 查不到chat，放弃，搞下一个link
                warn!("查询未找到群组名");
                return Ok(());
            }

            // 成功打开chat
            let chat = chat.expect("已检查");

            // 请求获取历史
            history_request.send(chat.pack()).await?;

            // 存入数据库，返回chat::Model
            context
                .persist
                .put_chat(chat::ActiveModel::from_chat(&chat, chat_msg.source))
                .await?;
            chat
        }; // 返回chat，之后存入channel

        // 限制加入聊天频率
        context.interval.join_chat.tick().await;
        // 加入聊天
        if context.client.join_chat(&chat).await?.is_some() {
            warn!("加入聊天");
        }
        // 存入channel，让子进程完成消息提取
        msg_request
            .send(ChatMessageExt::new(chat, chat_msg.msg_id, chat_msg.source))
            .await?;

        Ok(())
    }

    async fn parse_invite(
        invite: Invite,
        context: Context,
        history_request: &Sender<PackedChat>,
    ) -> Result<()> {
        let link = invite.invite_link.as_str();
        let span = info_span!("处理邀请链接", link);
        let _span = span.enter();

        // 限制加入聊天频率
        context.interval.join_chat.tick().await;
        // 取回chat实例
        let update_chat = match context.client.accept_invite_link(link).await? {
            tl::enums::Updates::Combined(updates) => Some(updates.chats),
            tl::enums::Updates::Updates(updates) => Some(updates.chats),
            _ => None,
        };
        let chat = match update_chat {
            Some(chats) if !chats.is_empty() => {
                Some(grammers_client::types::Chat::from_raw(chats[0].clone()))
            }
            Some(chats) if chats.is_empty() => None,
            None => None,
            Some(_) => None,
        };

        if let Some(chat) = chat {
            warn!("加入聊天");
            history_request.send(chat.pack()).await?;
            context
                .persist
                .put_chat(chat::ActiveModel::from_chat(&chat, invite.source))
                .await?;
        } else {
            warn!("返回值中未找到chat");
        }

        Ok(())
    }

    async fn parse_channel(
        may_channel: MaybeChannel,
        context: Context,
        history_request: &mut Sender<PackedChat>,
    ) -> Result<()> {
        let chat_username = may_channel.username.as_str();
        let span = info_span!("处理群组链接", chat_username);
        let _span = span.enter();
        if context
            .persist
            .find_chat(Some(chat_username))
            .await?
            .is_some()
        {
            // 已采集
            info!("已采集过群组名");
            return Ok(());
        }

        context.interval.resolve_username.tick().await;

        if let Some(chat) = context
            .client
            .resolve_username(&may_channel.username)
            .await?
        {
            warn!("新采集群组名");
            // 限制加入聊天频率
            context.interval.join_chat.tick().await;
            context.client.join_chat(chat.pack()).await?;

            history_request.send(chat.pack()).await?;

            context
                .persist
                .put_chat(chat::ActiveModel::from_chat(&chat, may_channel.source))
                .await?;
        } else {
            info!("未找到群组名");
        }

        Ok(())
    }
}

pub async fn fetch_chat_history(
    mut recv: Receiver<PackedChat>,
    context: Context,
    limit: usize,
) -> Result<()> {
    while let Some(chat) = recv.recv().await {
        let source = Source::from_chat(chat.id);
        let ctx = context.clone();

        context
            .add_background_task(
                info_span!("镜像聊天记录", chat_id = chat.id, backward_limit = limit),
                async move {
                    let mut history = ctx.client.iter_messages(chat).limit(limit).max_date(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .expect("时间不够倒退")
                            .as_secs() as i32,
                    );

                    let mut count = 0;
                    while let Some(Some(msg)) = history.next().await.unwrap_or_warn() {
                        ctx.interval.find_msg.tick().await;
                        count += 1;

                        info!(count, limit, "获取消息");
                        ctx.persist
                            .put_message(message::ActiveModel::from_inner_msg(&msg, source))
                            .await?;
                    }

                    if count <= limit {
                        info!("流提前结束");
                    }

                    Ok(())
                },
            )
            .await;
    }
    Ok(())
}
