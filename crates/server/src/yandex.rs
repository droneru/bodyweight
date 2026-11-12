//! OAuth with Yandex ID: https://yandex.ru/dev/id/doc/ru/codes/code-url

use anyhow::Context;
use bodyweight_domain::User;
use reqwest::Url;
use serde::Deserialize;

const AUTHORIZE_URL: &str = "https://oauth.yandex.ru/authorize";
const TOKEN_URL: &str = "https://oauth.yandex.ru/token";
const INFO_URL: &str = "https://login.yandex.ru/info?format=json";

pub struct YandexOAuth {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    http: reqwest::Client,
}

/// What Yandex tells about the person who signed in.
#[derive(Debug, PartialEq, Eq)]
pub struct Profile {
    pub uid: String,
    pub user: User,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

/// Response of `login.yandex.ru/info`. Avatar fields need the `login:avatar` permission.
#[derive(Deserialize)]
struct Info {
    id: String,
    login: String,
    display_name: Option<String>,
    default_avatar_id: Option<String>,
    #[serde(default)]
    is_avatar_empty: bool,
}

impl From<Info> for Profile {
    fn from(info: Info) -> Self {
        let avatar_id = info
            .default_avatar_id
            .filter(|id| !info.is_avatar_empty && !id.is_empty());
        let display_name = info
            .display_name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| info.login.clone());
        Profile {
            uid: info.id,
            user: User {
                login: info.login,
                display_name,
                avatar_id,
            },
        }
    }
}

impl YandexOAuth {
    /// `redirect_uri` must be registered in the Yandex OAuth application.
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri,
            http: reqwest::Client::new(),
        }
    }

    /// Where to send the browser to start signing in.
    pub fn authorize_url(&self, state: &str) -> String {
        let mut url = Url::parse(AUTHORIZE_URL).expect("valid URL");
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_uri)
            .append_pair("state", state);
        url.into()
    }

    /// Exchanges the confirmation code from the callback for the user's profile.
    pub async fn fetch_profile(&self, code: &str) -> anyhow::Result<Profile> {
        let token: TokenResponse = self
            .http
            .post(TOKEN_URL)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
            ])
            .send()
            .await
            .context("token request failed")?
            .error_for_status()
            .context("token request rejected")?
            .json()
            .await
            .context("unexpected token response")?;

        let info: Info = self
            .http
            .get(INFO_URL)
            .header("Authorization", format!("OAuth {}", token.access_token))
            .send()
            .await
            .context("profile request failed")?
            .error_for_status()
            .context("profile request rejected")?
            .json()
            .await
            .context("unexpected profile response")?;
        Ok(info.into())
    }
}

/// URL of the user's avatar, 84×84 px.
pub fn avatar_url(avatar_id: &str) -> String {
    format!("https://avatars.yandex.net/get-yapic/{avatar_id}/islands-retina-middle")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(json: &str) -> Profile {
        serde_json::from_str::<Info>(json).unwrap().into()
    }

    #[test]
    fn authorize_url_carries_the_escaped_redirect_uri_and_state() {
        let oauth = YandexOAuth::new(
            "id123".into(),
            "secret".into(),
            "https://bodyweight.ru/api/auth/yandex/callback".into(),
        );
        assert_eq!(
            oauth.authorize_url("st&ate"),
            "https://oauth.yandex.ru/authorize?response_type=code&client_id=id123\
             &redirect_uri=https%3A%2F%2Fbodyweight.ru%2Fapi%2Fauth%2Fyandex%2Fcallback\
             &state=st%26ate"
        );
    }

    #[test]
    fn profile_with_avatar() {
        let p = profile(
            r#"{"id":"1000034426","login":"ivan","client_id":"x","display_name":"Ivan",
                "default_avatar_id":"131652443","is_avatar_empty":false,"psuid":"1.AAA"}"#,
        );
        assert_eq!(p.uid, "1000034426");
        assert_eq!(p.user.login, "ivan");
        assert_eq!(p.user.display_name, "Ivan");
        assert_eq!(p.user.avatar_id.as_deref(), Some("131652443"));
    }

    #[test]
    fn empty_avatar_is_dropped() {
        let p = profile(
            r#"{"id":"1","login":"ivan","display_name":"Ivan",
                "default_avatar_id":"0/0-0","is_avatar_empty":true}"#,
        );
        assert_eq!(p.user.avatar_id, None);
    }

    #[test]
    fn without_avatar_permission_and_display_name() {
        let p = profile(r#"{"id":"1","login":"ivan"}"#);
        assert_eq!(p.user.avatar_id, None);
        assert_eq!(p.user.display_name, "ivan");
    }
}
