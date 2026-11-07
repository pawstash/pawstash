pub mod dropbox;
pub mod filen;
pub mod googledrive;
pub mod iframely;
pub mod mega;
pub mod models;
pub mod pixeldrain;

pub use models::{CloudFolderResult, CloudNode};

use crate::config::AppSettings;
use reqwest::Client;
use std::time::Duration;

/// Broader than the `supports_url` lists: Drive streams from `drive.usercontent.google.com`.
pub const NETWORK_HOSTS: &[&str] = &[
    "drive.google.com",
    "docs.google.com",
    "drive.usercontent.google.com",
    "mega.nz",
    "mega.co.nz",
    "dropbox.com",
    "dropboxusercontent.com",
    "pixeldrain.com",
    "filen.io",
    "iframely.net",
    "iframely.com",
    "iframe.ly",
];

pub const DEFAULT_CLOUD_TIMEOUT_SECS: u64 = 30;
pub const DEFAULT_CLOUD_MAX_REDIRECTS: u64 = 10;

pub struct CloudResolver {
    client: Client,
}

pub fn external_links_builder(settings: &AppSettings) -> Result<reqwest::ClientBuilder, String> {
    let proxy_override = settings.cloud_proxy_url.trim();
    let builder = if proxy_override.is_empty() {
        crate::net::builder_with_proxy(settings)?
    } else {
        let proxy = reqwest::Proxy::all(proxy_override)
            .map_err(|e| format!("Invalid External Links proxy URL: {e}"))?;
        crate::net::builder().proxy(proxy)
    };
    let user_agent = match settings.cloud_user_agent.trim() {
        "" => crate::net::browser_user_agent(),
        custom => custom.to_string(),
    };
    Ok(builder.user_agent(user_agent))
}

impl CloudResolver {
    pub fn new(settings: &AppSettings) -> Result<Self, String> {
        let timeout = match settings.cloud_timeout_secs {
            0 => DEFAULT_CLOUD_TIMEOUT_SECS,
            secs => secs,
        };
        let redirects = match settings.cloud_max_redirects {
            0 => DEFAULT_CLOUD_MAX_REDIRECTS,
            limit => limit,
        };

        let client = external_links_builder(settings)?
            .timeout(Duration::from_secs(timeout))
            .redirect(reqwest::redirect::Policy::limited(redirects as usize))
            .gzip(true)
            .build()
            .map_err(|error| format!("Failed to build external-links client: {error}"))?;
        Ok(Self { client })
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
        } else if filen::supports_url(trimmed) {
            filen::resolve_filen(&self.client, trimmed).await?
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
        let Some(stream_url) = node.stream_url.as_deref() else {
            continue;
        };
        let was_proxy = stream_url.starts_with("/cloud_stream/proxy?");
        let target = proxy_url_target(stream_url).unwrap_or_else(|| stream_url.to_string());
        if was_proxy || should_proxy_cloud_stream(&target) {
            let normalized = normalize_cloud_direct_url(&target);
            node.stream_url = Some(format!(
                "/cloud_stream/proxy?url={}&name={}",
                urlencoding::encode(&normalized),
                urlencoding::encode(&node.name)
            ));
        }
    }
}

fn proxy_url_target(stream_url: &str) -> Option<String> {
    let query = stream_url.strip_prefix("/cloud_stream/proxy?")?;
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        if key != "url" {
            return None;
        }
        urlencoding::decode(value).ok().map(|v| v.into_owned())
    })
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

pub async fn follow_cloud_stream_html_warning(
    client: &Client,
    target_url: &reqwest::Url,
    upstream_headers: &reqwest::header::HeaderMap,
    html_str: &str,
    is_head: bool,
    range_header: Option<&reqwest::header::HeaderValue>,
) -> Option<reqwest::Response> {
    if googledrive::should_proxy_stream(target_url.as_str()) {
        googledrive::follow_stream_confirmation(
            client,
            target_url,
            upstream_headers,
            html_str,
            is_head,
            range_header,
        )
        .await
    } else {
        None
    }
}

pub fn supports_url(url: &str) -> bool {
    iframely::supports_url(url)
        || mega::supports_url(url)
        || pixeldrain::supports_url(url)
        || dropbox::supports_url(url)
        || googledrive::supports_url(url)
        || filen::supports_url(url)
}

