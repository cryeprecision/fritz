mod logger;

use std::time::Duration;

use anyhow::Context;
use chrono::Local;
use fritz_api::{api, db};
use tokio::time::MissedTickBehavior;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    logger::init().context("initialize logger")?;

    match dotenv::dotenv() {
        Ok(path) => log::info!("loaded .env from {}", path.to_str().expect("utf-8")),
        Err(err) => log::warn!("couldn't load .env file: {:?}", err),
    };

    let db_url = std::env::var("DATABASE_URL").context("load DATABASE_URL")?;
    let db = db::Database::open(&db_url).await.context("open database")?;

    let client = api::Client::new(None, None, None, None, Some(&db)).await?;
    let _ = client.login().await.context("initial login attempt")?;

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
        // fetch all logs from the FRITZ!Box
        //
        // if the logs couldn't be fetched, try again because
        // the reason could be that the FRITZ!Box is restarting
        // or the reason is something else ¯\_(ツ)_/¯
        let logs = loop {
            // wait for next tick
            interval.tick().await;

            match client.logs().await {
                Ok(mut logs) => {
                    logs.reverse();
                    break logs;
                }
                Err(err) => {
                    log::warn!("couldn't fetch logs: {:?}", err);
                    continue;
                }
            }
        };

        // append all new logs to the database
        let upserted = db
            .append_new_logs(&logs)
            .await
            .context("insert logs")?
            .len();

        if let Err(err) = db
            .insert_update(&db::Update {
                id: None,
                datetime: db::util::local_to_utc_timestamp(Local::now()),
                upserted_rows: upserted.min(i64::MAX as usize) as i64,
            })
            .await
        {
            log::warn!("couldn't insert update metadata into db: {:?}", err);
        }

        log::info!("upserted {} logs", upserted);
    }
}
