use crate::api::models::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub api_url: String,
    #[serde(default)]
    pub fallback_urls: Vec<String>,
    #[serde(default)]
    pub file_url: Option<String>,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub session_cookie: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub services: Vec<String>,
    #[serde(default)]
    pub is_custom: bool,
    #[serde(default)]
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub provider_id: String,
    pub active_endpoint: String,
    pub is_healthy: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
    pub last_checked_at: String,
}

pub fn default_pawchive_services() -> Vec<String> {
    vec![
        "patreon".into(),
        "fanbox".into(),
        "fantia".into(),
        "boosty".into(),
        "subscribestar".into(),
        "gumroad".into(),
        "dlsite".into(),
        "discord".into(),
        "afdian".into(),
    ]
}

pub fn default_onlyhaven_services() -> Vec<String> {
    vec!["onlyfans".into(), "fansly".into(), "candfans".into()]
}

pub fn default_coomer_services() -> Vec<String> {
    default_onlyhaven_services()
}

#[async_trait]
pub trait SourceProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn config(&self) -> ProviderConfig;
    fn supports_service(&self, service: &str) -> bool;
    fn get_active_endpoint(&self) -> String;

    async fn test_connection(&self) -> Result<ProviderHealth, String>;
    async fn update_config(&self, config: ProviderConfig) -> Result<(), String>;

    async fn fetch_creators(&self) -> Result<Vec<Creator>, String>;
    async fn fetch_creator_profile(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Creator, String>;
    async fn fetch_creator_links(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<CreatorProfile>, String>;
    async fn fetch_similar_creators(
        &self,
        _service: &str,
        _creator_id: &str,
    ) -> Result<Vec<CreatorProfile>, String> {
        Ok(Vec::new())
    }
    async fn fetch_creator_tags(
        &self,
        _service: &str,
        _creator_id: &str,
    ) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }
    async fn fetch_announcements(
        &self,
        _service: &str,
        _creator_id: &str,
    ) -> Result<Vec<Announcement>, String> {
        Ok(Vec::new())
    }
    async fn fetch_posts(
        &self,
        service: &str,
        creator_id: &str,
        offset: u32,
        query: Option<&str>,
    ) -> Result<Vec<Post>, String>;
    async fn fetch_post(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Option<Post>, String>;
    async fn fetch_post_revisions(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Vec<PostRevision>, String>;
    async fn fetch_recent_posts(
        &self,
        query: Option<&str>,
        offset: u32,
    ) -> Result<Vec<Post>, String>;
    async fn fetch_popular_posts(
        &self,
        period: &str,
        date: Option<&str>,
        offset: u32,
    ) -> Result<Vec<Post>, String>;
    async fn fetch_post_comments(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Vec<Comment>, String>;

    async fn fetch_account_favorites(
        &self,
        favorite_type: Option<&str>,
    ) -> Result<Vec<Favorite>, String>;
    async fn set_creator_favorite(
        &self,
        service: &str,
        creator_id: &str,
        favorite: bool,
    ) -> Result<ApiActionResult, String>;
    async fn set_post_favorite(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
        favorite: bool,
    ) -> Result<ApiActionResult, String>;

    fn resolve_media_url(&self, file_path: &str, server: Option<&str>) -> String;
    fn resolve_thumbnail_url(&self, thumb_path: &str) -> String;
    async fn fetch_creator_artwork_data_url(
        &self,
        service: &str,
        creator_id: &str,
        artwork_type: &str,
    ) -> Result<String, String>;

    async fn search_hash(&self, file_hash: &str) -> Result<FileSearchResult, String>;
    async fn fetch_fancards(&self, service: &str, creator_id: &str)
        -> Result<Vec<Fancard>, String>;
    async fn flag_post(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<ApiActionResult, String>;
    async fn is_post_flagged(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<bool, String>;

    async fn login(&self, username: &str, password: &str) -> Result<String, String>;
    async fn logout(&self) -> Result<(), String>;
    async fn get_account_session(&self) -> Result<AccountSession, String>;
    async fn app_version(&self) -> Result<String, String>;

    async fn resolve_post_identity(
        &self,
        service: &str,
        post_id: &str,
    ) -> Result<Option<(String, String, String)>, String>;
    async fn expand_short_link(&self, raw_url: &str) -> Result<Option<String>, String>;
}
