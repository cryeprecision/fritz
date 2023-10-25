/// A log row from the Fritz!BOX logs
#[derive(Debug, Clone)]
pub struct Log {
    pub id: Option<i64>,
    pub datetime: i64,
    pub message: String,
    pub message_id: i64,
    pub category_id: i64,
    pub repetition_datetime: Option<i64>,
    pub repetition_count: Option<i64>,
}

/// Information about a request to the FRITZ!Box
#[derive(Debug, Clone, Default)]
pub struct Request {
    pub id: Option<i64>,
    pub datetime: i64,
    pub url: String,
    pub method: String,
    pub duration_ms: i64,
    pub response_code: Option<i64>,
    pub session_id: Option<String>,
}

/// Information about updates
#[derive(Debug, Clone)]
pub struct Update {
    pub id: Option<i64>,
    pub datetime: i64,
    pub upserted_rows: i64,
}
