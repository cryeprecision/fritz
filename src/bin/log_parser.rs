use anyhow::Context;
use chrono::Local;
use fritz_log_parser::logs::{InternetMsg, LogEntry};

fn main() {
    let data = std::fs::read_to_string("./example_logs.json")
        .context("couldn't read example response")
        .unwrap();
    let mut parsed = LogEntry::from_json(&data)
        .context("couldn't parse example response")
        .unwrap();

    let now = Local::now();
    parsed.retain(|l| l.age(&now).num_hours() <= 24);

    for line in &parsed {
        println!("{line:?}");
    }

    let disconnects = parsed
        .iter()
        .filter_map(|l| l.msg.internet())
        .filter(|&i| matches!(i, InternetMsg::Disconnected))
        .count();
    println!("Disconnects in the last 24 hours: {disconnects}");
}
