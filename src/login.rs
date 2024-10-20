use std::io::{self, BufRead};

use anyhow::{anyhow, Result};
use dotenv_codegen::dotenv;
use grammers_client::{session::Session, Client, Config, InitParams, SignInError};
use tracing::{info, info_span, warn};

// 编译时获取
const API_ID: &str = dotenv!("API_ID");
const API_HASH: &str = dotenv!("API_HASH");
const PHONE_NUMBER: &str = dotenv!("PHONE_NUMBER");
const SESSION_FILE: &str = dotenv!("SESSION_FILE");
const SOCKS5_PROXY: &str = dotenv!("SOCKS5_PROXY");

pub async fn login_with_dotenv() -> Result<Client> {
    let login_span = info_span!("客户端登陆");
    let _span = login_span.enter();

    info!("开始连接");
    let mut params: InitParams = Default::default();
    if SOCKS5_PROXY != "" {
        info!("使用Socks5代理{}", SOCKS5_PROXY);
        let _ = params.proxy_url.insert(SOCKS5_PROXY.to_string());
    }
    let config = Config {
        session: Session::load_file_or_create(SESSION_FILE)?, //
        api_id: API_ID.parse()?,
        api_hash: API_HASH.to_string(),
        params,
    };
    let client = grammers_client::Client::connect(config).await?;
    info!("连接成功");

    if !client.is_authorized().await? {
        warn!("会话未登陆");

        info!("使用账号{}", PHONE_NUMBER);
        let token = client.request_login_code(PHONE_NUMBER).await?;
        info!("请查看TG并输入验证码，回车结束");
        let code = io::stdin()
            .lock()
            .lines()
            .next()
            .ok_or(anyhow!("cannot iter stdin"))??;

        info!("开始登陆");
        let signed_in = client.sign_in(&token, &code).await;

        match signed_in {
            Err(SignInError::PasswordRequired(password_token)) => {
                warn!("此次登陆需要密码");
                if let Some(hint) = password_token.hint() {
                    info!("密码提示：{}", hint)
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

        info!("登陆成功");
        client.session().save_to_file(SESSION_FILE)?;
        info!("会话已保存");
    }
    info!("会话已登陆");
    Ok(client)
}
