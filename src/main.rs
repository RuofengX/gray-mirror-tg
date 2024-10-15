use anyhow::Result;
use tokio;

mod client;

/// 处理增量数据
mod update;

/// 处理存量数据
mod history;

/// 利用soso等机器人挖掘关联群组
mod finder;



#[tokio::main]
async fn main() -> Result<()> {
    println!("你好世界!");

    // 获取客户端
    let client = client::login_with_dotenv().await?;

    let mut dialogs = client.iter_dialogs();
    while let Some(d) = dialogs.next().await? {
        let chat = d.chat();
        println!("{}, {}", chat.name(), chat.id());
    }
    let a = client.next_update().await;


    Ok(())
}
