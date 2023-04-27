use anyhow::Context;
use dialoguer::theme::ColorfulTheme;
use fritz_log_parser::{logger, Client, Connection};

pub async fn prompt_username(usernames: &[String]) -> String {
    let usernames_copy = usernames.to_vec();
    tokio::task::spawn_blocking(move || {
        let index = dialoguer::Select::with_theme(&ColorfulTheme::default())
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
        dialoguer::Password::with_theme(&ColorfulTheme::default())
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
    logger::init()
        .context("couldn't initialize logger")
        .unwrap();

    let db = Connection::open("./logs.db3")
        .context("couldn't open logs database file")
        .unwrap();

    db.create_logs_table()
        .context("couldn't create logs table")
        .unwrap();

    let client = Client::new();

    let session_response = client.session_response().await.unwrap();

    let username = prompt_username(&session_response.users).await;
    let password = prompt_password(&username).await;
    let response = session_response.challenge.response(&password);

    let session = client.session_id(&username, response).await.unwrap();

    let logs = client.logs(&session).await.unwrap();

    client.logout(session).await.unwrap();
    println!("logged out");

    let new_count = db.append_logs(&logs).unwrap();
    println!("inserted {new_count} new logs");
}
