use std::fmt::Display;
use std::str::FromStr;

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Local, ParseError};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use reqwest::Client;
use serde::Deserialize;
use thiserror::Error;

use super::log_msg::LogMsg;
use super::traits::FromLogEntry;

use crate::Session;

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
#[derive(Debug)]
pub struct LogEntry {
    pub time: DateTime<Local>,
    pub msg: LogMsg,
    pub raw_msg: String,
    pub err_id: u32,
}

impl Display for LogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let age = self.age(&Local::now());
        let hours = age.num_hours();
        let minutes = age.num_minutes() % 60;
        write!(f, "{:02}h {:02}m - {:?}", hours, minutes, self.msg)
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
        let err_id = u32::from_str(&entry.msg_id).context("couldn't parse error message id")?;
        let msg = LogMsg::from_log_entry(&entry).context("couldn't parse into log msg")?;

        Ok(LogEntry {
            time,
            msg,
            raw_msg: entry.msg,
            err_id,
        })
    }

    pub fn from_json(json: &str) -> Result<Vec<LogEntry>> {
        let logs = serde_json::from_str::<Response>(json)
            .context("couldn't parse raw json for logs")?
            .data
            .logs;

        logs.into_iter()
            .map(Self::parse_entry)
            .collect::<Result<Vec<LogEntry>>>()
    }

    pub fn age(&self, now: &DateTime<Local>) -> Duration {
        now.signed_duration_since(self.time)
    }

    pub async fn fetch(client: &Client, session: &Session) -> Result<Vec<LogEntry>> {
        const URL: &str = "https://fritz.box/data.lua";

        let form = [
            ("page", "log".to_string()),
            ("lang", "de".to_string()),
            ("filter", "0".to_string()),
            ("sid", session.to_string()),
        ];

        let req = client.post(URL).form(&form);
        let resp = req.send().await?.text().await?;
        Self::from_json(&resp)
    }
}
