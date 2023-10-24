use anyhow::Context;
use chrono::{DateTime, Local};
use serde::Serialize;

use crate::{api, db};

#[derive(Debug, Clone, Serialize, Hash, PartialEq, Eq)]
pub struct Repetition {
    pub datetime: DateTime<Local>,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Hash, PartialEq, Eq)]
pub struct Log {
    pub datetime: DateTime<Local>,
    pub message: String,
    pub message_id: i64,
    pub category_id: i64,
    pub repetition: Option<Repetition>,
}

impl Log {
    pub fn earliest_timestamp(&self) -> i64 {
        self.repetition
            .as_ref()
            .map_or(self.datetime.timestamp_millis(), |rep| {
                rep.datetime.timestamp_millis()
            })
    }
    pub fn latest_timestamp(&self) -> i64 {
        self.datetime.timestamp_millis()
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
        db::Log {
            id: None,
            datetime: value.datetime.timestamp_millis(),
            message: value.message,
            message_id: value.message_id,
            category_id: value.category_id,
            repetition_datetime: value
                .repetition
                .as_ref()
                .map(|r| r.datetime.timestamp_millis()),
            repetition_count: value.repetition.as_ref().map(|r| r.count),
        }
    }
}

impl TryFrom<db::Log> for Log {
    type Error = anyhow::Error;
    fn try_from(value: db::Log) -> Result<Self, Self::Error> {
        Ok(Log {
            datetime: util::timestamp_to_local(value.datetime)?,
            message: value.message,
            message_id: value.message_id,
            category_id: value.category_id,
            repetition: util::parse_repetition(value.repetition_datetime, value.repetition_count)?,
        })
    }
}

impl TryFrom<api::Log> for Log {
    type Error = anyhow::Error;
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
                let datetime = util::parse_datetime(&date, &time)?;
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
    use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

    use super::Repetition;

    pub fn timestamp_to_local(timestamp: i64) -> anyhow::Result<DateTime<Local>> {
        Local
            .timestamp_millis_opt(timestamp)
            .single()
            .context("timestamp to local time")
    }

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
                let datetime = timestamp_to_local(datetime)?;
                Ok(Some(Repetition { datetime, count }))
            }
            (None, None) => Ok(None),
            // Either both are set or none
            v @ _ => Err(anyhow::anyhow!("invalid repetition {:?}", v)),
        }
    }
}
