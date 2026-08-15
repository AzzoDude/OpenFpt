use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use fuo::prelude::{Error, FuoClient};

pub fn session_path() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let dir = exe.parent().unwrap_or_else(|| Path::new("."));
    Ok(dir.join("session.txt"))
}

pub fn save_session(client: &FuoClient) -> Result<()> {
    std::fs::write(session_path()?, client.session_cookies())?;
    Ok(())
}

pub fn load_session() -> Result<Option<String>> {
    let path = session_path()?;
    if path.exists() {
        Ok(Some(std::fs::read_to_string(path)?))
    } else {
        Ok(None)
    }
}

/// Authenticate (interactively or with provided credentials), persist the
/// session next to the executable, and report the outcome. A banned account
/// prints a friendly notice instead of failing.
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

pub fn logout() -> Result<()> {
    let path = session_path()?;
    if path.exists() {
        std::fs::remove_file(&path)?;
        println!("Logged out.");
    } else {
        println!("No saved session.");
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
