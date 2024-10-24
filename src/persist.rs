use anyhow::Result;
use dotenv_codegen::dotenv;
use sea_orm::{
    sea_query::OnConflict, ActiveModelTrait, ColumnTrait, ConnectOptions, ConnectionTrait,
    DatabaseConnection, EntityTrait, QueryFilter, Schema,
};
use tracing::{debug, info_span, instrument, Level};

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

    #[instrument(skip(self), level = Level::DEBUG)]
    pub async fn put_message(&self, data: message::ActiveModel) -> Result<message::Model> {
        let span = info_span!("提交消息");
        let _span = span.enter();

        let exist = message::Entity::find()
            .filter(message::Column::ChatId.eq(data.chat_id.clone().into_value().unwrap()))
            .filter(message::Column::MsgId.eq(data.msg_id.clone().into_value().unwrap()))
            .one(&self.db)
            .await?;
        if let Some(exist) = exist {
            Ok(exist)
        } else {
            let rtn = data.insert(&self.db).await?;
            Ok(rtn)
        }
    }

    #[instrument(skip(self), level = Level::DEBUG)]
    pub async fn put_chat(&self, data: chat::ActiveModel) -> Result<chat::Model> {
        let span = info_span!("提交群组");
        let _span = span.enter();

        let exist = chat::Entity::find()
            .filter(chat::Column::ChatId.eq(data.chat_id.clone().into_value().unwrap()))
            .one(&self.db)
            .await?;
        if let Some(exist) = exist {
            debug!("重复");
            Ok(exist)
        } else {
            debug!("排除重复");
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

    #[instrument(skip(self), level = Level::DEBUG)]
    pub async fn put_link(&self, data: link::ActiveModel) -> Result<link::Model> {
        let exist = link::Entity::find()
            .filter(link::Column::Link.eq(data.link.clone().into_value().unwrap()))
            .one(&self.db)
            .await?;
        if let Some(exist) = exist {
            Ok(exist)
        } else {
            let rtn = data.insert(&self.db).await?;
            Ok(rtn)
        }
    }

    #[instrument(skip(self), level = Level::DEBUG)]
    pub async fn put_search(&self, data: search::ActiveModel) -> Result<search::Model> {
        let rtn = data.insert(&self.db).await?;
        Ok(rtn)
    }

    #[instrument(skip(self), level = Level::DEBUG)]
    pub async fn find_chat(&self, username: &str) -> Result<Option<chat::Model>> {
        let rtn = chat::Entity::find()
            .filter(chat::Column::Usernames.contains(username)) //FIXME: 此处需修改为pgsql能够识别的形式
            .one(&self.db)
            .await?;

        Ok(rtn)
    }
}
