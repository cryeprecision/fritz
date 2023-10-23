use anyhow::Context;
use fritz_log_parser::{db, logger, login};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    logger::init().context("initialize logger")?;
    let path = dotenv::dotenv().context("load .env file")?;
    log::info!("loaded .env from {}", path.to_str().expect("utf-8"));

    let db_url = std::env::var("DATABASE_URL").unwrap_or("sqlite://logs.db3".to_string());
    let db = db::Database::open(&db_url).await.context("open database")?;
    db.clear_logs().await?; // <-!-!-!-------------------------------------------

    let client = login::Client::new(None, None, None, None).await?;

    let mut logs = client.logs().await.context("fetch logs")?;
    logs.reverse();

    log::info!("entries in db: {}", db.logs_count().await?);
    let _ = db.append_new_logs(&logs).await.context("insert logs 1")?;
    log::info!("entries in db: {}", db.logs_count().await?);
    let appended_2 = db.append_new_logs(&logs).await.context("insert logs 2")?;
    log::info!("entries in db: {}", db.logs_count().await?);
    let appended_3 = db.append_new_logs(&logs).await.context("insert logs 3")?;
    log::info!("entries in db: {}", db.logs_count().await?);

    log::info!("appended 2: {:?}", appended_2);
    log::info!("appended 3: {:?}", appended_3);

    db.close().await;
    Ok(())
}
