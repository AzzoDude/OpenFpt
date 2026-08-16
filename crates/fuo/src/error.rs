use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("could not extract the CSRF token from the login page")]
    CsrfTokenNotFound,

    #[error(
        "the login page did not contain a CSRF token — the site may be blocking this IP or the page layout changed. Page preview: {preview}"
    )]
    UnexpectedLoginPage { preview: String },

    #[error(
        "the login page was rejected with HTTP 403 — the site is likely blocking this IP (datacenter/VPN). Run `openfpt login` from your own network, or use a self-hosted GitHub runner."
    )]
    ForbiddenLoginPage,

    #[error(
        "the site served a bot-check page instead of the login form — GitHub Actions runners (datacenter IPs) may be blocked. Run `openfpt login` from your own machine, or use a self-hosted runner."
    )]
    BotChallenge,

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
