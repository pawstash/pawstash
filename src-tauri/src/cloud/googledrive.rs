use super::models::{CloudFolderResult, CloudNode};
use futures_util::stream::{self, StreamExt};
use reqwest::Client;
use scraper::{Html, Selector};

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

const MAX_FOLDERS: usize = 200;
const MAX_FILES: usize = 300;
const LISTING_CONCURRENCY: usize = 4;
const FILE_CONCURRENCY: usize = 6;

pub async fn resolve_googledrive(
    client: &Client,
    url_str: &str,
) -> Result<CloudFolderResult, String> {
    let (target_id, is_folder) = parse_googledrive_url(url_str)
        .ok_or_else(|| format!("Invalid Google Drive URL: {url_str}"))?;

    if is_folder {
        return resolve_folder(client, url_str, &target_id).await;
    }

    let node = resolve_file(client, &target_id, None).await;
    Ok(CloudFolderResult {
        provider: "googledrive".into(),
        url: url_str.to_string(),
        title: node.name.clone(),
        total_files: 1,
        total_size: node.size.unwrap_or(0),
        is_single_file: true,
        nodes: vec![node],
    })
}

struct FolderEntry {
    id: String,
    name: String,
    is_folder: bool,
    thumbnail_url: Option<String>,
}

struct FolderListing {
    title: Option<String>,
    entries: Vec<FolderEntry>,
}

async fn resolve_folder(
    client: &Client,
    url_str: &str,
    root_id: &str,
) -> Result<CloudFolderResult, String> {
    let mut title = format!("Google Drive Folder ({root_id})");
    let mut nodes = vec![folder_node(root_id.to_string(), None, title.clone())];
    let mut files: Vec<(FolderEntry, String)> = Vec::new();
    let mut level = vec![root_id.to_string()];

    while !level.is_empty() {
        let listings: Vec<(String, Result<FolderListing, String>)> = stream::iter(level)
            .map(|parent| async move {
                let listing = list_folder(client, &parent).await;
                (parent, listing)
            })
            .buffer_unordered(LISTING_CONCURRENCY)
            .collect()
            .await;

        let mut next = Vec::new();
        for (parent, listing) in listings {
            let listing = match listing {
                Ok(listing) => listing,
                Err(error) => {
                    if parent == root_id {
                        return Err(error);
                    }
                    tracing::warn!(%error, folder = %parent, "Couldn't list a Google Drive folder");
                    continue;
                }
            };
            if parent == root_id {
                if let Some(name) = listing.title {
                    title = name.clone();
                    nodes[0].name = name;
                }
            }
            for entry in listing.entries {
                if entry.is_folder {
                    if nodes.len() >= MAX_FOLDERS {
                        continue;
                    }
                    nodes.push(folder_node(
                        entry.id.clone(),
                        Some(parent.clone()),
                        entry.name,
                    ));
                    next.push(entry.id);
                } else if files.len() < MAX_FILES {
                    files.push((entry, parent.clone()));
                }
            }
        }
        level = next;
    }

    let file_nodes: Vec<CloudNode> = stream::iter(files)
        .map(|(entry, parent)| async move {
            let mut node = resolve_file(client, &entry.id, Some(&entry.name)).await;
            node.parent_id = Some(parent);
            if node.thumbnail_url.is_none() {
                node.thumbnail_url = entry.thumbnail_url;
            }
            node
        })
        .buffered(FILE_CONCURRENCY)
        .collect()
        .await;

    let total_size = file_nodes.iter().filter_map(|node| node.size).sum();
    let total_files = file_nodes.len() as u64;
    nodes.extend(file_nodes);
    Ok(CloudFolderResult {
        provider: "googledrive".into(),
        url: url_str.to_string(),
        title,
        total_files,
        total_size,
        is_single_file: false,
        nodes,
    })
}

async fn list_folder(client: &Client, folder_id: &str) -> Result<FolderListing, String> {
    let url = format!("https://drive.google.com/embeddedfolderview?id={folder_id}");
    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Google Drive returned HTTP {}", response.status()));
    }
    let html = response.text().await.map_err(|e| e.to_string())?;
    if folder_requires_login(&html) {
        return Err("Google Drive folder requires sign-in or is private".to_string());
    }
    parse_folder_listing(&html)
}

fn folder_requires_login(html: &str) -> bool {
    html.contains("request-storage-access") || html.contains("show-login-page")
}

fn parse_folder_listing(html: &str) -> Result<FolderListing, String> {
    let selector = |css: &str| Selector::parse(css).map_err(|e| e.to_string());
    let entry_selector = selector("div.flip-entry")?;
    let link_selector = selector("a[href]")?;
    let title_selector = selector(".flip-entry-title")?;
    let thumb_selector = selector(".flip-entry-thumb img")?;
    let page_title_selector = selector("title")?;

    let document = Html::parse_document(html);
    let title = document
        .select(&page_title_selector)
        .next()
        .map(|element| element.text().collect::<String>().trim().to_string())
        .filter(|name| !name.is_empty());
    let entries = document
        .select(&entry_selector)
        .filter_map(|entry| {
            let id = entry
                .value()
                .attr("id")?
                .strip_prefix("entry-")?
                .to_string();
            let href = entry.select(&link_selector).next()?.value().attr("href")?;
            let name = entry
                .select(&title_selector)
                .next()?
                .text()
                .collect::<String>()
                .trim()
                .to_string();
            let thumbnail_url = entry
                .select(&thumb_selector)
                .next()
                .and_then(|img| img.value().attr("src"))
                .map(|src| src.replace("=s190", "=w800"));
            Some(FolderEntry {
                id,
                name,
                is_folder: href.contains("/drive/folders/"),
                thumbnail_url,
            })
        })
        .collect();
    Ok(FolderListing { title, entries })
}

