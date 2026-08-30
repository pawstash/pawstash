pub mod dropbox;
pub mod googledrive;
pub mod iframely;
pub mod mega;
pub mod models;
pub mod pixeldrain;

pub use models::{CloudFolderResult, CloudNode};

use crate::config::{AppSettings, ProxyMode};
use reqwest::Client;
use std::time::Duration;

pub struct CloudResolver {
    client: Client,
}

impl Default for CloudResolver {
    fn default() -> Self {
        Self::new(None)
    }
}

impl CloudResolver {
    pub fn new(settings: Option<&AppSettings>) -> Self {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36")
            .redirect(reqwest::redirect::Policy::limited(10))
            .gzip(true);

        if let Some(s) = settings {
            match s.proxy_mode {
                ProxyMode::None => builder = builder.no_proxy(),
                ProxyMode::System => {}
                ProxyMode::Custom if !s.proxy_url.trim().is_empty() => {
                    if let Ok(mut proxy) = reqwest::Proxy::all(s.proxy_url.trim()) {
                        if !s.proxy_username.is_empty() {
                            proxy = proxy.basic_auth(&s.proxy_username, &s.proxy_password);
                        }
                        builder = builder.proxy(proxy);
                    }
                }
                _ => {}
            }
        }

        let client = builder.build().unwrap_or_else(|_| Client::new());
        Self { client }
    }

    pub async fn resolve(&self, url: &str) -> Result<CloudFolderResult, String> {
        let trimmed = url.trim();
        let lower = trimmed.to_lowercase();

        if lower.contains("iframely.net") || lower.contains("iframe.ly") {
            return iframely::resolve_iframely(&self.client, trimmed).await;
        }

        if lower.contains("mega.nz") || lower.contains("mega.co.nz") {
            return mega::resolve_mega(&self.client, trimmed).await;
        }

        if lower.contains("pixeldrain.com") {
            return pixeldrain::resolve_pixeldrain(&self.client, trimmed).await;
        }

        if lower.contains("dropbox.com") {
            return dropbox::resolve_dropbox(&self.client, trimmed).await;
        }

        if lower.contains("drive.google.com") || lower.contains("docs.google.com") {
            return googledrive::resolve_googledrive(&self.client, trimmed).await;
        }

        Err(format!(
            "Unsupported cloud link provider for URL: {trimmed}"
        ))
    }
}

pub fn normalize_cloud_direct_url(url: &str) -> String {
    let mut target_url = url.trim().to_string();
    if let Ok(mut u) = reqwest::Url::parse(&target_url) {
        let host = u.host_str().unwrap_or("").to_lowercase();
        let path = u.path().to_string();

        // 1. Dropbox direct download parameter
        if host.contains("dropbox.com") {
            let query: Vec<(String, String)> = u
                .query_pairs()
                .filter(|(k, _)| k != "dl" && k != "raw")
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect();
            u.query_pairs_mut()
                .clear()
                .extend_pairs(query.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .append_pair("raw", "1");
            target_url = u.to_string();
        }

        // 2. Pixeldrain file link conversion
        if host.contains("pixeldrain.com") && path.starts_with("/u/") {
            let file_id = path.trim_start_matches("/u/");
            u.set_path(&format!("/api/file/{file_id}"));
            target_url = u.to_string();
        }
    }
    target_url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_cloud_direct_url() {
        assert_eq!(
            normalize_cloud_direct_url("https://www.dropbox.com/s/xyz/video.mp4?dl=0"),
            "https://www.dropbox.com/s/xyz/video.mp4?raw=1"
        );
        assert_eq!(
            normalize_cloud_direct_url("https://pixeldrain.com/u/abc12345"),
            "https://pixeldrain.com/api/file/abc12345"
        );
    }
}
