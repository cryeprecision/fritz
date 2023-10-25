mod connection;
pub use connection::*;

mod model;
pub use model::*;

pub mod util {
    use anyhow::Context;
    use chrono::{DateTime, Local, TimeZone, Utc};

    pub fn utc_timestamp_to_local(timestamp: i64) -> anyhow::Result<DateTime<Local>> {
        Ok(Utc
            .timestamp_millis_opt(timestamp)
            .single()
            .context("utc time from timestamp")?
            .with_timezone(&Local))
    }
    pub fn local_to_utc_timestamp(local: DateTime<Local>) -> i64 {
        local.with_timezone(&Utc).timestamp_millis()
    }
}
