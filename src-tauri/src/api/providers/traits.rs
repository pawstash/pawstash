use crate::api::models::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub fn path_has_image_mime(path: &str) -> bool {
    mime_guess::from_path(path)
        .first_raw()
        .is_some_and(|mime| mime.starts_with("image/"))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
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
    #[serde(default)]
    pub advanced_network: bool,
    /// Empty or 0 means the global value.
    #[serde(default)]
    pub user_agent: String,
    #[serde(default)]
    pub timeout_secs: u64,
    #[serde(default)]
    pub proxy_url: String,
    #[serde(default)]
    pub max_retries: u64,
    #[serde(default)]
    pub min_interval_ms: u64,
}

impl ProviderConfig {
    fn override_active(&self) -> bool {
        self.advanced_network
    }

    pub fn effective_user_agent(&self) -> String {
        if self.override_active() && !self.user_agent.trim().is_empty() {
            self.user_agent.trim().to_string()
        } else {
            crate::net::api_user_agent()
        }
    }

    pub fn effective_timeout(&self) -> std::time::Duration {
        if self.override_active() && self.timeout_secs > 0 {
            std::time::Duration::from_secs(self.timeout_secs)
        } else {
            crate::net::defaults().request_timeout
        }
    }

    pub fn effective_proxy_url(&self) -> Option<String> {
        if self.override_active() && !self.proxy_url.trim().is_empty() {
            Some(self.proxy_url.trim().to_string())
        } else {
            None
        }
    }

    pub fn apply_queue_overrides(
        &self,
        mut config: super::queue::ProviderQueueConfig,
    ) -> super::queue::ProviderQueueConfig {
        if !self.override_active() {
            return config;
        }
        if self.max_retries > 0 {
            config.max_retries = self.max_retries as usize;
        }
        if self.min_interval_ms > 0 {
            config.min_interval = std::time::Duration::from_millis(self.min_interval_ms);
        }
        config
    }

    pub fn apply_network_to(&self, settings: &mut crate::config::settings::AppSettings) {
        let global = crate::net::defaults();
        match self.effective_proxy_url() {
            Some(url) => {
                settings.proxy_mode = crate::config::settings::ProxyMode::Custom;
                settings.proxy_url = url;
                settings.proxy_username = String::new();
                settings.proxy_password = String::new();
                settings.proxy_bypass_local = true;
            }
            None => {
                settings.proxy_mode = global.proxy.mode;
                settings.proxy_url = global.proxy.url.clone();
                settings.proxy_username = global.proxy.username.clone();
                settings.proxy_password = global.proxy.password.clone();
                settings.proxy_bypass_local = global.proxy.bypass_local;
            }
        }
        if self.override_active() && self.timeout_secs > 0 {
            settings.network_timeout_secs = self.timeout_secs;
        }
    }

    pub fn network_builder(&self) -> reqwest::ClientBuilder {
        match self.effective_proxy_url() {
            Some(url) => match reqwest::Proxy::all(&url) {
                Ok(proxy) => crate::net::builder().proxy(proxy),
                Err(error) => {
                    tracing::warn!(provider = %self.id, %error, "Invalid provider proxy URL; using global proxy");
                    crate::net::builder_proxied()
                }
            },
            None => crate::net::builder_proxied(),
        }
    }
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
    pub field_type: String,
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

    fn normalize_media_url(&self, url: &str) -> String {
        url.to_string()
    }

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
            let normalized = crate::cloud::normalize_cloud_direct_url(&path);
            attachment.url = Some(self.normalize_media_url(&normalized));
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
        attachment.url = (!media_url.is_empty()).then(|| self.normalize_media_url(&media_url));
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

#[cfg(test)]
mod provider_network_tests {
    use super::super::queue::ProviderQueueConfig;
    use super::*;
    use std::time::Duration;

    fn tuned(advanced_network: bool) -> ProviderConfig {
        ProviderConfig {
            id: "test".to_string(),
            advanced_network,
            user_agent: "  Tuned/1.0  ".to_string(),
            timeout_secs: 12,
            proxy_url: "  http://127.0.0.1:9000  ".to_string(),
            max_retries: 7,
            min_interval_ms: 250,
            ..Default::default()
        }
    }

    #[test]
    fn overrides_apply_trimmed_only_when_advanced_network_is_on() {
        let off = tuned(false);
        assert_ne!(off.effective_user_agent(), "Tuned/1.0");
        assert_eq!(
            off.effective_timeout(),
            crate::net::defaults().request_timeout
        );
        assert_eq!(off.effective_proxy_url(), None);
        assert_eq!(
            off.apply_queue_overrides(ProviderQueueConfig::default())
                .max_retries,
            3
        );

        let on = tuned(true);
        assert_eq!(on.effective_user_agent(), "Tuned/1.0");
        assert_eq!(on.effective_timeout(), Duration::from_secs(12));
        assert_eq!(
            on.effective_proxy_url().as_deref(),
            Some("http://127.0.0.1:9000")
        );
        let queue = on.apply_queue_overrides(ProviderQueueConfig::default());
        assert_eq!(
            (queue.max_retries, queue.min_interval),
            (7, Duration::from_millis(250))
        );
    }

    #[test]
    fn blank_fields_fall_through_to_the_global_value() {
        let config = ProviderConfig {
            advanced_network: true,
            id: "test".to_string(),
            ..Default::default()
        };
        assert_eq!(config.effective_user_agent(), crate::net::api_user_agent());
        assert_eq!(
            config.effective_timeout(),
            crate::net::defaults().request_timeout
        );
        assert_eq!(config.effective_proxy_url(), None);
    }
}
