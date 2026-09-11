use super::models::{CloudFolderResult, CloudNode};
use reqwest::Client;

pub fn supports_url(url: &str) -> bool {
    super::url_host_matches(url, &["drive.google.com", "docs.google.com"])
}

pub fn should_proxy_stream(url: &str) -> bool {
    super::url_host_matches(
        url,
        &[
            "drive.google.com",
            "drive.usercontent.google.com",
            "docs.google.com",
        ],
    )
}

pub async fn resolve_googledrive(
    client: &Client,
    url_str: &str,
) -> Result<CloudFolderResult, String> {
    let (target_id, is_folder) = parse_googledrive_url(url_str)
        .ok_or_else(|| format!("Invalid Google Drive URL: {url_str}"))?;

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

    if size.is_none() {
        let direct_req = client.get(&direct_url).header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        );
        if let Ok(resp) = direct_req.send().await {
            let ct = resp
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            if !ct.starts_with("text/html") {
                if let Some(cd) = resp
                    .headers()
                    .get(reqwest::header::CONTENT_DISPOSITION)
                    .and_then(|v| v.to_str().ok())
                {
                    if filename.starts_with("Google_Drive_File_") || filename.is_empty() {
                        if let Some(extracted) = extract_filename_from_cd(cd) {
                            if !extracted.is_empty() {
                                filename = extracted;
                            }
                        }
                    }
                }
                if let Some(cl) = resp.content_length() {
                    if cl > 0 {
                        size = Some(cl);
                    }
                }
            } else {
                let upstream_headers = resp.headers().clone();
                if let Ok(html_str) = resp.text().await {
                    if let Some((extracted_name, parsed_sz)) =
                        extract_warning_name_and_size(&html_str)
                    {
                        if (filename.starts_with("Google_Drive_File_") || filename.is_empty())
                            && !extracted_name.is_empty()
                        {
                            filename = extracted_name;
                        }
                        if size.is_none() && parsed_sz > 0 {
                            size = Some(parsed_sz);
                        }
                    }

                    if let Some(uuid) = extract_gdrive_uuid(&html_str) {
                        let confirmed_url = format!(
                            "https://drive.usercontent.google.com/download?id={target_id}&export=download&confirm=t&uuid={uuid}"
                        );
                        let mut head_req = client.head(&confirmed_url).header(
                            "User-Agent",
                            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
                        );
                        if let Some(cookie_hdr) = upstream_headers.get(reqwest::header::SET_COOKIE)
                        {
                            if let Ok(cookie_str) = cookie_hdr.to_str() {
                                if let Some(first_cookie) = cookie_str.split(';').next() {
                                    if let Ok(c_val) =
                                        reqwest::header::HeaderValue::from_str(first_cookie)
                                    {
                                        head_req = head_req.header(reqwest::header::COOKIE, c_val);
                                    }
                                }
                            }
                        }
                        if let Ok(confirmed_head) = head_req.send().await {
                            if let Some(cl) = confirmed_head.content_length() {
                                if cl > 0 {
                                    size = Some(cl);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mime = mime_guess::from_path(&filename)
        .first_raw()
        .map(str::to_string);

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

fn parse_human_size(s: &str) -> Option<u64> {
    let s = s.trim();
    let num_part: String = s
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let unit_part: String = s
        .chars()
        .skip_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let num: f64 = num_part.parse().ok()?;
    let unit = unit_part.trim().to_ascii_uppercase();
    let multiplier = match unit.as_str() {
        "B" | "BYTES" | "" => 1.0,
        "K" | "KB" => 1024.0,
        "M" | "MB" => 1024.0 * 1024.0,
        "G" | "GB" => 1024.0 * 1024.0 * 1024.0,
        "T" | "TB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => 1.0,
    };
    Some((num * multiplier) as u64)
}

fn extract_warning_name_and_size(html: &str) -> Option<(String, u64)> {
    let marker = "class=\"uc-name-size\"";
    let pos = html.find(marker)?;
    let after = &html[pos + marker.len()..];
    let end_pos = after.find("</span>")?;
    let snippet = &after[..end_pos];

    let a_tag_start = snippet.find("<a")?;
    let a_content_start = snippet[a_tag_start..].find('>')? + a_tag_start + 1;
    let a_end = snippet[a_content_start..].find("</a>")? + a_content_start;
    let name = html_escape_decode(snippet[a_content_start..a_end].trim());

    let p_start = snippet[a_end..].find('(')? + a_end + 1;
    let p_end = snippet[p_start..].find(')')? + p_start;
    let size_str = &snippet[p_start..p_end];
    let size = parse_human_size(size_str).unwrap_or(0);

    Some((name, size))
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

pub fn parse_googledrive_url(url_str: &str) -> Option<(String, bool)> {
    let url = reqwest::Url::parse(url_str).ok()?;
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
    let target_id = folder_id.or(file_id)?;
    Some((target_id, is_folder))
}

pub fn extract_gdrive_uuid(html: &str) -> Option<String> {
    if let Some(pos) = html.find("name=\"uuid\"") {
        let after = &html[pos..];
        if let Some(val_pos) = after.find("value=\"") {
            let val_after = &after[val_pos + 7..];
            if let Some(end) = val_after.find('"') {
                return Some(val_after[..end].to_string());
            }
        }
    }
    if let Some(pos) = html.find("name=\"uuid\"") {
        let before = &html[..pos];
        if let Some(val_pos) = before.rfind("value=\"") {
            let val_after = &before[val_pos + 7..];
            if let Some(end) = val_after.find('"') {
                return Some(val_after[..end].to_string());
            }
        }
    }
    None
}

pub async fn follow_stream_confirmation(
    client: &Client,
    target_url: &reqwest::Url,
    upstream_headers: &reqwest::header::HeaderMap,
    html_str: &str,
    is_head: bool,
    range_header: Option<&reqwest::header::HeaderValue>,
) -> Option<reqwest::Response> {
    let uuid = extract_gdrive_uuid(html_str)?;
    tracing::info!(
        "Detected Google Drive virus scan warning with UUID {uuid}, following confirmation form"
    );

    let mut confirmed_url = target_url.clone();
    let mut pairs: Vec<(String, String)> = confirmed_url
        .query_pairs()
        .filter(|(k, _)| k != "confirm" && k != "uuid")
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    pairs.push(("confirm".into(), "t".into()));
    pairs.push(("uuid".into(), uuid));
    confirmed_url.query_pairs_mut().clear().extend_pairs(pairs);

    let confirmed_target = confirmed_url.as_str();
    let mut req = if is_head {
        client.head(confirmed_target)
    } else {
        client.get(confirmed_target)
    };
    req = req.headers(crate::downloader::derive_download_headers(confirmed_target));

    if let Some(cookie_hdr) = upstream_headers.get(reqwest::header::SET_COOKIE) {
        if let Ok(cookie_str) = cookie_hdr.to_str() {
            if let Some(first_cookie) = cookie_str.split(';').next() {
                if let Ok(c_val) = reqwest::header::HeaderValue::from_str(first_cookie) {
                    req = req.header(reqwest::header::COOKIE, c_val);
                }
            }
        }
    }
    if let Some(range_val) = range_header {
        req = req.header(reqwest::header::RANGE, range_val.clone());
    }

    let resp = req.send().await.ok()?;
    let status = resp.status();
    if status.is_success() || status == reqwest::StatusCode::PARTIAL_CONTENT {
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !content_type.starts_with("text/html") {
            return Some(resp);
        }
    }
    None
}

fn extract_filename_from_cd(cd: &str) -> Option<String> {
    let marker = "filename=";
    let pos = cd.find(marker)?;
    let rest = cd[pos + marker.len()..].trim();
    if let Some(stripped) = rest.strip_prefix('"') {
        let end = stripped.find('"')?;
        Some(stripped[..end].to_string())
    } else {
        let end = rest.find(';').unwrap_or(rest.len());
        Some(rest[..end].trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_filename_from_cd() {
        assert_eq!(
            extract_filename_from_cd("attachment; filename=\"DVA_Stuckplug_Sep_02.mp4\""),
            Some("DVA_Stuckplug_Sep_02.mp4".to_string())
        );
        assert_eq!(
            extract_filename_from_cd("attachment; filename=video.mp4; size=123"),
            Some("video.mp4".to_string())
        );
    }

    #[test]
    fn test_extract_gdrive_uuid() {
        let html = r#"<form id="download-form" action="https://drive.usercontent.google.com/download" method="get"><input type="hidden" name="id" value="1FM1EAkJvc6xJfOURbdcuSdCRzV_vBCbp"><input type="hidden" name="export" value="download"><input type="hidden" name="confirm" value="t"><input type="hidden" name="uuid" value="4290d341-254a-432b-99a4-b57d6a7b1cf7"></form>"#;
        assert_eq!(
            extract_gdrive_uuid(html),
            Some("4290d341-254a-432b-99a4-b57d6a7b1cf7".to_string())
        );

        let html_rev = r#"<input type="hidden" value="abcdef12-3456" name="uuid">"#;
        assert_eq!(
            extract_gdrive_uuid(html_rev),
            Some("abcdef12-3456".to_string())
        );
    }

    #[test]
    fn test_parse_googledrive_urls() {
        // File /file/d/{id}
        assert_eq!(
            parse_googledrive_url("https://drive.google.com/file/d/1a2b3c4d5e/view?usp=sharing"),
            Some(("1a2b3c4d5e".to_string(), false))
        );

        // Folder /drive/folders/{id}
        assert_eq!(
            parse_googledrive_url("https://drive.google.com/drive/folders/folder98765"),
            Some(("folder98765".to_string(), true))
        );

        // Query parameter ?id={id}
        assert_eq!(
            parse_googledrive_url("https://drive.google.com/uc?id=query_id_123&export=download"),
            Some(("query_id_123".to_string(), false))
        );
    }

    #[test]
    fn test_extract_warning_name_and_size() {
        let html = r#"<p class="uc-warning-subcaption"><span class="uc-name-size"><a href="/open?id=1eDj6DZpMmRMu0KG3jz3zGZSdQKh9y4T8">DVA_Stuckplug_Sep_02.mp4</a> (576M)</span> is too large for Google to scan for viruses.</p>"#;
        assert_eq!(
            extract_warning_name_and_size(html),
            Some(("DVA_Stuckplug_Sep_02.mp4".to_string(), 576 * 1024 * 1024))
        );
    }

    #[test]
    fn test_parse_human_size() {
        assert_eq!(parse_human_size("100B"), Some(100));
        assert_eq!(parse_human_size("500KB"), Some(500 * 1024));
        assert_eq!(parse_human_size("576M"), Some(576 * 1024 * 1024));
        assert_eq!(
            parse_human_size("1.5GB"),
            Some((1.5 * 1024.0 * 1024.0 * 1024.0) as u64)
        );
    }
}
