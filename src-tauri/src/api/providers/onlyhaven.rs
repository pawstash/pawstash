use super::traits::{
    AuthField, PopularCapabilities, PopularPeriodOption, ProviderAuthSchema, ProviderCapabilities,
    ProviderConfig, ProviderHealth, SortOption, SourceProvider,
};
use crate::api::models::*;
use crate::api::providers::queue::{ProviderQueueConfig, ProviderRequestQueue};
use async_trait::async_trait;
use base64::prelude::*;
use futures_util::future::join_all;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn clean_onlyhaven_title(raw: &str) -> String {
    let s = raw
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ");

    let mut in_tag = false;
    let mut clean_chars = String::with_capacity(s.len());
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
            clean_chars.push(' ');
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            clean_chars.push(c);
        }
    }

    let cleaned = clean_chars
        .replace("**", "")
        .replace("__", "")
        .replace("~~", "");

    let first_line = cleaned.lines().next().unwrap_or("").trim();
    let words: Vec<&str> = first_line.split_whitespace().collect();
    words.join(" ")
}

const DEFAULT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, Deserialize, Default)]
struct OnlyHavenListResponse<T> {
    #[serde(default)]
    #[allow(dead_code)]
    total: Option<u64>,
    #[serde(default)]
    posts: Option<Vec<T>>,
    #[serde(default)]
    creators: Option<Vec<T>>,
    #[serde(default)]
    #[allow(dead_code)]
    dms: Option<Vec<T>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct OnlyHavenCreatorRow {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    service: String,
    #[serde(rename = "displayName", default)]
    display_name: Option<String>,
    #[serde(
        rename = "avatarThumbhash",
        alias = "avatar_thumbhash",
        alias = "avatar",
        alias = "avatar_hash",
        default
    )]
    avatar_thumbhash: Option<String>,
    #[serde(
        rename = "headerThumbhash",
        alias = "header_thumbhash",
        alias = "header",
        alias = "bannerThumbhash",
        alias = "banner_thumbhash",
        alias = "banner",
        alias = "banner_hash",
        default
    )]
    header_thumbhash: Option<String>,
    #[serde(default)]
    indexed: Option<i64>,
    #[serde(default)]
    updated: Option<i64>,
    #[serde(default)]
    bookmarked: Option<u64>,
    #[serde(rename = "postCount", default)]
    post_count: Option<u64>,
    #[serde(rename = "dmCount", default)]
    #[allow(dead_code)]
    dm_count: Option<u64>,
    #[serde(rename = "videoCount", default)]
    video_count: Option<u64>,
    #[serde(rename = "imageCount", default)]
    image_count: Option<u64>,
    #[serde(default)]
    gender: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

impl From<OnlyHavenCreatorRow> for Creator {
    fn from(c: OnlyHavenCreatorRow) -> Self {
        let mut extra = c.extra;
        if let Some(av) = c.avatar_thumbhash {
            extra.insert("avatar_thumbhash".to_string(), Value::String(av));
        }
        if let Some(hd) = c.header_thumbhash {
            extra.insert("header_thumbhash".to_string(), Value::String(hd));
        }
        if let Some(pc) = c.post_count {
            extra.insert("post_count".to_string(), Value::from(pc));
        }
        if let Some(vc) = c.video_count {
            extra.insert("video_count".to_string(), Value::from(vc));
        }
        if let Some(ic) = c.image_count {
            extra.insert("image_count".to_string(), Value::from(ic));
        }
        if let Some(g) = c.gender {
            extra.insert("gender".to_string(), Value::String(g));
        }
        extra.insert(
            "provider_id".to_string(),
            Value::String("onlyhaven".to_string()),
        );

        let name = c.display_name.unwrap_or(c.name);
        Creator {
            id: c.id,
            name,
            service: c.service,
            public_id: None,
            relation_id: None,
            indexed: c.indexed,
            updated: c.updated,
            favorited: c.bookmarked,
            ever_imported: Some(true),
            extra,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
struct OnlyHavenAttachment {
    #[serde(default)]
    locked: Option<bool>,
    #[allow(dead_code)]
    #[serde(default)]
    position: Option<u32>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    sha256: Option<String>,
    #[serde(default, alias = "type", alias = "media_type", alias = "file_type")]
    kind: Option<String>,
    #[serde(
        rename = "mimeType",
        alias = "mime_type",
        alias = "contentType",
        alias = "content_type",
        default
    )]
    mime_type: Option<String>,
    #[serde(default, alias = "size", alias = "file_size")]
    bytes: Option<u64>,
    #[serde(rename = "storageKey", alias = "storage_key", default)]
    storage_key: Option<String>,
    #[serde(
        rename = "originalFilename",
        alias = "original_filename",
        alias = "filename",
        default
    )]
    original_filename: Option<String>,
    #[serde(
        rename = "previewThumbhash",
        alias = "preview_thumbhash",
        alias = "thumbhash",
        default
    )]
    preview_thumbhash: Option<String>,
    #[serde(default)]
    variants: Option<Vec<Value>>,
    #[serde(default, alias = "url")]
    path: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct OnlyHavenPostRow {
    #[serde(default)]
    id: String,
    #[serde(default)]
    service: String,
    #[serde(
        rename = "creatorId",
        alias = "creator_id",
        alias = "user",
        alias = "userId",
        alias = "user_id",
        alias = "author_id",
        default
    )]
    creator_id: Option<String>,
    #[serde(
        rename = "creatorName",
        alias = "creator_name",
        alias = "username",
        alias = "user_name",
        alias = "author",
        alias = "author_name",
        alias = "name",
        default
    )]
    creator_name: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(rename = "captionHtml", default)]
    caption_html: Option<String>,
    #[serde(default)]
    caption: Option<String>,
    #[serde(default)]
    added: Option<i64>,
    #[serde(default)]
    published: Option<i64>,
    #[serde(
        default,
        alias = "favorites",
        alias = "favs",
        alias = "fav_count",
        alias = "favCount",
        alias = "likes",
        alias = "likeCount",
        alias = "like_count",
        alias = "favoriteCount",
        alias = "favorite_count",
        alias = "bookmarks",
        alias = "bookmark_count"
    )]
    bookmarked: Option<u64>,
    #[serde(default)]
    score: Option<u64>,
    #[serde(default)]
    attachments: Option<Vec<OnlyHavenAttachment>>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum OnlyHavenSinglePostResponse {
    Wrapped {
        post: OnlyHavenPostRow,
        #[serde(default)]
        attachments: Option<Vec<OnlyHavenAttachment>>,
    },
    Array(Vec<OnlyHavenPostRow>),
    Direct(OnlyHavenPostRow),
}

