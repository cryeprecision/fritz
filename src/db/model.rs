/// Database representation of a log row from the Fritz!BOX
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
