use crate::config::settings::AppSettings;
use std::path::Path;

pub struct PostMetadataExport<'a> {
    pub service: &'a str,
    pub creator_id: &'a str,
    pub creator_name: &'a str,
    pub post_id: &'a str,
    pub post_title: &'a str,
    pub published: Option<&'a str>,
    pub content: Option<&'a str>,
    pub tags: Option<&'a [String]>,
    pub origin_url: Option<String>,
}

pub fn save_post_metadata(
    target_dir: &Path,
    metadata: &PostMetadataExport<'_>,
    settings: &AppSettings,
) -> Result<(), String> {
    if !settings.download_save_metadata {
        return Ok(());
    }

    let format = settings.download_metadata_format.to_lowercase();
    let write_txt = format == "txt" || format == "both";
    let write_json = format == "json" || format == "both";

    if write_txt {
        let txt_path = target_dir.join("info.txt");
        if !txt_path.exists() {
            let mut lines = Vec::new();
            if !metadata.post_title.is_empty() {
                lines.push(format!("Title: {}", metadata.post_title));
            }
            if !metadata.creator_name.is_empty() {
                lines.push(format!(
                    "Creator: {} ({})",
                    metadata.creator_name, metadata.service
                ));
            } else {
                lines.push(format!(
                    "Creator: {} ({})",
                    metadata.creator_id, metadata.service
                ));
            }
            if let Some(pub_date) = metadata.published {
                if !pub_date.is_empty() {
                    lines.push(format!("Published: {}", pub_date));
                }
            }
            if let Some(url) = &metadata.origin_url {
                lines.push(format!("Source: {}", url));
            } else {
                let domain = if settings.api_domain.is_empty() {
                    "pawchive.pw"
                } else {
                    &settings.api_domain
                };
                lines.push(format!(
                    "Source: https://{}/{}/user/{}/post/{}",
                    domain, metadata.service, metadata.creator_id, metadata.post_id
                ));
            }

            if let Some(tags) = metadata.tags {
                if !tags.is_empty() {
                    lines.push(format!("Tags: {}", tags.join(", ")));
                }
            }

            if let Some(content) = metadata.content {
                let clean_text = strip_html_tags(content);
                if !clean_text.trim().is_empty() {
                    lines.push("\n--- Description ---".to_string());
                    lines.push(clean_text);
                }
            }

            let _ = std::fs::write(&txt_path, lines.join("\n"));
        }
    }

    if write_json {
        let json_path = target_dir.join("post.json");
        if !json_path.exists() {
            let domain = if settings.api_domain.is_empty() {
                "pawchive.pw"
            } else {
                &settings.api_domain
            };
            let source_url = metadata.origin_url.clone().unwrap_or_else(|| {
                format!(
                    "https://{}/{}/user/{}/post/{}",
                    domain, metadata.service, metadata.creator_id, metadata.post_id
                )
            });
            let json_value = serde_json::json!({
                "service": metadata.service,
                "creator_id": metadata.creator_id,
                "creator_name": metadata.creator_name,
                "post_id": metadata.post_id,
                "title": metadata.post_title,
                "published": metadata.published,
                "content": metadata.content,
                "tags": metadata.tags,
                "source_url": source_url
            });
            if let Ok(json_str) = serde_json::to_string_pretty(&json_value) {
                let _ = std::fs::write(&json_path, json_str);
            }
        }
    }

    Ok(())
}

fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_string()
}
