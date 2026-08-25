use super::models::{CloudFolderResult, CloudNode};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PixeldrainFileInfo {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PixeldrainListFileItem {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PixeldrainListInfo {
    id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    files: Vec<PixeldrainListFileItem>,
}

pub async fn resolve_pixeldrain(
    client: &Client,
    url_str: &str,
) -> Result<CloudFolderResult, String> {
    let url = reqwest::Url::parse(url_str).map_err(|e| format!("Invalid URL: {e}"))?;
    let path_segments: Vec<&str> = url.path_segments().map(|s| s.collect()).unwrap_or_default();

    if path_segments.is_empty() {
        return Err("Missing Pixeldrain path segment".into());
    }

    let is_list = path_segments.first() == Some(&"l") || path_segments.first() == Some(&"list");
    let is_file = path_segments.first() == Some(&"u") || path_segments.first() == Some(&"file");

    if is_list {
        let list_id = path_segments.get(1).ok_or("Missing Pixeldrain list ID")?;
        let api_url = format!("https://pixeldrain.com/api/list/{list_id}");
        let resp = client
            .get(&api_url)
            .send()
            .await
            .map_err(|e| format!("Pixeldrain API request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Pixeldrain list returned HTTP {}", resp.status()));
        }

        let list_data: PixeldrainListInfo = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Pixeldrain list response: {e}"))?;

        let mut total_size: u64 = 0;
        let mut nodes = Vec::new();

        for file in list_data.files {
            let f_size = file.size.unwrap_or(0);
            total_size += f_size;
            let name = file.name.unwrap_or_else(|| format!("file_{}", file.id));
            let mime = file
                .mime_type
                .or_else(|| mime_guess::from_path(&name).first_raw().map(str::to_string));

            let stream_url = format!("https://pixeldrain.com/api/file/{}", file.id);
            nodes.push(CloudNode {
                id: file.id.clone(),
                parent_id: None,
                name,
                size: file.size,
                is_folder: false,
                mime_type: mime,
                download_url: Some(stream_url.clone()),
                stream_url: Some(stream_url),
                thumbnail_url: Some(format!(
                    "https://pixeldrain.com/api/file/{}/thumbnail",
                    file.id
                )),
                children: None,
            });
        }

        let title = list_data
            .title
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| format!("Pixeldrain List ({})", list_data.id));

        Ok(CloudFolderResult {
            provider: "pixeldrain".into(),
            url: url_str.to_string(),
            title,
            total_files: nodes.len() as u64,
            total_size,
            is_single_file: false,
            nodes,
        })
    } else if is_file {
        let file_id = path_segments.get(1).ok_or("Missing Pixeldrain file ID")?;
        let api_url = format!("https://pixeldrain.com/api/file/{file_id}/info");
        let resp = client
            .get(&api_url)
            .send()
            .await
            .map_err(|e| format!("Pixeldrain API request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Pixeldrain file returned HTTP {}", resp.status()));
        }

        let file_data: PixeldrainFileInfo = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Pixeldrain file response: {e}"))?;

        let name = file_data
            .name
            .unwrap_or_else(|| format!("file_{}", file_data.id));
        let mime = file_data
            .mime_type
            .or_else(|| mime_guess::from_path(&name).first_raw().map(str::to_string));
        let size = file_data.size.unwrap_or(0);
        let stream_url = format!("https://pixeldrain.com/api/file/{}", file_data.id);

        let node = CloudNode {
            id: file_data.id.clone(),
            parent_id: None,
            name: name.clone(),
            size: file_data.size,
            is_folder: false,
            mime_type: mime,
            download_url: Some(stream_url.clone()),
            stream_url: Some(stream_url),
            thumbnail_url: Some(format!(
                "https://pixeldrain.com/api/file/{}/thumbnail",
                file_data.id
            )),
            children: None,
        };

        Ok(CloudFolderResult {
            provider: "pixeldrain".into(),
            url: url_str.to_string(),
            title: name,
            total_files: 1,
            total_size: size,
            is_single_file: true,
            nodes: vec![node],
        })
    } else {
        Err("Unsupported Pixeldrain URL format".into())
    }
}
