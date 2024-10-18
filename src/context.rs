use anyhow::Result;
use const_random::const_random;
use dotenv_codegen::dotenv;
use grammers_client::{Client, Update};
use tokio::{
    sync::broadcast::{self, Sender},
    task::JoinSet,
};
use tracing::{error, info, info_span, trace, Instrument};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use url::Url;

use crate::app::{App, BackgroundTask, UpdateRuntime, Updater};

pub struct Context {
    pub client: Client,
    update_parser: Vec<UpdateRuntime>,
    update_sender: Sender<Update>,
    background_tasks: JoinSet<()>,
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
                .label("project", "gray-mirror-tg")?
                .label("version", std::env::var("CARGO_PKG_VERSION").unwrap())?
                .build_url(Url::parse(&loki_url)?)?;

            background_tasks.spawn(task);
            logger
                .with(layer)
                .with(tracing_subscriber::fmt::Layer::new())
                .init();
        }

        let rtn = Self {
            client: crate::client::login_with_dotenv().await?,
            update_parser: Vec::new(),
            update_sender: s,
            background_tasks,
        };

        Ok(rtn)
    }

    pub async fn add_app(&mut self, mut app: impl App + 'static) -> Result<()> {
        info!("add app > {}", app);
        trace!("running ignite > {}", app);
        app.ignite(self).await?;
        trace!("auto add app's updater > {}", app);
        self.add_updater(app);
        Ok(())
    }

    pub fn add_updater(&mut self, updater: impl Updater + 'static) -> () {
        info!("add updater > {}", updater);
        let recv = self.update_sender.subscribe();
        let parser = UpdateRuntime::new(recv, self.client.clone(), Box::new(updater));
        self.update_parser.push(parser);
    }

    pub fn add_background_task<T: BackgroundTask + 'static>(&mut self, mut task: T) -> () {
        info!("[background]{} > start", task);
        let bg_span = info_span!("background");
        let client = self.client.clone();
        self.background_tasks.spawn(async move {
            if let Err(e) = task.start(client).instrument(bg_span).await {
                error!("[background]{task} > end with error >> {e}");
            };
        });
    }

    pub async fn run(self) -> Result<()> {
        let mut tasks = tokio::task::JoinSet::new();

        for mut i in self.update_parser {
            let update_span = info_span!(concat!("update-parser-", const_random!(u64)));
            tasks.spawn(
                async move {
                    i.update_daemon().await;
                }
                .instrument(update_span),
            );
        }
        let sender_span = info_span!("udpate-sender");
        tasks.spawn(
            async move {
                loop {
                    if let Err(e) = Self::update_send(&self.client, &self.update_sender).await {
                        error!("end with error >> {e}")
                    };
                }
            }
            .instrument(sender_span),
        );
        tasks.join_all().await;
        Ok(())
    }

    async fn update_send(client: &Client, sender: &Sender<Update>) -> Result<()> {
        let update = client.next_update().await?;
        trace!("received new update");
        sender.send(update)?;
        Ok(())
    }
}
