use anyhow::Result;
use dotenv_codegen::dotenv;
use grammers_client::types::PackedChat;
use sea_orm::{
    prelude::*, sea_query::OnConflict, ConnectOptions, DbBackend, IntoActiveModel, Order,
    QueryOrder, Schema, Set, Statement, TransactionTrait,
};
use tracing::debug;

use crate::types::{chat, link, message, search};

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
        let chat_id = data.chat_id.clone().unwrap();
        let msg_id = data.msg_id.clone().unwrap();

        let trans = self.db.begin().await?;
        let _ = message::Entity::insert(data)
            .on_conflict_do_nothing()
            .exec(&trans)
            .await?;

        let rtn = message::Entity::find_by_id((chat_id, msg_id))
            .one(&trans)
            .await?
            .expect("事务进行中");

        trans.commit().await?;
        Ok(rtn)
    }

    pub async fn put_chat(&self, data: chat::ActiveModel) -> Result<chat::Model> {
        let exist = chat::Entity::find()
            .filter(chat::Column::ChatId.eq(data.chat_id.clone().unwrap()))
            .one(&self.db)
            .await?;
        if let Some(exist) = exist {
            Ok(exist)
        } else {
            let rtn = chat::Entity::insert(data)
                .on_conflict(
                    OnConflict::column(chat::Column::ChatId)
                        .do_nothing()
                        .to_owned(),
                )
                .exec_with_returning(&self.db)
                .await?;
            Ok(rtn)
        }
    }

    pub async fn put_link(&self, data: link::ActiveModel) -> Result<link::Model> {
        let exist = link::Entity::find()
            .filter(link::Column::Link.eq(data.link.clone().unwrap()))
            .one(&self.db)
            .await?;
        if let Some(exist) = exist {
            Ok(exist)
        } else {
            let rtn = link::Entity::insert(data)
                .on_conflict(
                    OnConflict::column(link::Column::Link)
                        .do_nothing()
                        .to_owned(),
                )
                .exec_with_returning(&self.db)
                .await?;
            Ok(rtn)
        }
    }

    pub async fn put_search(&self, data: search::ActiveModel) -> Result<search::Model> {
        let rtn = data.insert(&self.db).await?;
        Ok(rtn)
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
        let rtn = chat::Entity::find()
            .from_raw_sql(raw_sql)
            .one(&self.db)
            .await?;

        Ok(rtn)
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

    pub async fn find_update_candidate(&self) -> Result<Option<chat::Model>> {
        let ret = chat::Entity::find()
            .order_by(chat::Column::LastUpdated, Order::Asc)
            .one(&self.db).await?;
        Ok(ret)

    }
}