impl OnlyHavenPostRow {
    fn into_post(self, default_user: &str, provider_id: &str) -> Post {
        let user = self.creator_id.unwrap_or_else(|| default_user.to_string());
        let raw_attachments = self.attachments.unwrap_or_default();
        let total_raw = raw_attachments.len();
        let locked_count = raw_attachments
            .iter()
            .filter(|a| a.locked.unwrap_or(false))
            .count();
        let is_locked = locked_count > 0 && (locked_count == total_raw);
        let locked_thumbhash = raw_attachments
            .iter()
            .find_map(|a| a.preview_thumbhash.clone());

        let attachments: Vec<Attachment> = raw_attachments
            .into_iter()
            .map(|a| {
                let is_att_locked = a.locked.unwrap_or(false);
                let storage_key = a
                    .storage_key
                    .clone()
                    .or_else(|| a.sha256.clone())
                    .or_else(|| a.path.clone())
                    .or_else(|| a.id.clone());
                let clean_path = storage_key.as_deref().map(|p| {
                    let c = p
                        .trim_start_matches('/')
                        .trim_start_matches("data/")
                        .trim_start_matches('/');
                    format!("/{c}")
                });

                let variant_name = a
                    .variants
                    .as_ref()
                    .and_then(|v| v.first())
                    .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(String::from));

                let mut extra = HashMap::new();
                if is_att_locked {
                    extra.insert("locked".to_string(), Value::Bool(true));
                }
                if let Some(th) = a.preview_thumbhash {
                    extra.insert("preview_thumbhash".to_string(), Value::String(th));
                }
                if let Some(ref sk) = storage_key {
                    extra.insert("storage_key".to_string(), Value::String(sk.clone()));
                }
                if let Some(k) = a.kind {
                    extra.insert("kind".to_string(), Value::String(k));
                }
                if let Some(m) = a.mime_type {
                    extra.insert("mime_type".to_string(), Value::String(m));
                }
                if let Some(v) = a.variants {
                    extra.insert("variants".to_string(), Value::Array(v));
                }
                extra.insert(
                    "provider_id".to_string(),
                    Value::String(provider_id.to_string()),
                );

                Attachment {
                    name: a.original_filename.or(a.name).or(variant_name),
                    path: clean_path,
                    server: None,
                    size: a.bytes,
                    extra,
                }
            })
            .collect();

        let file = attachments
            .iter()
            .find(|a| {
                !a.extra
                    .get("locked")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                    && a.path.is_some()
            })
            .cloned()
            .or_else(|| attachments.first().cloned());
        let att_count = attachments.len() as u64;
        let published_str = self.published.map(|ts| ts.to_string());
        let added_str = self.added.map(|ts| ts.to_string());
        let raw_caption = self.caption.clone().or_else(|| self.caption_html.clone());

        let title = if let Some(t) = self.title.filter(|t| !t.trim().is_empty()) {
            let cl = clean_onlyhaven_title(&t);
            if cl.is_empty() {
                format!("{} post #{}", self.service, self.id)
            } else {
                cl
            }
        } else if let Some(ref c) = raw_caption {
            let cl = clean_onlyhaven_title(c);
            if cl.is_empty() {
                format!("{} post #{}", self.service, self.id)
            } else if cl.chars().count() > 70 {
                format!("{}...", cl.chars().take(67).collect::<String>())
            } else {
                cl
            }
        } else {
            format!("{} post #{}", self.service, self.id)
        };

        let mut extra = self.extra;
        if let Some(ref cn) = self.creator_name {
            extra.insert("creator_name".to_string(), Value::String(cn.clone()));
            extra.insert("creatorName".to_string(), Value::String(cn.clone()));
            extra.insert("username".to_string(), Value::String(cn.clone()));
        }
        if is_locked {
            extra.insert("is_locked".to_string(), Value::Bool(true));
        }
        if locked_count > 0 {
            extra.insert(
                "locked_attachments_count".to_string(),
                Value::Number(locked_count.into()),
            );
        }
        if let Some(th) = locked_thumbhash {
            if !extra.contains_key("preview_thumbhash") {
                extra.insert("preview_thumbhash".to_string(), Value::String(th));
            }
        }
        extra.insert(
            "provider_id".to_string(),
            Value::String(provider_id.to_string()),
        );

        let prev = extra.remove("prev").and_then(|v| match v {
            Value::String(s) => Some(s),
            Value::Number(n) => Some(n.to_string()),
            Value::Object(map) => map.get("id").and_then(|id| {
                if let Some(s) = id.as_str() {
                    Some(s.to_string())
                } else {
                    id.as_i64().map(|n| n.to_string())
                }
            }),
            _ => None,
        });

        let next = extra.remove("next").and_then(|v| match v {
            Value::String(s) => Some(s),
            Value::Number(n) => Some(n.to_string()),
            Value::Object(map) => map.get("id").and_then(|id| {
                if let Some(s) = id.as_str() {
                    Some(s.to_string())
                } else {
                    id.as_i64().map(|n| n.to_string())
                }
            }),
            _ => None,
        });

        let post_prev = next;
        let post_next = prev;

