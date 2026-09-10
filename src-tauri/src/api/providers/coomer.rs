use super::queue::{ProviderQueueConfig, ProviderRequestQueue};
use super::traits::{
    path_has_image_mime, AuthField, PopularCapabilities, PopularPeriodOption, ProviderAuthSchema,
    ProviderCapabilities, ProviderConfig, ProviderHealth, SortOption, SourceProvider,
};
use crate::api::models::*;
use async_trait::async_trait;
use base64::Engine;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, COOKIE, USER_AGENT};
use reqwest::{Client, Url};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct CoomerProvider {
    id: String,
    name: String,
    config: Arc<RwLock<ProviderConfig>>,
    client: Arc<RwLock<Client>>,
    queue: Arc<ProviderRequestQueue>,
}

impl CoomerProvider {
    fn archive_host_matches(url: &Url) -> bool {
        url.host_str()
            .map(str::to_ascii_lowercase)
            .is_some_and(|host| {
                ["coomer.st", "coomer.su", "coomer.party"]
                    .iter()
                    .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
            })
    }

    pub fn parse_archive_post_url(url: &Url) -> Option<super::ProviderPostLink> {
        if !Self::archive_host_matches(url) {
            return None;
        }
        let segments: Vec<&str> = url
            .path_segments()?
            .filter(|part| !part.is_empty())
            .collect();
        if segments.len() < 5
            || !matches!(segments[1], "user" | "server" | "channel")
            || segments[3] != "post"
        {
            return None;
        }
        Some(super::ProviderPostLink {
            service: super::clean_link_segment(segments[0])?,
            creator_hint: Some(super::clean_link_segment(segments[2])?),
            post_id: super::clean_link_segment(segments[4])?,
        })
    }

    pub fn parse_archive_creator_url(url: &Url) -> Option<super::ProviderCreatorLink> {
        if !Self::archive_host_matches(url) {
            return None;
        }
        let segments: Vec<&str> = url
            .path_segments()?
            .filter(|part| !part.is_empty())
            .collect();
        if segments.len() < 3 || !matches!(segments[1], "user" | "server" | "channel") {
            return None;
        }
        Some(super::ProviderCreatorLink {
            service: super::clean_link_segment(segments[0])?,
            creator_hint: super::clean_link_segment(segments[2])?,
        })
    }

    pub fn matches_config(config: &ProviderConfig) -> bool {
        config.id.eq_ignore_ascii_case("coomer")
            || config
                .api_url
                .parse::<Url>()
                .ok()
                .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
                .is_some_and(|host| host == "coomer.st" || host.ends_with(".coomer.st"))
    }

    pub fn default_services() -> Vec<String> {
        vec!["onlyfans".into(), "fansly".into(), "candfans".into()]
    }

    pub fn default_queue_config() -> ProviderQueueConfig {
        ProviderQueueConfig {
            max_concurrent: 2,
            min_interval: std::time::Duration::from_millis(200),
            max_retries: 4,
            default_retry_after: std::time::Duration::from_millis(1500),
            max_cooldown: std::time::Duration::from_secs(15),
        }
    }

    pub fn default_config() -> ProviderConfig {
        ProviderConfig {
            id: "coomer".into(),
            name: "Coomer".into(),
            enabled: false,
            api_url: "https://coomer.st".into(),
            fallback_urls: vec![],
            file_url: Some("https://c1.coomer.st".into()),
            image_url: Some("https://img.coomer.st".into()),
            file_prefix: Some("c1".into()),
            image_prefix: Some("img".into()),
            session_cookie: String::new(),
            username: String::new(),
            services: Self::default_services(),
            is_custom: false,
            priority: 2,
        }
    }

    pub fn new(config: ProviderConfig) -> Result<Self, String> {
        Self::with_queue_config(config, Self::default_queue_config())
    }

    pub fn with_queue_config(
        config: ProviderConfig,
        queue_config: ProviderQueueConfig,
    ) -> Result<Self, String> {
        let id = if config.id.trim().is_empty() {
            "coomer".to_string()
        } else {
            config.id.clone()
        };
        let name = if config.name.trim().is_empty() {
            "Coomer".to_string()
        } else {
            config.name.clone()
        };

        let client = Self::build_client(&config)?;
        let queue = Arc::new(ProviderRequestQueue::new(id.clone(), queue_config));
        Ok(Self {
            id,
            name,
            config: Arc::new(RwLock::new(config)),
            client: Arc::new(RwLock::new(client)),
            queue,
        })
    }

    pub fn queue(&self) -> &Arc<ProviderRequestQueue> {
        &self.queue
    }

