use std::fmt::Display;

use anyhow::Result;
use sea_orm::{EntityTrait, PaginatorTrait};
use tokio::{sync::mpsc::Sender, time::Interval};
use tracing::{debug, info, info_span, instrument, warn};

use crate::{
    context::{Context, FIND_MSG_FREQ, JOIN_CHAT_FREQ, RESOLVE_USER_NAME_FREQ, UNPACK_CHAT_FREQ},
    types::{chat, link, message},
};

use super::App;
use convert::{ChatMessage, ChatMessageExt, Invite, LinkParse, MaybeChannel};

pub mod convert;

pub struct AddChat {}

impl Display for AddChat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Self::NAME.fmt(f)
    }
}
impl App for AddChat {
    async fn ignite(&mut self, context: Context) -> Result<()> {
        let (s, r) = tokio::sync::mpsc::channel(64);
        context
            .add_background_task("链接消息提取", Self::fetch_message(context.clone(), r))
            .await;
        context
            .add_background_task("群组探针", Self::fetch_group(context.clone(), s))
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
        let mut unpack_limit = tokio::time::interval(UNPACK_CHAT_FREQ);
        let mut resolve_limit = tokio::time::interval(RESOLVE_USER_NAME_FREQ);
        let mut join_limit = tokio::time::interval(JOIN_CHAT_FREQ);

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
                    debug!("处理链接 >> {:?}", link_parse);
                    match link_parse {
                        LinkParse::ChatMessage(chat_msg) => {
                            Self::parse_chat_msg(
                                chat_msg,
                                context.clone(),
                                &chat_send,
                                &mut unpack_limit,
                                &mut resolve_limit,
                                &mut join_limit,
                            )
                            .await
                            .err()
                            .and_then(|e| {
                                warn!("处理消息链接时报错 >> {}", e);
                                None::<anyhow::Error>
                            });
                        }
                        LinkParse::Invite(invite) => {
                            Self::parse_invite(
                                invite,
                                context.clone(),
                                &chat_send,
                                &mut join_limit,
                            )
                            .await
                            .err()
                            .and_then(|e| {
                                warn!("处理邀请链接时报错 >> {}", e);
                                None::<anyhow::Error>
                            });
                        }
                        LinkParse::MaybeChannel(channel) => {
                            Self::parse_channel(
                                channel,
                                context.clone(),
                                &mut resolve_limit,
                                &mut join_limit,
                            )
                            .await
                            .err()
                            .and_then(|e| {
                                warn!("处理频道链接时报错 >> {}", e);
                                None::<anyhow::Error>
                            });
                        }
                    }
                }
            }
        }
    }

    /// 后台获取具体msg
    async fn fetch_message(
        context: Context,
        mut chat_recv: tokio::sync::mpsc::Receiver<ChatMessageExt>,
    ) -> Result<()> {
        let group_span = info_span!("获取消息");
        let _span = group_span.enter();

        // 限制解包频率
        let mut rate_limit = tokio::time::interval(FIND_MSG_FREQ);

        // 但凡channel存在
        while let Some(chat_msg) = chat_recv.recv().await {
            debug!("接收消息链接 > id >> {}", chat_msg.msg_id);
            // 解包数据
            let chat = &chat_msg.chat;
            let msg_id = chat_msg.msg_id;
            let source = chat_msg.source;

            // 从chat提取消息
            if let Some(msg) = &context.client.get_messages_by_id(chat, &[msg_id]).await?[0] {
                // 存储msg
                info!("找到消息 >> {}@{}", msg_id, chat.id());
                context
                    .persist
                    .put_message(message::ActiveModel::from_msg(&msg, source))
                    .await?;
                rate_limit.tick().await;
            } else {
                info!("未找到消息 >> {}@{}", msg_id, chat.id());
            }
        }
        Ok(())
    }

    async fn parse_chat_msg(
        chat_msg: ChatMessage,
        context: Context,
        chat_send: &Sender<ChatMessageExt>,
        unpack_limit: &mut Interval,
        resolve_limit: &mut Interval,
        join_limit: &mut Interval,
    ) -> Result<()> {
        let chat_name = &chat_msg.username; // 群组名

        // 获取chat::Model
        // 判断是否已采集，避免频繁调用resolve_username
        let chat = if let Some(chat) = context.persist.find_chat(chat_name).await? {
            // 已采集
            info!("已采集群组名 >> {}", chat_name);

            // 从Model读取PackedChat
            let packed_chat = chat.to_packed()?;
            // 限制unpack频率
            unpack_limit.tick().await;
            // 用Client解包PackedChat，并返回
            let chat = context.client.unpack_chat(packed_chat).await?;
            chat
        } else {
            // 未采集
            warn!("新采集群组名 >> {}", chat_name);
            // 限制resolve频率
            resolve_limit.tick().await;
            let chat = context.client.resolve_username(chat_name).await;
            if matches!(chat, Err(_)) {
                // 查不到chat出错，放弃，搞下一个link
                warn!(
                    "查询群组名出错 > {} >> {}",
                    chat_name,
                    chat.expect_err("已检查")
                );
                return Ok(());
            }
            let chat = chat.expect("已检查");
            if matches!(chat, None) {
                // 查不到chat，放弃，搞下一个link
                warn!("查询未找到群组名 >> {}", chat_name);
                return Ok(());
            }
            let chat = chat.expect("已检查");
            // 成功打开chat
            // 存入数据库，返回chat::Model
            context
                .persist
                .put_chat(chat::ActiveModel::from_chat(&chat, &chat_msg.source))
                .await?;
            chat

            // 返回chat，之后存入channel
        };

        // 限制加入聊天频率
        join_limit.tick().await;

        // 加入聊天
        if context.client.join_chat(&chat).await?.is_some() {
            warn!("加入聊天 >> {}", chat.name());
        }

        // 存入channel，让子进程完成消息提取
        chat_send
            .send(ChatMessageExt::new(chat, chat_msg.msg_id, chat_msg.source))
            .await?;

        Ok(())
    }

    #[allow(unused_variables)]
    async fn parse_invite(
        invite: Invite,
        context: Context,
        _chat_send: &Sender<ChatMessageExt>,
        join_limit: &mut Interval,
    ) -> Result<()> {
        // TODO:  更改上游表现后添加至chat_send中

        // let _chat = context
        //     .client
        //     .accept_invite_link(&invite.invite_link)
        //     .await?;

        // // 限制加入聊天频率
        // join_limit.tick().await;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn parse_channel(
        may_channel: MaybeChannel,
        context: Context,
        resolve_limit: &mut Interval,
        join_limit: &mut Interval,
    ) -> Result<()> {
        if context
            .persist
            .find_chat(&may_channel.username)
            .await?
            .is_some()
        {
            // 已采集
            return Ok(());
        }

        resolve_limit.tick().await;

        if let Some(chat) = context
            .client
            .resolve_username(&may_channel.username)
            .await?
        {
            // 限制加入聊天频率
            join_limit.tick().await;

            context
                .persist
                .put_chat(chat::ActiveModel::from_chat(&chat, &may_channel.source))
                .await?;
            context.client.join_chat(chat).await?;
        }

        Ok(())
    }
}
