use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use reqwest::cookie::{CookieStore, Jar};
use reqwest::{Client, Url};
use tokio::io::AsyncWriteExt;

use crate::constants::{BAN_MARKER, BASE_URL, HOME_URL, LOGIN_EP, LOGIN_URL, USER_AGENT};
use crate::error::{Error, Result};
use crate::model::attachment::Attachment;
use crate::model::comment::Comment;
use crate::model::thread::{ForumPage, Thread, ThreadPage};
use crate::repository::forum::{parse_threads, total_pages};
use crate::repository::login::LoginRequest;
use crate::repository::thread::{parse_attachments, parse_comments, parse_thread_title};

#[derive(Clone)]
pub struct FuoClient {
    client: Client,
    jar: Arc<Jar>,
    authenticated: bool,
    banned: bool,
    delay: Duration,
}

impl FuoClient {
    pub fn builder() -> FuoClientBuilder {
        FuoClientBuilder::default()
    }

    pub fn new() -> Result<Self> {
        Self::builder().build()
    }

    pub async fn login(&mut self, login: &str, password: &str, remember: bool) -> Result<()> {
        let xf_token = self.fetch_csrf_token().await?;
        let body = LoginRequest::new(xf_token, login, password, remember);

        let res = self
            .client
            .post(LOGIN_EP)
            .form(&body.to_form())
            .send()
            .await?;

        if !res.status().is_redirection() {
            return Err(Error::InvalidCredentials);
        }

        if let Some(location) = res.headers().get(reqwest::header::LOCATION) {
            let location = location.to_str().unwrap_or(HOME_URL);
            let url = if location.starts_with('/') {
                format!("{BASE_URL}{location}")
            } else {
                location.to_owned()
            };
            let res = self.client.get(&url).send().await?;
            let html = res.text().await.unwrap_or_default();
            if is_banned(&html) {
                self.banned = true;
                return Err(Error::AccountBanned);
            }
        }

        self.authenticated = true;
        Ok(())
    }

    pub const fn is_banned(&self) -> bool {
        self.banned
    }

    pub fn set_cookie(&self, cookie: &str, url: &str) {
        let url = Url::parse(url).expect("cookie url must be valid");
        self.jar.add_cookie_str(cookie, &url);
    }

    pub fn session_cookies(&self) -> String {
        self.cookies(HOME_URL)
    }

    pub fn restore_cookies(&self, cookies: &str) {
        let url = Url::parse(HOME_URL).expect("valid url");
        for pair in cookies
            .split(';')
            .map(str::trim)
            .filter(|pair| !pair.is_empty())
        {
            self.jar.add_cookie_str(&format!("{pair}; Path=/"), &url);
        }
    }

    pub fn is_logged_in(&self) -> bool {
        self.authenticated || self.cookies(HOME_URL).contains("xf_user=")
    }

    pub async fn get(&self, path: &str) -> Result<String> {
        let bytes = self.get_bytes(path).await?;
        let html = String::from_utf8_lossy(&bytes).into_owned();
        if is_banned(&html) {
            return Err(Error::AccountBanned);
        }
        Ok(html)
    }

    pub async fn get_bytes(&self, path: &str) -> Result<Vec<u8>> {
        let mut url = format!("{BASE_URL}{path}");
        for _ in 0..10 {
            if !self.delay.is_zero() {
                tokio::time::sleep(self.delay).await;
            }
            let res = self.client.get(&url).send().await?;
            if res.status().is_redirection() {
                let location = res
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or(Error::InvalidRedirect)?;
                url = if location.starts_with('/') {
                    format!("{BASE_URL}{location}")
                } else {
                    location.to_owned()
                };
                continue;
            }
            let res = res.error_for_status()?;
            return Ok(res.bytes().await?.to_vec());
        }
        Err(Error::TooManyRedirects)
    }

    pub async fn threads(&self, subject: &str, page: u32) -> Result<ForumPage> {
        let subject = subject.to_uppercase();
        let path = if page <= 1 {
            format!("/forums/{subject}/")
        } else {
            format!("/forums/{subject}/page-{page}")
        };
        let html = self.get(&path).await?;
        Ok(ForumPage {
            threads: parse_threads(&html),
            page,
            total_pages: total_pages(&html),
        })
    }

    pub async fn search_subject(&self, subject: &str) -> Result<Vec<Thread>> {
        let first = self.threads(subject, 1).await?;
        let mut all = first.threads;
        for page in 2..=first.total_pages {
            all.extend(self.threads(subject, page).await?.threads);
        }
        Ok(all)
    }

    pub async fn thread_page(&self, id: u32) -> Result<ThreadPage> {
        let html = self.get(&format!("/threads/t.{id}/")).await?;
        Ok(ThreadPage {
            title: parse_thread_title(&html),
            attachments: parse_attachments(&html),
        })
    }

    pub async fn attachment_comments(&self, attachment: &Attachment) -> Result<Vec<Comment>> {
        let (Some(slug), Some(media_id)) = (&attachment.media_slug, attachment.media_id) else {
            return Ok(Vec::new());
        };
        let html = self
            .get(&format!("/media/{slug}.{media_id}/?lightbox=1"))
            .await?;
        Ok(parse_comments(&html))
    }

    pub async fn download_attachment(&self, attachment: &Attachment) -> Result<Vec<u8>> {
        self.get_bytes(&attachment.url).await
    }

    pub async fn download_attachment_to(
        &self,
        attachment: &Attachment,
        dest: &Path,
    ) -> Result<u64> {
        self.download_to(&attachment.url, dest).await
    }