    fn prepare_post(
        &self,
        mut post: Post,
        authoritative_attachments: Option<Vec<Attachment>>,
    ) -> Post {
        if authoritative_attachments
            .as_ref()
            .is_some_and(|attachments| !attachments.is_empty())
        {
            post.attachments = authoritative_attachments;
        }
        if post
            .file
            .as_ref()
            .is_some_and(Attachment::is_empty_placeholder)
        {
            post.file = None;
        }

        let provider_id = serde_json::Value::String(self.id.clone());
        post.extra
            .insert("provider_id".to_string(), provider_id.clone());
        if let Some(file) = post.file.as_mut() {
            file.extra
                .insert("provider_id".to_string(), provider_id.clone());
        }
        if let Some(attachments) = post.attachments.as_mut() {
            for attachment in attachments {
                attachment
                    .extra
                    .insert("provider_id".to_string(), provider_id.clone());
            }
        }
        post
    }

    pub fn build_headers(config: &ProviderConfig) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(crate::downloader::PAWSTASH_USER_AGENT),
        );
        // DDoS-Guard rejects this endpoint unless it receives the CSS accept header.
        headers.insert(ACCEPT, HeaderValue::from_static("text/css"));
        let cookie_raw = config.session_cookie.trim();
        if !cookie_raw.is_empty() {
            let cookie_val = if cookie_raw.contains('=') {
                cookie_raw.to_string()
            } else {
                format!("session={cookie_raw}")
            };
            if let Ok(hv) = HeaderValue::from_str(&cookie_val) {
                headers.insert(COOKIE, hv);
            }
        }
        headers
    }

    fn build_client(config: &ProviderConfig) -> Result<Client, String> {
        let headers = Self::build_headers(config);
        Client::builder()
            .timeout(Duration::from_secs(30))
            .gzip(true)
            .default_headers(headers)
            .build()
            .map_err(|e| format!("Failed to create Coomer HTTP client: {e}"))
    }

    fn base_url(raw: &str) -> String {
        let clean = raw.trim().trim_end_matches('/');
        if clean.starts_with("http://") || clean.starts_with("https://") {
            format!("{clean}/api/v1")
        } else {
            format!("https://{clean}/api/v1")
        }
    }

    fn segment(val: &str) -> String {
        urlencoding::encode(val).to_string()
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<T, String> {
        let (conf, client) = {
            (
                self.config.read().unwrap().clone(),
                self.client.read().unwrap().clone(),
            )
        };

        let mut candidate_urls = vec![Self::base_url(&conf.api_url)];
        for fallback in &conf.fallback_urls {
            let trimmed = fallback.trim();
            if !trimmed.is_empty() {
                candidate_urls.push(Self::base_url(trimmed));
            }
        }

        let mut last_error = String::new();
        for base in candidate_urls {
            let url = format!("{base}{path}");
            let params_vec = params.to_vec();
            let c = client.clone();
            let u = url.clone();

            let resp = match self
                .queue
                .send(move || {
                    let req = c.get(&u).query(&params_vec);
                    async move { req.send().await }
                })
                .await
            {
                Ok(r) => r,
                Err(err) => {
                    last_error = err;
                    continue;
                }
            };

            let status = resp.status();
            if status == reqwest::StatusCode::NOT_FOUND {
                return Err("Coomer API HTTP 404: Not Found".to_string());
            }
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                last_error = format!("Coomer API HTTP {status}: {}", body.trim());
                continue;
            }
            let body = match resp.text().await {
                Ok(b) => b,
                Err(e) => {
                    last_error = e.to_string();
                    continue;
                }
            };
            match serde_json::from_str::<T>(&body) {
                Ok(data) => return Ok(data),
                Err(e) => {
                    last_error = format!("Failed to parse Coomer response: {e}");
                    continue;
                }
            }
        }

        Err(if last_error.is_empty() {
            "Coomer request failed".to_string()
        } else {
            last_error
        })
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
enum CoomerSinglePostResponse {
    Wrapped {
        post: Post,
        #[serde(default)]
        attachments: Option<Vec<Attachment>>,
    },
    Array(Vec<Post>),
    Direct(Post),
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
enum CoomerPostsResponse {
    Wrapped {
        #[serde(default)]
        posts: Vec<Post>,
    },
    Direct(Vec<Post>),
}

impl CoomerPostsResponse {
    fn into_posts(self) -> Vec<Post> {
        match self {
            Self::Wrapped { posts } => posts,
            Self::Direct(posts) => posts,
        }
    }
}

#[async_trait]
impl SourceProvider for CoomerProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn config(&self) -> ProviderConfig {
        self.config.read().unwrap().clone()
    }

    fn supports_service(&self, service: &str) -> bool {
        self.config()
            .services
            .iter()
            .any(|s| s.eq_ignore_ascii_case(service))
    }

    fn get_active_endpoint(&self) -> String {
        self.config.read().unwrap().api_url.clone()
    }

    fn request_queue(&self) -> Option<Arc<ProviderRequestQueue>> {
        Some(self.queue.clone())
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            provider_id: self.id.clone(),
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
                    PopularPeriodOption {
                        id: "recent".to_string(),
                        label_key: "feed.recent".to_string(),
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
            supports_hash_search: true,
            supports_announcements: false,
            supports_fancards: false,
            supports_similar_creators: false,
            supports_creator_tags: true,
        }
    }

    fn auth_schema(&self) -> ProviderAuthSchema {
        ProviderAuthSchema {
            provider_id: self.id.clone(),
            supports_auth: true,
            supports_remote_favorites: false,
            supports_push_favorites: false,
            auth_fields: vec![AuthField {
                key: "session_cookie".to_string(),
                label_key: "settings.providers.session_cookie".to_string(),
                field_type: "password".to_string(),
                placeholder: Some("session=...".to_string()),
                help_text_key: Some("settings.providers.cookie_help".to_string()),
                required: false,
            }],
            help_url: {
                let base = self.get_active_endpoint();
                let clean = base.trim_end_matches('/');
                if clean.is_empty() {
                    None
                } else {
                    Some(clean.to_string())
                }
            },
        }
    }

    async fn test_connection(&self) -> Result<ProviderHealth, String> {
        let start = Instant::now();
        let endpoint = self.get_active_endpoint();
        let now_str = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();

        match self.app_version().await {
            Ok(_) => Ok(ProviderHealth {
                provider_id: self.id.clone(),
                active_endpoint: endpoint,
                is_healthy: true,
                latency_ms: start.elapsed().as_millis() as u64,
                error: None,
                last_checked_at: now_str,
            }),
            Err(e) => Ok(ProviderHealth {
                provider_id: self.id.clone(),
                active_endpoint: endpoint,
                is_healthy: false,
                latency_ms: start.elapsed().as_millis() as u64,
                error: Some(e),
                last_checked_at: now_str,
            }),
        }
    }

    async fn update_config(&self, config: ProviderConfig) -> Result<(), String> {
        let new_client = Self::build_client(&config)?;
        *self.client.write().unwrap() = new_client;
        *self.config.write().unwrap() = config;
        Ok(())
    }

    async fn fetch_creators(&self) -> Result<Vec<Creator>, String> {
        let mut list: Vec<Creator> = self.get_json("/creators", &[]).await?;
        let prov_id = self.id.clone();
        for c in &mut list {
            c.extra.insert(
                "provider_id".to_string(),
                serde_json::Value::String(prov_id.clone()),
            );
        }
        Ok(list)
    }

    async fn fetch_creator_profile(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Creator, String> {
        let path = format!(
            "/{}/user/{}/profile",
            Self::segment(service),
            Self::segment(creator_id)
        );
        match self.get_json::<CreatorProfile>(&path, &[]).await {
            Ok(prof) => {
                let id = if prof.id.trim().is_empty() {
                    creator_id.to_string()
                } else {
                    prof.id
                };
                let service = if prof.service.trim().is_empty() {
                    service.to_string()
                } else {
                    prof.service
                };
                let name = if prof.name.trim().is_empty() {
                    creator_id.to_string()
                } else {
                    prof.name
                };
                let mut extra = prof.extra;
                extra.insert(
                    "provider_id".to_string(),
                    serde_json::Value::String(self.id.clone()),
                );
                let avatar_url = prof
                    .avatar_url
                    .filter(|s| !s.trim().is_empty())
                    .or_else(|| {
                        let av = self.resolve_avatar_url(&service, &id);
                        if av.is_empty() {
                            None
                        } else {
                            Some(av)
                        }
                    });
                let banner_url = prof
                    .banner_url
                    .filter(|s| !s.trim().is_empty())
                    .or_else(|| {
                        let bn = self.resolve_banner_url(&service, &id);
                        if bn.is_empty() {
                            None
                        } else {
                            Some(bn)
                        }
                    });
                Ok(Creator {
                    id,
                    name,
                    service,
                    public_id: prof.public_id,
                    relation_id: prof.relation_id,
                    indexed: parse_timestamp_value(prof.indexed.as_ref()),
                    updated: parse_timestamp_value(prof.updated.as_ref()),
                    favorited: prof.favorited,
                    ever_imported: prof.ever_imported,
                    avatar_url,
                    avatar_path: prof.avatar_path,
                    banner_url,
                    banner_path: prof.banner_path,
                    page_url: prof.page_url,
                    extra,
                })
            }
            Err(_) => {
                let mut extra = HashMap::new();
                extra.insert(
                    "provider_id".to_string(),
                    serde_json::Value::String(self.id.clone()),
                );
                let av = self.resolve_avatar_url(service, creator_id);
                let avatar_url = if av.is_empty() { None } else { Some(av) };
                Ok(Creator {
                    id: creator_id.to_string(),
                    name: creator_id.to_string(),
                    service: service.to_string(),
                    public_id: None,
                    relation_id: None,
                    indexed: None,
                    updated: None,
                    favorited: None,
                    ever_imported: None,
                    avatar_url,
                    avatar_path: None,
                    banner_url: None,
                    banner_path: None,
                    page_url: None,
                    extra,
                })
            }
        }
    }

    async fn fetch_creator_links(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<CreatorProfile>, String> {
        let path = format!(
            "/{}/user/{}/links",
            Self::segment(service),
            Self::segment(creator_id)
        );
        self.get_json(&path, &[]).await.or_else(|_| Ok(Vec::new()))
    }

    async fn fetch_similar_creators(
        &self,
        _service: &str,
        _creator_id: &str,
    ) -> Result<Vec<CreatorProfile>, String> {
        Ok(Vec::new())
    }

    async fn fetch_creator_tags(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<String>, String> {
        let path = format!(
            "/{}/user/{}/tags",
            Self::segment(service),
            Self::segment(creator_id)
        );
        self.get_json(&path, &[]).await.or_else(|_| Ok(Vec::new()))
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
    ) -> Result<Vec<Post>, String> {
        let path = format!(
            "/{}/user/{}/posts",
            Self::segment(service),
            Self::segment(creator_id)
        );
        let mut params = vec![("o", offset.to_string())];
        if let Some(q) = query.map(str::trim).filter(|q| !q.is_empty()) {
            params.push(("q", q.to_string()));
        }

        let resp: CoomerPostsResponse = self.get_json(&path, &params).await?;
        Ok(resp
            .into_posts()
            .into_iter()
            .map(|post| self.prepare_post(post, None))
            .collect())
    }

    async fn fetch_post(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Option<Post>, String> {
        let path = format!(
            "/{}/user/{}/post/{}",
            Self::segment(service),
            Self::segment(creator_id),
            Self::segment(post_id)
        );
        match self.get_json::<CoomerSinglePostResponse>(&path, &[]).await {
            Ok(CoomerSinglePostResponse::Wrapped { post, attachments }) => {
                Ok(Some(self.prepare_post(post, attachments)))
            }
            Ok(CoomerSinglePostResponse::Direct(post)) => Ok(Some(self.prepare_post(post, None))),
            Ok(CoomerSinglePostResponse::Array(posts)) => Ok(posts
                .into_iter()
                .next()
                .map(|post| self.prepare_post(post, None))),
            Err(e) if e.contains("404") => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn fetch_post_revisions(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Vec<PostRevision>, String> {
        let path = format!(
            "/{}/user/{}/post/{}/revisions",
            Self::segment(service),
            Self::segment(creator_id),
            Self::segment(post_id)
        );
        self.get_json(&path, &[]).await.or_else(|_| Ok(Vec::new()))
    }

    async fn fetch_recent_posts(
        &self,
        query: Option<&str>,
        offset: u32,
    ) -> Result<Vec<Post>, String> {
        let mut params = vec![("o", offset.to_string())];
        if let Some(q) = query.map(str::trim).filter(|q| !q.is_empty()) {
            params.push(("q", q.to_string()));
        }
        let resp: CoomerPostsResponse = self.get_json("/posts", &params).await?;
        Ok(resp
            .into_posts()
            .into_iter()
            .map(|post| self.prepare_post(post, None))
            .collect())
    }

    async fn fetch_popular_posts(
        &self,
        period: &str,
        date: Option<&str>,
        offset: u32,
    ) -> Result<Vec<Post>, String> {
        let mut params = vec![("o", offset.to_string())];
        let p_trimmed = period.trim();
        if !p_trimmed.is_empty() && p_trimmed != "none" {
            params.push(("period", p_trimmed.to_string()));
        }
        if let Some(d) = date.map(str::trim).filter(|d| !d.is_empty()) {
            params.push(("date", d.to_string()));
        }
        let res = self
            .get_json::<CoomerPostsResponse>("/posts/popular", &params)
            .await;
        let posts = match res {
            Ok(p) => p.into_posts(),
            Err(_) => self
                .get_json::<CoomerPostsResponse>("/popular", &params)
                .await?
                .into_posts(),
        };
        Ok(posts
            .into_iter()
            .map(|post| self.prepare_post(post, None))
            .collect())
    }

    async fn fetch_post_comments(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Vec<Comment>, String> {
        let path = format!(
            "/{}/user/{}/post/{}/comments",
            Self::segment(service),
            Self::segment(creator_id),
            Self::segment(post_id)
        );
        self.get_json(&path, &[]).await.or_else(|_| Ok(Vec::new()))
    }

    async fn fetch_account_favorites(
        &self,
        _favorite_type: Option<&str>,
    ) -> Result<Vec<Favorite>, String> {
        Ok(Vec::new())
    }

    async fn set_creator_favorite(
        &self,
        _service: &str,
        _creator_id: &str,
        _favorite: bool,
    ) -> Result<ApiActionResult, String> {
        Err("Coomer remote favorites are not supported".to_string())
    }

    async fn set_post_favorite(
        &self,
        _service: &str,
        _creator_id: &str,
        _post_id: &str,
        _favorite: bool,
    ) -> Result<ApiActionResult, String> {
        Err("Coomer remote favorites are not supported".to_string())
    }

    fn resolve_media_url(&self, file_path: &str, server: Option<&str>) -> String {
        let conf = self.config.read().unwrap();
        let clean = file_path
            .trim_start_matches('/')
            .trim_start_matches("data/")
            .trim_start_matches('/');

        let default_prefix = conf.file_prefix.as_deref().unwrap_or("c1");

        if let Some(srv) = server.filter(|s| !s.trim().is_empty()) {
            let srv = srv.trim();
            let base = if srv.starts_with("http://") || srv.starts_with("https://") {
                srv.trim_end_matches('/').to_string()
            } else if srv.contains('.') {
                format!("https://{}", srv.trim_end_matches('/'))
            } else {
                let file_base = conf
                    .file_url
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        super::traits::derive_subdomain_url(&conf.api_url, default_prefix)
                    });
                if let Ok(parsed) = Url::parse(&file_base) {
                    let host = parsed.host_str().unwrap_or("");
                    let parts: Vec<&str> = host.split('.').collect();
                    let base_domain = if parts.len() > 2 {
                        parts[1..].join(".")
                    } else {
                        host.to_string()
                    };
                    format!(
                        "{}://{}.{}",
                        parsed.scheme(),
                        srv.trim_end_matches('/'),
                        base_domain
                    )
                } else {
                    file_base
                }
            };
            return format!("{base}/data/{clean}");
        }

        if let Some(file_url) = conf.file_url.as_deref().filter(|s| !s.trim().is_empty()) {
            let base = if file_url.starts_with("http://") || file_url.starts_with("https://") {
                file_url.trim_end_matches('/').to_string()
            } else {
                format!("https://{}", file_url.trim_end_matches('/'))
            };
            return format!("{base}/data/{clean}");
        }

        let base = super::traits::derive_subdomain_url(&conf.api_url, default_prefix);
        format!("{base}/data/{clean}")
    }

    fn resolve_thumbnail_url(&self, thumb_path: &str) -> String {
        if !path_has_image_mime(thumb_path) {
            return String::new();
        }
        let conf = self.config.read().unwrap();
        let clean = thumb_path
            .trim_start_matches('/')
            .trim_start_matches("data/")
            .trim_start_matches('/');

        let default_prefix = conf.image_prefix.as_deref().unwrap_or("img");

        if let Some(img_url) = conf.image_url.as_deref().filter(|s| !s.trim().is_empty()) {
            let base = if img_url.starts_with("http://") || img_url.starts_with("https://") {
                img_url.trim_end_matches('/').to_string()
            } else {
                format!("https://{}", img_url.trim_end_matches('/'))
            };
            return format!("{base}/thumbnail/data/{clean}");
        }

        let base = super::traits::derive_subdomain_url(&conf.api_url, default_prefix);
        format!("{base}/thumbnail/data/{clean}")
    }

    fn resolve_post_url(&self, service: &str, creator_id: &str, post_id: &str) -> String {
        let endpoint = self.get_active_endpoint();
        let base = endpoint.trim_end_matches('/');
        format!(
            "{base}/{}/user/{}/post/{}",
            Self::segment(service),
            Self::segment(creator_id),
            Self::segment(post_id)
        )
    }

    fn resolve_creator_url(&self, service: &str, creator_id: &str) -> String {
        let endpoint = self.get_active_endpoint();
        let base = endpoint.trim_end_matches('/');
        format!(
            "{base}/{}/user/{}",
            Self::segment(service),
            Self::segment(creator_id)
        )
    }

    fn resolve_avatar_url(&self, service: &str, creator_id: &str) -> String {
        let conf = self.config.read().unwrap();
        let base = conf
            .image_url
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| super::traits::derive_subdomain_url(&conf.api_url, "img"));
        format!(
            "{base}/icons/{}/{}",
            Self::segment(service),
            Self::segment(creator_id)
        )
    }

    fn resolve_banner_url(&self, service: &str, creator_id: &str) -> String {
        let conf = self.config.read().unwrap();
        let base = conf
            .image_url
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| super::traits::derive_subdomain_url(&conf.api_url, "img"));
        format!(
            "{base}/banners/{}/{}",
            Self::segment(service),
            Self::segment(creator_id)
        )
    }

    async fn fetch_creator_artwork_data_url(
        &self,
        service: &str,
        creator_id: &str,
        artwork_type: &str,
    ) -> Result<String, String> {
        let url = if artwork_type == "banner" {
            self.resolve_banner_url(service, creator_id)
        } else {
            self.resolve_avatar_url(service, creator_id)
        };
        let client = self.client.read().unwrap().clone();
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch Coomer artwork: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("Coomer artwork HTTP {}", resp.status()));
        }
        let mime = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_string();
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        Ok(format!(
            "data:{mime};base64,{}",
            base64::prelude::BASE64_STANDARD.encode(bytes)
        ))
    }

    async fn search_hash(&self, file_hash: &str) -> Result<FileSearchResult, String> {
        let path = format!("/search_hash/{}", Self::segment(file_hash));
        self.get_json(&path, &[]).await
    }

    async fn fetch_fancards(
        &self,
        _service: &str,
        _creator_id: &str,
    ) -> Result<Vec<Fancard>, String> {
        Ok(Vec::new())
    }

    async fn flag_post(
        &self,
        _service: &str,
        _creator_id: &str,
        _post_id: &str,
    ) -> Result<ApiActionResult, String> {
        Err("Coomer flag post is not supported".to_string())
    }

    async fn is_post_flagged(
        &self,
        _service: &str,
        _creator_id: &str,
        _post_id: &str,
    ) -> Result<bool, String> {
        Ok(false)
    }

    async fn login(&self, _username: &str, _password: &str) -> Result<String, String> {
        Err("Coomer login via password is not supported".to_string())
    }

    async fn logout(&self) -> Result<(), String> {
        Ok(())
    }

    async fn get_account_session(&self) -> Result<AccountSession, String> {
        Err("Coomer account session is not supported".to_string())
    }

    async fn app_version(&self) -> Result<String, String> {
        let (url, client) = {
            let conf = self.config.read().unwrap();
            let c = self.client.read().unwrap().clone();
            (format!("{}/app_version", Self::base_url(&conf.api_url)), c)
        };
        let resp = self
            .queue
            .send(move || {
                let c = client.clone();
                let u = url.clone();
                async move { c.get(&u).send().await }
            })
            .await?;
        if resp.status().is_success() {
            resp.text().await.map_err(|e| e.to_string())
        } else {
            Err(format!("Coomer API HTTP {}", resp.status()))
        }
    }

    async fn resolve_post_identity(
        &self,
        _service: &str,
        _post_id: &str,
    ) -> Result<Option<(String, String, String)>, String> {
        Ok(None)
    }

    async fn expand_short_link(&self, _raw_url: &str) -> Result<Option<String>, String> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::providers::traits::SourceProvider;

    fn test_config(api_url: String) -> ProviderConfig {
        ProviderConfig {
            id: "coomer".into(),
            name: "Coomer".into(),
            enabled: true,
            api_url,
            fallback_urls: vec![],
            file_url: Some("https://c1.coomer.st".into()),
            image_url: Some("https://img.coomer.st".into()),
            file_prefix: Some("c1".into()),
            image_prefix: Some("img".into()),
            session_cookie: "my_session_cookie".into(),
            username: "".into(),
            services: vec!["onlyfans".into(), "fansly".into()],
            is_custom: false,
            priority: 2,
        }
    }

    #[test]
    fn test_coomer_build_headers_ddg_accept_and_cookie() {
        let conf = test_config("https://coomer.st".into());
        let headers = CoomerProvider::build_headers(&conf);
        assert_eq!(headers.get(ACCEPT).unwrap(), "text/css");
        assert_eq!(
            headers.get(USER_AGENT).unwrap(),
            crate::downloader::PAWSTASH_USER_AGENT
        );
        assert_eq!(headers.get(COOKIE).unwrap(), "session=my_session_cookie");
    }

    #[test]
    fn prepare_post_removes_empty_file_and_uses_wrapper_attachments() {
        let provider = CoomerProvider::new(test_config("https://coomer.st".into())).unwrap();
        let post = Post {
            id: "812407167322693632".into(),
            user: "800702847065796609".into(),
            service: "fansly".into(),
            file: Some(Attachment::default()),
            attachments: Some(vec![Attachment {
                name: Some("video.mp4".into()),
                path: Some("/15/d4/video.mp4".into()),
                ..Attachment::default()
            }]),
            ..Post::default()
        };
        let post = provider.prepare_post(
            post,
            Some(vec![Attachment {
                name: Some("video.mp4".into()),
                path: Some("/15/d4/video.mp4".into()),
                server: Some("https://n1.coomer.st".into()),
                ..Attachment::default()
            }]),
        );

        assert!(post.file.is_none());
        let attachment = &post.attachments.unwrap()[0];
        assert_eq!(attachment.server.as_deref(), Some("https://n1.coomer.st"));
        assert_eq!(
            attachment
                .extra
                .get("provider_id")
                .and_then(|value| value.as_str()),
            Some("coomer")
        );
    }

    #[test]
    fn test_coomer_url_resolution() {
        let conf = test_config("https://coomer.st".into());
        let provider = CoomerProvider::new(conf).unwrap();
        assert_eq!(
            provider.resolve_media_url("/data/aa/bb/video.mp4", None),
            "https://c1.coomer.st/data/aa/bb/video.mp4"
        );
        assert_eq!(
            provider.resolve_media_url("aa/bb/video.mp4", None),
            "https://c1.coomer.st/data/aa/bb/video.mp4"
        );
        assert_eq!(
            provider.resolve_thumbnail_url("/data/aa/bb/thumb.jpg"),
            "https://img.coomer.st/thumbnail/data/aa/bb/thumb.jpg"
        );
        assert!(provider
            .resolve_thumbnail_url("/data/aa/bb/video.mp4")
            .is_empty());
        assert_eq!(
            provider.resolve_media_url("/data/aa/bb/video.mp4", Some("c2")),
            "https://c2.coomer.st/data/aa/bb/video.mp4"
        );
    }

    #[test]
    fn test_coomer_auth_schema_and_rejections() {
        let conf = test_config("https://coomer.st".into());
        let provider = CoomerProvider::new(conf).unwrap();
        let schema = provider.auth_schema();
        assert_eq!(schema.provider_id, "coomer");
        assert!(schema.supports_auth);
        assert!(!schema.supports_remote_favorites);
        assert!(!schema.supports_push_favorites);
    }

    #[tokio::test]
    async fn live_coomer_public_contracts() {
        let conf = ProviderConfig {
            id: "coomer".into(),
            name: "Coomer".into(),
            enabled: true,
            api_url: "https://coomer.st".into(),
            fallback_urls: vec![],
            file_url: Some("https://c1.coomer.st".into()),
            image_url: Some("https://img.coomer.st".into()),
            file_prefix: Some("c1".into()),
            image_prefix: Some("img".into()),
            session_cookie: "".into(),
            username: "".into(),
            services: vec!["onlyfans".into(), "fansly".into()],
            is_custom: false,
            priority: 2,
        };
        let provider = CoomerProvider::new(conf).expect("create CoomerProvider");

        let version = provider.app_version().await.expect("app_version");
        assert!(!version.trim().is_empty(), "Coomer app_version is empty");

        let health = provider.test_connection().await.expect("test_connection");
        assert!(
            health.is_healthy,
            "Coomer health check failed: {:?}",
            health.error
        );

        let creators = provider.fetch_creators().await.expect("fetch_creators");
        assert!(
            !creators.is_empty(),
            "Coomer creators list returned 0 creators"
        );

        let recent = provider
            .fetch_recent_posts(None, 0)
            .await
            .expect("fetch_recent_posts");
        assert!(!recent.is_empty(), "Coomer recent feed returned 0 posts");
        assert_eq!(
            recent[0].extra.get("provider_id").and_then(|v| v.as_str()),
            Some("coomer")
        );

        let paged = provider
            .fetch_recent_posts(None, 50)
            .await
            .expect("fetch_recent_posts paged");
        assert!(!paged.is_empty(), "Coomer offset 50 returned 0 posts");

        let searched = provider
            .fetch_recent_posts(Some("slut"), 0)
            .await
            .expect("fetch_recent_posts query");
        assert!(!searched.is_empty(), "Coomer search returned 0 posts");

        for period in ["day", "week", "month"] {
            let popular = provider
                .fetch_popular_posts(period, None, 0)
                .await
                .expect("fetch_popular_posts");
            assert!(
                !popular.is_empty(),
                "Coomer popular feed ({period}) returned 0 posts"
            );
        }

        let profile = provider
            .fetch_creator_profile("onlyfans", "prettykitttt")
            .await
            .expect("fetch_creator_profile");
        assert_eq!(profile.id, "prettykitttt");
        assert_eq!(profile.service, "onlyfans");

        let links = provider
            .fetch_creator_links("onlyfans", "prettykitttt")
            .await
            .expect("fetch_creator_links");
        let _ = links;

        let tags = provider
            .fetch_creator_tags("onlyfans", "prettykitttt")
            .await
            .expect("fetch_creator_tags");
        let _ = tags;

        let posts = provider
            .fetch_posts("onlyfans", "prettykitttt", 0, None)
            .await
            .expect("fetch_posts");
        assert!(!posts.is_empty(), "Coomer creator posts returned 0 posts");

        let paged_creator_posts = provider
            .fetch_posts("onlyfans", "prettykitttt", 50, None)
            .await
            .expect("fetch_posts paged");
        assert!(
            !paged_creator_posts.is_empty(),
            "Coomer creator offset 50 returned 0 posts"
        );

        let filtered_posts = provider
            .fetch_posts("onlyfans", "prettykitttt", 0, Some("slut"))
            .await
            .expect("fetch_posts with query");
        assert!(
            !filtered_posts.is_empty(),
            "Coomer creator posts with query returned 0 posts"
        );

        let single_post = provider
            .fetch_post("onlyfans", "prettykitttt", "366178580")
            .await
            .expect("fetch_post");
        assert!(single_post.is_some(), "Coomer post 366178580 returned None");
        let post = single_post.unwrap();
        assert_eq!(post.id, "366178580");
        assert_eq!(post.user, "prettykitttt");
        assert_eq!(post.service, "onlyfans");
        assert!(
            post.file.is_some() || post.attachments.as_ref().is_some_and(|a| !a.is_empty()),
            "Coomer post has neither file nor attachments"
        );

        let non_existent = provider
            .fetch_post("onlyfans", "prettykitttt", "999999999999999")
            .await
            .expect("fetch non-existent post");
        assert!(non_existent.is_none(), "Non-existent post returned Some");

        let revisions = provider
            .fetch_post_revisions("onlyfans", "prettykitttt", "366178580")
            .await
            .expect("fetch_post_revisions");
        let _ = revisions;

        let comments = provider
            .fetch_post_comments("onlyfans", "prettykitttt", "366178580")
            .await
            .expect("fetch_post_comments");
        let _ = comments;

        if let Some(file) = &post.file {
            if let Some(path) = &file.path {
                let media_url = provider.resolve_media_url(path, None);
                assert!(media_url.starts_with("https://c1.coomer.st/data/"));
                let thumb_url = provider.resolve_thumbnail_url(path);
                if path_has_image_mime(path) {
                    assert!(thumb_url.starts_with("https://img.coomer.st/thumbnail/data/"));
                } else {
                    assert!(thumb_url.is_empty());
                }
            }
        }
        let custom_srv_url = provider.resolve_media_url("/data/aa/bb/video.mp4", Some("c2"));
        assert_eq!(custom_srv_url, "https://c2.coomer.st/data/aa/bb/video.mp4");
        assert_eq!(provider.id(), "coomer");
        assert_eq!(provider.name(), "Coomer");
        assert!(provider.supports_service("onlyfans"));
        assert!(provider.logout().await.is_ok());
        assert!(provider.get_account_session().await.is_err());
        assert!(provider
            .fetch_fancards("onlyfans", "prettykitttt")
            .await
            .unwrap()
            .is_empty());
        assert!(provider.login("dummy", "secret").await.is_err());
        assert!(provider
            .set_creator_favorite("onlyfans", "prettykitttt", true)
            .await
            .is_err());
        assert!(provider
            .set_post_favorite("onlyfans", "prettykitttt", "366178580", true)
            .await
            .is_err());
    }
}
