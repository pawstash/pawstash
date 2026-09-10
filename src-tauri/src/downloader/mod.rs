pub mod aria2c;
pub mod manager;
pub mod metadata;
pub mod native;
pub mod notifications;
pub mod template;

use crate::config::settings::ProxyMode;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub id: String,
    pub url: String,
    pub output_dir: String,
    pub temp_path: String,
    pub final_path: String,
    pub filename: String,
    pub session_cookie: Option<String>,
    pub proxy_mode: ProxyMode,
    pub proxy_url: String,
    pub proxy_username: String,
    pub proxy_password: String,
    pub proxy_bypass_local: bool,
    pub connections: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interruption {
    Pause,
    Cancel,
}

#[derive(Debug)]
pub enum DownloadRunError {
    Interrupted(Interruption),
    Failed(String),
}

impl From<String> for DownloadRunError {
    fn from(value: String) -> Self {
        Self::Failed(value)
    }
}

pub struct DownloadControl {
    request: AtomicU8,
    finished: AtomicBool,
    notify: tokio::sync::Notify,
}

impl DownloadControl {
    pub fn new() -> Self {
        Self {
            request: AtomicU8::new(0),
            finished: AtomicBool::new(false),
            notify: tokio::sync::Notify::new(),
        }
    }

    pub fn pause(&self) {
        self.request.store(1, Ordering::Release);
    }

    pub fn cancel(&self) {
        self.request.store(2, Ordering::Release);
    }

    pub fn interruption(&self) -> Option<Interruption> {
        match self.request.load(Ordering::Acquire) {
            1 => Some(Interruption::Pause),
            2 => Some(Interruption::Cancel),
            _ => None,
        }
    }

    pub fn mark_finished(&self) {
        self.finished.store(true, Ordering::Release);
        self.notify.notify_waiters();
    }

    pub async fn wait_finished(&self) {
        if self.finished.load(Ordering::Acquire) {
            return;
        }
        self.notify.notified().await;
    }
}

impl Default for DownloadControl {
    fn default() -> Self {
        Self::new()
    }
}

pub fn derive_download_referer(url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    if host.is_empty() || crate::cloud::supports_url(url) {
        return None;
    }

    let parts: Vec<&str> = host.split('.').collect();
    let base_host = if parts.len() > 2 {
        parts[parts.len() - 2..].join(".")
    } else {
        host.to_string()
    };
    Some(format!("{}://{}/", parsed.scheme(), base_host))
}

pub fn derive_download_cookie(url: &str, session_cookie: &str) -> Option<String> {
    if session_cookie.trim().is_empty() {
        return None;
    }
    // Never leak a provider session cookie to an external cloud host.
    let _ = derive_download_referer(url)?;
    if session_cookie.contains('=') {
        Some(session_cookie.to_string())
    } else {
        Some(format!("session={}", session_cookie.trim()))
    }
}

pub const PAWSTASH_USER_AGENT: &str =
    concat!("Github:Pawstash/Pawstash;v=", env!("CARGO_PKG_VERSION"));

pub fn standard_browser_headers() -> reqwest::header::HeaderMap {
    use reqwest::header::*;
    let mut map = HeaderMap::new();
    map.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
        ),
    );
    map.insert(ACCEPT, HeaderValue::from_static("*/*"));
    map.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
    map.insert(
        HeaderName::from_static("sec-ch-ua"),
        HeaderValue::from_static(
            "\"Not(A:Brand\";v=\"99\", \"Google Chrome\";v=\"133\", \"Chromium\";v=\"133\"",
        ),
    );
    map.insert(
        HeaderName::from_static("sec-ch-ua-platform"),
        HeaderValue::from_static("\"Windows\""),
    );
    map.insert(
        HeaderName::from_static("sec-ch-ua-mobile"),
        HeaderValue::from_static("?0"),
    );
    map.insert(
        HeaderName::from_static("sec-fetch-dest"),
        HeaderValue::from_static("empty"),
    );
    map.insert(
        HeaderName::from_static("sec-fetch-mode"),
        HeaderValue::from_static("cors"),
    );
    map.insert(
        HeaderName::from_static("sec-fetch-site"),
        HeaderValue::from_static("same-site"),
    );
    map
}

