use std::fmt::Display;
use std::str::FromStr;

use anyhow::{Context, Result};
use chrono::{
    DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, ParseError, TimeZone,
};
use serde::Deserialize;
use thiserror::Error;

use super::log_msg::LogMsg;
use super::traits::FromLogEntry;

pub struct RawLogEntry {
    pub date: String,
    pub time: String,
    pub msg: String,
    pub msg_id: String,
    pub category: String,
    pub help: String,
}

impl From<[String; 6]> for RawLogEntry {
    fn from(array: [String; 6]) -> Self {
        let mut iter = array.into_iter();
        RawLogEntry {
            date: iter.next().unwrap(),
            time: iter.next().unwrap(),
            msg: iter.next().unwrap(),
            msg_id: iter.next().unwrap(),
            category: iter.next().unwrap(),
            help: iter.next().unwrap(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ResponseData {
    #[serde(rename = "log")]
    pub logs: Vec<[String; 6]>,
}

#[derive(Debug, Deserialize)]
struct Response {
    pub data: ResponseData,
}

/// The whole log entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub time: DateTime<Local>,
    pub msg: LogMsg,
    pub raw_msg: String,
    pub msg_id: i64,
}

impl LogEntry {
    pub fn new(raw_msg: String, msg_id: i64, category: i64, time: i64) -> LogEntry {
        let time = Local.timestamp_opt(time, 0).unwrap();
        let msg = LogMsg::from_category_and_msg(category, &raw_msg).unwrap();

        LogEntry {
            time,
            msg,
            raw_msg,
            msg_id,
        }
    }
}

impl Display for LogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let age = self.age(&Local::now());
        let days = age.num_days();
        let hours = age.num_hours() % 24;
        let minutes = age.num_minutes() % 60;
        let seconds = age.num_seconds() % 60;
        write!(f, "{days:02}d {hours:02}h {minutes:02}m {seconds:02}s")?;
        write!(f, " - {:?}", self.msg)
    }
}

#[derive(Debug, Error)]
pub enum ParseLogError {
    #[error("invalid date format: {0}")]
    Date(ParseError),
    #[error("invalid time format: {0}")]
    Time(ParseError),
    #[error("ambiguous date-time")]
    Single,
}
type ParseLogResult<T> = std::result::Result<T, ParseLogError>;

impl LogEntry {
    fn parse_time(entry: &RawLogEntry) -> ParseLogResult<DateTime<Local>> {
        let naive_date =
            NaiveDate::parse_from_str(&entry.date, "%d.%m.%y").map_err(ParseLogError::Date)?;
        let naive_time =
            NaiveTime::parse_from_str(&entry.time, "%H:%M:%S").map_err(ParseLogError::Time)?;

        NaiveDateTime::new(naive_date, naive_time)
            .and_local_timezone(Local)
            .single()
            .ok_or(ParseLogError::Single)
    }

    fn parse_entry(arr: [String; 6]) -> Result<LogEntry> {
        let entry = RawLogEntry::from(arr);
        let time = Self::parse_time(&entry).context("couldn't parse date/time")?;
        let msg_id = i64::from_str(&entry.msg_id).context("couldn't parse error message id")?;
        let msg = LogMsg::from_log_entry(&entry).context("couldn't parse into log msg")?;

        Ok(LogEntry {
            time,
            msg,
            raw_msg: entry.msg,
            msg_id,
        })
    }

    pub fn from_json(json: &str) -> Result<Vec<LogEntry>> {
        let logs = serde_json::from_str::<Response>(json)
            .context("couldn't parse raw json for logs")?
            .data
            .logs;

        let mut logs = logs
            .into_iter()
            .map(Self::parse_entry)
            .collect::<Result<Vec<_>>>()?;

        // sorted from old to new instead of from new to old
        // while keeping the order between elements with the
        // same timestamp.
        logs.reverse();

        Ok(logs)
    }

    pub fn age(&self, now: &DateTime<Local>) -> Duration {
        now.signed_duration_since(self.time)
    }
}
