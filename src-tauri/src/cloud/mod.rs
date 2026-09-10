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

        let mut result = if iframely::supports_url(trimmed) {
            iframely::resolve_iframely(&self.client, trimmed).await?
        } else if mega::supports_url(trimmed) {
            mega::resolve_mega(&self.client, trimmed).await?
        } else if pixeldrain::supports_url(trimmed) {
            pixeldrain::resolve_pixeldrain(&self.client, trimmed).await?
        } else if dropbox::supports_url(trimmed) {
            dropbox::resolve_dropbox(&self.client, trimmed).await?
        } else if googledrive::supports_url(trimmed) {
            googledrive::resolve_googledrive(&self.client, trimmed).await?
        } else {
            return Err(format!(
                "Unsupported cloud link provider for URL: {trimmed}"
            ));
        };

        canonicalize_cloud_result(&mut result);
        Ok(result)
    }
}

pub fn canonicalize_cloud_result(result: &mut CloudFolderResult) {
    for node in &mut result.nodes {
        if node.is_folder {
            continue;
        }
        if let Some(stream_url) = node.stream_url.as_deref() {
            if should_proxy_cloud_stream(stream_url) {
                let target = normalize_cloud_direct_url(stream_url);
                node.stream_url = Some(format!(
                    "/cloud_stream/proxy?url={}&name={}",
                    urlencoding::encode(&target),
                    urlencoding::encode(&node.name)
                ));
            }
        }
    }
}

fn should_proxy_cloud_stream(url: &str) -> bool {
    dropbox::should_proxy_stream(url)
        || pixeldrain::should_proxy_stream(url)
        || googledrive::should_proxy_stream(url)
}

pub fn normalize_cloud_direct_url(url: &str) -> String {
    dropbox::normalize_direct_url(url)
        .or_else(|| pixeldrain::normalize_direct_url(url))
        .unwrap_or_else(|| url.trim().to_string())
}

pub fn supports_url(url: &str) -> bool {
    iframely::supports_url(url)
        || mega::supports_url(url)
        || pixeldrain::supports_url(url)
        || dropbox::supports_url(url)
        || googledrive::supports_url(url)
}

pub fn extract_supported_urls(raw: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut cursor = 0;
    while let Some(relative_start) = [
        raw[cursor..].find("https://"),
        raw[cursor..].find("http://"),
    ]
    .into_iter()
    .flatten()
    .min()
    {
        let start = cursor + relative_start;
        let tail = &raw[start..];
        let end = tail
            .find(|character: char| {
                character.is_whitespace() || matches!(character, '<' | '>' | '"' | '\'' | ')')
            })
            .unwrap_or(tail.len());
        let candidate = tail[..end].trim_end_matches([',', '.', ';', ']', '}']);
        if supports_url(candidate) && !urls.iter().any(|existing| existing == candidate) {
            urls.push(candidate.to_string());
        }
        cursor = start + end.max(1);
    }
    urls
}

pub(crate) fn url_host_matches(url: &str, domains: &[&str]) -> bool {
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return false;
    };
    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
    domains
        .iter()
        .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_supported_urls_without_frontend_classification() {
        let supported = dropbox::example_url();
        let raw = format!("before {supported} after https://example.test/file");
        assert_eq!(extract_supported_urls(&raw), vec![supported.to_string()]);
    }
}