pub fn extract_supported_urls(raw: &str) -> Vec<String> {
    if raw.is_empty() {
        return Vec::new();
    }
    let normalized = raw.replace(r"\/", "/").replace("&amp;", "&");
    let mut urls = Vec::new();
    let mut cursor = 0;
    while let Some(relative_start) = [
        normalized[cursor..].find("https://"),
        normalized[cursor..].find("http://"),
    ]
    .into_iter()
    .flatten()
    .min()
    {
        let start = cursor + relative_start;
        let tail = &normalized[start..];
        let end = tail
            .find(|character: char| {
                character.is_whitespace()
                    || matches!(character, '<' | '>' | '"' | '\'' | ')' | '\\')
            })
            .unwrap_or(tail.len());
        let mut candidate = tail[..end]
            .trim_end_matches([',', '.', ';', ']', '}', ')', '>', '"', '\'', '\\'])
            .to_string();
        if (candidate.contains("mega.nz/folder/") || candidate.contains("mega.nz/file/"))
            && !candidate.contains('#')
        {
            if let Some(key) =
                extract_adjacent_mega_key(&tail[end..], candidate.contains("mega.nz/folder/"))
            {
                candidate = format!("{candidate}#{key}");
            }
        }
        if supports_url(&candidate) && !urls.iter().any(|existing| existing == &candidate) {
            urls.push(candidate);
        }
        cursor = start + end.max(1);
    }
    urls
}

fn extract_adjacent_mega_key(text: &str, is_folder: bool) -> Option<String> {
    let mut window_len = text.len().min(250);
    while !text.is_char_boundary(window_len) {
        window_len -= 1;
    }
    let window = text[..window_len].replace('：', ":");
    let lower = window.to_ascii_lowercase();
    for prefix in [
        "code:",
        "code :",
        "key:",
        "key :",
        "pass:",
        "pass :",
        "pw:",
        "pw :",
        "password:",
        "password :",
    ] {
        if let Some(pos) = lower.find(prefix) {
            let after = &window[pos + prefix.len()..];
            let trimmed = after.trim_start();
            let key_len = trimmed
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
                .unwrap_or(trimmed.len());
            let key = &trimmed[..key_len];
            if (20..=50).contains(&key.len()) {
                return Some(key.to_string());
            }
        }
    }

    let expected_len = if is_folder { 22 } else { 43 };
    window
        .split(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '"' | '\'' | '#'))
        .filter(|token| !token.is_empty())
        .take(4)
        .find(|token| {
            token.len() == expected_len
                && token
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
        .map(str::to_string)
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
    fn extracts_supported_urls_from_post_text() {
        let filen = "https://app.filen.io/#/f/ed204269-428e-42e0-af64-220471a7a124%236576743441697556695335625a584f446c7967716842427567575f6334453839";
        let filen_json = format!(
            r#"{{"content":"<a href=\"{}\">4K</a>"}}"#,
            filen.replace('/', r"\/")
        );
        let dropbox_text = format!(
            "before {} after https://example.test/file",
            dropbox::example_url()
        );
        for (raw, expected) in [
            (dropbox_text.as_str(), dropbox::example_url()),
            (
                r#"{"url":"https:\/\/mega.nz\/folder\/xyz#abc&amp;node=123\"}"#,
                "https://mega.nz/folder/xyz#abc&node=123",
            ),
            (
                "https://mega.nz/folder/Jj1liCiQ   Key：TeHAplSadyE1w--Mt861ig    リンク",
                "https://mega.nz/folder/Jj1liCiQ#TeHAplSadyE1w--Mt861ig",
            ),
            (
                "<a>https://mega.nz/file/jPwAmYqZ</a>     V4qd52dIsjW4Qi3zfkfESL0TrwzLZIt13C27R6c30R4",
                "https://mega.nz/file/jPwAmYqZ#V4qd52dIsjW4Qi3zfkfESL0TrwzLZIt13C27R6c30R4",
            ),
            (
                "https://mega.nz/folder/Jj1liCiQ and see you next month",
                "https://mega.nz/folder/Jj1liCiQ",
            ),
            (
                r#"<p><a href="https://mega.nz/folder/0jUU2SxQ">Originalfiles</a> </p><p>code: QWMvysdMd30JRHNtF7j-Ew</p>"#,
                "https://mega.nz/folder/0jUU2SxQ#QWMvysdMd30JRHNtF7j-Ew",
            ),
            (filen_json.as_str(), filen),
        ] {
            assert_eq!(extract_supported_urls(raw), vec![expected.to_string()], "{raw}");
        }
    }

    #[test]
    fn adjacent_key_window_respects_char_boundaries() {
        let text = format!("{}Key：TeHAplSadyE1w--Mt861ig", " ".repeat(245));
        assert_eq!(extract_adjacent_mega_key(&text, true), None);
        let text = format!("{}リンクは月初めに更新予定です", " ".repeat(248));
        assert_eq!(extract_adjacent_mega_key(&text, true), None);
    }
}
