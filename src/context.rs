use std::{ops::Deref, sync::Arc, time::Duration};

use anyhow::Result;
use dotenv_codegen::dotenv;
use grammers_client::{
    types::{Chat, PackedChat},
    Client, InvocationError,
};
use tokio::{
    sync::{Mutex, RwLock},
    task::JoinSet,
};
use tracing::{error, level_filters::STATIC_MAX_LEVEL, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use url::Url;

use crate::{
    persist::Database,
    update::{UpdateApp, Updater},
    App, PrintError, Runable,
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

        let rtn = Self(Arc::new(ContextInner {
            client: crate::login::login_with_dotenv().await?,
            background_tasks: Mutex::new(background_tasks),
            persist: Database::new().await?,
            update: RwLock::new(UpdateApp::new()),
            interval: Default::default(),
        }));

        Ok(rtn)
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
            result.unwrap_or_log();
        }
        warn!("全部任务结束");
        Ok(())
    }

    pub async fn resolve_username(&self, username: &str) -> Result<Option<Chat>> {
        self.interval.resolve_username.tick().await;
        let mut rtn = self.client.resolve_username(username).await;
        if let Err(e) = rtn {
            wait_on_flood(e).await;
            warn!("重新尝试");
            self.interval.resolve_username.tick().await;
            rtn = self.client.resolve_username(username).await;
        }
        let rtn = rtn.unwrap_or_log().flatten();
        Ok(rtn)
    }

    pub async fn join_chat(&self, chat: impl Into<PackedChat>) -> Result<Option<Chat>> {
        self.interval.join_chat.tick().await;
        let chat = Into::<PackedChat>::into(chat);
        let id = chat.id;
        let mut rtn = self.client.join_chat(chat).await;
        if let Err(e) = rtn {
            wait_on_flood(e).await;
            warn!("重新尝试");
            self.interval.join_chat.tick().await;
            rtn = self.client.join_chat(chat).await;
        }
        let rtn = rtn.unwrap_or_log().flatten();
        if let Some(rtn) = &rtn {
            warn!(chat_name = rtn.name(), chat_id = rtn.id(), "加入聊天");
        } else {
            error!(chat_id = id, "加入聊天失败");
        }
        Ok(rtn)
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
    pub unpack_chat: Interval,
    pub resolve_username: Interval,
    pub find_msg: Interval,
}
impl Default for IntervalSet {
    fn default() -> Self {
        Self {
            join_chat: Interval::from_secs(300),
            resolve_username: Interval::from_secs(60),
            unpack_chat: Interval::from_millis(500),
            find_msg: Interval::from_millis(15),
        }
    }
}

async fn wait_on_flood(e: InvocationError) -> Option<()> {
    match e {
        InvocationError::Rpc(e) => {
            if e.code == 420 {
                warn!("服务器警告FLOOD_WAIT");
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
    return None;
}
