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

use super::traits::{ProviderConfig, ProviderHealth, SourceProvider};
use crate::api::models::*;
use async_trait::async_trait;
use base64::prelude::*;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
            Value::String("coomer".to_string()),
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
            kemono_favorited: c.bookmarked,
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
    #[serde(default)]
    kind: Option<String>,
    #[serde(rename = "mimeType", default)]
    mime_type: Option<String>,
    #[serde(default)]
    bytes: Option<u64>,
    #[serde(rename = "storageKey", default)]
    storage_key: Option<String>,
    #[serde(rename = "originalFilename", default)]
    original_filename: Option<String>,
    #[serde(rename = "previewThumbhash", default)]
    preview_thumbhash: Option<String>,
    #[serde(default)]
    variants: Option<Vec<Value>>,
    #[serde(default)]
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
        alias = "bookmark_count",
        alias = "score"
    )]
    bookmarked: Option<u64>,
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
        let attachments: Vec<Attachment> = self
            .attachments
            .unwrap_or_default()
            .into_iter()
            .filter(|a| !a.locked.unwrap_or(false))
            .map(|a| {
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

                let mut extra = HashMap::new();
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
                    name: a.original_filename.or(a.name),
                    path: clean_path,
                    server: None,
                    size: a.bytes,
                    extra,
                }
            })
            .collect();

        let file = attachments.first().cloned();
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
            next,
            prev,
            favorite_count: self.bookmarked,
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
}

impl OnlyHavenProvider {
    pub fn new(config: ProviderConfig) -> Result<Self, String> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Pawstash/OnlyHaven",
            ),
        );

        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .gzip(true)
            .default_headers(headers)
            .build()
            .map_err(|e| format!("Failed to build HTTP client for OnlyHaven: {e}"))?;

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            client,
            current_mirror_idx: AtomicUsize::new(0),
        })
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
            endpoints.push("https://cum.st".to_string());
        }
        endpoints
    }

    async fn request<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let endpoints = self.get_endpoints();
        let start_idx = self.current_mirror_idx.load(Ordering::Relaxed) % endpoints.len();

        let mut last_err = String::new();
        for i in 0..endpoints.len() {
            let idx = (start_idx + i) % endpoints.len();
            let base = &endpoints[idx];
            let url = format!("{base}{path}");

            match self.client.get(&url).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        self.current_mirror_idx.store(idx, Ordering::Relaxed);
                        match resp.json::<T>().await {
                            Ok(parsed) => return Ok(parsed),
                            Err(e) => {
                                last_err = format!("Failed to parse response from {url}: {e}");
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
        "coomer"
    }

    fn name(&self) -> &str {
        "OnlyHaven"
    }

    fn config(&self) -> ProviderConfig {
        self.config.read().unwrap().clone()
    }

    fn supports_service(&self, service: &str) -> bool {
        matches!(
            service.to_lowercase().as_str(),
            "onlyfans" | "fansly" | "patreon" | "candfans"
        )
    }

    fn get_active_endpoint(&self) -> String {
        let endpoints = self.get_endpoints();
        let idx = self.current_mirror_idx.load(Ordering::Relaxed) % endpoints.len();
        endpoints[idx].clone()
    }

    async fn test_connection(&self) -> Result<ProviderHealth, String> {
        let endpoint = self.get_active_endpoint();
        let start = Instant::now();
        let url = format!("{endpoint}/api/v1/creators?n=1");
        let now_str = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();

        match self.client.get(&url).send().await {
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
        let path = "/api/v1/creators";
        let res: OnlyHavenListResponse<OnlyHavenCreatorRow> = self.request(path).await?;
        let rows = res.creators.unwrap_or_default();
        Ok(rows.into_iter().map(Creator::from).collect())
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
        _period: &str,
        _date: Option<&str>,
        offset: u32,
    ) -> Result<Vec<Post>, String> {
        let path = format!("/api/v1/posts?n=50&o={offset}&sort=popular");
        let res: OnlyHavenListResponse<OnlyHavenPostRow> = self.request(&path).await?;
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
            .unwrap_or_else(|| super::pawchive::derive_subdomain_url(&conf.api_url, "file"));
        format!("{base}/media/{key}/original.jpg")
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
            .unwrap_or_else(|| super::pawchive::derive_subdomain_url(&conf.api_url, "img"));
        format!("{base}/thumbnail/{key}/preview.webp")
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
                .unwrap_or_else(|| super::pawchive::derive_subdomain_url(&conf.api_url, "img"))
        };

        let candidate_urls = vec![format!(
            "{img_base}/creator/{service}/{creator_id}/{file_name}"
        )];

        for url in candidate_urls {
            let req = self.client.get(&url).header(USER_AGENT, "Pawstash/0.1");
            if let Ok(resp) = req.send().await {
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

    async fn login(&self, _username: &str, _password: &str) -> Result<String, String> {
        Err("OnlyHaven login via credentials not supported".to_string())
    }

    async fn logout(&self) -> Result<(), String> {
        Ok(())
    }

    async fn get_account_session(&self) -> Result<AccountSession, String> {
        let conf = self.config.read().unwrap();
        Ok(AccountSession {
            authenticated: !conf.session_cookie.trim().is_empty(),
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
            is_custom: false,
        })
        .unwrap();

        assert_eq!(
            provider.resolve_media_url("/74/26/7426b2f88640e8807ec0f23a00e9702eb99ff2fd51913d6b27be12887e295fe2.jpg", None),
            "https://e1.cum.st/media/74/26/7426b2f88640e8807ec0f23a00e9702eb99ff2fd51913d6b27be12887e295fe2.jpg/original.jpg"
        );
        assert_eq!(
            provider.resolve_thumbnail_url("/74/26/7426b2f88640e8807ec0f23a00e9702eb99ff2fd51913d6b27be12887e295fe2.jpg"),
            "https://img.cum.st/thumbnail/74/26/7426b2f88640e8807ec0f23a00e9702eb99ff2fd51913d6b27be12887e295fe2.jpg/preview.webp"
        );
    }
}
