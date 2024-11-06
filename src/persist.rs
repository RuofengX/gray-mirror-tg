use anyhow::{bail, Result};
use dotenv_codegen::dotenv;
use grammers_client::types::PackedChat;
use sea_orm::{
    prelude::*, sea_query::OnConflict, ConnectOptions, DbBackend, IntoActiveModel, Order,
    QueryOrder, Schema, Set, Statement, TransactionTrait,
};
use tracing::debug;

use crate::{
    types::{chat, link, message, search},
    Context,
};

pub struct Database {
    pub db: DatabaseConnection,
}
impl Database {
    pub const DB_URL: &'static str = dotenv!("DATABASE_URL");
    pub async fn new() -> Result<Self> {
        debug!("{}", Self::DB_URL);
        let mut opt = ConnectOptions::new(Self::DB_URL.to_owned());
        opt.sqlx_logging(false); // Disable SQLx log

        let db = sea_orm::Database::connect(opt).await?;

        let builder = db.get_database_backend();
        let schema = Schema::new(builder);

        db.execute(
            builder.build(
                schema
                    .create_table_from_entity(link::Entity)
                    .if_not_exists(),
            ),
        )
        .await?;
        db.execute(
            builder.build(
                schema
                    .create_table_from_entity(search::Entity)
                    .if_not_exists(),
            ),
        )
        .await?;
        db.execute(
            builder.build(
                schema
                    .create_table_from_entity(message::Entity)
                    .if_not_exists(),
            ),
        )
        .await?;
        db.execute(
            builder.build(
                schema
                    .create_table_from_entity(chat::Entity)
                    .if_not_exists(),
            ),
        )
        .await?;

        Ok(Self { db })
    }

    pub async fn put_message(&self, data: message::ActiveModel) -> Result<message::Model> {
        let (chat_id, msg_id) = if let (Some(chat_id), Some(msg_id)) =
            (data.chat_id.clone().take(), data.msg_id.clone().take())
        {
            (chat_id, msg_id)
        } else {
            bail!("put_message方法未提供chat_id与msg_id")
        };

        let trans = self.db.begin().await?;
        let _ = message::Entity::insert(data)
            .on_conflict_do_nothing()
            .exec(&trans)
            .await?;

        let ret = message::Entity::find_by_id((chat_id, msg_id))
            .one(&trans)
            .await?
            .expect("事务进行中");

        trans.commit().await?;
        Ok(ret)
    }

    pub async fn put_chat(&self, data: chat::ActiveModel) -> Result<chat::Model> {
        let chat_id = if let Some(chat_id) = data.chat_id.clone().take() {
            chat_id
        } else {
            bail!("put_chat未提供chat_id")
        };
        let exist = chat::Entity::find()
            .filter(chat::Column::ChatId.eq(chat_id))
            .one(&self.db)
            .await?;
        if let Some(exist) = exist {
            Ok(exist)
        } else {
            let ret = chat::Entity::insert(data)
                .on_conflict(
                    OnConflict::column(chat::Column::ChatId)
                        .do_nothing()
                        .to_owned(),
                )
                .exec_with_returning(&self.db)
                .await?;
            Ok(ret)
        }
    }

    pub async fn put_link(&self, data: link::ActiveModel) -> Result<link::Model> {
        let link = if let Some(link) = data.link.clone().take() {
            link
        } else {
            bail!("put_link未提供link")
        };
        let exist = link::Entity::find()
            .filter(link::Column::Link.eq(link))
            .one(&self.db)
            .await?;
        if let Some(exist) = exist {
            Ok(exist)
        } else {
            let ret = link::Entity::insert(data)
                .on_conflict(
                    OnConflict::column(link::Column::Link)
                        .do_nothing()
                        .to_owned(),
                )
                .exec_with_returning(&self.db)
                .await?;
            Ok(ret)
        }
    }

    pub async fn put_search(&self, data: search::ActiveModel) -> Result<search::Model> {
        let ret = data.insert(&self.db).await?;
        Ok(ret)
    }