fn folder_node(id: String, parent_id: Option<String>, name: String) -> CloudNode {
    CloudNode {
        id,
        parent_id,
        name,
        size: None,
        is_folder: true,
        mime_type: Some("application/vnd.google-apps.folder".into()),
        download_url: None,
        stream_url: None,
        thumbnail_url: None,
        children: None,
        is_encrypted: false,
    }
}

async fn resolve_file(client: &Client, target_id: &str, known_name: Option<&str>) -> CloudNode {
    let direct_url = format!(
        "https://drive.usercontent.google.com/download?id={target_id}&export=download&confirm=t"
    );
    let fallback_download_url =
        format!("https://drive.google.com/uc?export=download&id={target_id}&confirm=t");

    let mut filename = known_name
        .map(str::to_string)
        .unwrap_or_else(|| format!("Google_Drive_File_{target_id}"));
    let mut size = None;

    let view_url = format!("https://drive.google.com/file/d/{target_id}/view");
    if let Ok(resp) = client.get(&view_url).send().await {
        if let Ok(html) = resp.text().await {
            if known_name.is_none() {
                if let Some(name) = page_file_name(&html) {
                    filename = name;
                }
            }

            if let Some(parsed_size) = extract_size_bytes(&html) {
                size = Some(parsed_size);
            }
        }
    }

    if size.is_none() {
        let direct_req = client.get(&direct_url);
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
                        let mut head_req = client.head(&confirmed_url);
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

    CloudNode {
        id: target_id.to_string(),
        parent_id: None,
        name: filename.clone(),
        size,
        is_folder: false,
        mime_type: mime,
        download_url: Some(fallback_download_url),
        stream_url: Some(direct_url),
        thumbnail_url,
        children: None,
        is_encrypted: false,
    }
}

fn page_file_name(html: &str) -> Option<String> {
    let name = match extract_meta_content(html, "og:title") {
        Some(title) => title.trim().to_string(),
        None => extract_title(html)?
            .trim()
            .trim_end_matches("- Google Drive")
            .trim()
            .to_string(),
    };
    (!name.is_empty() && name != "Google Drive").then_some(name)
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
    fn folder_listing_reads_files_and_subfolders() {
        let html = r#"<html><head><title>Doggy</title></head><body><div class="flip-entries">
            <div class="flip-entry" id="entry-1GEL0gIAa" tabindex="0" role="link"><div class="flip-entry-info"><a href="https://drive.google.com/drive/folders/1GEL0gIAa" target="_blank"><div class="flip-entry-title">Outfit 1</div></a></div></div>
            <div class="flip-entry" id="entry-1M2CGvlnP" tabindex="0" role="link"><div class="flip-entry-info"><a href="https://drive.google.com/file/d/1M2CGvlnP/view?usp=drive_web" target="_blank"><div class="flip-entry-visual"><div class="flip-entry-visual-card"><div class="flip-entry-thumb"><img src="https://lh3.googleusercontent.com/drive-storage/abc=s190" alt="Video"/></div></div></div><div class="flip-entry-title">Outfit 1 Cam 3 4K.mp4</div></a></div></div>
        </div></body></html>"#;

        let listing = parse_folder_listing(html).unwrap();
        assert_eq!(listing.title.as_deref(), Some("Doggy"));
        assert_eq!(listing.entries.len(), 2);
        assert!(listing.entries[0].is_folder);
        assert_eq!(listing.entries[0].id, "1GEL0gIAa");
        let file = &listing.entries[1];
        assert!(!file.is_folder);
        assert_eq!(file.name, "Outfit 1 Cam 3 4K.mp4");
        assert_eq!(
            file.thumbnail_url.as_deref(),
            Some("https://lh3.googleusercontent.com/drive-storage/abc=w800")
        );
    }

    #[tokio::test]
    #[ignore = "hits Google Drive"]
    async fn public_folder_lists_nested_files() {
        let client = Client::builder().user_agent("Mozilla/5.0").build().unwrap();
        let result = resolve_googledrive(
            &client,
            "https://drive.google.com/drive/folders/1YdwGRj55E_KjW4740AQ4Gmo42STDa7l2",
        )
        .await
        .unwrap();
        for node in &result.nodes {
            println!(
                "{} {:?} {} {:?}",
                node.is_folder, node.parent_id, node.name, node.size
            );
        }
        assert_eq!(result.title, "Doggy");
        assert!(result.total_files > 0);
    }

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
        for (url, id, is_folder) in [
            (
                "https://drive.google.com/file/d/1a2b3c4d5e/view?usp=sharing",
                "1a2b3c4d5e",
                false,
            ),
            (
                "https://drive.google.com/drive/folders/folder98765",
                "folder98765",
                true,
            ),
            (
                "https://drive.google.com/uc?id=query_id_123&export=download",
                "query_id_123",
                false,
            ),
        ] {
            assert_eq!(
                parse_googledrive_url(url),
                Some((id.to_string(), is_folder))
            );
        }
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

    #[test]
    fn private_folder_page_is_rejected() {
        assert!(folder_requires_login(
            r#"<div class="request-storage-access"><div class="show-login-page">"#
        ));
        assert!(!folder_requires_login(r#"<div class="flip-entry">"#));
    }
}
