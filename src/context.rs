use std::future::Future;

use anyhow::Result;
use dotenv_codegen::dotenv;
use grammers_client::{Client, Update};
use tokio::{
    sync::broadcast::{self, Sender},
    task::JoinSet,
};
use tracing::{info, info_span, trace, Instrument};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use url::Url;

use crate::app::{App, UpdateRuntime, Updater};

pub struct Context {
    pub client: Client,
    update_sender: Sender<Update>,
    background_tasks: JoinSet<Result<()>>,
}
impl Context {
    pub async fn new() -> Result<Self> {
        let (s, _r) = broadcast::channel(1024);
        let mut background_tasks = JoinSet::new();

        let logger = tracing_subscriber::registry();
        let loki_url = dotenv!("LOKI_URL");
        if loki_url == "" {
            logger.with(tracing_subscriber::fmt::Layer::new()).init();
        } else {
            let (layer, task) = tracing_loki::builder()
                .label("service_name", "gray-mirror-tg")?
                .label("version", std::env::var("CARGO_PKG_VERSION").unwrap())?
                .build_url(Url::parse(&loki_url)?)?;

            background_tasks.spawn(async move {
                task.await;
                Ok(()) // map () -> Result(())
            });
            logger
                .with(layer)
                .with(tracing_subscriber::fmt::Layer::new())
                .init();
        }

        let rtn = Self {
            client: crate::login::login_with_dotenv().await?,
            update_sender: s,
            background_tasks,
        };

        Ok(rtn)
    }

    pub async fn add_app(&mut self, mut app: impl App + 'static) -> Result<()> {
        let name = format!("{}", app);
        let update_span = info_span!("应用", name);
        let _span = update_span.enter();

        trace!("初始化(ignite)");
        app.ignite(self).await?;
        trace!("自动注册更新器");
        self.add_updater(app);
        Ok(())
    }

    pub fn add_updater(&mut self, updater: impl Updater + 'static) -> () {
        let name = format!("{}", &updater);
        let update_span = info_span!("更新器", name);

        let recv = self.update_sender.subscribe();
        let runtime = UpdateRuntime::new(recv, self.client.clone(), Box::new(updater));

        self.add_background_task(
            &format!("{}", runtime),
            async move {
                info!("{} > 启动", &runtime);
                runtime.run().await
            }
            .instrument(update_span),
        );
    }

    pub fn add_background_task(
        &mut self,
        name: &str,
        task: impl Future<Output = Result<()>> + Send + 'static,
    ) -> () {
        let bg_span = info_span!("后台任务", name);

        self.background_tasks.spawn(
            async move {
                info!("启动");
                task.await
            }
            .instrument(bg_span),
        );
    }

    pub fn start_listen_updates(&mut self) -> () {
        let client = self.client.clone();
        let sender = self.update_sender.clone();

        self.add_background_task("更新监听", async move {
            loop {
                let update = client.next_update().await?;
                trace!("发送");
                sender.send(update)?;
            }
        });
    }

    /// Run until error occurs. Return first error.
    pub async fn run(mut self) -> Result<()> {
        let main_span = info_span!("主进程");
        let _span = main_span.enter();

        while let Some(result) = self.background_tasks.join_next().await {
            result??;
        }
        Ok(())
    }
}
