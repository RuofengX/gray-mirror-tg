use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use tokio::sync::mpsc;

use crate::{context::Context, types::{chat, favorite, message}};

//！ 递归搜索群组聊天中的群组链接
pub struct DeepDive;

async fn get_favorites(ctx: Context, sender: mpsc::Sender<favorite::Model>){
    let db = &ctx.persist.db;
    let iter = favorite::Entity::find().paginate(db, 8);
    while let Some(favorites) = iter.fetch_and_next().await?{
        let tasks = tokio::task::JoinSet::new();
    }

}

async fn dive(ctx: Context, chat:favorite::Model){
    let db = &ctx.persist.db;
    message::Entity::find().filter(
        message::Column::ChatId.eq(chat.chat_id)
    ).all(db).await.;
}
