use chrono::{DateTime, Utc};

/// A log row from the Fritz!BOX logs
#[derive(Debug, Clone, serde::Serialize)]
pub struct Log {
    pub id: Option<i64>,
    pub datetime: DateTime<Utc>,
    pub message: String,
    pub message_id: i64,
    pub category_id: i64,
    pub repetition_datetime: Option<DateTime<Utc>>,
    pub repetition_count: Option<i64>,
}

/// Information about a request to the FRITZ!Box
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Request {
    pub id: Option<i64>,
    pub datetime: DateTime<Utc>,
    pub name: String,
    pub url: String,
    pub method: String,
    pub duration_ms: i64,
    pub response_code: Option<i64>,
    pub session_id: Option<String>,
}

/// Information about updates
#[derive(Debug, Clone, serde::Serialize)]
pub struct Update {
    pub id: Option<i64>,
    pub datetime: DateTime<Utc>,
    pub upserted_rows: i64,
}

/// Information about pings
#[derive(Debug, Clone)]
pub struct Ping {
    pub id: Option<i64>,
    pub datetime: DateTime<Utc>,
    pub target: String,
    pub duration_ms: Option<i64>,
    pub ttl: Option<i64>,
    pub bytes: Option<i64>,
}