pub fn standard_browser_header_args() -> Vec<String> {
    vec![
        "--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36".into(),
        "--header=Accept: */*".into(),
        "--header=Accept-Language: en-US,en;q=0.9".into(),
        "--header=sec-ch-ua: \"Not(A:Brand\";v=\"99\", \"Google Chrome\";v=\"133\", \"Chromium\";v=\"133\"".into(),
        "--header=sec-ch-ua-platform: \"Windows\"".into(),
        "--header=sec-ch-ua-mobile: ?0".into(),
        "--header=Sec-Fetch-Dest: empty".into(),
        "--header=Sec-Fetch-Mode: cors".into(),
        "--header=Sec-Fetch-Site: same-site".into(),
    ]
}

pub fn derive_download_headers(url: &str) -> reqwest::header::HeaderMap {
    use reqwest::header::*;
    if crate::api::providers::uses_app_user_agent(url) {
        let mut map = HeaderMap::new();
        map.insert(USER_AGENT, HeaderValue::from_static(PAWSTASH_USER_AGENT));
        map.insert(ACCEPT, HeaderValue::from_static("*/*"));
        map.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
        map
    } else {
        standard_browser_headers()
    }
}

pub fn derive_download_header_args(url: &str) -> Vec<String> {
    if crate::api::providers::uses_app_user_agent(url) {
        vec![
            format!("--user-agent={PAWSTASH_USER_AGENT}"),
            "--header=Accept: */*".into(),
            "--header=Accept-Language: en-US,en;q=0.9".into(),
        ]
    } else {
        standard_browser_header_args()
    }
}

pub use crate::cloud::normalize_cloud_direct_url as normalize_download_url;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_download_referer_domains() {
        for config in crate::api::providers::ProviderManager::default_configs() {
            let expected = format!("{}/", config.api_url.trim_end_matches('/'));
            for endpoint in [Some(config.api_url), config.file_url, config.image_url]
                .into_iter()
                .flatten()
            {
                assert_eq!(derive_download_referer(&endpoint), Some(expected.clone()));
            }
        }

        assert_eq!(
            derive_download_referer("https://cdn.custom-provider.org/data/11/22/archive.zip"),
            Some("https://custom-provider.org/".into())
        );
        assert_eq!(
            derive_download_referer(
                "https://img.custom-provider.org/thumbnail/data/11/22/thumb.jpg"
            ),
            Some("https://custom-provider.org/".into())
        );

        assert_eq!(
            derive_download_referer(crate::cloud::dropbox::example_url()),
            None
        );
    }

    #[test]
    fn test_derive_download_cookie() {
        let provider_url = crate::api::providers::ProviderManager::default_configs()
            .into_iter()
            .next()
            .and_then(|config| config.file_url)
            .expect("default provider has a file endpoint");
        assert_eq!(
            derive_download_cookie(&provider_url, "abc_session"),
            Some("session=abc_session".into())
        );
        assert_eq!(
            derive_download_cookie(&provider_url, "session=abc_session"),
            Some("session=abc_session".into())
        );

        assert_eq!(
            derive_download_cookie(crate::cloud::dropbox::example_url(), "abc_session"),
            None
        );
    }

    #[test]
    fn test_standard_browser_headers() {
        let headers = standard_browser_headers();
        assert_eq!(
            headers.get("user-agent").and_then(|v| v.to_str().ok()),
            Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36")
        );
        assert_eq!(
            headers
                .get("sec-ch-ua-platform")
                .and_then(|v| v.to_str().ok()),
            Some("\"Windows\"")
        );
        assert_eq!(
            headers
                .get("sec-ch-ua-mobile")
                .and_then(|v| v.to_str().ok()),
            Some("?0")
        );
        assert!(headers.contains_key("sec-ch-ua"));
        assert!(headers.contains_key("sec-fetch-dest"));

        let args = standard_browser_header_args();
        assert!(args.iter().any(|a| a.starts_with("--user-agent=")));
        assert!(args.iter().any(|a| a.starts_with("--header=sec-ch-ua:")));
        assert!(args
            .iter()
            .any(|a| a == "--header=sec-ch-ua-platform: \"Windows\""));
    }

    #[test]
    fn test_provider_user_agent_headers() {
        let url = crate::api::providers::PawchiveProvider::default_config()
            .file_url
            .expect("default provider has a file endpoint");
        let headers = derive_download_headers(&url);
        assert_eq!(
            headers.get("user-agent").and_then(|v| v.to_str().ok()),
            Some(PAWSTASH_USER_AGENT)
        );

        let args = derive_download_header_args(&url);
        assert!(args
            .iter()
            .any(|a| a == &format!("--user-agent={PAWSTASH_USER_AGENT}")));
    }
}