        let mut post = Post {
            id: self.id,
            user,
            service: self.service,
            title,
            content: raw_caption,
            substring: None,
            published: published_str,
            added: added_str,
            edited: None,
            embed: None,
            shared_file: None,
            attachments: Some(attachments),
            file,
            poll: None,
            captions: None,
            tags: None,
            origin: None,
            preview_state: None,
            has_full: Some(true),
            detail_fetched: Some(false),
            next: post_next,
            prev: post_prev,
            favorite_count: self.bookmarked.or(self.score),
            attachment_count: Some(att_count),
            extra,
        };
        post.clean_extra();
        post
    }
}

pub struct OnlyHavenProvider {
    config: Arc<RwLock<ProviderConfig>>,
    client: Client,
    current_mirror_idx: AtomicUsize,
    queue: Arc<ProviderRequestQueue>,
}

impl OnlyHavenProvider {
    pub fn default_services() -> Vec<String> {
        vec!["onlyfans".into(), "fansly".into()]
    }

    pub fn default_queue_config() -> ProviderQueueConfig {
        ProviderQueueConfig {
            max_concurrent: 3,
            min_interval: Duration::from_millis(100),
            max_retries: 3,
            default_retry_after: Duration::from_millis(1500),
            max_cooldown: Duration::from_secs(10),
        }
    }

    pub fn default_config() -> ProviderConfig {
        ProviderConfig {
            id: "onlyhaven".into(),
            name: "OnlyHaven".into(),
            enabled: false,
            api_url: "https://cum.st".into(),
            fallback_urls: vec![],
            file_url: Some("https://e1.cum.st".into()),
            image_url: Some("https://img.cum.st".into()),
            file_prefix: Some("e1".into()),
            image_prefix: Some("img".into()),
            session_cookie: String::new(),
            username: String::new(),
            services: Self::default_services(),
            is_custom: false,
            priority: 3,
        }
    }

    pub fn new(config: ProviderConfig) -> Result<Self, String> {
        Self::with_queue_config(config, Self::default_queue_config())
    }

    pub fn with_queue_config(
        config: ProviderConfig,
        queue_config: ProviderQueueConfig,
    ) -> Result<Self, String> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(crate::downloader::PAWSTASH_USER_AGENT),
        );

        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .gzip(true)
            .default_headers(headers)
            .build()
            .map_err(|e| format!("Failed to build HTTP client for OnlyHaven: {e}"))?;

        let queue = Arc::new(ProviderRequestQueue::new("onlyhaven", queue_config));

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            client,
            current_mirror_idx: AtomicUsize::new(0),
            queue,
        })
    }

    pub fn queue(&self) -> Arc<ProviderRequestQueue> {
        Arc::clone(&self.queue)
    }

    fn get_endpoints(&self) -> Vec<String> {
        let conf = self.config.read().unwrap();
        let mut endpoints = Vec::new();
        let primary = conf.api_url.trim().trim_end_matches('/').to_string();
        if !primary.is_empty() {
            endpoints.push(primary);
        }
        for fb in &conf.fallback_urls {
            let cleaned = fb.trim().trim_end_matches('/').to_string();
            if !cleaned.is_empty() && !endpoints.contains(&cleaned) {
                endpoints.push(cleaned);
            }
        }
        if endpoints.is_empty() {
            let primary = self.config.read().unwrap().api_url.clone();
            if !primary.trim().is_empty() {
                endpoints.push(primary.trim().trim_end_matches('/').to_string());
            }
        }
        endpoints
    }

    async fn request<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let endpoints = self.get_endpoints();
        let start_idx = self.current_mirror_idx.load(Ordering::Relaxed) % endpoints.len();

        let cookie = {
            let conf = self.config.read().unwrap();
            conf.session_cookie.clone()
        };

        let mut last_err = String::new();
        for i in 0..endpoints.len() {
            let idx = (start_idx + i) % endpoints.len();
            let base = &endpoints[idx];
            let url = format!("{base}{path}");

            let client = self.client.clone();
            let cookie_for_req = cookie.clone();
            let url_for_req = url.clone();

            let resp_result = self
                .queue
                .send_request(move || {
                    let mut req = client.get(&url_for_req);
                    if !cookie_for_req.trim().is_empty() {
                        let header_val = if cookie_for_req.contains('=') {
                            cookie_for_req.clone()
                        } else {
                            format!("__Secure-oh.session_token={cookie_for_req}")
                        };
                        if let Ok(val) = reqwest::header::HeaderValue::from_str(&header_val) {
                            req = req.header(reqwest::header::COOKIE, val);
                        }
                    }
                    req
                })
                .await;

            match resp_result {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        self.current_mirror_idx.store(idx, Ordering::Relaxed);
                        match resp.text().await {
                            Ok(body_str) => match serde_json::from_str::<T>(&body_str) {
                                Ok(parsed) => return Ok(parsed),
                                Err(e) => {
                                    last_err = format!("Failed to parse response from {url}: {e}");
                                }
                            },
                            Err(e) => {
                                last_err = format!("Failed to read response body from {url}: {e}");
                            }
                        }
                    } else if status.as_u16() == 404 {
                        return Err(format!("Not found: {url} (HTTP 404)"));
                    } else {
                        last_err = format!("HTTP {status} from {url}");
                    }
                }
                Err(e) => {
                    last_err = format!("Network error on {url}: {e}");
                }
            }
        }

        Err(format!(
            "OnlyHaven provider failed across all mirrors. Last error: {last_err}"
        ))
    }
}

#[async_trait]
impl SourceProvider for OnlyHavenProvider {
    fn id(&self) -> &str {
        "onlyhaven"
    }

    fn name(&self) -> &str {
        "OnlyHaven"
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
        let endpoints = self.get_endpoints();
        let idx = self.current_mirror_idx.load(Ordering::Relaxed) % endpoints.len();
        endpoints[idx].clone()
    }

