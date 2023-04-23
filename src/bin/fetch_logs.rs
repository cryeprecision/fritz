use fritz_log_parser::logs::LogEntry;
use fritz_log_parser::{Connection, Session};

pub async fn prompt_username(usernames: &[String]) -> String {
    let usernames_copy = usernames.to_vec();
    tokio::task::spawn_blocking(move || {
        let index = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Select a user")
            .clear(true)
            .default(0)
            .items(&usernames_copy)
            .report(true)
            .interact()
            .unwrap();
        usernames_copy.into_iter().nth(index).unwrap()
    })
    .await
    .unwrap()
}
pub async fn prompt_password(username: &str) -> Vec<u8> {
    let prompt = format!("Enter password for `{username}`");
    tokio::task::spawn_blocking(move || {
        dialoguer::Password::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(&prompt)
            .allow_empty_password(false)
            .report(true)
            .interact()
            .unwrap()
    })
    .await
    .unwrap()
    .into_bytes()
}

#[tokio::main(flavor = "current_thread")]
pub async fn main() {
    let db = Connection::open("./logs.db3").unwrap();
    db.create_logs_table().unwrap();

    if db.latest_logs(Some(1)).unwrap().is_empty() {
        println!("db empty, appending example logs");

        let data_1 = std::fs::read_to_string("./example_logs.json").unwrap();
        let parsed_1 = LogEntry::from_json(&data_1).unwrap();

        let data_2 = std::fs::read_to_string("./example_logs_2.json").unwrap();
        let parsed_2 = LogEntry::from_json(&data_2).unwrap();

        db.append_logs(&parsed_1).unwrap();
        db.append_logs(&parsed_2).unwrap();
    }

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    let (challenge, users) = Session::get_challenge_and_users(&client).await.unwrap();
    println!("got the challenge");

    let username = prompt_username(&users).await;
    let password = prompt_password(&username).await;
    let response = challenge.response(&password);

    let session = Session::get_session_id(&client, &username, &response.to_string())
        .await
        .unwrap();
    println!("session: {:?}", session.id);

    let logs = LogEntry::fetch(&client, &session).await.unwrap();
    println!("fetched logs ({})", logs.len());

    session.logout(&client).await.unwrap();
    println!("logged out");

    let new_count = db.append_logs(&logs).unwrap();
    println!("inserted {new_count} new logs");
}
