use std::fmt::{Display, Formatter};
use std::time::Duration;

use anyhow::Result;
use grammers_client::grammers_tl_types::enums::MessageEntity;
use grammers_client::grammers_tl_types::functions::messages::GetBotCallbackAnswer;
use grammers_client::grammers_tl_types::{self as tl, types::KeyboardButtonCallback};
use grammers_client::Client;
use sea_orm::entity::prelude::*;
use sea_orm::Set;
use tracing::{info, warn};

use super::{link, Source, SourceType};

#[derive(Debug, Clone)]
pub struct MessageExt {
    pub inner: grammers_client::types::Message,
}

impl Display for MessageExt {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.inner.raw.message.fmt(f)
    }
}

impl From<grammers_client::types::Message> for MessageExt {
    fn from(value: grammers_client::types::Message) -> Self {
        MessageExt { inner: value }
    }
}
impl From<&grammers_client::types::Message> for MessageExt {
    fn from(value: &grammers_client::types::Message) -> Self {
        MessageExt {
            inner: value.clone(),
        }
    }
}
impl MessageExt {
    pub fn text(&self) -> &str {
        self.inner.text()
    }

    pub fn links(&self) -> Vec<link::Link> {
        let mut ret = Vec::new();
        let words: Vec<u16> = self.inner.raw.message.encode_utf16().collect();

        if let Some(ref ents) = self.inner.raw.entities {
            for ent in ents {
                match ent {
                    MessageEntity::TextUrl(url) => {
                        let link = url.url.clone();

                        let offset = url.offset as usize;
                        let len = url.length as usize;

                        if let Ok(desc) = String::from_utf16(&words[offset..offset + len]) {
                            ret.push(link::Link { link, desc });
                        } else {
                            warn!("提取链接时错误");
                        }
                    }
                    _ => (),
                }
            }
        }

        ret
    }

    pub fn callback_buttons(&self) -> Vec<KeyboardButtonCallback> {
        let reply_markup = &self.inner.raw.reply_markup;

        let mut ret = Vec::new();
        if let Some(tl::enums::ReplyMarkup::ReplyInlineMarkup(markup)) = reply_markup {
            for row in markup.rows.iter() {
                let tl::enums::KeyboardButtonRow::Row(row) = row;
                for b in row.buttons.iter() {
                    match b {
                        tl::enums::KeyboardButton::Callback(callback_b) => {
                            ret.push(callback_b.clone());
                        }
                        _ => (),
                    }
                }
            }
        }

        ret
    }

    pub async fn click_callback_button(
        &self,
        client: &Client,
        button: &KeyboardButtonCallback,
        delay: Duration,
    ) -> Result<()> {
        tokio::time::sleep(delay).await;
        info!("{}", button.text);
        client
            .invoke(&GetBotCallbackAnswer {
                game: false,
                peer: self.inner.chat().pack().to_input_peer(),
                msg_id: self.inner.raw.id,
                data: Some(button.data.clone()),
                password: None,
            })
            .await?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "message")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub chat_id: i64,
    #[sea_orm(primary_key)]
    pub msg_id: i32,
    pub raw: Json,
    pub source: SourceType,
    pub source_id: i64,
    // TODO: add photo and video support
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl ActiveModel {
    pub fn from_inner_msg(msg: &grammers_client::types::Message, source: Source) -> Self {
        let raw = Set(serde_json::to_value(&msg.raw).unwrap());
        Self {
            chat_id: Set(msg.chat().id()),
            msg_id: Set(msg.id()),
            raw,
            source: Set(source.ty),
            source_id: Set(source.id),
            ..Default::default()
        }
    }
}
