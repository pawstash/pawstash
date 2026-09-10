use crate::api::models::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub fn path_has_image_mime(path: &str) -> bool {
    mime_guess::from_path(path)
        .first_raw()
        .is_some_and(|mime| mime.starts_with("image/"))
}

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
    pub file_prefix: Option<String>,
    #[serde(default)]
    pub image_prefix: Option<String>,
    #[serde(default, serialize_with = "serialize_redacted_secret")]
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

fn serialize_redacted_secret<S>(_: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str("")
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

pub fn derive_subdomain_url(base_url: &str, prefix: &str) -> String {
    let clean_url = if base_url.starts_with("http://") || base_url.starts_with("https://") {
        base_url.trim_end_matches('/').to_string()
    } else {
        format!("https://{}", base_url.trim_end_matches('/'))
    };

    if let Ok(parsed) = reqwest::Url::parse(&clean_url) {
        if let Some(host) = parsed.host_str() {
            let scheme = parsed.scheme();
            let base_host = host.trim_start_matches("www.").trim_start_matches("api.");
            let parts: Vec<&str> = base_host.split('.').collect();
            let domain = if parts.len() > 2 {
                parts[parts.len() - 2..].join(".")
            } else {
                base_host.to_string()
            };
            return format!("{scheme}://{prefix}.{domain}");
        }
    }
    clean_url
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthField {
    pub key: String,
    pub label_key: String,
    pub field_type: String, // "text" | "password" | "textarea"
    pub placeholder: Option<String>,
    pub help_text_key: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderAuthSchema {
    pub provider_id: String,
    pub supports_auth: bool,
    pub supports_remote_favorites: bool,
    pub supports_push_favorites: bool,
    pub auth_fields: Vec<AuthField>,
    pub help_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PopularPeriodOption {
    pub id: String,
    pub label_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PopularCapabilities {
    pub supported: bool,
    pub periods: Vec<PopularPeriodOption>,
    pub default_period: Option<String>,
    pub supports_date: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SortOption {
    pub id: String,
    pub label_key: String,
    pub is_server_side: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub provider_id: String,
    pub popular: PopularCapabilities,
    pub creator_sorts: Vec<SortOption>,
    pub post_sorts: Vec<SortOption>,
    pub supports_query_search: bool,
    pub supports_date_filter: bool,
    pub supports_hash_search: bool,
    pub supports_announcements: bool,
    pub supports_fancards: bool,
    pub supports_similar_creators: bool,
    pub supports_creator_tags: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoritesSyncResult {
    pub provider_id: String,
    pub pulled_count: usize,
    pub pushed_count: usize,
    pub errors: Vec<String>,
}

#[async_trait]
pub trait SourceProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn config(&self) -> ProviderConfig;
    fn supports_service(&self, service: &str) -> bool;
    fn get_active_endpoint(&self) -> String;

    fn request_queue(
        &self,
    ) -> Option<std::sync::Arc<crate::api::providers::queue::ProviderRequestQueue>> {
        None
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            provider_id: self.id().to_string(),
            popular: PopularCapabilities {
                supported: true,
                periods: vec![
                    PopularPeriodOption {
                        id: "day".to_string(),
                        label_key: "feed.day".to_string(),
                    },
                    PopularPeriodOption {
                        id: "week".to_string(),
                        label_key: "feed.week".to_string(),
                    },
                    PopularPeriodOption {
                        id: "month".to_string(),
                        label_key: "feed.month".to_string(),
                    },
                ],
                default_period: Some("day".to_string()),
                supports_date: true,
            },
            creator_sorts: vec![
                SortOption {
                    id: "favorited".to_string(),
                    label_key: "creators.sort_favorited".to_string(),
                    is_server_side: false,
                },
                SortOption {
                    id: "updated".to_string(),
                    label_key: "creators.sort_updated".to_string(),
                    is_server_side: false,
                },
                SortOption {
                    id: "indexed".to_string(),
                    label_key: "creators.sort_indexed".to_string(),
                    is_server_side: false,
                },
                SortOption {
                    id: "name".to_string(),
                    label_key: "creators.sort_name".to_string(),
                    is_server_side: false,
                },
            ],
            post_sorts: vec![
                SortOption {
                    id: "recent".to_string(),
                    label_key: "feed.recent".to_string(),
                    is_server_side: true,
                },
                SortOption {
                    id: "popular".to_string(),
                    label_key: "feed.popular".to_string(),
                    is_server_side: true,
                },
            ],
            supports_query_search: true,
            supports_date_filter: true,
            supports_hash_search: false,
            supports_announcements: false,
            supports_fancards: false,
            supports_similar_creators: false,
            supports_creator_tags: false,
        }
    }

    fn auth_schema(&self) -> ProviderAuthSchema {
        ProviderAuthSchema {
            provider_id: self.id().to_string(),
            supports_auth: false,
            supports_remote_favorites: false,
            supports_push_favorites: false,
            auth_fields: Vec::new(),
            help_url: None,
        }
    }

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
        Err(format!(
            "Provider '{}' does not support similar creators",
            self.id()
        ))
    }
    async fn fetch_creator_tags(
        &self,
        _service: &str,
        _creator_id: &str,
    ) -> Result<Vec<String>, String> {
        Err(format!(
            "Provider '{}' does not support creator tags",
            self.id()
        ))
    }
    async fn fetch_announcements(
        &self,
        _service: &str,
        _creator_id: &str,
    ) -> Result<Vec<Announcement>, String> {
        Err(format!(
            "Provider '{}' does not support announcements",
            self.id()
        ))
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

    fn canonical_attachment_path(&self, _attachment: &Attachment, path: &str) -> String {
        path.to_string()
    }

    fn attachment_uses_thumbnail(&self, _attachment: &Attachment) -> bool {
        false
    }

    fn enrich_attachment(&self, attachment: &mut Attachment) {
        let Some(path) = attachment.path.clone() else {
            return;
        };
        if path.starts_with("http://") || path.starts_with("https://") {
            attachment.url = Some(crate::cloud::normalize_cloud_direct_url(&path));
            return;
        }
        if path.starts_with("/cloud_stream/") {
            attachment.url = Some(path);
            return;
        }
        if path.starts_with("cloud:") || path.starts_with("cloud_folder:") {
            return;
        }

        let canonical_path = self.canonical_attachment_path(attachment, &path);
        let thumbnail_url = self.resolve_thumbnail_url(&canonical_path);
        attachment.thumbnail_url = (!thumbnail_url.is_empty()).then_some(thumbnail_url);

        if self.attachment_uses_thumbnail(attachment) {
            attachment.url = attachment.thumbnail_url.clone();
            return;
        }

        let media_url = self.resolve_media_url(&canonical_path, attachment.server.as_deref());
        attachment.url = (!media_url.is_empty()).then_some(media_url);
    }

    fn resolve_post_url(&self, service: &str, creator_id: &str, post_id: &str) -> String;
    fn resolve_creator_url(&self, service: &str, creator_id: &str) -> String;
    fn resolve_avatar_url(&self, service: &str, creator_id: &str) -> String;
    fn resolve_banner_url(&self, service: &str, creator_id: &str) -> String;
    fn resolve_fancard_media_url(&self, _service: &str, _hash: &str, _ext: &str) -> String {
        String::new()
    }
    fn resolve_fancard_thumbnail_url(&self, _service: &str, _hash: &str, _ext: &str) -> String {
        String::new()
    }
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
