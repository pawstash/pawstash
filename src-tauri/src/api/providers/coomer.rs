use super::traits::{
    AuthField, ProviderAuthSchema, ProviderConfig, ProviderHealth, SourceProvider,
};
use crate::api::models::*;
use async_trait::async_trait;
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
}

impl CoomerProvider {
    pub fn new(config: ProviderConfig) -> Result<Self, String> {
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
        Ok(Self {
            id,
            name,
            config: Arc::new(RwLock::new(config)),
            client: Arc::new(RwLock::new(client)),
        })
    }

    fn build_headers(config: &ProviderConfig) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(crate::downloader::PAWSTASH_USER_AGENT),
        );
        // Coomer is behind DDoS-Guard (DDG), which requires Accept: text/css to avoid 403 blocks
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
            let req = client.get(&url).query(params);
            match req.send().await {
                Ok(resp) => {
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
                Err(e) => {
                    last_error = e.to_string();
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
            help_url: Some("https://coomer.st".to_string()),
        }
    }

    async fn test_connection(&self) -> Result<ProviderHealth, String> {
        let start = Instant::now();
        let endpoint = self.get_active_endpoint();
        let now_str = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();

        match self.fetch_creators().await {
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
                let mut extra = prof.extra;
                extra.insert(
                    "provider_id".to_string(),
                    serde_json::Value::String(self.id.clone()),
                );
                Ok(Creator {
                    id: prof.id,
                    name: prof.name,
                    service: prof.service,
                    public_id: prof.public_id,
                    relation_id: prof.relation_id,
                    indexed: prof.indexed.and_then(|v| v.as_i64()),
                    updated: prof.updated.and_then(|v| v.as_i64()),
                    favorited: prof.favorited,
                    ever_imported: prof.ever_imported,
                    extra,
                })
            }
            Err(_) => {
                let mut extra = HashMap::new();
                extra.insert(
                    "provider_id".to_string(),
                    serde_json::Value::String(self.id.clone()),
                );
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

        let mut posts: Vec<Post> = self.get_json(&path, &params).await?;
        let prov_id = self.id.clone();
        for p in &mut posts {
            p.extra.insert(
                "provider_id".to_string(),
                serde_json::Value::String(prov_id.clone()),
            );
        }
        Ok(posts)
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
        match self.get_json::<Post>(&path, &[]).await {
            Ok(mut p) => {
                p.extra.insert(
                    "provider_id".to_string(),
                    serde_json::Value::String(self.id.clone()),
                );
                Ok(Some(p))
            }
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
        let mut posts: Vec<Post> = self.get_json("/posts", &params).await?;
        let prov_id = self.id.clone();
        for p in &mut posts {
            p.extra.insert(
                "provider_id".to_string(),
                serde_json::Value::String(prov_id.clone()),
            );
        }
        Ok(posts)
    }

    async fn fetch_popular_posts(
        &self,
        _period: &str,
        _date: Option<&str>,
        offset: u32,
    ) -> Result<Vec<Post>, String> {
        let params = vec![("o", offset.to_string())];
        let res = self.get_json("/posts/popular", &params).await;
        let mut posts: Vec<Post> = match res {
            Ok(p) => p,
            Err(_) => self.get_json("/popular", &params).await?,
        };
        let prov_id = self.id.clone();
        for p in &mut posts {
            p.extra.insert(
                "provider_id".to_string(),
                serde_json::Value::String(prov_id.clone()),
            );
        }
        Ok(posts)
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

    async fn fetch_creator_artwork_data_url(
        &self,
        _service: &str,
        _creator_id: &str,
        _artwork_type: &str,
    ) -> Result<String, String> {
        Err("Coomer creator artwork is not supported".to_string())
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
        let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
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
