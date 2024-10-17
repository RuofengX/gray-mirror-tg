use std::io::{self, BufRead};

use anyhow::{anyhow, Result};
use dotenv_codegen::dotenv;
use grammers_client::{session::Session, Client, Config, InitParams, SignInError};

// 编译时获取
const API_ID: &str = dotenv!("API_ID");
const API_HASH: &str = dotenv!("API_HASH");
const PHONE_NUMBER: &str = dotenv!("PHONE_NUMBER");
const SESSION_FILE: &str = dotenv!("SESSION_FILE");

pub async fn login_with_dotenv() -> Result<Client> {
    println!("开始连接");
    let mut params: InitParams = Default::default();
    let _ = params.proxy_url.insert("socks5://localhost:2018".to_string());
    let config = Config {
        session: Session::load_file_or_create(SESSION_FILE)?, //
        api_id: API_ID.parse()?,
        api_hash: API_HASH.to_string(),
        params,
    };
    let client = grammers_client::Client::connect(config).await?;
    println!("连接成功");

    if !client.is_authorized().await? {
        println!("会话未登陆");

        println!("使用账号{}", PHONE_NUMBER);
        let token = client.request_login_code(PHONE_NUMBER).await?;
        println!("请查看TG并输入验证码，回车结束");
        let code = io::stdin()
            .lock()
            .lines()
            .next()
            .ok_or(anyhow!("cannot iter stdin"))??;

        println!("开始登陆");
        let signed_in = client.sign_in(&token, &code).await;

        match signed_in {
            Err(SignInError::PasswordRequired(password_token)) => {
                println!("此次登陆需要密码");
                if let Some(hint) = password_token.hint() {
                    println!("密码提示：{}", hint)
                }
                let password = rpassword::prompt_password(
                    "请输入密码，回车结束（出于安全考虑，密码不会显示）",
                )?;

                client
                    .check_password(password_token, password.trim())
                    .await?;
            }
            Err(e) => return Err(anyhow!(e)),
            _ => (),
        };

        println!("登陆成功");
        client.session().save_to_file(SESSION_FILE)?;
        println!("会话已保存");
    }
    println!("会话已登陆");
    Ok(client)
}
