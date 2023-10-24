use std::time::Duration;

use anyhow::Context;
use fritz_log_parser::{db, logger, login};
use tokio::time::MissedTickBehavior;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    logger::init().context("initialize logger")?;
    let path = dotenv::dotenv().context("load .env file")?;
    log::info!("loaded .env from {}", path.to_str().expect("utf-8"));

    let db_url = std::env::var("DATABASE_URL").context("load DATABASE_URL")?;
    let db = db::Database::open(&db_url).await.context("open database")?;
    let client = login::Client::new(None, None, None, None).await?;

    let mut interval = {
        let pause_seconds = std::env::var("FRITZBOX_REFRESH_PAUSE_SECONDS")
            .context("load FRITZBOX_REFRESH_PAUSE_SECONDS")?
            .parse::<u64>()
            .context("parse FRITZBOX_REFRESH_PAUSE_SECONDS")?;

        let mut interval = tokio::time::interval(Duration::from_secs(pause_seconds));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        interval
    };

    loop {
        // wait for next tick
        interval.tick().await;

        // fetch all logs from the FRITZ!Box
        let mut logs = client.logs().await.context("fetch logs")?;
        logs.reverse();

        // append all new logs to the database
        let upserted = db
            .append_new_logs(&logs)
            .await
            .context("insert logs")?
            .len();
        log::info!("upserted {} logs", upserted);
    }
}
