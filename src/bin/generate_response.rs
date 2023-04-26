use std::str::FromStr;

use anyhow::{Context, Result};
use dialoguer::theme::ColorfulTheme;
use fritz_log_parser::Challenge;

fn ask_challenge() -> Result<Challenge> {
    let input = dialoguer::Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Challenge")
        .report(false)
        .allow_empty(false)
        .validate_with(|input: &String| Challenge::from_str(input).map(|_| ()))
        .interact()?;
    Ok(Challenge::from_str(&input).unwrap())
}
pub fn ask_password() -> Result<Vec<u8>> {
    Ok(dialoguer::Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Password")
        .allow_empty_password(false)
        .report(false)
        .interact()?
        .into_bytes())
}

fn main() {
    let ch = ask_challenge()
        .context("couldn't ask for challenge")
        .unwrap();
    let pw = ask_password().context("couldn't ask for password").unwrap();
    println!("{}", ch.response(&pw));
}
