use anyhow::Result;
use tokio;

mod client;


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

    Ok(())
}
