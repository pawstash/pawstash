use super::models::{CloudFolderResult, CloudNode};
use reqwest::Client;

pub async fn resolve_dropbox(client: &Client, url_str: &str) -> Result<CloudFolderResult, String> {
    let mut url = reqwest::Url::parse(url_str).map_err(|e| format!("Invalid Dropbox URL: {e}"))?;

    let mut query_pairs: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| k != "dl" && k != "raw")
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    query_pairs.push(("raw".to_string(), "1".to_string()));

    url.query_pairs_mut()
        .clear()
        .extend_pairs(query_pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    let direct_url = url.to_string();

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
        if let Ok(head_resp) = client.head(&direct_url).send().await {
            if let Some(cl) = head_resp.content_length() {
                if cl > 0 {
                    size = Some(cl);
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
        download_url: Some(direct_url.clone()),
        stream_url: Some(direct_url.clone()),
        thumbnail_url: if is_folder { None } else { Some(direct_url) },
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
