use std::time::Duration;

use anyhow::Context;
use chrono::{DateTime, Local};
use dialoguer::theme::ColorfulTheme;
use fritz_log_parser::{logger, Client, Connection};
use log::{info, warn};

pub async fn prompt_username(usernames: &[String]) -> String {
    let usernames_copy = usernames.to_vec();
    tokio::task::spawn_blocking(move || {
        let index = dialoguer::Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a user")
            .clear(true)
            .default(0)
            .items(&usernames_copy)
            .report(false)
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
            .report(false)
            .interact()
            .unwrap()
    })
    .await
    .unwrap()
    .into_bytes()
}
pub async fn ask_reboot() -> bool {
    tokio::task::spawn_blocking(move || {
        let index = dialoguer::Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Reboot?")
            .clear(true)
            .default(0)
            .items(&["No", "Yes"])
            .report(false)
            .interact()
            .unwrap();
        index == 1
    })
    .await
    .unwrap()
}

#[derive(Default)]
struct Timer {
    inner: DateTime<Local>,
}
impl Timer {
    pub fn start(&mut self) {
        self.inner = Local::now();
    }
    pub fn elapsed_ms(&mut self) -> i64 {
        let now = Local::now();
        now.signed_duration_since(self.inner).num_milliseconds()
    }
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

    let client = match Client::new_with_cert("cert.pem").await {
        Ok(client) => {
            info!("found root certificate");
            client
        }
        Err(err) => {
            warn!("accepting invalid certificates ({err})");
            Client::new()
        }
    };

    let mut timer = Timer::default();

    timer.start();
    let session_response = client.session_response().await.unwrap();
    info!("got session response ({}ms)", timer.elapsed_ms());

    let username = prompt_username(&session_response.users).await;
    let password = prompt_password(&username).await;
    let response = session_response.challenge.response(&password);

    timer.start();
    let session = client.session_id(&username, response).await.unwrap();
    info!("authenticated ({}ms)", timer.elapsed_ms());

    timer.start();
    let logs = client.logs(&session).await.unwrap();
    info!("fetched logs ({}ms)", timer.elapsed_ms());

    timer.start();
    let new_count = db.append_logs(&logs).unwrap();
    info!("inserted {} new logs ({}ms)", new_count, timer.elapsed_ms());

    if ask_reboot().await {
        timer.start();
        client.reboot(&session).await.unwrap();
        info!("requested reboot ({}ms)", timer.elapsed_ms());

        info!("waiting until reboot is done...");
        timer.start();
        tokio::time::sleep(Duration::from_secs(1)).await;
        while let Err(err) = client.session_response().await {
            info!("waiting... ({err})");
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        info!("reboot is done ({}ms)", timer.elapsed_ms());
    } else {
        timer.start();
        client.logout(session).await.unwrap();
        info!("invalidated session id ({}ms)", timer.elapsed_ms());
    }
}
