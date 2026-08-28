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

        // Kemono
        assert_eq!(
            derive_download_referer("https://c1.kemono.su/data/11/22/archive.zip"),
            Some("https://kemono.su/".into())
        );
        assert_eq!(
            derive_download_referer("https://img.kemono.su/thumbnail/data/11/22/thumb.jpg"),
            Some("https://kemono.su/".into())
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
}
