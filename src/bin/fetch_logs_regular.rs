use std::time::Duration;

use anyhow::Context;
use fritz_log_parser::{logger, Client, Connection};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    logger::init()
        .context("couldn't initialize logger")
        .unwrap();

    let path = dotenv::dotenv().context("couldn't load .env").unwrap();
    log::info!(
        "loaded .env from {}",
        path.to_str().unwrap_or("[invalid utf-8]")
    );

    let db_url = std::env::var("DATABASE_URL").unwrap_or("sqlite://logs.db3".to_string());
    let db = Connection::open(&db_url)
        .await
        .context("couldn't open logs database")
        .unwrap();

    db.create_logs_table()
        .await
        .context("couldn't create logs table")
        .unwrap();

    let client = Client::new(None, None, None, None).await.unwrap();

    loop {
        let logs = client.logs().await.context("couldn't fetch logs").unwrap();

        let new_count = db
            .append_logs(&logs)
            .await
            .context("couldn't insert new entries")
            .unwrap();

        log::info!("inserted {} new logs", new_count);
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
