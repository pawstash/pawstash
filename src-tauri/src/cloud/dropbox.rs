use super::models::{CloudFolderResult, CloudNode};
use reqwest::Client;

pub fn supports_url(url: &str) -> bool {
    super::url_host_matches(url, &["dropbox.com"])
}

#[cfg(test)]
pub(crate) fn example_url() -> &'static str {
    "https://www.dropbox.com/s/example/file.zip?dl=1"
}

pub fn should_proxy_stream(url: &str) -> bool {
    super::url_host_matches(url, &["dropbox.com", "dropboxusercontent.com"])
}

pub fn normalize_direct_url(url: &str) -> Option<String> {
    if !supports_url(url) {
        return None;
    }
    let mut parsed = reqwest::Url::parse(url).ok()?;
    let path = parsed.path();
    let is_file = path.starts_with("/s/") || path.starts_with("/scl/fi/");
    if is_file {
        let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
        if host == "dropbox.com" || host == "www.dropbox.com" {
            parsed.set_host(Some("dl.dropboxusercontent.com")).ok()?;
        }
    }
    let query = parsed
        .query_pairs()
        .filter(|(key, _)| key != "dl" && key != "raw")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    parsed
        .query_pairs_mut()
        .clear()
        .extend_pairs(
            query
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        )
        .append_pair("raw", "1");
    Some(parsed.to_string())
}

pub async fn resolve_dropbox(client: &Client, url_str: &str) -> Result<CloudFolderResult, String> {
    let mut url = reqwest::Url::parse(url_str).map_err(|e| format!("Invalid Dropbox URL: {e}"))?;

    let stream_query: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| k != "dl" && k != "raw")
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    url.query_pairs_mut()
        .clear()
        .extend_pairs(stream_query.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    url.query_pairs_mut().append_pair("raw", "1");
    let stream_url = url.to_string();
    let direct = normalize_direct_url(&stream_url).unwrap_or(stream_url);

    let path_segments: Vec<&str> = url.path_segments().map(|s| s.collect()).unwrap_or_default();

    let filename = path_segments
        .last()
        .map(|s| {
            urlencoding::decode(s)
                .unwrap_or_else(|_| (*s).into())
                .into_owned()
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Dropbox File".to_string());

    let is_folder = path_segments.iter().any(|&s| s == "sh" || s == "fo");
    if is_folder {
        return Err("Dropbox folders are not supported; open the link in a browser".to_string());
    }
    let mime = mime_guess::from_path(&filename)
        .first_raw()
        .map(str::to_string);

    let mut size = None;
    if let Ok(resp) = client
        .get(&direct)
        .header(reqwest::header::RANGE, "bytes=0-1")
        .send()
        .await
    {
        let status = resp.status();
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if status == reqwest::StatusCode::FORBIDDEN
            || status == reqwest::StatusCode::NOT_FOUND
            || content_type.starts_with("text/html")
        {
            return Err(
                "Dropbox file is unavailable (deleted, private or access denied)".to_string(),
            );
        }
        if status.is_success() || status == reqwest::StatusCode::PARTIAL_CONTENT {
            if let Some(total) = resp
                .headers()
                .get(reqwest::header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.rsplit_once('/').map(|(_, total)| total))
                .and_then(|t| t.parse::<u64>().ok())
                .filter(|t| *t > 0)
            {
                size = Some(total);
            } else if let Some(cl) = resp.content_length().filter(|len| *len > 1) {
                size = Some(cl);
            }
        }
    }

    let node = CloudNode {
        id: url_str.to_string(),
        parent_id: None,
        name: filename.clone(),
        size,
        is_folder,
        mime_type: mime,
        download_url: Some(direct.clone()),
        stream_url: Some(direct.clone()),
        thumbnail_url: Some(direct),
        children: None,
        is_encrypted: false,
    };

    let total_size = size.unwrap_or(0);

    Ok(CloudFolderResult {
        provider: "dropbox".into(),
        url: url_str.to_string(),
        title: filename,
        total_files: 1,
        total_size,
        is_single_file: !is_folder,
        nodes: vec![node],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_streams_for_the_local_proxy() {
        let mut result = CloudFolderResult {
            provider: "dropbox".into(),
            url: "https://www.dropbox.com/s/xyz/video.mp4?dl=0".into(),
            title: "video.mp4".into(),
            total_files: 1,
            total_size: 10,
            is_single_file: true,
            nodes: vec![CloudNode {
                id: "xyz".into(),
                parent_id: None,
                name: "video.mp4".into(),
                size: Some(10),
                is_folder: false,
                mime_type: Some("video/mp4".into()),
                download_url: Some("https://www.dropbox.com/s/xyz/video.mp4?dl=1".into()),
                stream_url: Some("https://www.dropbox.com/s/xyz/video.mp4?dl=0".into()),
                thumbnail_url: None,
                children: None,
                is_encrypted: false,
            }],
        };

        crate::cloud::canonicalize_cloud_result(&mut result);

        let stream_url = result.nodes[0].stream_url.as_deref().unwrap();
        assert!(stream_url.starts_with("/cloud_stream/proxy?url="));
        assert!(stream_url.contains("name=video.mp4"));
        assert!(!stream_url.contains("dl%3D0"));
        assert!(stream_url.contains("raw%3D1"));
        assert!(stream_url.contains("dl.dropboxusercontent.com"));
    }

    #[test]
    fn refreshes_cached_proxy_targets() {
        let mut result = CloudFolderResult {
            provider: "dropbox".into(),
            url: "https://www.dropbox.com/s/xyz/video.mp4?dl=0".into(),
            title: "video.mp4".into(),
            total_files: 1,
            total_size: 10,
            is_single_file: true,
            nodes: vec![CloudNode {
                id: "xyz".into(),
                parent_id: None,
                name: "video.mp4".into(),
                size: Some(10),
                is_folder: false,
                mime_type: Some("video/mp4".into()),
                download_url: None,
                stream_url: Some(
                    "/cloud_stream/proxy?url=https%3A%2F%2Fwww.dropbox.com%2Fs%2Fxyz%2Fvideo.mp4%3Fraw%3D1&name=video.mp4"
                        .into(),
                ),
                thumbnail_url: None,
                children: None,
                is_encrypted: false,
            }],
        };

        crate::cloud::canonicalize_cloud_result(&mut result);

        let stream_url = result.nodes[0].stream_url.as_deref().unwrap();
        assert!(stream_url.contains("dl.dropboxusercontent.com"));
        assert!(!stream_url.contains("www.dropbox.com"));
        assert!(stream_url.contains("raw%3D1"));
    }

    #[test]
    fn rewrites_page_links_to_the_direct_host() {
        let url = "https://www.dropbox.com/scl/fi/abc123/file.mp4?rlkey=key&st=token&dl=0";
        let normalized = normalize_direct_url(url).unwrap();
        assert!(normalized.starts_with("https://dl.dropboxusercontent.com/scl/fi/abc123/file.mp4"));
        assert!(normalized.contains("rlkey=key"));
        assert!(normalized.contains("st=token"));
        assert!(normalized.contains("raw=1"));
        assert!(!normalized.contains("dl=0"));
    }
}
