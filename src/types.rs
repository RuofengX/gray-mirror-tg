use std::fmt::{Display, Formatter};

use anyhow::Result;
use grammers_client::grammers_tl_types::enums::{InputPeer, MessageEntity, Peer};
use grammers_client::grammers_tl_types::functions::messages::GetBotCallbackAnswer;
use grammers_client::grammers_tl_types::types::InputPeerChat;
use grammers_client::grammers_tl_types::{self as tl, types::KeyboardButtonCallback};
use grammers_client::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, info_span, warn};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MirrorMessage {
    raw: tl::types::Message,
}
impl Display for MirrorMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.raw.message.fmt(f)
    }
}

impl From<&grammers_client::types::Message> for MirrorMessage {
    fn from(value: &grammers_client::types::Message) -> Self {
        Self {
            raw: value.raw.clone(),
        }
    }
}
impl MirrorMessage {
    pub fn new(value: &grammers_client::types::Message) -> Self {
        MirrorMessage {
            raw: value.raw.clone(),
        }
    }

    pub fn peer_id(&self) -> i64 {
        match &self.raw.peer_id {
            Peer::Channel(channel) => channel.channel_id,
            Peer::Chat(chat) => chat.chat_id,
            Peer::User(user) => user.user_id,
        }
    }
    pub fn extract_links(&self, source: Source) -> Vec<RelatedLink> {
        let fetch_span = info_span!("提取消息内文本链接");
        let _span = fetch_span.enter();

        let mut rtn = Vec::new();
        let words: Vec<u16> = self.raw.message.encode_utf16().collect();

        if let Some(ref ents) = self.raw.entities {
            for ent in ents {
                match ent {
                    MessageEntity::TextUrl(url) => {
                        let link = url.url.clone();

                        let offset = url.offset as usize;
                        let len = url.length as usize;

                        if let Ok(desc) = String::from_utf16(&words[offset..offset + len]) {
                            info!(stage = "数据发现", "{}", desc);
                            rtn.push(RelatedLink::new(link, desc, Some(source.clone())));
                        } else {
                            warn!("提取链接时错误 >> offset: {offset}; len: {len}");
                        }
                    }
                    _ => (),
                }
            }
        }

        rtn
    }

    pub fn extract_inline_buttons(&self) -> Vec<KeyboardButtonCallback> {
        let fetch_button_span = info_span!("提取反馈按钮");
        let _span = fetch_button_span.enter();

        let reply_markup = &self.raw.reply_markup;

        let mut rtn = Vec::new();
        if let Some(tl::enums::ReplyMarkup::ReplyInlineMarkup(markup)) = reply_markup {
            for row in markup.rows.iter() {
                let tl::enums::KeyboardButtonRow::Row(row) = row;
                for b in row.buttons.iter() {
                    match b {
                        tl::enums::KeyboardButton::Callback(callback_b) => {
                            info!("{}", ron::to_string(&callback_b).unwrap());
                            rtn.push(callback_b.clone());
                        }
                        _ => (),
                    }
                }
            }
        }

        rtn
    }

    pub async fn click_callback_buttons(
        &self,
        client: &Client,
        button: &KeyboardButtonCallback,
    ) -> Result<()> {
        client
            .invoke(&GetBotCallbackAnswer {
                game: false,
                peer: InputPeer::Chat(InputPeerChat {
                    chat_id: self.peer_id(),
                }),
                msg_id: self.raw.id,
                data: Some(button.data.clone()),
                password: None,
            })
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Source {
    SearchEngine { keyword: String },
    None,
}
// TODO: impl Display for Source{}

impl Source {
    pub fn search(keyword: &str) -> Self {
        Source::SearchEngine {
            keyword: keyword.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RelatedLink {
    pub link: String,
    pub desc: String,
    pub from: Option<Source>,
}
impl PartialEq for RelatedLink {
    fn eq(&self, other: &Self) -> bool {
        self.link == other.link
    }
}
impl RelatedLink {
    pub fn new(link: String, desc: String, from: Option<Source>) -> Self {
        Self { link, desc, from }
    }
}
impl Display for RelatedLink {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.desc.fmt(f)
    }
}
