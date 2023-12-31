use anyhow::Context;
use chrono::{DateTime, Local};
use serde::Serialize;

use crate::api;
use crate::db::util::{local_to_utc_timestamp, utc_timestamp_to_local};
use crate::db::{self};

/// If a message was logged multiple times, this struct contains
/// the date at which it was *first* logged and the number of times it was logged.
#[derive(Debug, Clone, Serialize, Hash, PartialEq, Eq)]
pub struct Repetition {
    pub datetime: DateTime<Local>,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Hash, PartialEq, Eq)]
pub struct Log {
    /// Timestamp at which this log entry was last updated.
    ///
    /// If a [`Repetition`] is added or updated, that counts as an update.
    pub datetime: DateTime<Local>,
    pub message: String,
    pub message_id: i64,
    pub category_id: i64,
    pub repetition: Option<Repetition>,
}

impl Log {
    pub fn earliest_timestamp_utc(&self) -> i64 {
        self.repetition
            .as_ref()
            .map_or(local_to_utc_timestamp(self.datetime), |rep| {
                local_to_utc_timestamp(rep.datetime)
            })
    }
    pub fn latest_timestamp_utc(&self) -> i64 {
        local_to_utc_timestamp(self.datetime)
    }
}

impl std::fmt::Display for Log {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{:>4}, {:>2}] {}",
            self.message_id, self.category_id, self.datetime
        )?;
        if let Some(rep) = self.repetition.as_ref() {
            write!(f, " ({:>2} since {})", rep.count, rep.datetime)?;
        }
        Ok(())
    }
}

impl From<Log> for db::Log {
    fn from(value: Log) -> Self {
        let (datetime, count) = match value.repetition {
            Some(Repetition { datetime, count }) => {
                (Some(local_to_utc_timestamp(datetime)), Some(count))
            }
            None => (None, None),
        };

        db::Log {
            id: None,
            datetime: local_to_utc_timestamp(value.datetime),
            message: value.message,
            message_id: value.message_id,
            category_id: value.category_id,
            repetition_datetime: datetime,
            repetition_count: count,
        }
    }
}

impl TryFrom<db::Log> for Log {
    type Error = anyhow::Error;
    /// Convert logs from the database format into a common format
    fn try_from(value: db::Log) -> Result<Self, Self::Error> {
        Ok(Log {
            datetime: utc_timestamp_to_local(value.datetime)?,
            message: value.message,
            message_id: value.message_id,
            category_id: value.category_id,
            repetition: util::parse_repetition(value.repetition_datetime, value.repetition_count)?,
        })
    }
}

impl TryFrom<api::Log> for Log {
    type Error = anyhow::Error;
    /// Convert logs from the API into a common format.
    fn try_from(value: api::Log) -> Result<Self, Self::Error> {
        let [date, time, mut message, message_id, category_id, _] = value.0;
        let datetime = util::parse_datetime(&date, &time)?;
        let message_id = message_id.parse().context("parse message id")?;
        let category_id = category_id.parse().context("parse category id")?;

        // this code is in its own block beucase it deserves it
        let repetition = {
            // extract important parts from the repetition message
            lazy_regex::regex_captures!(
                r#" \[(\d+) Meldungen seit (\d+\.\d+\.\d+) (\d+:\d+:\d+)\]$"#,
                &message
            )
            // if important parts are there, parse them
            .map(|(whole_match, count, date, time)| -> anyhow::Result<_> {
                let datetime = util::parse_datetime(date, time)?;
                let count = count.parse().context("parse count")?;
                let repetition = Repetition { datetime, count };
                Ok((repetition, whole_match.len()))
            })
            // handle possible error from parsing
            .transpose()
            .context("parse repetition message")?
            // remove the repetition message from the string
            .map(|(repetition, len)| {
                message.truncate(message.len() - len);
                repetition
            })
        };

        Ok(Log {
            datetime,
            message,
            message_id,
            category_id,
            repetition,
        })
    }
}

mod util {
    use anyhow::Context;
    use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime};

    use super::Repetition;
    use crate::db::util::utc_timestamp_to_local;

    /// DateTimes from the API are in the local timezone
    pub fn parse_datetime(date: &str, time: &str) -> anyhow::Result<DateTime<Local>> {
        let date = NaiveDate::parse_from_str(date, "%d.%m.%y").context("parse datetime date")?;
        let time = NaiveTime::parse_from_str(time, "%H:%M:%S").context("parse datetime time")?;
        NaiveDateTime::new(date, time)
            .and_local_timezone(Local)
            .single()
            .context("datetime into local timezone")
    }

    pub fn parse_repetition(
        datetime: Option<i64>,
        count: Option<i64>,
    ) -> anyhow::Result<Option<Repetition>> {
        match (datetime, count) {
            (Some(datetime), Some(count)) => {
                let datetime = utc_timestamp_to_local(datetime)?;
                Ok(Some(Repetition { datetime, count }))
            }
            (None, None) => Ok(None),
            // Either both are set or none
            v => Err(anyhow::anyhow!("invalid repetition {:?}", v)),
        }
    }
}
