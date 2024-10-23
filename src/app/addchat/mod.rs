use std::fmt::Display;

use anyhow::Result;
use sea_orm::{EntityTrait, PaginatorTrait};
use tokio::sync::mpsc::Sender;
use tracing::{debug, info, info_span, warn};

use crate::{
    context::{Context, RESOLVE_USER_NAME_FREQ, UNPACK_MSG_FREQ},
    types::{chat, link, message},
};

use super::App;
use convert::{ChatMessage, ChatMessageExt};

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
        let mut rate_limit = tokio::time::interval(RESOLVE_USER_NAME_FREQ);

        loop {
            let mut link_iter = link::Entity::find().paginate(db, 60);
            loop {
                // 从数据库获取一批链接
                if let Some(links) = link_iter.fetch_and_next().await? {
                    // 将链接尝试转换为 (群组名-消息id) 结构
                    let msg_link_vec: Vec<ChatMessage> = links
                        .into_iter()
                        .map(|link| ChatMessage::try_from(link))
                        .filter_map(|x| {
                            if let Err(e) = &x {
                                warn!("链接转换失败 > {}", e);
                            }
                            x.ok()
                        })
                        .collect();

                    // 遍历link
                    for msg_link in msg_link_vec {
                        let chat_name = &msg_link.username; // 群组名

                        // 获取chat::Model
                        // 判断是否已采集，避免频繁调用resolve_username
                        let chat = if let Some(chat) = context.persist.find_chat(chat_name).await? {
                            // 已采集
                            info!("已采集群组名 >> {}", chat_name);

                            // 从Model读取PackedChat
                            let packed_chat = chat.to_packed()?;
                            // 用Client解包PackedChat，并返回
                            context.client.unpack_chat(packed_chat).await?
                        } else {
                            // 未采集
                            warn!("新采集群组名 >> {}", chat_name);
                            let chat = context.client.resolve_username(chat_name).await;
                            if matches!(chat, Err(_)) {
                                // 查不到chat出错，放弃，搞下一个link
                                warn!(
                                    "查询群组名出错 > {} >> {}",
                                    chat_name,
                                    chat.expect_err("已检查")
                                );
                                continue;
                            }
                            let chat = chat.expect("已检查");
                            if matches!(chat, None) {
                                // 查不到chat，放弃，搞下一个link
                                warn!("查询未找到群组名 >> {}", chat_name);
                                continue;
                            }
                            let chat = chat.expect("已检查");
                            // 成功打开chat
                            // 存入数据库，返回chat::Model
                            context
                                .persist
                                .put_chat(chat::ActiveModel::from_chat(&chat, &msg_link.source))
                                .await?;

                            // 限制unpack, resolve频率
                            rate_limit.tick().await;

                            // 返回chat，之后存入channel
                            chat
                        };

                        // 存入channel，让子进程完成消息提取
                        chat_send
                            .send(ChatMessageExt::new(chat, msg_link.msg_id, msg_link.source))
                            .await?;
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
        let mut rate_limit = tokio::time::interval(UNPACK_MSG_FREQ);

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
}
