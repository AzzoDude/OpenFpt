use crate::constants::XF_REDIRECT;

pub struct LoginRequest {
    xf_token: String,
    login: String,
    password: String,
    remember: bool,
    xf_redirect: String,
}

impl LoginRequest {
    pub fn new(
        xf_token: impl Into<String>,
        login: impl Into<String>,
        password: impl Into<String>,
        remember: bool,
    ) -> Self {
        Self {
            xf_token: xf_token.into(),
            login: login.into(),
            password: password.into(),
            remember,
            xf_redirect: XF_REDIRECT.to_owned(),
        }
    }

    pub fn to_form(&self) -> Vec<(&'static str, String)> {
        vec![
            ("_xfToken", self.xf_token.clone()),
            ("login", self.login.clone()),
            ("password", self.password.clone()),
            ("remember", if self.remember { "1" } else { "0" }.to_owned()),
            ("_xfRedirect", self.xf_redirect.clone()),
        ]
    }
}
