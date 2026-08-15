use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("could not extract the CSRF token from the login page")]
    CsrfTokenNotFound,

    #[error("login failed: the server did not set an authenticated session cookie")]
    InvalidCredentials,

    #[error(
        "account is locked: the site detected abnormal access (too fast / too many downloads). Send an appeal if this is a mistake: https://fuoverflow.com/anti-crawl-appeal/"
    )]
    AccountBanned,

    #[error("invalid redirect location")]
    InvalidRedirect,

    #[error("too many redirects")]
    TooManyRedirects,
}

pub type Result<T> = std::result::Result<T, Error>;
