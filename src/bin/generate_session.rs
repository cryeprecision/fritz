use anyhow::Context;
use fritz_log_parser::{logger, login};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    logger::init().context("initialize logger")?;

    match dotenv::dotenv() {
        Ok(path) => log::info!("loaded .env from {}", path.to_str().expect("utf-8")),
        Err(err) => log::warn!("couldn't load .env file: {:?}", err),
    };

    let client = login::Client::new(None, None, None, None, None).await?;
    let session = client.login().await.context("initial login attempt")?;

    log::info!("session-id: {}", session);

    Ok(())
}
