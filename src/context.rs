use std::{ops::Deref, sync::Arc, time::Duration};

use anyhow::Result;
use dotenv_codegen::dotenv;
use grammers_client::Client;
use tokio::{
    sync::{Mutex, RwLock},
    task::JoinSet,
};
use tracing::{error, level_filters::STATIC_MAX_LEVEL, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use url::Url;

use crate::{
    channel::Channel,
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
    pub channel: Channel,
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
                // .label("version", std::env::var("CARGO_PKG_VERSION").unwrap())?
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
            channel: Default::default(),
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
        self.background_tasks
            .lock()
            .await
            .spawn(async move { value.run(ctx).await.into_log() });
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
    pub bot_resend: Interval,
}
impl Default for IntervalSet {
    fn default() -> Self {
        Self {
            join_chat: Interval::from_secs(300),
            bot_resend: Interval::from_secs(15),
            resolve_username: Interval::from_secs(10),
            unpack_chat: Interval::from_millis(500),
            find_msg: Interval::from_millis(20),
        }
    }
}
