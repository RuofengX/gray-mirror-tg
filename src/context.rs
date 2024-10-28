use std::{future::Future, ops::Deref, sync::Arc, time::Duration};

use anyhow::Result;
use dotenv_codegen::dotenv;
use grammers_client::{types::PackedChat, Client, Update};
use sea_orm::{EntityTrait, PaginatorTrait};
use tokio::{
    sync::{
        broadcast::{self, Sender},
        Mutex,
    },
    task::JoinSet,
};
use tracing::{debug, info, info_span, level_filters::STATIC_MAX_LEVEL, warn, Instrument, Span};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use url::Url;

use crate::{
    app::{self, App, UpdateRuntime, Updater},
    runable::Runable,
    persist::Database,
    types::chat,
    PrintError,
};

pub const BOT_RESP_TIMEOUT: Duration = std::time::Duration::from_secs(120);

pub struct ContextInner {
    pub client: Client,
    pub persist: Database,
    pub interval: IntervalSet,
    background_tasks: Mutex<JoinSet<()>>,
}

#[derive(Clone)]
pub struct Context(Arc<ContextInner>);

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
            interval: Default::default(),
        }));

        Ok(rtn)
    }

    #[deprecated]
    pub async fn add_app(&self, mut app: impl App + 'static) -> Result<()> {
        let span = info_span!("应用", name = format!("{}", app));
        let _span = span.enter();

        warn!("初始化(ignite)");
        app.ignite(self.clone()).in_current_span().await?;
        Ok(())
    }

    #[deprecated]
    pub async fn add_updater(&self, updater: impl Updater + 'static) -> Result<()> {
        let span = info_span!("更新处理器", updater = format!("{}", updater));

        let recv = self.update_sender.subscribe();
        let runtime = UpdateRuntime::new(recv, self.clone(), Box::new(updater));

        self.add_background_task(span, runtime.run())
            .in_current_span()
            .await;
        Ok(())
    }

    #[deprecated]
    pub async fn add_background_task(
        &self,
        span: Span,
        task: impl Future<Output = Result<()>> + Send + 'static,
    ) -> () {
        self.background_tasks.lock().await.spawn(
            async move {
                info!("启动");
                task.await.into_log();
                info!("退出");
            }
            .instrument(span),
        );
    }

    pub async fn add_runable(&self, mut value: impl Runable) -> () {
        let ctx = self.clone();
        self.background_tasks
            .lock()
            .await
            .spawn(async move { value.run(ctx).await.into_log() });
    }

    pub async fn enable_update(&self) -> Result<()> {
        let client = self.client.clone();
        let sender = self.update_sender.clone();

        self.add_background_task(info_span!("监听更新"), async move {
            loop {
                let update = client.next_update().await.unwrap_or_warn();

                if update.is_none() {
                    continue;
                }

                debug!("发送");
                sender.send(update.expect("已处理"))?;
            }
        })
        .await;
        Ok(())
    }

    pub async fn fetch_all_chat_history(&self, limit: usize) -> Result<()> {
        let (send, recv) = tokio::sync::mpsc::channel(64);
        let ctx = self.clone();
        self.add_background_task(info_span!("全库聊天历史"), async move {
            app::fetch_chat::fetch_chat_history(recv, ctx, limit).await?;
            Ok(())
        })
        .await;

        let mut iter = chat::Entity::find()
            .into_partial_model::<chat::PackedChatOnly>()
            .paginate(&self.persist.db, 8);
        while let Some(chats) = iter.fetch_and_next().await? {
            for chat in chats {
                send.send(PackedChat::from_hex(&chat.packed)?).await?;
            }
        }
        Ok(())
    }

    /// Run until error occurs. Return first error.
    pub async fn run(self) -> Result<()> {
        while let Some(result) = self.background_tasks.lock().await.join_next().await {
            let span = info_span!("守护进程");
            let _span = span.enter();
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
            bot_resend: Interval::from_secs(1200),
            resolve_username: Interval::from_secs(10),
            unpack_chat: Interval::from_millis(500),
            find_msg: Interval::from_millis(20),
        }
    }
}
