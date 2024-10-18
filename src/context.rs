use anyhow::Result;
use grammers_client::{Client, Update};
use tokio::sync::broadcast::{self, Sender};

use crate::app::{App, UpdateRuntime, Updater};

pub struct Context {
    pub client: Client,
    update_parser: Vec<UpdateRuntime>,
    update_sender: Sender<Update>,
}
impl Context {
    pub async fn new(client: Client) -> Result<Self> {
        let (s, _r) = broadcast::channel(1024);
        Ok(Self {
            client,
            update_parser: Vec::new(),
            update_sender: s,
        })
    }

    pub async fn add_app<T: App + 'static>(&mut self, mut app: T) -> Result<()> {
        app.ignite(self).await?;
        self.add_updater(app);
        Ok(())
    }

    pub fn add_updater(&mut self, updater: impl Updater + 'static) -> () {
        let recv = self.update_sender.subscribe();
        let parser = UpdateRuntime::new(recv, self.client.clone(), Box::new(updater));
        self.update_parser.push(parser);
    }

    pub async fn run(self) -> Result<()> {
        let mut tasks = tokio::task::JoinSet::new();

        for mut i in self.update_parser {
            tasks.spawn({
                async move {
                    i.update_daemon()
                        .await
                        .expect("update daemon exit with error");
                }
            });
        }
        tasks.spawn(async move {
            loop {
                let _ = Self::update_send(&self.client, &self.update_sender).await;
            }
        });
        tasks.join_all().await;
        Ok(())
    }

    async fn update_send(client: &Client, sender: &Sender<Update>) -> Result<()> {
        let update = client.next_update().await?;
        sender.send(update)?;
        Ok(())
    }
}
