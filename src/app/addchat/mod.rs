use std::{fmt::Display, time::Duration};

use anyhow::Result;
use sea_orm::{EntityTrait, PaginatorTrait};
use tracing::{info, warn};

use crate::{
    context::Context,
    types::{chat, link, message},
};

use super::App;
use iter::ChatMessage;

pub mod iter;

pub struct AddChat {}

impl Display for AddChat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        "添加群组".fmt(f)
    }
}

impl AddChat {
    pub fn new() -> Self {
        Self {}
    }
    pub async fn background_tast(context: Context) -> Result<()> {
        let db = &context.persist.db;
        let mut link_iter = link::Entity::find().paginate(db, 1024);
        loop {
            // 从数据库获取一批链接
            if let Some(links) = link_iter.fetch_and_next().await? {
                // 将链接尝试转换为 (群组名-消息id) 结构
                let msg_link_vec: Vec<ChatMessage> = links
                    .into_iter()
                    .map(|link| ChatMessage::try_from(link))
                    .filter_map(|x| {
                        if let Err(e) = &x {
                            warn!("链接转换失败 >> {}", e);
                        }
                        x.ok()
                    })
                    .collect();

                for msg_link in msg_link_vec {
                    let chat_name = &msg_link.name; // 群组名

                    // 获取群组chat实例
                    match context.client.resolve_username(chat_name).await? {
                        // 成功打开chat
                        Some(chat) => {
                            // 判断是否已采集
                            if context.persist.chat_name_duplicate(chat_name).await? {
                                info!("已采集群组名 >> {}", chat_name);
                            } else {
                                info!("新采集群组名 >> {}", chat_name);
                                let _ = context
                                    .persist
                                    .put_chat(chat::ActiveModel::from_chat(&chat, &msg_link.source))
                                    .await?;
                                let msg = &context
                                    .client
                                    .get_messages_by_id(chat, &[msg_link.msg_id])
                                    .await?[0];
                                if let Some(msg) = msg {
                                    context
                                        .persist
                                        .put_message(message::ActiveModel::from_msg(
                                            &msg.into(),
                                            &msg_link.source,
                                        ))
                                        .await?;
                                } else {
                                    info!("未找到消息 >> {:?}", msg_link);
                                }
                            }
                        }
                        None => {
                            warn!("查询未找到群组名 >> {}", chat_name);
                        }
                    };

                    // 限制resolve频率
                    tokio::time::sleep(Duration::from_secs(30)).await;
                }
            }
        }
    }
}

impl App for AddChat {
    async fn ignite(&mut self, context: crate::context::Context) -> Result<()> {
        context
            .add_background_task(
                &format!("{}", &self),
                Self::background_tast(context.clone()),
            )
            .await;
        Ok(())
    }
}
