use super::models::CloudFolderResult;
use reqwest::Client;

pub async fn resolve_iframely(client: &Client, url_str: &str) -> Result<CloudFolderResult, String> {
    let resp = client
        .get(url_str)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .send()
        .await
        .map_err(|e| format!("Iframely request failed: {e}"))?;

    let html = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read Iframely response body: {e}"))?;

    let mut candidate_url: Option<String> = None;

    // 1. Look for linkUri in Javascript payload: "linkUri":"https://..."
    if let Some(pos) = html.find("\"linkUri\":\"") {
        let after = &html[pos + 11..];
        if let Some(end) = after.find('"') {
            let u = &after[..end];
            let clean = u.replace("\\/", "/");
            if !clean.contains("iframely.com")
                && (clean.starts_with("http://") || clean.starts_with("https://"))
            {
                candidate_url = Some(clean);
            }
        }
    }

    // 2. Look for redirect in Javascript payload: "redirect":"https://..."
    if candidate_url.is_none() {
        if let Some(pos) = html.find("\"redirect\":\"") {
            let after = &html[pos + 12..];
            if let Some(end) = after.find('"') {
                let u = &after[..end];
                let clean = u.replace("\\/", "/");
                if !clean.contains("iframely.com")
                    && (clean.starts_with("http://") || clean.starts_with("https://"))
                {
                    candidate_url = Some(clean);
                }
            }
        }
    }

    // 3. Search for known cloud links in raw HTML
    if candidate_url.is_none() {
        for prefix in &[
            "https://mega.nz/",
            "https://mega.co.nz/",
            "https://pixeldrain.com/",
            "https://www.dropbox.com/",
            "https://dropbox.com/",
            "https://drive.google.com/",
            "https://docs.google.com/",
        ] {
            if let Some(pos) = html.find(prefix) {
                let after = &html[pos..];
                let end = after
                    .find(|c: char| {
                        c == '"' || c == '\'' || c == ' ' || c == '<' || c == '>' || c == '\\'
                    })
                    .unwrap_or(after.len());
                let found = after[..end].replace("\\/", "/");
                if found.len() > prefix.len() {
                    candidate_url = Some(found);
                    break;
                }
            }
        }
    }

    let target_url = candidate_url.ok_or_else(|| {
        "Could not extract target media or cloud URL from Iframely embed".to_string()
    })?;

    // Now resolve the extracted target URL
    let lower = target_url.to_lowercase();
    if lower.contains("mega.nz") || lower.contains("mega.co.nz") {
        return super::mega::resolve_mega(client, &target_url).await;
    }
    if lower.contains("pixeldrain.com") {
        return super::pixeldrain::resolve_pixeldrain(client, &target_url).await;
    }
    if lower.contains("dropbox.com") {
        return super::dropbox::resolve_dropbox(client, &target_url).await;
    }
    if lower.contains("drive.google.com") || lower.contains("docs.google.com") {
        return super::googledrive::resolve_googledrive(client, &target_url).await;
    }

    Err(format!(
        "Extracted URL is not a supported cloud provider: {target_url}"
    ))
}
