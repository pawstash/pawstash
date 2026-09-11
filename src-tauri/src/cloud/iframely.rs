use super::models::CloudFolderResult;
use reqwest::Client;

pub fn supports_url(url: &str) -> bool {
    super::url_host_matches(url, &["iframely.net", "iframe.ly", "iframely.com"])
}

pub fn extract_iframely_target_url(html: &str) -> Option<String> {
    let mut candidate_url: Option<String> = None;

    if let Some(pos) = html.find("\"linkUri\":\"") {
        let after = &html[pos + 11..];
        if let Some(end) = after.find('"') {
            let u = &after[..end];
            let clean = u.replace(r"\/", "/");
            let not_iframely = !clean.contains("iframely.com")
                && !clean.contains("iframely.net")
                && !clean.contains("iframe.ly");
            if not_iframely && (clean.starts_with("http://") || clean.starts_with("https://")) {
                candidate_url = Some(clean);
            }
        }
    }

    if candidate_url.is_none() {
        if let Some(pos) = html.find("\"redirect\":\"") {
            let after = &html[pos + 12..];
            if let Some(end) = after.find('"') {
                let u = &after[..end];
                let clean = u.replace(r"\/", "/");
                let not_iframely = !clean.contains("iframely.com")
                    && !clean.contains("iframely.net")
                    && !clean.contains("iframe.ly");
                if not_iframely && (clean.starts_with("http://") || clean.starts_with("https://")) {
                    candidate_url = Some(clean);
                }
            }
        }
    }

    if candidate_url.is_none() {
        let clean_html = html.replace(r"\/", "/").replace("&amp;", "&");
        for prefix in &[
            "https://mega.nz/",
            "https://mega.co.nz/",
            "https://pixeldrain.com/",
            "https://www.dropbox.com/",
            "https://dropbox.com/",
            "https://drive.google.com/",
            "https://docs.google.com/",
        ] {
            if let Some(pos) = clean_html.find(prefix) {
                let after = &clean_html[pos..];
                let end = after
                    .find(|c: char| {
                        c == '"' || c == '\'' || c == ' ' || c == '<' || c == '>' || c == '\\'
                    })
                    .unwrap_or(after.len());
                let found = after[..end].to_string();
                if found.len() > prefix.len() {
                    candidate_url = Some(found);
                    break;
                }
            }
        }
    }

    candidate_url.map(|u| {
        u.trim_end_matches([',', '.', ';', ']', '}', ')', '>', '"', '\'', '\\'])
            .to_string()
    })
}

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

    let target_url = extract_iframely_target_url(&html).ok_or_else(|| {
        "Could not extract target media or cloud URL from Iframely embed".to_string()
    })?;

    if super::mega::supports_url(&target_url) {
        return super::mega::resolve_mega(client, &target_url).await;
    }
    if super::pixeldrain::supports_url(&target_url) {
        return super::pixeldrain::resolve_pixeldrain(client, &target_url).await;
    }
    if super::dropbox::supports_url(&target_url) {
        return super::dropbox::resolve_dropbox(client, &target_url).await;
    }
    if super::googledrive::supports_url(&target_url) {
        return super::googledrive::resolve_googledrive(client, &target_url).await;
    }

    Err(format!(
        "Extracted URL is not a supported cloud provider: {target_url}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_link_uri_from_iframely_html() {
        let html = r#"{"linkUri":"https:\/\/mega.nz\/folder\/abcdef#12345"}"#;
        assert_eq!(
            extract_iframely_target_url(html),
            Some("https://mega.nz/folder/abcdef#12345".to_string())
        );
    }

    #[test]
    fn extracts_redirect_from_iframely_html() {
        let html = r#"{"redirect":"https:\/\/pixeldrain.com\/u\/abc12345"}"#;
        assert_eq!(
            extract_iframely_target_url(html),
            Some("https://pixeldrain.com/u/abc12345".to_string())
        );
    }

    #[test]
    fn extracts_embedded_cloud_url_prefix() {
        let html = r#"<script>var data = {"meta":{"href":"https:\/\/drive.google.com\/drive\/folders\/12345?usp=sharing"}};</script>"#;
        assert_eq!(
            extract_iframely_target_url(html),
            Some("https://drive.google.com/drive/folders/12345?usp=sharing".to_string())
        );
    }
}
