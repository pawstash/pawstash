use super::models::{CloudFolderResult, CloudNode};
use reqwest::Client;

pub async fn resolve_googledrive(
    client: &Client,
    url_str: &str,
) -> Result<CloudFolderResult, String> {
    let url = reqwest::Url::parse(url_str).map_err(|e| format!("Invalid Google Drive URL: {e}"))?;

    let path_segments: Vec<&str> = url.path_segments().map(|s| s.collect()).unwrap_or_default();

    let folder_id = if let Some(pos) = path_segments.iter().position(|&s| s == "folders") {
        path_segments.get(pos + 1).map(|s| s.to_string())
    } else {
        None
    };

    let file_id = if let Some(pos) = path_segments.iter().position(|&s| s == "d") {
        path_segments.get(pos + 1).map(|s| s.to_string())
    } else {
        url.query_pairs()
            .find(|(k, _)| k == "id")
            .map(|(_, v)| v.into_owned())
    };

    let is_folder = folder_id.is_some();
    let target_id = folder_id
        .or(file_id)
        .unwrap_or_else(|| "unknown".to_string());

    if is_folder {
        let folder_title = format!("Google Drive Folder ({target_id})");
        let direct_url = format!("https://drive.google.com/drive/folders/{target_id}");
        let node = CloudNode {
            id: target_id.clone(),
            parent_id: None,
            name: folder_title.clone(),
            size: None,
            is_folder: true,
            mime_type: Some("application/vnd.google-apps.folder".into()),
            download_url: Some(direct_url.clone()),
            stream_url: Some(direct_url.clone()),
            thumbnail_url: None,
            children: None,
        };

        return Ok(CloudFolderResult {
            provider: "googledrive".into(),
            url: url_str.to_string(),
            title: folder_title,
            total_files: 1,
            total_size: 0,
            is_single_file: false,
            nodes: vec![node],
        });
    }

    let direct_url = format!(
        "https://drive.usercontent.google.com/download?id={target_id}&export=download&confirm=t"
    );
    let fallback_download_url =
        format!("https://drive.google.com/uc?export=download&id={target_id}&confirm=t");

    let mut filename = format!("Google_Drive_File_{target_id}");
    let mut size = None;

    let view_url = format!("https://drive.google.com/file/d/{target_id}/view");
    if let Ok(resp) = client
        .get(&view_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .send()
        .await
    {
        if let Ok(html) = resp.text().await {
            if let Some(og_title) = extract_meta_content(&html, "og:title") {
                let trimmed = og_title.trim();
                if !trimmed.is_empty() && trimmed != "Google Drive" {
                    filename = trimmed.to_string();
                }
            } else if let Some(page_title) = extract_title(&html) {
                let cleaned = page_title.trim().trim_end_matches("- Google Drive").trim();
                if !cleaned.is_empty() && cleaned != "Google Drive" {
                    filename = cleaned.to_string();
                }
            }

            if let Some(parsed_size) = extract_size_bytes(&html) {
                size = Some(parsed_size);
            }
        }
    }

    let mime = mime_guess::from_path(&filename)
        .first_raw()
        .map(str::to_string);

    if size.is_none() {
        if let Ok(head_resp) = client.head(&direct_url).send().await {
            let is_html = head_resp
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .map(|ct| ct.starts_with("text/html"))
                .unwrap_or(false);
            if !is_html {
                if let Some(cl) = head_resp.content_length() {
                    if cl > 0 {
                        size = Some(cl);
                    }
                }
            }
        }
    }

    let is_img = mime
        .as_deref()
        .map(|m| m.starts_with("image/"))
        .unwrap_or(false);
    let thumbnail_url = if is_img {
        Some(format!(
            "https://drive.google.com/thumbnail?id={target_id}&sz=w800"
        ))
    } else {
        None
    };

    let node = CloudNode {
        id: target_id.clone(),
        parent_id: None,
        name: filename.clone(),
        size,
        is_folder: false,
        mime_type: mime,
        download_url: Some(fallback_download_url),
        stream_url: Some(direct_url),
        thumbnail_url,
        children: None,
    };

    let total_size = size.unwrap_or(0);

    Ok(CloudFolderResult {
        provider: "googledrive".into(),
        url: url_str.to_string(),
        title: filename,
        total_files: 1,
        total_size,
        is_single_file: true,
        nodes: vec![node],
    })
}

fn extract_size_bytes(html: &str) -> Option<u64> {
    let patterns = [
        "\"sizeBytes\":\"",
        "\"sizeBytes\":",
        "\"fileSize\":\"",
        "\"fileSize\":",
    ];
    for pat in patterns {
        if let Some(pos) = html.find(pat) {
            let start = pos + pat.len();
            let mut num_str = String::new();
            for ch in html[start..].chars() {
                if ch.is_ascii_digit() {
                    num_str.push(ch);
                } else if !num_str.is_empty() {
                    break;
                } else if ch == '"' || ch.is_whitespace() {
                    continue;
                } else {
                    break;
                }
            }
            if let Ok(s) = num_str.parse::<u64>() {
                if s > 0 {
                    return Some(s);
                }
            }
        }
    }
    None
}

fn extract_meta_content(html: &str, property: &str) -> Option<String> {
    let prop_pattern = format!("property=\"{property}\"");
    let prop_pattern_alt = format!("name=\"{property}\"");
    let pos = html
        .find(&prop_pattern)
        .or_else(|| html.find(&prop_pattern_alt))?;

    let tag_start = html[..pos].rfind('<')?;
    let tag_end = html[pos..].find('>')? + pos;
    let tag = &html[tag_start..=tag_end];

    let content_marker = "content=\"";
    let c_pos = tag.find(content_marker)? + content_marker.len();
    let c_end = tag[c_pos..].find('"')? + c_pos;
    Some(html_escape_decode(&tag[c_pos..c_end]))
}

fn extract_title(html: &str) -> Option<String> {
    let start_tag = "<title>";
    let end_tag = "</title>";
    let start = html.find(start_tag)? + start_tag.len();
    let end = html[start..].find(end_tag)? + start;
    Some(html_escape_decode(&html[start..end]))
}

fn html_escape_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}
