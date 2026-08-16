use std::io::Write as _;
use std::time::Duration;

use anyhow::{Context, Result};
use fuo::prelude::{Error, FuoClient};
use keyring::{Entry, Error as KeyringError};

/// Credential-manager identity under which the session cookies are stored.
const SERVICE: &str = "openfpt";
const USERNAME: &str = "session";

/// Persist the session cookies in the OS credential manager (Windows
/// Credential Manager, macOS Keychain, Linux Secret Service).
pub fn save_session(client: &FuoClient) -> Result<()> {
    secure_entry()?
        .set_password(&client.session_cookies())
        .context("could not save the session to the credential manager")
}

pub fn load_session() -> Result<Option<String>> {
    match secure_entry().and_then(|entry| entry.get_password()) {
        Ok(cookies) => Ok(Some(cookies)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(err) => Err(err).context("could not read the session from the credential manager"),
    }
}

pub fn logout() -> Result<()> {
    match secure_entry().and_then(|entry| entry.delete_credential()) {
        Ok(()) => println!("Logged out."),
        Err(KeyringError::NoEntry) => println!("No saved session."),
        Err(err) => {
            return Err(err).context("could not remove the credential-manager entry");
        }
    }
    Ok(())
}

fn secure_entry() -> Result<Entry, KeyringError> {
    Entry::new(SERVICE, USERNAME)
}

/// Authenticate (interactively or with provided credentials), persist the
/// session, and report the outcome. A banned account prints a friendly notice
/// instead of failing.
pub async fn login(login: Option<String>, password: Option<String>) -> Result<()> {
    let login = match login {
        Some(login) => login,
        None => prompt("Login: ")?,
    };
    let password = match password {
        Some(password) => password,
        None => rpassword::prompt_password("Password: ")?,
    };

    let mut client = FuoClient::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    match client.login(&login, &password, true).await {
        Ok(()) => {
            save_session(&client)?;
            println!("Authenticated: {}", client.is_logged_in());
        }
        Err(Error::AccountBanned) => {
            println!(
                "Account is locked: the site detected abnormal access (truy cập quá nhanh / tải nhiều ảnh)."
            );
            println!(
                "Send an appeal if this is a mistake: https://fuoverflow.com/anti-crawl-appeal/"
            );
        }
        Err(err) => return Err(err.into()),
    }
    Ok(())
}

fn prompt(text: &str) -> Result<String> {
    print!("{text}");
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_owned())
}
