use serde::Deserialize;

/// A single log entry.
///
/// - `[0]`: Date (`31.12.23`)
/// - `[1]`: Time (`23:59:59`)
/// - `[2]`: Message
/// - `[3]`: Message ID
/// - `[4]`: Category ID
/// - `[5]`: Link to help page
#[derive(Debug, Clone, Deserialize)]
pub struct Log(pub [String; 6]);

/// The `data` field in the response.
#[derive(Debug, Clone, Deserialize)]
pub struct Data {
    #[serde(rename = "log")]
    pub logs: Vec<Log>,
}

/// The whole response.
#[derive(Debug, Clone, Deserialize)]
pub struct Response {
    pub data: Data,
}