    async fn download_to(&self, path: &str, dest: &Path) -> Result<u64> {
        let mut url = format!("{BASE_URL}{path}");
        for _ in 0..10 {
            if !self.delay.is_zero() {
                tokio::time::sleep(self.delay).await;
            }
            let res = self.client.get(&url).send().await?;
            if res.status().is_redirection() {
                let location = res
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or(Error::InvalidRedirect)?;
                url = if location.starts_with('/') {
                    format!("{BASE_URL}{location}")
                } else {
                    location.to_owned()
                };
                continue;
            }
            let mut res = res.error_for_status()?;
            let mut file = tokio::fs::File::create(dest).await?;
            while let Some(chunk) = res.chunk().await? {
                file.write_all(&chunk).await?;
            }
            file.flush().await?;
            return Ok(file.metadata().await?.len());
        }
        Err(Error::TooManyRedirects)
    }

    async fn fetch_csrf_token(&self) -> Result<String> {
        let html = self
            .client
            .get(LOGIN_URL)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        if is_banned(&html) {
            return Err(Error::AccountBanned);
        }
        if is_bot_check(&html) {
            return Err(Error::BotChallenge);
        }
        extract_csrf_token(&html).map_err(|_| Error::UnexpectedLoginPage {
            preview: page_preview(&html),
        })
    }

    fn cookies(&self, url: &str) -> String {
        let url = Url::parse(url).expect("url must be valid");
        self.jar
            .cookies(&url)
            .map(|value| value.to_str().unwrap_or_default().to_owned())
            .unwrap_or_default()
    }
}

pub struct FuoClientBuilder {
    user_agent: String,
    timeout: Option<Duration>,
    delay: Duration,
    cookies: Vec<(String, String)>,
}

impl Default for FuoClientBuilder {
    fn default() -> Self {
        Self {
            user_agent: USER_AGENT.to_owned(),
            timeout: None,
            delay: Duration::from_millis(500),
            cookies: Vec::new(),
        }
    }
}

impl FuoClientBuilder {
    #[must_use]
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    #[must_use]
    pub const fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    #[must_use]
    pub fn cookie(mut self, cookie: impl Into<String>, url: impl Into<String>) -> Self {
        self.cookies.push((cookie.into(), url.into()));
        self
    }

    pub fn build(self) -> Result<FuoClient> {
        let jar = Arc::new(Jar::default());
        let mut builder = Client::builder()
            .cookie_provider(jar.clone())
            .user_agent(self.user_agent)
            .redirect(reqwest::redirect::Policy::none());
        if let Some(timeout) = self.timeout {
            builder = builder.timeout(timeout);
        }
        let client = builder.build()?;

        let client = FuoClient {
            client,
            jar,
            authenticated: false,
            banned: false,
            delay: self.delay,
        };
        for (cookie, url) in self.cookies {
            client.set_cookie(&cookie, &url);
        }
        Ok(client)
    }
}

fn extract_csrf_token(html: &str) -> Result<String> {
    // XenForo 2.2+ puts the token on the root <html> element.
    const MARKER: &str = r#"data-csrf=""#;
    if let Some(start) = html.find(MARKER).map(|index| index + MARKER.len()) {
        if let Some(end) = html[start..].find('"') {
            return Ok(html[start..start + end].to_owned());
        }
    }
    // Older templates only expose it as a hidden `_xfToken` input in forms.
    const INPUT: &str = r#"name="_xfToken" value=""#;
    if let Some(start) = html.find(INPUT).map(|index| index + INPUT.len()) {
        if let Some(end) = html[start..].find('"') {
            return Ok(html[start..start + end].to_owned());
        }
    }
    Err(Error::CsrfTokenNotFound)
}

fn is_bot_check(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    // NB: `challenge-platform` is NOT a reliable marker — the real login page
    // embeds Cloudflare Turnstile from that domain. Only match the
    // interstitial challenge page itself.
    [
        "just a moment",
        "cf-chl-",
        "cf-browser-verification",
        "captcha",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn page_preview(html: &str) -> String {
    let title = html
        .find("<title>")
        .and_then(|start| {
            let after = html.get(start + 7..)?;
            let end = after.find("</title>")?;
            Some(&after[..end])
        })
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| html.chars().take(160).collect());
    title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(200)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_token_from_data_csrf_attribute() {
        let html =
            r#"<html data-csrf="1786850610,962bc08506b151da26694d33fa4b86ef"><body></body></html>"#;
        assert_eq!(
            extract_csrf_token(html).unwrap(),
            "1786850610,962bc08506b151da26694d33fa4b86ef"
        );
    }

    #[test]
    fn falls_back_to_xf_token_input() {
        let html = r#"<form action="/login/login" method="post"><input type="hidden" name="_xfToken" value="tok1,tok2" /></form>"#;
        assert_eq!(extract_csrf_token(html).unwrap(), "tok1,tok2");
    }

    #[test]
    fn missing_token_is_an_error() {
        assert!(extract_csrf_token("<html><body>no token</body></html>").is_err());
    }

    #[test]
    fn detects_bot_check_markers() {
        assert!(is_bot_check("<html><title>Just a moment...</title></html>"));
        assert!(is_bot_check("<div class=\"cf-chl-container\"></div>"));
        assert!(!is_bot_check("<html data-csrf=\"x\"></html>"));
    }

    #[test]
    fn turnstile_script_is_not_a_bot_check() {
        let html =
            "<html data-csrf=\"x\"><script>challenge-platform/scripts/jsd/main.js</script></html>";
        assert!(!is_bot_check(html));
    }
}

pub fn is_banned(html: &str) -> bool {
    html.contains(BAN_MARKER)
}
