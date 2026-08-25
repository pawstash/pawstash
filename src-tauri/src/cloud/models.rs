use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudNode {
    pub id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub size: Option<u64>,
    pub is_folder: bool,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub stream_url: Option<String>,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub children: Option<Vec<CloudNode>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudFolderResult {
    pub provider: String,
    pub url: String,
    pub title: String,
    pub total_files: u64,
    pub total_size: u64,
    pub is_single_file: bool,
    pub nodes: Vec<CloudNode>,
}
