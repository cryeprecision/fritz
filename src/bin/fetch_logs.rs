use fritz_log_parser::{
    logs::{InternetMsg, LogEntry, LogMsg},
    Session,
};

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
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    let (challenge, users) = Session::get_challenge_and_users(&client).await.unwrap();

    let username = prompt_username(&users).await;
    let password = prompt_password(&username).await;
    let response = challenge.hash(&password);

    let session = Session::get_session_id(&client, &username, &response)
        .await
        .unwrap();
    println!("session: {:?}", session.id);

    let logs = LogEntry::fetch(&client, &session).await.unwrap();
    println!("fetched logs ({})", logs.len());

    let disconnects = logs
        .iter()
        .filter(|l| l.msg.is_internet())
        .filter(|&i| matches!(&i.msg, LogMsg::Internet(InternetMsg::Disconnected)))
        .collect::<Vec<_>>();

    session.logout(&client).await.unwrap();
    println!("logged out");

    for disconnect in disconnects {
        println!("{disconnect}");
    }
}
