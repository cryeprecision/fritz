use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use fritz_log_parser::{db, logger, login};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    logger::init().context("initialize logger")?;
    let path = dotenv::dotenv().context("load .env file")?;
    log::info!("loaded .env from {}", path.to_str().expect("utf-8"));

    let stop = Arc::new(AtomicBool::new(false));
    let stop_spawn = Arc::clone(&stop);
    let _ = tokio::task::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        log::info!("received ctrl+c signal");
        stop_spawn.store(true, Ordering::Relaxed);
    });

    let db_url = std::env::var("DATABASE_URL").unwrap_or("sqlite://logs.db3".to_string());
    let db = db::Database::open(&db_url).await.context("open database")?;
    let client = login::Client::new(None, None, None, None).await?;

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        let logs = client.logs().await.context("fetch logs")?;
        db.append_logs(&logs).await.context("insert logs")?;
        log::info!("inserted {} logs", logs.len());
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    db.close().await;

    Ok(())
}