    fn request_queue(&self) -> Option<Arc<ProviderRequestQueue>> {
        Some(self.queue.clone())
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
                    is_server_side: true,
                },
                SortOption {
                    id: "updated".to_string(),
                    label_key: "creators.sort_updated".to_string(),
                    is_server_side: true,
                },
                SortOption {
                    id: "indexed".to_string(),
                    label_key: "creators.sort_indexed".to_string(),
                    is_server_side: true,
                },
                SortOption {
                    id: "name".to_string(),
                    label_key: "creators.sort_name".to_string(),
                    is_server_side: true,
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
            supports_date_filter: false,
            supports_hash_search: false,
            supports_announcements: false,
            supports_fancards: false,
            supports_similar_creators: false,
            supports_creator_tags: false,
        }
    }

    async fn test_connection(&self) -> Result<ProviderHealth, String> {
        let endpoint = self.get_active_endpoint();
        let start = Instant::now();
        let url = format!("{endpoint}/api/v1/creators?n=1");
        let now_str = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();

        let client = self.client.clone();
        let url_clone = url.clone();
        match self
            .queue
            .send_request(move || client.get(&url_clone))
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let latency_ms = start.elapsed().as_millis() as u64;
                Ok(ProviderHealth {
                    provider_id: self.id().to_string(),
                    active_endpoint: endpoint,
                    is_healthy: true,
                    latency_ms,
                    error: None,
                    last_checked_at: now_str,
                })
            }
            Ok(resp) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                Ok(ProviderHealth {
                    provider_id: self.id().to_string(),
                    active_endpoint: endpoint,
                    is_healthy: false,
                    latency_ms,
                    error: Some(format!("HTTP {}", resp.status())),
                    last_checked_at: now_str,
                })
            }
            Err(e) => Ok(ProviderHealth {
                provider_id: self.id().to_string(),
                active_endpoint: endpoint,
                is_healthy: false,
                latency_ms: 0,
                error: Some(e.to_string()),
                last_checked_at: now_str,
            }),
        }
    }

    async fn update_config(&self, config: ProviderConfig) -> Result<(), String> {
        *self.config.write().unwrap() = config;
        Ok(())
    }

    async fn fetch_creators(&self) -> Result<Vec<Creator>, String> {
        let mut all_rows: Vec<OnlyHavenCreatorRow> = Vec::new();
        let chunk_size = 10;
        let max_pages = 30; // Up to 1,500 creators concurrently fetched

        for batch_start in (0..max_pages).step_by(chunk_size) {
            let tasks: Vec<_> = (batch_start..std::cmp::min(batch_start + chunk_size, max_pages))
                .map(|page| {
                    let offset = page * 50;
                    let path = format!("/api/v1/creators?n=50&o={offset}");
                    async move {
                        let res: Result<OnlyHavenListResponse<OnlyHavenCreatorRow>, String> =
                            self.request(&path).await;
                        res.map(|r| r.creators.unwrap_or_default())
                    }
                })
                .collect();

            let results: Vec<Result<Vec<OnlyHavenCreatorRow>, String>> = join_all(tasks).await;
            let mut stop = false;
            for res in results {
                match res {
                    Ok(rows) => {
                        if rows.len() < 50 {
                            stop = true;
                        }
                        all_rows.extend(rows);
                    }
                    Err(_) => {
                        stop = true;
                    }
                }
            }
            if stop || all_rows.is_empty() {
                break;
            }
        }

        let mut seen = std::collections::HashSet::new();
        let mut creators = Vec::new();
        for row in all_rows {
            let key = format!("{}:{}", row.service.to_lowercase(), row.id.to_lowercase());
            if seen.insert(key) {
                creators.push(Creator::from(row));
            }
        }
        Ok(creators)
    }

    async fn fetch_creator_profile(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Creator, String> {
        let path = format!(
            "/api/v1/{}/user/{}/profile",
            urlencoding::encode(service),
            urlencoding::encode(creator_id)
        );
        let row: OnlyHavenCreatorRow = self.request(&path).await?;
        Ok(Creator::from(row))
    }

    async fn fetch_creator_links(
        &self,
        _service: &str,
        _creator_id: &str,
    ) -> Result<Vec<CreatorProfile>, String> {
        Ok(Vec::new())
    }

    async fn fetch_posts(
        &self,
        service: &str,
        creator_id: &str,
        offset: u32,
        query: Option<&str>,
    ) -> Result<Vec<Post>, String> {
        let mut path = format!(
            "/api/v1/{}/user/{}/posts?n=50&o={offset}",
            urlencoding::encode(service),
            urlencoding::encode(creator_id)
        );
        if let Some(q) = query.filter(|q| !q.trim().is_empty()) {
            path.push_str(&format!("&q={}", urlencoding::encode(q.trim())));
        }

        let res: OnlyHavenListResponse<OnlyHavenPostRow> = self.request(&path).await?;
        let rows = res.posts.unwrap_or_default();
        let provider_id = self.id().to_string();
        Ok(rows
            .into_iter()
            .map(|r| r.into_post(creator_id, &provider_id))
            .collect())
    }

    async fn fetch_post(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Option<Post>, String> {
        let path = format!(
            "/api/v1/{}/user/{}/post/{}",
            urlencoding::encode(service),
            urlencoding::encode(creator_id),
            urlencoding::encode(post_id)
        );

        match self.request::<OnlyHavenSinglePostResponse>(&path).await {
            Ok(OnlyHavenSinglePostResponse::Wrapped {
                mut post,
                attachments,
            }) => {
                if post.attachments.is_none() && attachments.is_some() {
                    post.attachments = attachments;
                }
                Ok(Some(post.into_post(creator_id, self.id())))
            }
            Ok(OnlyHavenSinglePostResponse::Array(rows)) => Ok(rows
                .into_iter()
                .next()
                .map(|r| r.into_post(creator_id, self.id()))),
            Ok(OnlyHavenSinglePostResponse::Direct(row)) => {
                Ok(Some(row.into_post(creator_id, self.id())))
            }
            Err(e) if e.contains("404") => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn fetch_post_revisions(
        &self,
        _service: &str,
        _creator_id: &str,
        _post_id: &str,
    ) -> Result<Vec<PostRevision>, String> {
        Ok(Vec::new())
    }

    async fn fetch_recent_posts(
        &self,
        query: Option<&str>,
        offset: u32,
    ) -> Result<Vec<Post>, String> {
        let mut path = format!("/api/v1/posts?n=50&o={offset}");
        if let Some(q) = query.filter(|q| !q.trim().is_empty()) {
            path.push_str(&format!("&q={}", urlencoding::encode(q.trim())));
        }

        let res: OnlyHavenListResponse<OnlyHavenPostRow> = self.request(&path).await?;
        let rows = res.posts.unwrap_or_default();
        let provider_id = self.id().to_string();
        Ok(rows
            .into_iter()
            .map(|r| r.into_post("", &provider_id))
            .collect())
    }

    async fn fetch_popular_posts(
        &self,
        period: &str,
        date: Option<&str>,
        offset: u32,
    ) -> Result<Vec<Post>, String> {
        let mut params = vec![format!("n=50&o={offset}")];
        let p_trimmed = period.trim();
        if !p_trimmed.is_empty() && p_trimmed != "none" {
            params.push(format!("period={p_trimmed}"));
        }
        if let Some(d) = date.map(str::trim).filter(|d| !d.is_empty()) {
            let normalized_date = if d.len() == 7 && d.chars().nth(4) == Some('-') {
                format!("{d}-01")
            } else if d.len() == 6 && d.chars().all(|c| c.is_ascii_digit()) {
                format!("{}-{}-01", &d[..4], &d[4..6])
            } else {
                d.to_string()
            };
            params.push(format!("date={normalized_date}"));
        }
        let query_str = params.join("&");
        let path = format!("/api/v1/posts/popular?{query_str}");
        let res: OnlyHavenListResponse<OnlyHavenPostRow> = match self.request(&path).await {
            Ok(r) => r,
            Err(err) => {
                if err.contains("404") {
                    tracing::warn!("OnlyHaven /posts/popular 404: {err}; falling back to /posts?sort=popular");
                    let fallback_path = format!("/api/v1/posts?n=50&o={offset}&sort=popular");
                    self.request(&fallback_path).await?
                } else {
                    return Err(err);
                }
            }
        };
        let rows = res.posts.unwrap_or_default();
        let provider_id = self.id().to_string();
        Ok(rows
            .into_iter()
            .map(|r| r.into_post("", &provider_id))
            .collect())
    }

    async fn fetch_post_comments(
        &self,
        _service: &str,
        _creator_id: &str,
        _post_id: &str,
    ) -> Result<Vec<Comment>, String> {
        Ok(Vec::new())
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
        Ok(ApiActionResult {
            status: 200,
            success: true,
        })
    }

    async fn set_post_favorite(
        &self,
        _service: &str,
        _creator_id: &str,
        _post_id: &str,
        _favorite: bool,
    ) -> Result<ApiActionResult, String> {
        Ok(ApiActionResult {
            status: 200,
            success: true,
        })
    }

    fn resolve_media_url(&self, file_path: &str, _server: Option<&str>) -> String {
        let conf = self.config.read().unwrap();
        let key = file_path
            .trim_start_matches('/')
            .trim_start_matches("data/")
            .trim_start_matches('/');
        let base = conf
            .file_url
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| super::traits::derive_subdomain_url(&conf.api_url, "e1"));

        let path = if key.starts_with("media/") {
            key.to_string()
        } else {
            format!("media/{key}")
        };

        if key.contains('.') {
            format!("{base}/{path}")
        } else {
            format!("{base}/{path}/original.jpg")
        }
    }

    fn resolve_thumbnail_url(&self, thumb_path: &str) -> String {
        let conf = self.config.read().unwrap();
        let key = thumb_path
            .trim_start_matches('/')
            .trim_start_matches("data/")
            .trim_start_matches('/');
        let base = conf
            .image_url
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| super::traits::derive_subdomain_url(&conf.api_url, "img"));
        format!("{base}/thumbnail/{key}/preview.webp")
    }

    fn resolve_post_url(&self, service: &str, creator_id: &str, post_id: &str) -> String {
        let endpoint = self.get_active_endpoint();
        let base = endpoint.trim_end_matches('/');
        let s = urlencoding::encode(service);
        let c = urlencoding::encode(creator_id);
        let p = urlencoding::encode(post_id);
        format!("{base}/creators/{s}/{c}/post/{p}")
    }

    fn resolve_creator_url(&self, service: &str, creator_id: &str) -> String {
        let endpoint = self.get_active_endpoint();
        let base = endpoint.trim_end_matches('/');
        let s = urlencoding::encode(service);
        let c = urlencoding::encode(creator_id);
        format!("{base}/creators/{s}/{c}")
    }

    async fn fetch_creator_artwork_data_url(
        &self,
        service: &str,
        creator_id: &str,
        artwork_type: &str,
    ) -> Result<String, String> {
        let file_name = match artwork_type {
            "banner" => "header.webp",
            "avatar" => "avatar.webp",
            _ => return Err("Unsupported creator artwork kind".to_string()),
        };

        let img_base = {
            let conf = self.config.read().unwrap();
            conf.image_url
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.trim_end_matches('/').to_string())
                .unwrap_or_else(|| super::traits::derive_subdomain_url(&conf.api_url, "img"))
        };

        let candidate_urls = vec![format!(
            "{img_base}/creator/{service}/{creator_id}/{file_name}"
        )];

        for url in candidate_urls {
            let client = self.client.clone();
            let u = url.clone();
            if let Ok(resp) = self
                .queue
                .send_request(move || {
                    client
                        .get(&u)
                        .header(USER_AGENT, crate::downloader::PAWSTASH_USER_AGENT)
                })
                .await
            {
                if resp.status().is_success() {
                    let content_type = resp
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.split(';').next())
                        .filter(|v| v.starts_with("image/"))
                        .unwrap_or("image/webp")
                        .to_string();
                    if let Ok(bytes) = resp.bytes().await {
                        if !bytes.is_empty() && bytes.len() <= 8 * 1024 * 1024 {
                            return Ok(format!(
                                "data:{content_type};base64,{}",
                                BASE64_STANDARD.encode(&bytes)
                            ));
                        }
                    }
                }
            }
        }

        Err("Failed to fetch OnlyHaven artwork".to_string())
    }

    async fn search_hash(&self, _file_hash: &str) -> Result<FileSearchResult, String> {
        Err("OnlyHaven hash search not supported".to_string())
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
        Ok(ApiActionResult {
            status: 200,
            success: true,
        })
    }

    async fn is_post_flagged(
        &self,
        _service: &str,
        _creator_id: &str,
        _post_id: &str,
    ) -> Result<bool, String> {
        Ok(false)
    }

    fn auth_schema(&self) -> ProviderAuthSchema {
        let base = self.get_active_endpoint();
        let clean_base = base.trim_end_matches('/');
        let help_url = if clean_base.is_empty() {
            None
        } else {
            Some(format!("{clean_base}/auth/signin"))
        };

        ProviderAuthSchema {
            provider_id: self.id().to_string(),
            supports_auth: true,
            supports_remote_favorites: false,
            supports_push_favorites: false,
            auth_fields: vec![
                AuthField {
                    key: "username".to_string(),
                    label_key: "settings.username".to_string(),
                    field_type: "text".to_string(),
                    placeholder: Some("Username".to_string()),
                    help_text_key: None,
                    required: true,
                },
                AuthField {
                    key: "password".to_string(),
                    label_key: "settings.password".to_string(),
                    field_type: "password".to_string(),
                    placeholder: Some("••••••••".to_string()),
                    help_text_key: None,
                    required: true,
                },
            ],
            help_url,
        }
    }

    async fn login(&self, username: &str, password: &str) -> Result<String, String> {
        let base = self.get_active_endpoint();
        let clean_base = base.trim_end_matches('/');
        let url = format!("{clean_base}/api/auth/sign-in/username");

        let payload = serde_json::json!({
            "username": username.trim(),
            "password": password,
            "callbackURL": "/"
        });

        let origin = if clean_base.starts_with("http://") || clean_base.starts_with("https://") {
            clean_base.to_string()
        } else {
            format!("https://{clean_base}")
        };

        let client = self.client.clone();
        let u = url.clone();
        let p = payload.clone();
        let o = origin.clone();
        let r = format!("{origin}/auth/signin");
        let resp = self
            .queue
            .send_request(move || {
                client
                    .post(&u)
                    .header("Content-Type", "application/json")
                    .header("Origin", &o)
                    .header("Referer", &r)
                    .json(&p)
            })
            .await
            .map_err(|e| format!("Network error connecting to OnlyHaven auth: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let err_json: Value = resp.json().await.unwrap_or_default();
            let msg = err_json
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Invalid username or password");
            return Err(msg.to_string());
        }

        let mut session_cookie = String::new();
        for (name, val) in resp.headers().iter() {
            if name.as_str().eq_ignore_ascii_case("set-cookie") {
                if let Ok(cookie_str) = val.to_str() {
                    if cookie_str.contains("__Secure-oh.session_token")
                        || cookie_str.contains("oh.session_token")
                    {
                        let cookie_part = cookie_str.split(';').next().unwrap_or(cookie_str).trim();
                        session_cookie = cookie_part.to_string();
                        break;
                    }
                }
            }
        }

        let body_json: Value = resp.json().await.unwrap_or_default();
        let returned_user = body_json
            .get("user")
            .and_then(|u| u.get("username"))
            .and_then(|u| u.as_str())
            .unwrap_or(username.trim())
            .to_string();

        if session_cookie.is_empty() {
            if let Some(token) = body_json.get("token").and_then(|t| t.as_str()) {
                session_cookie = format!("__Secure-oh.session_token={token}");
            }
        }

        if session_cookie.is_empty() {
            return Err(
                "Login succeeded but no session token was received from server".to_string(),
            );
        }

        {
            let mut conf = self.config.write().unwrap();
            conf.session_cookie = session_cookie.clone();
            conf.username = returned_user.clone();
        }

        Ok(session_cookie)
    }

    async fn logout(&self) -> Result<(), String> {
        let base = self.get_active_endpoint();
        let clean_base = base.trim_end_matches('/');
        let url = format!("{clean_base}/api/auth/sign-out");
        let cookie = {
            let conf = self.config.read().unwrap();
            conf.session_cookie.clone()
        };

        if !cookie.is_empty() {
            let origin = if clean_base.starts_with("http://") || clean_base.starts_with("https://")
            {
                clean_base.to_string()
            } else {
                format!("https://{clean_base}")
            };
            let client = self.client.clone();
            let u = url.clone();
            let o = origin.clone();
            let r = format!("{origin}/");
            let c = cookie.clone();
            let _ = self
                .queue
                .send_request(move || {
                    let mut req = client
                        .post(&u)
                        .header("Origin", &o)
                        .header("Referer", &r)
                        .json(&serde_json::json!({}));
                    if let Ok(val) = reqwest::header::HeaderValue::from_str(&c) {
                        req = req.header(reqwest::header::COOKIE, val);
                    }
                    req
                })
                .await;
        }

        {
            let mut conf = self.config.write().unwrap();
            conf.session_cookie.clear();
            conf.username.clear();
        }
        Ok(())
    }

    async fn get_account_session(&self) -> Result<AccountSession, String> {
        let conf = self.config.read().unwrap();
        if conf.session_cookie.trim().is_empty() {
            return Ok(AccountSession {
                authenticated: false,
                username: None,
            });
        }
        Ok(AccountSession {
            authenticated: true,
            username: if conf.username.trim().is_empty() {
                None
            } else {
                Some(conf.username.clone())
            },
        })
    }

    async fn app_version(&self) -> Result<String, String> {
        Ok(env!("CARGO_PKG_VERSION").to_string())
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

    #[test]
    fn test_clean_onlyhaven_title() {
        assert_eq!(
            clean_onlyhaven_title("<p>Hello <b>World</b> &amp; Friends</p>"),
            "Hello World & Friends"
        );
        assert_eq!(
            clean_onlyhaven_title("**Bold** __Underline__ ~~Strike~~"),
            "Bold Underline Strike"
        );
        assert_eq!(clean_onlyhaven_title(""), "");
    }

    #[test]
    fn test_onlyhaven_url_resolution() {
        let provider = OnlyHavenProvider::new(ProviderConfig {
            id: "onlyhaven".to_string(),
            name: "OnlyHaven".to_string(),
            enabled: true,
            priority: 1,
            api_url: "https://cum.st".to_string(),
            file_url: Some("https://e1.cum.st".to_string()),
            image_url: Some("https://img.cum.st".to_string()),
            fallback_urls: vec![],
            session_cookie: "".to_string(),
            username: "".to_string(),
            services: vec!["onlyfans".to_string()],
            file_prefix: None,
            image_prefix: None,
            is_custom: false,
        })
        .unwrap();

        assert_eq!(
            provider.resolve_media_url("7426b2f88640e8807ec0f23a00e9702eb99ff2fd51913d6b27be12887e295fe2", None),
            "https://e1.cum.st/media/7426b2f88640e8807ec0f23a00e9702eb99ff2fd51913d6b27be12887e295fe2/original.jpg"
        );
        assert_eq!(
            provider.resolve_media_url("7426b2f88640e8807ec0f23a00e9702eb99ff2fd51913d6b27be12887e295fe2/original.mp4", None),
            "https://e1.cum.st/media/7426b2f88640e8807ec0f23a00e9702eb99ff2fd51913d6b27be12887e295fe2/original.mp4"
        );
        assert_eq!(
            provider.resolve_thumbnail_url("7426b2f88640e8807ec0f23a00e9702eb99ff2fd51913d6b27be12887e295fe2"),
            "https://img.cum.st/thumbnail/7426b2f88640e8807ec0f23a00e9702eb99ff2fd51913d6b27be12887e295fe2/preview.webp"
        );
    }

    #[test]
    fn test_onlyhaven_prev_next_normalization() {
        let raw_json = r#"{
            "id": "950597293931769856",
            "service": "fansly",
            "creatorId": "665110462043533312",
            "title": null,
            "caption": "💙",
            "prev": { "id": "949880048393932800" },
            "next": null
        }"#;
        let row: OnlyHavenPostRow = serde_json::from_str(raw_json).unwrap();
        let post = row.into_post("665110462043533312", "onlyhaven");
        assert_eq!(post.prev, None);
        assert_eq!(post.next.as_deref(), Some("949880048393932800"));
    }

    #[test]
    fn test_onlyhaven_locked_attachments_preserved() {
        let raw_json = r#"{
            "id": "917213282249502720",
            "service": "fansly",
            "creatorId": "717101518569873409",
            "title": null,
            "caption": "Audio RP",
            "attachments": [
                {
                    "locked": true,
                    "position": 0,
                    "externalId": "917212607612474548",
                    "type": "audio",
                    "priceCents": 15000
                }
            ]
        }"#;
        let row: OnlyHavenPostRow = serde_json::from_str(raw_json).unwrap();
        let post = row.into_post("717101518569873409", "onlyhaven");
        let attachments = post.attachments.expect("attachments should be present");
        assert_eq!(attachments.len(), 1);
        let att = &attachments[0];
        assert_eq!(att.extra.get("locked").and_then(Value::as_bool), Some(true));
        assert_eq!(att.extra.get("kind").and_then(Value::as_str), Some("audio"));
        assert!(post.file.is_some());
    }

    fn test_oh_config(api_url: String) -> ProviderConfig {
        ProviderConfig {
            id: "onlyhaven".to_string(),
            name: "OnlyHaven".to_string(),
            enabled: true,
            priority: 1,
            api_url,
            file_url: Some("https://e1.cum.st".to_string()),
            image_url: Some("https://img.cum.st".to_string()),
            fallback_urls: vec![],
            session_cookie: "".to_string(),
            username: "".to_string(),
            services: vec!["fansly".to_string(), "onlyfans".to_string()],
            file_prefix: None,
            image_prefix: None,
            is_custom: false,
        }
    }

    #[test]
    fn test_onlyhaven_auth_schema() {
        let conf = test_oh_config("https://cum.st".into());
        let provider = OnlyHavenProvider::new(conf).unwrap();
        let schema = provider.auth_schema();
        assert_eq!(schema.provider_id, "onlyhaven");
        assert!(schema.supports_auth);
        assert!(!schema.supports_remote_favorites);
        assert!(!schema.supports_push_favorites);
    }

    #[tokio::test]
    async fn live_onlyhaven_public_contracts() {
        let conf = ProviderConfig {
            id: "onlyhaven".into(),
            name: "OnlyHaven".into(),
            enabled: true,
            api_url: "https://cum.st".into(),
            fallback_urls: vec![],
            file_url: Some("https://e1.cum.st".into()),
            image_url: Some("https://img.cum.st".into()),
            file_prefix: None,
            image_prefix: None,
            session_cookie: "".into(),
            username: "".into(),
            services: vec!["onlyfans".into(), "fansly".into()],
            is_custom: false,
            priority: 1,
        };
        let provider = OnlyHavenProvider::new(conf).expect("create OnlyHavenProvider");

        let health = provider.test_connection().await.expect("test_connection");
        assert!(
            health.is_healthy,
            "OnlyHaven health check failed: {:?}",
            health.error
        );

        let creators = provider.fetch_creators().await.expect("fetch_creators");
        assert!(
            !creators.is_empty(),
            "OnlyHaven fetch_creators returned 0 creators"
        );

        let profile = provider
            .fetch_creator_profile("onlyfans", "30340311")
            .await
            .expect("fetch_creator_profile");
        assert_eq!(profile.id, "30340311");

        let recent = provider
            .fetch_recent_posts(None, 0)
            .await
            .expect("fetch_recent_posts");
        assert!(!recent.is_empty(), "OnlyHaven recent feed returned 0 posts");

        let paged = provider
            .fetch_recent_posts(None, 50)
            .await
            .expect("fetch_recent_posts paged");
        assert!(!paged.is_empty(), "OnlyHaven offset 50 returned 0 posts");

        let searched = provider
            .fetch_recent_posts(Some("cat"), 0)
            .await
            .expect("fetch_recent_posts query");
        assert!(!searched.is_empty(), "OnlyHaven search returned 0 posts");

        let popular = provider
            .fetch_popular_posts("day", None, 0)
            .await
            .expect("fetch_popular_posts day");
        assert!(
            !popular.is_empty(),
            "OnlyHaven popular feed returned 0 posts"
        );

        let popular_month = provider
            .fetch_popular_posts("month", None, 0)
            .await
            .expect("fetch_popular_posts month");
        assert!(
            !popular_month.is_empty(),
            "OnlyHaven popular month returned 0 posts"
        );

        let posts = provider
            .fetch_posts("onlyfans", "30340311", 0, None)
            .await
            .expect("fetch_posts");
        assert!(
            !posts.is_empty(),
            "OnlyHaven creator posts returned 0 posts"
        );

        let filtered_posts = provider
            .fetch_posts("onlyfans", "30340311", 0, Some("test"))
            .await
            .expect("fetch_posts with query");
        assert!(
            !filtered_posts.is_empty(),
            "OnlyHaven creator posts with query returned 0 posts"
        );

        let single = provider
            .fetch_post("onlyfans", "30340311", "2642729960")
            .await
            .expect("fetch_post");
        assert!(single.is_some(), "OnlyHaven post 2642729960 returned None");
        let post = single.unwrap();
        assert_eq!(post.id, "2642729960");

        let non_existent = provider
            .fetch_post("onlyfans", "30340311", "999999999999999")
            .await
            .expect("fetch non-existent post");
        assert!(non_existent.is_none(), "Non-existent post returned Some");

        let revisions = provider
            .fetch_post_revisions("onlyfans", "30340311", "2642729960")
            .await
            .expect("fetch_post_revisions");
        let _ = revisions;

        let comments = provider
            .fetch_post_comments("onlyfans", "30340311", "2642729960")
            .await
            .expect("fetch_post_comments");
        let _ = comments;

        let sample_hash = "7426b2f88640e8807ec0f23a00e9702eb99ff2fd51913d6b27be12887e295fe2";
        let media_url = provider.resolve_media_url(sample_hash, None);
        assert!(media_url.starts_with("https://e1.cum.st/media/"));
        let thumb_url = provider.resolve_thumbnail_url(sample_hash);
        assert!(thumb_url.starts_with("https://img.cum.st/thumbnail/"));

        assert_eq!(provider.id(), "onlyhaven");
        assert_eq!(provider.name(), "OnlyHaven");
        assert!(provider.supports_service("onlyfans"));
        assert!(provider.logout().await.is_ok());
        assert!(provider.search_hash(sample_hash).await.is_err());
        assert!(provider
            .fetch_fancards("onlyfans", "30340311")
            .await
            .unwrap()
            .is_empty());
        assert!(!provider
            .is_post_flagged("onlyfans", "30340311", "2642729960")
            .await
            .unwrap());
        assert!(provider
            .login("bad_user_9999", "bad_pass_9999")
            .await
            .is_err());
    }

    #[test]
    fn test_deserialize_popular_response() {
        let sample = r#"{
            "total": 500,
            "uncapped": 500,
            "period": "day",
            "date": "2026-09-07",
            "posts": [
                {
                    "id": "1329029322",
                    "service": "onlyfans",
                    "creatorId": "307380936",
                    "creatorName": "dutifuldolly",
                    "captionHtml": "<p>hello</p>",
                    "added": 1788670357,
                    "published": 1729288830,
                    "bookmarked": 9,
                    "score": 9,
                    "attachments": [
                        {
                            "locked": false,
                            "position": 0,
                            "id": "7b6fac0f-7c44-4dfb-a0fe-444fd292833e",
                            "sha256": "148d475a2d444bc461da309c491fe015c6372ea6b31a3cb3430f6187510191a0",
                            "kind": "video",
                            "mimeType": "video/mp4",
                            "bytes": 131191210,
                            "storageKey": "148d475a2d444bc461da309c491fe015c6372ea6b31a3cb3430f6187510191a0",
                            "previewThumbhash": "2JcGHAYMYNkpmCZ4mnd/pIaQhg=="
                        }
                    ]
                }
            ]
        }"#;
        let parsed: Result<OnlyHavenListResponse<OnlyHavenPostRow>, _> = serde_json::from_str(sample);
        assert!(parsed.is_ok(), "Failed to deserialize popular response: {:?}", parsed.err());
        let res = parsed.unwrap();
        let posts = res.posts.unwrap();
        assert_eq!(posts.len(), 1);
        let first = &posts[0];
        let post = first.clone().into_post("", "onlyhaven");
        assert_eq!(post.id, "1329029322");
        assert_eq!(post.user, "307380936");
        assert_eq!(post.service, "onlyfans");
        assert_eq!(post.favorite_count, Some(9));
        assert!(post.file.is_some());
    }
}