    pub async fn find_chat(&self, username: Option<&str>) -> Result<Option<chat::Model>> {
        if username.is_none() {
            return Ok(None);
        }
        let username = username.unwrap();

        let raw_sql = Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"SELECT * FROM "chat" WHERE $1 = ANY("usernames")"#,
            [username.into()],
        );
        let ret = chat::Entity::find()
            .from_raw_sql(raw_sql)
            .one(&self.db)
            .await?;

        Ok(ret)
    }

    pub async fn set_link_extracted(
        &self,
        link_id: i32,
        packed: Option<PackedChat>,
    ) -> Result<Option<link::Model>> {
        let exist = link::Entity::find_by_id(link_id).one(&self.db).await?;
        if let Some(exist) = exist {
            let mut model = exist.into_active_model();
            model.parsed = Set(true);
            model.packed = Set(packed.map(|p| p.to_hex()));
            let updated = model.update(&self.db).await?;
            Ok(Some(updated))
        } else {
            Ok(None)
        }
    }

    pub async fn find_oldest_chat(
        &self,
        joined: Option<bool>,
    ) -> Result<Option<chat::Model>> {
        let mut select = chat::Entity::find();
        if let Some(j) = joined {
            select = select.filter(chat::Column::Joined.eq(j));
        };
        let ret = select
            .order_by(chat::Column::LastUpdate, Order::Asc)
            .one(&self.db)
            .await?;
        Ok(ret)
    }

    pub async fn find_latest_chat(
        &self,
        joined: Option<bool>,
    ) -> Result<Option<chat::Model>> {
        let mut select = chat::Entity::find();
        if let Some(j) = joined {
            select = select.filter(chat::Column::Joined.eq(j));
        };
        let ret = select
            .order_by(chat::Column::LastUpdate, Order::Desc)
            .one(&self.db)
            .await?;

        Ok(ret)
    }

    pub async fn set_chat_joined(&self, chat_id: i64) -> Result<Option<chat::Model>> {
        let exist = chat::Entity::find_by_id(chat_id).one(&self.db).await?;
        if let Some(exist) = exist {
            let mut model = exist.into_active_model();
            model.joined = Set(true);
            let updated = model.update(&self.db).await?;
            Ok(Some(updated))
        } else {
            Ok(None)
        }
    }

    pub async fn set_chat_quited(&self, chat_id: i64) -> Result<Option<chat::Model>> {
        let exist = chat::Entity::find_by_id(chat_id).one(&self.db).await?;
        if let Some(exist) = exist {
            let mut model = exist.into_active_model();
            model.joined = Set(false);
            let updated = model.update(&self.db).await?;
            Ok(Some(updated))
        } else {
            Ok(None)
        }
    }

    pub async fn set_chat_updated(
        &self,
        chat_id: i64,
        last_update: DateTime,
    ) -> Result<Option<chat::Model>> {
        let exist = chat::Entity::find_by_id(chat_id).one(&self.db).await?;
        if let Some(exist) = exist {
            let mut model = exist.into_active_model();
            model.last_update = Set(last_update);
            let updated = model.update(&self.db).await?;
            Ok(Some(updated))
        } else {
            Ok(None)
        }
    }

    pub async fn sync_chat_joined(&self, ctx: Context) -> Result<()> {
        let trans = self.db.begin().await?;
        chat::Entity::update_many()
            .col_expr(chat::Column::Joined, Expr::value(false))
            .exec(&trans)
            .await?;
        let mut chats = ctx.client.iter_dialogs();
        while let Some(chat) = chats.next().await? {
            let chat_id = chat.chat.id();
            let exist = chat::Entity::find_by_id(chat_id).one(&self.db).await?;
            if let Some(exist) = exist {
                let mut model = exist.into_active_model();
                model.joined = Set(true);
                model.update(&trans).await?;
            }
        }
        Ok(())
    }
}
