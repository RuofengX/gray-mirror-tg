use std::{ops::Deref, sync::Arc, time::Duration};

use anyhow::{bail, Result};
use dotenv_codegen::dotenv;
use grammers_client::{
    types::{Chat, PackedChat},
    Client, InvocationError,
};
use sea_orm::EntityTrait;
use tokio::{
    sync::{Mutex, RwLock},
    task::JoinSet,
};
use tracing::{error, info, level_filters::STATIC_MAX_LEVEL, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use url::Url;

use crate::{
    chat,
    persist::Database,
    update::{UpdateApp, Updater},
    App, PrintError, Runable, Source,
};

#[derive(Clone)]
pub struct Context(Arc<ContextInner>);

pub struct ContextInner {
    pub client: Client,
    pub persist: Database,
    pub interval: IntervalSet,
    background_tasks: Mutex<JoinSet<()>>,
    update: RwLock<UpdateApp>,
}

impl Deref for Context {
    type Target = ContextInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Context {
    pub async fn new() -> Result<Self> {
        let mut background_tasks = JoinSet::new();

        let logger = tracing_subscriber::registry();
        let loki_url = dotenv!("LOKI_URL");

        let logger = logger
            .with(STATIC_MAX_LEVEL)
            .with(tracing_subscriber::fmt::Layer::new());

        if loki_url != "" {
            let (layer, task) = tracing_loki::builder()
                .label("service_name", "gray-mirror-tg")?
                .label("version", std::env::var("CARGO_PKG_VERSION").unwrap())?
                .build_url(Url::parse(&loki_url)?)?;

            background_tasks.spawn(async move {
                task.await;
            });
            logger.with(layer).init();
        } else {
            logger.init();
        }

        let ret = Self(Arc::new(ContextInner {
            client: crate::login::login_with_dotenv().await?,
            background_tasks: Mutex::new(background_tasks),
            persist: Database::new().await?,
            update: RwLock::new(UpdateApp::new()),
            interval: Default::default(),
        }));

        Ok(ret)
    }

    pub async fn add_app(&self, mut value: impl App) -> () {
        let ctx = self.clone();
        warn!(app = value.name(), "添加应用");
        if value.ignite(ctx).await.is_none() {
            error!(app = value.name(), "应用启动失败");
        }
    }

    pub async fn add_runable(&self, mut value: impl Runable) -> () {
        let ctx = self.clone();
        let name = value.name();
        self.background_tasks.lock().await.spawn(async move {
            value.run(ctx).await.into_log();
            warn!(name, "任务退出");
        });
    }

    pub async fn add_parser(&self, value: impl Updater) -> () {
        let mut update = self.update.write().await;
        update.add_parser(value);
    }

    pub async fn start_update_parser(&self) -> () {
        let mut updater = self.update.write().await;
        let mut buf = UpdateApp::new();
        std::mem::swap(&mut *updater, &mut buf);
        self.add_runable(buf).await;
    }

    /// Run until error occurs. Return first error.
    pub async fn run(self) -> Result<()> {
        while let Some(result) = self.background_tasks.lock().await.join_next().await {
            result.ok_or_log();
        }
        warn!("全部任务结束");
        Ok(())
    }

    pub async fn resolve_username(&self, username: &str) -> Result<Option<Chat>> {
        self.interval.resolve_username.tick().await;
        let mut ret = self.client.resolve_username(username).await;
        if wait_on_flood(&ret).await.is_some() {
            warn!("重新尝试");
            self.interval.resolve_username.tick().await;
            ret = self.client.resolve_username(username).await;
        }
        let ret = ret.ok_or_log().flatten();
        Ok(ret)
    }

    pub async fn quit_chat(&self, chat: impl Into<PackedChat>) -> Result<Option<()>> {
        let chat = Into::<PackedChat>::into(chat);
        let id = chat.id;
        warn!(chat_id = id, "退出聊天");

        self.interval.quit_chat.tick().await;

        let mut ret = self.client.delete_dialog(chat).await;
        if wait_on_flood(&ret).await.is_some() {
            warn!("重新尝试");
            self.interval.join_chat.tick().await;
            ret = self.client.delete_dialog(chat).await;
        }
        let ret = ret.ok_or_log();
        if ret.is_some() {
            self.persist.set_chat_quited(id).await?;
            warn!(chat_id = id, "退出聊天成功");
        } else {
            warn!(chat_id = id, "退出聊天失败");
        }
        Ok(ret)
    }

    /// Join chat, quit exist chat if chat list is full.
    pub async fn join_new_chat(&self, chat: impl Into<PackedChat>, source: Source) -> Result<Chat> {
        let chat = Into::<PackedChat>::into(chat);
        let id = chat.id;
        let mut ret = self.join_chat_raw(chat).await;
        if self.quit_chat_on_too_much(&ret).await.is_some() {
            info!(id, "重新尝试");
            ret = self.join_chat_raw(chat).await;
        };
        let ret = ret?;
        self.persist
            .put_chat(chat::ActiveModel::from_chat(&ret, true, source))
            .await?;
        Ok(ret)
    }

    pub async fn join_quited_chat(&self, chat_id: i64) -> Result<Chat> {
        let db = &self.persist.db;
        let chat = chat::Entity::find_by_id(chat_id).one(db).await?;
        if chat.is_none() {
            bail!("不存在chat_id{chat_id}")
        }
        let chat = chat.unwrap().packed()?;
        let id = chat.id;

        let mut ret = self.join_chat_raw(chat).await;
        if self.quit_chat_on_too_much(&ret).await.is_some() {
            info!(id, "重新尝试");
            ret = self.join_chat_raw(chat).await;
        };
        let ret = ret?;

        self.persist.set_chat_joined(chat_id).await?;
        Ok(ret)
    }

    pub async fn join_invite_link(&self, link: &str, source: Source) -> Result<Option<Chat>> {
        self.interval.join_chat.tick().await;
        let mut chat = self.client.accept_invite_link(link).await;
        if self.quit_chat_on_too_much(&chat).await.is_some() {
            warn!(link, "重新尝试");
            self.interval.join_chat.tick().await;
            chat = self.client.accept_invite_link(link).await;
        }

        if let Some(chat) = chat? {
            self.persist
                .put_chat(chat::ActiveModel::from_chat(&chat, true, source))
                .await?;
            info!(link, "加入邀请链接成功");
            return Ok(Some(chat));
        }

        error!(link, "加入邀请链接失败");
        Ok(None)
    }

    async fn join_chat_raw(&self, chat: impl Into<PackedChat>) -> Result<Chat, InvocationError> {
        self.interval.join_chat.tick().await;
        let chat = Into::<PackedChat>::into(chat);
        let mut ret = self.client.join_chat(chat).await;
        if wait_on_flood(&ret).await.is_some() {
            warn!("重新尝试");
            self.interval.join_chat.tick().await;
            ret = self.client.join_chat(chat).await;
        }
        match ret {
            Ok(Some(chat)) => {
                warn!(chat_name = chat.name(), chat_id = chat.id(), "加入聊天");
                Ok(chat)
            }
            Ok(None) => self.client.unpack_chat(chat).await,
            Err(e) => Err(e),
        }
    }

    async fn quit_chat_on_too_much(
        &self,
        result: &Result<impl std::fmt::Debug, InvocationError>,
    ) -> Option<()> {
        let e = if let Err(e) = result { e } else { return None };
        match e {
            InvocationError::Rpc(e) => {
                if e.code == 400 && e.name.eq("CHANNELS_TOO_MUCH") {
                    warn!("捕获服务器警告CHANNELS_TOO_MUCH");
                    info!("尝试退出聊天以腾出空间");
                    let chat = self
                        .persist
                        .find_oldest_chat(Some(true), false)
                        .await
                        .ok_or_log()
                        .flatten()
                        .map(|c| c.packed().ok_or_log())
                        .flatten();
                    if let Some(chat) = chat {
                        self.quit_chat(chat).await.into_log();
                    } else {
                        warn!("尝试退出聊天以腾出空间时发生错误，放弃处理");
                        return None;
                    }
                    return Some(());
                }
            }
            _ => (),
        };
        None
    }
}

pub struct Interval(Mutex<tokio::time::Interval>);
impl Interval {
    pub fn from_secs(freq: u64) -> Self {
        Self(Mutex::new(tokio::time::interval(Duration::from_secs(freq))))
    }

    pub fn from_millis(freq: u64) -> Self {
        Self(Mutex::new(tokio::time::interval(Duration::from_millis(
            freq,
        ))))
    }

    pub async fn tick(&self) -> () {
        self.0.lock().await.tick().await;
    }
}

pub struct IntervalSet {
    pub join_chat: Interval,
    pub quit_chat: Interval,
    pub unpack_chat: Interval,
    pub resolve_username: Interval,
    pub find_msg: Interval,
}
impl Default for IntervalSet {
    fn default() -> Self {
        Self {
            join_chat: Interval::from_secs(300),
            quit_chat: Interval::from_secs(300),
            resolve_username: Interval::from_secs(60),
            unpack_chat: Interval::from_millis(500),
            find_msg: Interval::from_millis(15),
        }
    }
}

async fn wait_on_flood<T>(result: &Result<T, InvocationError>) -> Option<()> {
    let e = if let Err(e) = result { e } else { return None };
    match e {
        InvocationError::Rpc(e) => {
            if e.code == 420 {
                // TODO: 添加and逻辑，e.name.eq(name of FLOOD)
                warn!("捕获服务器警告FLOOD_WAIT");
                if let Some(cooldown) = e.value {
                    warn!(cooldown, "尝试休眠");
                    tokio::time::sleep(Duration::from_secs(cooldown as u64)).await;
                    warn!(cooldown, "结束休眠");
                    return Some(());
                }
            }
        }
        _ => (),
    };
    None
}
