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

    let mut dl_query: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| k != "dl" && k != "raw")
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    dl_query.push(("dl".to_string(), "1".to_string()));

    let mut stream_query: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| k != "dl" && k != "raw")
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    stream_query.push(("raw".to_string(), "1".to_string()));

    url.query_pairs_mut()
        .clear()
        .extend_pairs(dl_query.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    let download_url = url.to_string();

    url.query_pairs_mut()
        .clear()
        .extend_pairs(stream_query.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    let stream_url = url.to_string();

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
    let mime = if !is_folder {
        mime_guess::from_path(&filename)
            .first_raw()
            .map(str::to_string)
    } else {
        None
    };

    let mut size = None;
    if !is_folder {
        if let Ok(resp) = client
            .get(&download_url)
            .header(reqwest::header::RANGE, "bytes=0-0")
            .send()
            .await
        {
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
        if size.is_none() {
            if let Ok(head_resp) = client.head(&download_url).send().await {
                if let Some(cl) = head_resp.content_length() {
                    if cl > 0 {
                        size = Some(cl);
                    }
                }
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
        download_url: Some(download_url),
        stream_url: Some(stream_url.clone()),
        thumbnail_url: if is_folder { None } else { Some(stream_url) },
        children: None,
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
            }],
        };

        crate::cloud::canonicalize_cloud_result(&mut result);

        let stream_url = result.nodes[0].stream_url.as_deref().unwrap();
        assert!(stream_url.starts_with("/cloud_stream/proxy?url="));
        assert!(stream_url.contains("name=video.mp4"));
        assert!(!stream_url.contains("dl%3D0"));
        assert!(stream_url.contains("raw%3D1"));
    }
}
