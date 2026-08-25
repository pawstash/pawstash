pub mod dropbox;
pub mod googledrive;
pub mod iframely;
pub mod mega;
pub mod models;
pub mod pixeldrain;

pub use models::{CloudFolderResult, CloudNode};

use reqwest::Client;
use std::time::Duration;

pub struct CloudResolver {
    client: Client,
}

impl Default for CloudResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudResolver {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .gzip(true)
            .build()
            .unwrap_or_else(|_| Client::new());
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
