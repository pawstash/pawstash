pub mod aria2c;
pub mod manager;
pub mod metadata;
pub mod native;
pub mod notifications;
pub mod template;

use crate::config::settings::ProxyMode;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

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
}

impl DownloadControl {
    pub fn new() -> Self {
        Self {
            request: AtomicU8::new(0),
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
}

impl Default for DownloadControl {
    fn default() -> Self {
        Self::new()
    }
}

pub fn derive_download_referer(url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    if host.is_empty()
        || host.contains("dropbox")
        || host.contains("google")
        || host.contains("mega")
        || host.contains("pixeldrain")
    {
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
    // Only send session cookie if the target URL belongs to a provider origin (not external clouds)
    let _ = derive_download_referer(url)?;
    if session_cookie.contains('=') {
        Some(session_cookie.to_string())
    } else {
        Some(format!("session={}", session_cookie.trim()))
    }
}

pub use crate::cloud::normalize_cloud_direct_url as normalize_download_url;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_download_referer_domains() {
        // Pawchive and subdomains
        assert_eq!(
            derive_download_referer("https://file.pawchive.pw/data/fc/b2/image.jpg"),
            Some("https://pawchive.pw/".into())
        );
        assert_eq!(
            derive_download_referer("https://file1.pawchive.pw/data/fc/b2/video.mp4"),
            Some("https://pawchive.pw/".into())
        );
        assert_eq!(
            derive_download_referer("https://img.pawchive.pw/thumbnail/data/fc/b2/thumb.jpg"),
            Some("https://pawchive.pw/".into())
        );
        assert_eq!(
            derive_download_referer("https://pawchive.pw/patreon/user/123/post/456"),
            Some("https://pawchive.pw/".into())
        );

        // OnlyHaven (cum.st) - must NEVER leak to coomer.su
        assert_eq!(
            derive_download_referer("https://cum.st/data/aa/bb/image.png"),
            Some("https://cum.st/".into())
        );
        assert_eq!(
            derive_download_referer("https://file.cum.st/data/aa/bb/image.png"),
            Some("https://cum.st/".into())
        );
        assert_eq!(
            derive_download_referer("https://img.cum.st/thumbnail/data/aa/bb/thumb.png"),
            Some("https://cum.st/".into())
        );

        // Custom 3-part subdomains
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

        // Cloud hosts must not send referers (to avoid hotlink/cross-origin blocks)
        assert_eq!(
            derive_download_referer("https://mega.nz/file/abc#key"),
            None
        );
        assert_eq!(
            derive_download_referer("https://www.dropbox.com/s/xyz/file.zip?dl=1"),
            None
        );
        assert_eq!(
            derive_download_referer("https://pixeldrain.com/api/file/12345"),
            None
        );
    }

    #[test]
    fn test_derive_download_cookie() {
        // Provider URL gets cookie
        assert_eq!(
            derive_download_cookie("https://file.pawchive.pw/data/123", "abc_session"),
            Some("session=abc_session".into())
        );
        assert_eq!(
            derive_download_cookie("https://file.pawchive.pw/data/123", "session=abc_session"),
            Some("session=abc_session".into())
        );

        // Cloud URLs MUST NEVER receive session cookie
        assert_eq!(
            derive_download_cookie("https://www.dropbox.com/s/xyz/file.zip?dl=1", "abc_session"),
            None
        );
        assert_eq!(
            derive_download_cookie("https://mega.nz/file/abc#key", "abc_session"),
            None
        );
        assert_eq!(
            derive_download_cookie("https://pixeldrain.com/api/file/12345", "abc_session"),
            None
        );
    }
}
