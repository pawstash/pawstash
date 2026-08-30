use super::traits::{ProviderConfig, ProviderHealth, SourceProvider};
use crate::api::models::*;
use crate::config::settings::{AppSettings, ProxyMode};
use crate::smart_links::{
    is_known_shortener_url, parse_external_post_link, parse_pawchive_post_url,
};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use reqwest::header::{
    HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, COOKIE, LOCATION, SET_COOKIE, USER_AGENT,
};
use reqwest::{Client, Method, Response, StatusCode, Url};
use scraper::{Html, Selector};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock as AsyncRwLock;

pub struct PawchiveClient {
    client: Arc<AsyncRwLock<Client>>,
    settings: Arc<AsyncRwLock<AppSettings>>,
}

impl PawchiveClient {
    pub async fn login(&self, username: &str, password: &str) -> Result<String, String> {
        let username = username.trim();
        if username.is_empty() || password.is_empty() {
            return Err("Username and password are required".to_string());
        }
        let settings = self.settings.read().await.clone();
        let url = Url::parse(&format!(
            "{}/account/login",
            Self::site_url(&settings.api_domain)
        ))
        .map_err(|e| e.to_string())?;
        let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
        if url.scheme() != "https" && !local {
            return Err("Pawchive login requires HTTPS".to_string());
        }
        let login_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .gzip(true)
            .build()
            .map_err(|e| e.to_string())?;
        let response = login_client
            .post(url)
            .headers(Self::build_headers(&AppSettings {
                session_cookie: String::new(),
                ..settings
            }))
            .form(&[
                ("location", "/account"),
                ("username", username),
                ("password", password),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = response.status();
        let redirected = status.is_redirection()
            && response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|location| !location.contains("/account/login"));
        if !redirected {
            return Err("Pawchive rejected the username or password".to_string());
        }
        let cookies: Vec<String> = response
            .headers()
            .get_all(SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .filter_map(|value| value.split(';').next())
            .map(str::to_string)
            .collect();
        if cookies.is_empty() {
            return Err("Pawchive did not return a session cookie".to_string());
        }
        Ok(cookies.join("; "))
    }

    pub async fn logout(&self) -> Result<(), String> {
        let settings = self.settings.read().await.clone();
        if settings.session_cookie.trim().is_empty() {
            return Ok(());
        }
        let client = self.client.read().await.clone();
        client
            .get(format!(
                "{}/account/logout",
                Self::site_url(&settings.api_domain)
            ))
            .headers(Self::build_headers(&settings))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn new(settings: AppSettings) -> Result<Self, String> {
        let client = Self::build_client(&settings)?;
        Ok(Self {
            client: Arc::new(AsyncRwLock::new(client)),
            settings: Arc::new(AsyncRwLock::new(settings)),
        })
    }

    pub async fn update_settings(&self, settings: AppSettings) -> Result<(), String> {
        let client = Self::build_client(&settings)?;
        *self.client.write().await = client;
        *self.settings.write().await = settings;
        Ok(())
    }

    fn build_client(settings: &AppSettings) -> Result<Client, String> {
        let redirect_policy = reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 10 {
                return attempt.error("Too many redirects");
            }
            let next_url = attempt.url();
            let path = next_url.path();
            if path.contains("/account/login") || path.contains("/login") || path == "/account" {
                return attempt.stop();
            }
            attempt.follow()
        });

        let mut builder = Client::builder()
            .timeout(Duration::from_secs(45))
            .redirect(redirect_policy)
            .gzip(true);

        match settings.proxy_mode {
            ProxyMode::None => builder = builder.no_proxy(),
            ProxyMode::System => {}
            ProxyMode::Custom => {
                if settings.proxy_url.trim().is_empty() {
                    builder = builder.no_proxy();
                    return builder.build().map_err(|e| e.to_string());
                }
                let mut proxy = reqwest::Proxy::all(settings.proxy_url.trim())
                    .map_err(|e| format!("Invalid proxy URL: {e}"))?;
                if !settings.proxy_username.is_empty() {
                    proxy = proxy.basic_auth(&settings.proxy_username, &settings.proxy_password);
                }
                if settings.proxy_bypass_local {
                    proxy =
                        proxy.no_proxy(reqwest::NoProxy::from_string("localhost,127.0.0.1,::1"));
                }
                builder = builder.proxy(proxy);
            }
        }

        builder.build().map_err(|e| e.to_string())
    }

    fn cookie_header(raw: &str) -> Option<HeaderValue> {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }
        let value = if raw.contains('=') {
            raw.to_string()
        } else {
            format!("session={raw}")
        };
        HeaderValue::from_str(&value).ok()
    }

    fn build_headers(settings: &AppSettings) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Pawstash/0.1.0"),
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/json, text/plain, */*"),
        );
        if let Some(cookie) = Self::cookie_header(&settings.session_cookie) {
            headers.insert(COOKIE, cookie);
        }
        headers
    }

    fn base_url(domain: &str) -> String {
        let domain = domain.trim().trim_end_matches('/');
        if domain.starts_with("http://") || domain.starts_with("https://") {
            format!("{domain}/api/v1")
        } else {
            format!("https://{domain}/api/v1")
        }
    }

    fn site_url(domain: &str) -> String {
        let domain = domain.trim().trim_end_matches('/');
        if domain.starts_with("http://") || domain.starts_with("https://") {
            domain.to_string()
        } else {
            format!("https://{domain}")
        }
    }

    fn segment(value: &str) -> String {
        urlencoding::encode(value).into_owned()
    }

    pub async fn fetch_creator_artwork_data_url(
        &self,
        service: &str,
        creator_id: &str,
        artwork_kind: &str,
    ) -> Result<String, String> {
        let directory = match artwork_kind {
            "banner" => "banners",
            "avatar" => "icons",
            _ => return Err("Unsupported creator artwork kind".to_string()),
        };
        let settings = self.settings.read().await.clone();
        let client = self.client.read().await.clone();
        let url = format!(
            "{}/{}/{}/{}",
            Self::site_url(&settings.api_domain),
            directory,
            Self::segment(service),
            Self::segment(creator_id)
        );
        let response = client
            .get(url)
            .headers(Self::build_headers(&settings))
            .send()
            .await
            .map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(format!("Pawchive artwork HTTP {}", response.status()));
        }
        if response
            .content_length()
            .is_some_and(|length| length > 8 * 1024 * 1024)
        {
            return Err("Creator artwork is too large".to_string());
        }
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .filter(|value| value.starts_with("image/"))
            .unwrap_or("image/jpeg")
            .to_string();
        let bytes = response.bytes().await.map_err(|error| error.to_string())?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err("Creator artwork is too large".to_string());
        }
        Ok(format!(
            "data:{content_type};base64,{}",
            BASE64_STANDARD.encode(bytes)
        ))
    }

    async fn list_params(
        &self,
        query: Option<&str>,
        offset: u32,
    ) -> Result<Vec<(&'static str, String)>, String> {
        let mut params = vec![("o", offset.to_string())];
        let mut hide_ai_directive = false;
        let mut show_ai_directive = false;
        let mut clean_query = String::new();

        if let Some(raw_q) = query.map(str::trim).filter(|q| !q.is_empty()) {
            if raw_q.contains("hide=ai")
                || raw_q.contains("hide=(ai)")
                || raw_q.contains("?hide=ai")
                || raw_q.contains("?hide=(ai)")
                || raw_q.contains("ai:hide")
            {
                hide_ai_directive = true;
            } else if raw_q.contains("only=ai")
                || raw_q.contains("only=(ai)")
                || raw_q.contains("show=ai")
                || raw_q.contains("show=(ai)")
                || raw_q.contains("?only=ai")
                || raw_q.contains("?only=(ai)")
                || raw_q.contains("?show=ai")
                || raw_q.contains("?show=(ai)")
                || raw_q.contains("ai:only")
                || raw_q.contains("ai:show")
            {
                show_ai_directive = true;
            }
            let cleaned = raw_q
                .replace("?hide=(ai)", "")
                .replace("hide=(ai)", "")
                .replace("?hide=ai", "")
                .replace("hide=ai", "")
                .replace("?only=(ai)", "")
                .replace("only=(ai)", "")
                .replace("?only=ai", "")
                .replace("only=ai", "")
                .replace("?show=(ai)", "")
                .replace("show=(ai)", "")
                .replace("?show=ai", "")
                .replace("show=ai", "")
                .replace("ai:hide", "")
                .replace("ai:only", "")
                .replace("ai:show", "")
                .trim()
                .to_string();
            if !cleaned.is_empty() {
                if cleaned.chars().count() < 2 {
                    return Err(
                        "Pawchive search query must contain at least 2 characters".to_string()
                    );
                }
                clean_query = cleaned;
            }
        }

        if !clean_query.is_empty() {
            params.push(("q", clean_query));
        }

        let settings = self.settings.read().await;
        if settings.pawchive_hide_ai || hide_ai_directive {
            params.push(("hide", "ai".to_string()));
        } else if show_ai_directive {
            params.push(("only", "ai".to_string()));
        }

        Ok(params)
    }

    async fn send(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<Response, String> {
        let settings = self.settings.read().await.clone();
        let client = self.client.read().await.clone();
        let url = format!("{}{}", Self::base_url(&settings.api_domain), path);
        client
            .request(method, url)
            .headers(Self::build_headers(&settings))
            .query(query)
            .send()
            .await
            .map_err(|e| e.to_string())
    }

    async fn json<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T, String> {
        let response = self.send(method.clone(), path, query).await?;
        let status = response.status();
        let settings = self.settings.read().await.clone();
        let is_auth = !settings.session_cookie.trim().is_empty();

        let is_auth_error = status == StatusCode::UNAUTHORIZED
            || status == StatusCode::FORBIDDEN
            || status.is_redirection();

        if is_auth && is_auth_error {
            let guest_settings = AppSettings {
                session_cookie: String::new(),
                ..settings.clone()
            };
            let client = self.client.read().await.clone();
            let url = format!("{}{}", Self::base_url(&guest_settings.api_domain), path);
            if let Ok(guest_resp) = client
                .request(method.clone(), url)
                .headers(Self::build_headers(&guest_settings))
                .query(query)
                .send()
                .await
            {
                if guest_resp.status().is_success() {
                    let guest_body = guest_resp.text().await.unwrap_or_default();
                    if let Ok(data) = serde_json::from_str::<T>(&guest_body) {
                        return Ok(data);
                    }
                }
            }
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Pawchive API HTTP {status}: {}", body.trim()));
        }

        let body = response.text().await.map_err(|e| e.to_string())?;

        match serde_json::from_str::<T>(&body) {
            Ok(data) => Ok(data),
            Err(err) => {
                if is_auth {
                    let guest_settings = AppSettings {
                        session_cookie: String::new(),
                        ..settings
                    };
                    let client = self.client.read().await.clone();
                    let url = format!("{}{}", Self::base_url(&guest_settings.api_domain), path);
                    if let Ok(guest_resp) = client
                        .request(method, url)
                        .headers(Self::build_headers(&guest_settings))
                        .query(query)
                        .send()
                        .await
                    {
                        if guest_resp.status().is_success() {
                            let guest_body = guest_resp.text().await.unwrap_or_default();
                            if let Ok(data) = serde_json::from_str::<T>(&guest_body) {
                                return Ok(data);
                            }
                        }
                    }
                }
                Err(format!("Invalid Pawchive response: {err}"))
            }
        }
    }

    async fn action(&self, method: Method, path: &str) -> Result<ApiActionResult, String> {
        let response = self.send(method, path, &[]).await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Pawchive API HTTP {status}: {}", body.trim()));
        }
        Ok(ApiActionResult {
            status: status.as_u16(),
            success: true,
        })
    }

    async fn get_site_page(
        &self,
        mut url: Url,
        settings: &AppSettings,
    ) -> Result<Response, String> {
        const MAX_REDIRECTS: usize = 5;
        let client = self.client.read().await.clone();
        let original_origin = (
            url.scheme().to_string(),
            url.host_str().map(str::to_string),
            url.port_or_known_default(),
        );

        for redirect_count in 0..=MAX_REDIRECTS {
            let mut headers = Self::build_headers(settings);
            let current_origin = (
                url.scheme().to_string(),
                url.host_str().map(str::to_string),
                url.port_or_known_default(),
            );
            if current_origin != original_origin {
                headers.remove(COOKIE);
            }

            let response = client
                .get(url.clone())
                .headers(headers)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if !response.status().is_redirection() {
                return Ok(response);
            }
            if redirect_count == MAX_REDIRECTS {
                return Err(format!("Pawchive exceeded {MAX_REDIRECTS} redirects"));
            }

            let location = response
                .headers()
                .get(LOCATION)
                .ok_or_else(|| {
                    format!(
                        "Pawchive HTTP {} redirect has no Location header",
                        response.status()
                    )
                })?
                .to_str()
                .map_err(|_| "Pawchive redirect Location is not valid UTF-8".to_string())?;
            let next = url
                .join(location)
                .map_err(|e| format!("Invalid Pawchive redirect URL: {e}"))?;
            if !matches!(next.scheme(), "http" | "https") {
                return Err(format!(
                    "Unsupported Pawchive redirect scheme: {}",
                    next.scheme()
                ));
            }
            url = next;
        }

        unreachable!()
    }

    pub async fn resolve_post_identity(
        &self,
        service: &str,
        post_id: &str,
    ) -> Result<Option<(String, String, String)>, String> {
        const MAX_RESOLVER_PAGE_BYTES: u64 = 2 * 1024 * 1024;
        let settings = self.settings.read().await.clone();
        let url = Url::parse(&format!(
            "{}/{}/post/{}",
            Self::site_url(&settings.api_domain),
            Self::segment(service),
            Self::segment(post_id)
        ))
        .map_err(|error| error.to_string())?;
        let response = self.get_site_page(url, &settings).await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(format!("Pawchive resolver HTTP {}", response.status()));
        }

        let final_url = response.url().clone();
        if let Some(identity) = parse_pawchive_post_url(&final_url, service, post_id) {
            return Ok(Some(identity));
        }
        if response
            .content_length()
            .is_some_and(|size| size > MAX_RESOLVER_PAGE_BYTES)
        {
            return Ok(None);
        }

        let bytes = response.bytes().await.map_err(|error| error.to_string())?;
        if bytes.len() as u64 > MAX_RESOLVER_PAGE_BYTES {
            return Ok(None);
        }
        let html = String::from_utf8_lossy(&bytes);
        let document = Html::parse_document(&html);
        let selectors = [
            ("link[rel='canonical']", "href"),
            ("meta[property='og:url']", "content"),
            ("a[href]", "href"),
        ];
        for (selector_text, attribute) in selectors {
            let selector = Selector::parse(selector_text).map_err(|error| error.to_string())?;
            for element in document.select(&selector).take(256) {
                let Some(candidate) = element.value().attr(attribute) else {
                    continue;
                };
                let Ok(candidate_url) = final_url.join(candidate) else {
                    continue;
                };
                if let Some(identity) = parse_pawchive_post_url(&candidate_url, service, post_id) {
                    return Ok(Some(identity));
                }
            }
        }
        Ok(None)
    }

    pub async fn expand_short_link(&self, raw_url: &str) -> Result<Option<String>, String> {
        const MAX_SHORTENER_REDIRECTS: usize = 5;
        let mut url = Url::parse(raw_url).map_err(|error| error.to_string())?;
        if !is_known_shortener_url(&url) {
            return Ok(None);
        }

        let client = self.client.read().await.clone();
        for redirect_count in 0..=MAX_SHORTENER_REDIRECTS {
            let response = client
                .get(url.clone())
                .header(USER_AGENT, "Pawstash/0.1 link resolver")
                .send()
                .await
                .map_err(|error| error.to_string())?;
            if !response.status().is_redirection() {
                return Ok(None);
            }
            if redirect_count == MAX_SHORTENER_REDIRECTS {
                return Ok(None);
            }

            let location = response
                .headers()
                .get(LOCATION)
                .ok_or_else(|| "Short-link redirect has no Location header".to_string())?
                .to_str()
                .map_err(|_| "Short-link redirect Location is not valid UTF-8".to_string())?;
            let next = url
                .join(location)
                .map_err(|error| format!("Invalid short-link redirect URL: {error}"))?;
            if parse_external_post_link(next.as_str()).is_some() {
                return Ok(Some(next.to_string()));
            }
            if !is_known_shortener_url(&next) {
                return Ok(None);
            }
            url = next;
        }

        Ok(None)
    }

    pub async fn fetch_creators(&self) -> Result<Vec<Creator>, String> {
        let settings = self.settings.read().await;
        let mut params = Vec::new();
        if settings.pawchive_hide_ai {
            params.push(("hide", "ai".to_string()));
        }
        drop(settings);
        self.json(Method::GET, "/creators", &params).await
    }

    pub async fn fetch_recent_posts(
        &self,
        query: Option<&str>,
        offset: u32,
    ) -> Result<Vec<Post>, String> {
        let params = self.list_params(query, offset).await?;
        self.json(Method::GET, "/posts", &params).await
    }

    pub async fn fetch_popular_posts(
        &self,
        period: &str,
        date: Option<&str>,
        offset: u32,
    ) -> Result<Vec<Post>, String> {
        if !offset.is_multiple_of(50) {
            return Err("Pawchive offset must be a multiple of 50".to_string());
        }
        if !matches!(period, "day" | "week" | "month") {
            return Err("Popular period must be day, week, or month".to_string());
        }

        let settings = self.settings.read().await.clone();
        let mut url = Url::parse(&format!(
            "{}/posts/popular",
            Self::site_url(&settings.api_domain)
        ))
        .map_err(|e| format!("Invalid Pawchive site URL: {e}"))?;
        let mut query = vec![("period", period.to_string()), ("o", offset.to_string())];
        if let Some(date) = date.map(str::trim).filter(|date| !date.is_empty()) {
            query.push(("date", date.to_string()));
        }
        url.query_pairs_mut()
            .extend_pairs(query.iter().map(|(key, value)| (*key, value.as_str())));
        let mut response = self.get_site_page(url, &settings).await?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("Pawchive HTTP {status}"));
        }
        if offset > 0 && !response.url().query_pairs().any(|(k, _)| k == "o") {
            let mut final_url = response.url().clone();
            final_url
                .query_pairs_mut()
                .append_pair("o", &offset.to_string());
            response = self.get_site_page(final_url, &settings).await?;
        }

        let html = response.text().await.map_err(|e| e.to_string())?;
        Self::parse_popular_posts(&html)
    }

    fn parse_popular_posts(html: &str) -> Result<Vec<Post>, String> {
        let document = Html::parse_document(html);
        let card_selector = Selector::parse("article.post-card").map_err(|e| e.to_string())?;
        let title_selector = Selector::parse(".post-card__header").map_err(|e| e.to_string())?;
        let time_selector = Selector::parse("time").map_err(|e| e.to_string())?;
        let image_selector = Selector::parse("img.post-card__image").map_err(|e| e.to_string())?;
        let footer_selector = Selector::parse(".post-card__footer").map_err(|e| e.to_string())?;

        document
            .select(&card_selector)
            .map(|card| {
                let value = card.value();
                let id = value.attr("data-id").unwrap_or_default().to_string();
                let user = value.attr("data-user").unwrap_or_default().to_string();
                let service = value.attr("data-service").unwrap_or_default().to_string();
                if id.is_empty() || user.is_empty() || service.is_empty() {
                    return Err("Popular post card is missing identity fields".to_string());
                }

                let title = card
                    .select(&title_selector)
                    .next()
                    .map(|node| node.text().collect::<String>().trim().to_string())
                    .unwrap_or_default();
                let published = card
                    .select(&time_selector)
                    .next()
                    .and_then(|node| node.value().attr("datetime"))
                    .map(str::to_string);
                let footer_text = card
                    .select(&footer_selector)
                    .next()
                    .map(|node| node.text().collect::<Vec<_>>().join(" "))
                    .unwrap_or_default();
                let numbers = |label: &str| {
                    footer_text
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .windows(2)
                        .find(|parts| parts[1].trim_end_matches('s') == label)
                        .and_then(|parts| parts[0].parse::<u64>().ok())
                };
                let attachment_count = numbers("attachment")
                    .or_else(|| footer_text.contains("No attachments").then_some(0));
                let favorite_count = numbers("favorite");
                let file = card.select(&image_selector).next().and_then(|image| {
                    let src = image.value().attr("src")?;
                    let (_, path) = src.split_once("/thumbnail/data")?;
                    Some(Attachment {
                        name: Some("preview".to_string()),
                        path: Some(path.to_string()),
                        ..Attachment::default()
                    })
                });

                Ok(Post {
                    id,
                    user,
                    service,
                    title,
                    content: None,
                    substring: None,
                    published,
                    added: None,
                    edited: None,
                    embed: None,
                    shared_file: None,
                    attachments: Some(Vec::new()),
                    file,
                    poll: None,
                    captions: None,
                    tags: None,
                    origin: Some("popular".to_string()),
                    preview_state: Some("scraped".to_string()),
                    has_full: Some(false),
                    detail_fetched: Some(false),
                    next: None,
                    prev: None,
                    favorite_count,
                    attachment_count,
                    extra: HashMap::new(),
                })
            })
            .collect()
    }

    pub async fn fetch_creator_profile(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<CreatorProfile, String> {
        let path = format!(
            "/{}/user/{}/profile",
            Self::segment(service),
            Self::segment(creator_id)
        );
        self.json(Method::GET, &path, &[]).await
    }

    pub async fn fetch_creator_posts(
        &self,
        service: &str,
        creator_id: &str,
        query: Option<&str>,
        offset: u32,
    ) -> Result<Vec<Post>, String> {
        let path = format!(
            "/{}/user/{}",
            Self::segment(service),
            Self::segment(creator_id)
        );
        let params = self.list_params(query, offset).await?;
        self.json(Method::GET, &path, &params).await
    }

    pub async fn fetch_creator_tags(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<String>, String> {
        let path = format!(
            "/{}/user/{}/tags",
            Self::segment(service),
            Self::segment(creator_id)
        );
        let response = match self.send(Method::GET, &path, &[]).await {
            Ok(res) if res.status().is_success() => res,
            _ => return Ok(Vec::new()),
        };
        let body = response.text().await.unwrap_or_default();
        if let Ok(tags) = serde_json::from_str::<Vec<String>>(&body) {
            return Ok(tags.into_iter().filter(|t| !t.trim().is_empty()).collect());
        }
        #[derive(serde::Deserialize)]
        struct TagObj {
            #[serde(default)]
            tag: Option<String>,
            #[serde(default)]
            name: Option<String>,
            #[serde(default)]
            val: Option<String>,
        }
        if let Ok(objs) = serde_json::from_str::<Vec<TagObj>>(&body) {
            return Ok(objs
                .into_iter()
                .filter_map(|o| o.tag.or(o.name).or(o.val))
                .filter(|s| !s.trim().is_empty())
                .collect());
        }
        Ok(Vec::new())
    }

    pub async fn fetch_announcements(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<Announcement>, String> {
        let path = format!(
            "/{}/user/{}/announcements",
            Self::segment(service),
            Self::segment(creator_id)
        );
        self.json(Method::GET, &path, &[]).await
    }

    pub async fn fetch_fancards(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<Fancard>, String> {
        let path = format!(
            "/{}/user/{}/fancards",
            Self::segment(service),
            Self::segment(creator_id)
        );
        self.json(Method::GET, &path, &[]).await
    }

    pub async fn fetch_creator_links(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<CreatorProfile>, String> {
        let path = format!(
            "/{}/user/{}/links",
            Self::segment(service),
            Self::segment(creator_id)
        );
        self.json(Method::GET, &path, &[]).await
    }

    pub async fn fetch_similar_creators(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<CreatorProfile>, String> {
        let settings = self.settings.read().await.clone();
        let page_url = Url::parse(&format!(
            "{}/{}/user/{}/recommended",
            Self::site_url(&settings.api_domain),
            Self::segment(service),
            Self::segment(creator_id)
        ))
        .map_err(|e| e.to_string())?;

        let response = match self.get_site_page(page_url, &settings).await {
            Ok(res) if res.status().is_success() => res,
            _ => return Ok(Vec::new()),
        };

        let html_content = response.text().await.map_err(|e| e.to_string())?;
        let document = Html::parse_document(&html_content);
        let card_selector = match Selector::parse(".card-list__items a.user-card, a.user-card") {
            Ok(s) => s,
            Err(_) => return Ok(Vec::new()),
        };
        let name_selector = match Selector::parse(".user-card__name") {
            Ok(s) => s,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();
        seen.insert((service.to_string(), creator_id.to_string()));

        for card in document.select(&card_selector) {
            let id = card
                .value()
                .attr("data-id")
                .unwrap_or_default()
                .trim()
                .to_string();
            let srv = card
                .value()
                .attr("data-service")
                .unwrap_or(service)
                .trim()
                .to_string();
            let name = card
                .select(&name_selector)
                .next()
                .map(|n| n.text().collect::<Vec<_>>().join("").trim().to_string())
                .unwrap_or_else(|| id.clone());

            if !id.is_empty() && seen.insert((srv.clone(), id.clone())) {
                results.push(CreatorProfile {
                    id,
                    name,
                    service: srv,
                    public_id: None,
                    relation_id: None,
                    updated: None,
                    indexed: None,
                    kemono_favorited: None,
                    ever_imported: None,
                    extra: Default::default(),
                });
            }
        }
        Ok(results)
    }

    pub async fn fetch_post(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Post, String> {
        let path = format!(
            "/{}/user/{}/post/{}",
            Self::segment(service),
            Self::segment(creator_id),
            Self::segment(post_id)
        );
        self.json(Method::GET, &path, &[]).await
    }

    pub async fn fetch_account_favorites(
        &self,
        favorite_type: Option<&str>,
    ) -> Result<Vec<Favorite>, String> {
        let params = favorite_type
            .filter(|kind| !kind.is_empty())
            .map(|kind| vec![("type", kind.to_string())])
            .unwrap_or_default();
        self.json(Method::GET, "/account/favorites", &params).await
    }

    pub async fn set_post_favorite(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
        favorite: bool,
    ) -> Result<ApiActionResult, String> {
        let path = format!(
            "/favorites/post/{}/{}/{}",
            Self::segment(service),
            Self::segment(creator_id),
            Self::segment(post_id)
        );
        self.action(
            if favorite {
                Method::POST
            } else {
                Method::DELETE
            },
            &path,
        )
        .await
    }

    pub async fn set_creator_favorite(
        &self,
        service: &str,
        creator_id: &str,
        favorite: bool,
    ) -> Result<ApiActionResult, String> {
        let path = format!(
            "/favorites/creator/{}/{}",
            Self::segment(service),
            Self::segment(creator_id)
        );
        self.action(
            if favorite {
                Method::POST
            } else {
                Method::DELETE
            },
            &path,
        )
        .await
    }

    pub async fn search_hash(&self, file_hash: &str) -> Result<FileSearchResult, String> {
        let path = format!("/search_hash/{}", Self::segment(file_hash));
        self.json(Method::GET, &path, &[]).await
    }

    pub async fn flag_post(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<ApiActionResult, String> {
        let path = Self::flag_path(service, creator_id, post_id);
        let response = self.send(Method::POST, &path, &[]).await?;
        let status = response.status();
        if status.is_success() || status == StatusCode::CONFLICT {
            return Ok(ApiActionResult {
                status: status.as_u16(),
                success: status.is_success(),
            });
        }
        let body = response.text().await.unwrap_or_default();
        Err(format!("Pawchive API HTTP {status}: {}", body.trim()))
    }

    pub async fn is_post_flagged(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<bool, String> {
        let response = self
            .send(
                Method::GET,
                &Self::flag_path(service, creator_id, post_id),
                &[],
            )
            .await?;
        match response.status() {
            StatusCode::OK => Ok(true),
            StatusCode::NOT_FOUND => Ok(false),
            status => {
                let body = response.text().await.unwrap_or_default();
                Err(format!("Pawchive API HTTP {status}: {}", body.trim()))
            }
        }
    }

    fn flag_path(service: &str, creator_id: &str, post_id: &str) -> String {
        format!(
            "/{}/user/{}/post/{}/flag",
            Self::segment(service),
            Self::segment(creator_id),
            Self::segment(post_id)
        )
    }

    pub async fn fetch_post_revisions(
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
        self.json(Method::GET, &path, &[]).await
    }

    pub async fn fetch_post_comments(
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
        self.json(Method::GET, &path, &[]).await
    }

    pub async fn app_version(&self) -> Result<String, String> {
        let response = self.send(Method::GET, "/app_version", &[]).await?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("Pawchive API HTTP {status}"));
        }
        response.text().await.map_err(|e| e.to_string())
    }
}

pub struct PawchiveProvider {
    config: Arc<RwLock<ProviderConfig>>,
    client: Arc<PawchiveClient>,
}

impl PawchiveProvider {
    pub fn new(config: ProviderConfig) -> Result<Self, String> {
        let app_settings = AppSettings {
            api_domain: config.api_url.clone(),
            file_domain: config.file_url.clone().unwrap_or_default(),
            image_domain: config.image_url.clone().unwrap_or_default(),
            session_cookie: config.session_cookie.clone(),
            ..AppSettings::default()
        };

        let client = Arc::new(PawchiveClient::new(app_settings)?);
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            client,
        })
    }
}

pub(crate) fn derive_subdomain_url(api_url: &str, kind: &str) -> String {
    let clean_url = if api_url.starts_with("http://") || api_url.starts_with("https://") {
        api_url.trim_end_matches('/').to_string()
    } else {
        format!("https://{}", api_url.trim_end_matches('/'))
    };

    if let Ok(parsed) = reqwest::Url::parse(&clean_url) {
        let host = parsed.host_str().unwrap_or("pawchive.pw");
        let scheme = parsed.scheme();
        let base_host = host.trim_start_matches("www.").trim_start_matches("api.");
        let parts: Vec<&str> = base_host.split('.').collect();
        let domain = if parts.len() > 2 {
            parts[parts.len() - 2..].join(".")
        } else {
            base_host.to_string()
        };
        let prefix = if kind == "image" || kind == "img" {
            "img"
        } else {
            "file"
        };
        return format!("{scheme}://{prefix}.{domain}");
    }
    clean_url
}

#[async_trait]
impl SourceProvider for PawchiveProvider {
    fn id(&self) -> &str {
        "pawchive"
    }

    fn name(&self) -> &str {
        "Pawchive"
    }

    fn config(&self) -> ProviderConfig {
        self.config.read().unwrap().clone()
    }

    fn supports_service(&self, service: &str) -> bool {
        matches!(
            service.to_lowercase().as_str(),
            "patreon"
                | "fanbox"
                | "fantia"
                | "boosty"
                | "subscribestar"
                | "gumroad"
                | "dlsite"
                | "discord"
                | "afdian"
        )
    }

    fn get_active_endpoint(&self) -> String {
        self.config.read().unwrap().api_url.clone()
    }

    async fn test_connection(&self) -> Result<ProviderHealth, String> {
        let start = Instant::now();
        let endpoint = self.get_active_endpoint();
        let now_str = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();

        match self.client.fetch_creators().await {
            Ok(_) => {
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
            Err(e) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                Ok(ProviderHealth {
                    provider_id: self.id().to_string(),
                    active_endpoint: endpoint,
                    is_healthy: false,
                    latency_ms,
                    error: Some(e),
                    last_checked_at: now_str,
                })
            }
        }
    }

    async fn update_config(&self, config: ProviderConfig) -> Result<(), String> {
        let app_settings = AppSettings {
            api_domain: config.api_url.clone(),
            file_domain: config.file_url.clone().unwrap_or_default(),
            image_domain: config.image_url.clone().unwrap_or_default(),
            session_cookie: config.session_cookie.clone(),
            ..AppSettings::default()
        };
        self.client.update_settings(app_settings).await?;
        *self.config.write().unwrap() = config;
        Ok(())
    }

    async fn fetch_creators(&self) -> Result<Vec<Creator>, String> {
        self.client.fetch_creators().await
    }

    async fn fetch_creator_profile(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Creator, String> {
        let prof = self
            .client
            .fetch_creator_profile(service, creator_id)
            .await?;
        Ok(Creator {
            id: prof.id,
            name: prof.name,
            service: prof.service,
            public_id: prof.public_id,
            relation_id: prof.relation_id,
            indexed: None,
            updated: None,
            favorited: prof.kemono_favorited,
            kemono_favorited: prof.kemono_favorited,
            ever_imported: prof.ever_imported,
            extra: prof.extra,
        })
    }

    async fn fetch_creator_links(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<CreatorProfile>, String> {
        self.client.fetch_creator_links(service, creator_id).await
    }

    async fn fetch_similar_creators(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<CreatorProfile>, String> {
        self.client
            .fetch_similar_creators(service, creator_id)
            .await
    }

    async fn fetch_creator_tags(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<String>, String> {
        self.client.fetch_creator_tags(service, creator_id).await
    }

    async fn fetch_announcements(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<Announcement>, String> {
        self.client.fetch_announcements(service, creator_id).await
    }

    async fn fetch_posts(
        &self,
        service: &str,
        creator_id: &str,
        offset: u32,
        query: Option<&str>,
    ) -> Result<Vec<Post>, String> {
        self.client
            .fetch_creator_posts(service, creator_id, query, offset)
            .await
    }

    async fn fetch_post(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Option<Post>, String> {
        match self.client.fetch_post(service, creator_id, post_id).await {
            Ok(post) => Ok(Some(post)),
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
        self.client
            .fetch_post_revisions(service, creator_id, post_id)
            .await
    }

    async fn fetch_recent_posts(
        &self,
        query: Option<&str>,
        offset: u32,
    ) -> Result<Vec<Post>, String> {
        self.client.fetch_recent_posts(query, offset).await
    }

    async fn fetch_popular_posts(
        &self,
        period: &str,
        date: Option<&str>,
        offset: u32,
    ) -> Result<Vec<Post>, String> {
        self.client.fetch_popular_posts(period, date, offset).await
    }

    async fn fetch_post_comments(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Vec<Comment>, String> {
        self.client
            .fetch_post_comments(service, creator_id, post_id)
            .await
    }

    async fn fetch_account_favorites(
        &self,
        favorite_type: Option<&str>,
    ) -> Result<Vec<Favorite>, String> {
        self.client.fetch_account_favorites(favorite_type).await
    }

    async fn set_creator_favorite(
        &self,
        service: &str,
        creator_id: &str,
        favorite: bool,
    ) -> Result<ApiActionResult, String> {
        self.client
            .set_creator_favorite(service, creator_id, favorite)
            .await
    }

    async fn set_post_favorite(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
        favorite: bool,
    ) -> Result<ApiActionResult, String> {
        self.client
            .set_post_favorite(service, creator_id, post_id, favorite)
            .await
    }

    fn resolve_media_url(&self, file_path: &str, server: Option<&str>) -> String {
        let conf = self.config.read().unwrap();
        let clean = file_path
            .trim_start_matches('/')
            .trim_start_matches("data/")
            .trim_start_matches('/');

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
                    .unwrap_or_else(|| derive_subdomain_url(&conf.api_url, "file"));

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

        let base = derive_subdomain_url(&conf.api_url, "file");
        format!("{base}/data/{clean}")
    }

    fn resolve_thumbnail_url(&self, thumb_path: &str) -> String {
        let conf = self.config.read().unwrap();
        let clean = thumb_path
            .trim_start_matches('/')
            .trim_start_matches("data/")
            .trim_start_matches('/');

        if let Some(img_url) = conf.image_url.as_deref().filter(|s| !s.trim().is_empty()) {
            let base = if img_url.starts_with("http://") || img_url.starts_with("https://") {
                img_url.trim_end_matches('/').to_string()
            } else {
                format!("https://{}", img_url.trim_end_matches('/'))
            };
            return format!("{base}/thumbnail/data/{clean}");
        }

        let base = derive_subdomain_url(&conf.api_url, "image");
        format!("{base}/thumbnail/data/{clean}")
    }

    async fn fetch_creator_artwork_data_url(
        &self,
        service: &str,
        creator_id: &str,
        artwork_type: &str,
    ) -> Result<String, String> {
        self.client
            .fetch_creator_artwork_data_url(service, creator_id, artwork_type)
            .await
    }

    async fn search_hash(&self, file_hash: &str) -> Result<FileSearchResult, String> {
        self.client.search_hash(file_hash).await
    }

    async fn fetch_fancards(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<Fancard>, String> {
        self.client.fetch_fancards(service, creator_id).await
    }

    async fn flag_post(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<ApiActionResult, String> {
        self.client.flag_post(service, creator_id, post_id).await
    }

    async fn is_post_flagged(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<bool, String> {
        self.client
            .is_post_flagged(service, creator_id, post_id)
            .await
    }

    async fn login(&self, username: &str, password: &str) -> Result<String, String> {
        self.client.login(username, password).await
    }

    async fn logout(&self) -> Result<(), String> {
        self.client.logout().await
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
        self.client.app_version().await
    }

    async fn resolve_post_identity(
        &self,
        service: &str,
        post_id: &str,
    ) -> Result<Option<(String, String, String)>, String> {
        self.client.resolve_post_identity(service, post_id).await
    }

    async fn expand_short_link(&self, raw_url: &str) -> Result<Option<String>, String> {
        self.client.expand_short_link(raw_url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_cookie_is_normalized() {
        assert_eq!(
            PawchiveClient::cookie_header("token")
                .unwrap()
                .to_str()
                .unwrap(),
            "session=token"
        );
        assert_eq!(
            PawchiveClient::cookie_header("session=token; other=x")
                .unwrap()
                .to_str()
                .unwrap(),
            "session=token; other=x"
        );
    }

    #[test]
    fn mirror_with_scheme_is_supported() {
        assert_eq!(
            PawchiveClient::base_url("http://localhost:8080/"),
            "http://localhost:8080/api/v1"
        );
    }

    #[test]
    fn server_subdomain_prefix_resolution() {
        let conf = ProviderConfig {
            id: "pawchive".into(),
            name: "Pawchive".into(),
            enabled: true,
            api_url: "https://pawchive.pw".into(),
            fallback_urls: vec![],
            file_url: Some("https://file.pawchive.pw".into()),
            image_url: Some("https://img.pawchive.pw".into()),
            session_cookie: String::new(),
            username: String::new(),
            services: vec![],
            is_custom: false,
            priority: 1,
        };
        let provider = PawchiveProvider::new(conf).unwrap();
        assert_eq!(
            provider.resolve_media_url("/data/ab/cd/video.mp4", Some("file1")),
            "https://file1.pawchive.pw/data/ab/cd/video.mp4"
        );
        assert_eq!(
            provider.resolve_media_url("/data/ab/cd/video.mp4", Some("https://cdn.example.com")),
            "https://cdn.example.com/data/ab/cd/video.mp4"
        );
        assert_eq!(
            provider.resolve_media_url("/data/ab/cd/video.mp4", None),
            "https://file.pawchive.pw/data/ab/cd/video.mp4"
        );
        assert_eq!(
            provider.resolve_media_url("data/ab/cd/video.mp4", None),
            "https://file.pawchive.pw/data/ab/cd/video.mp4"
        );
        assert_eq!(
            provider.resolve_media_url("ab/cd/video.mp4", None),
            "https://file.pawchive.pw/data/ab/cd/video.mp4"
        );
        assert_eq!(
            provider.resolve_thumbnail_url("/data/ab/cd/thumb.jpg"),
            "https://img.pawchive.pw/thumbnail/data/ab/cd/thumb.jpg"
        );
        assert_eq!(
            provider.resolve_thumbnail_url("data/ab/cd/thumb.jpg"),
            "https://img.pawchive.pw/thumbnail/data/ab/cd/thumb.jpg"
        );
        assert_eq!(
            provider.resolve_thumbnail_url("ab/cd/thumb.jpg"),
            "https://img.pawchive.pw/thumbnail/data/ab/cd/thumb.jpg"
        );

        let fallback_conf = ProviderConfig {
            id: "pawchive".into(),
            name: "Pawchive".into(),
            enabled: true,
            api_url: "https://pawchive.pw".into(),
            fallback_urls: vec![],
            file_url: None,
            image_url: None,
            session_cookie: String::new(),
            username: String::new(),
            services: vec![],
            is_custom: false,
            priority: 1,
        };
        let fallback_provider = PawchiveProvider::new(fallback_conf).unwrap();
        assert_eq!(
            fallback_provider.resolve_media_url("/data/ab/cd/video.mp4", None),
            "https://file.pawchive.pw/data/ab/cd/video.mp4"
        );
        assert_eq!(
            fallback_provider.resolve_thumbnail_url("/data/ab/cd/thumb.jpg"),
            "https://img.pawchive.pw/thumbnail/data/ab/cd/thumb.jpg"
        );
    }

    #[test]
    fn popular_cards_include_server_favorite_counts() {
        let html = r#"
          <article class="post-card" data-id="42" data-service="patreon" data-user="7">
            <header class="post-card__header">Freaky Horse</header>
            <div class="post-card__image-container">
              <img class="post-card__image" src="https://img.pawchive.pw/thumbnail/data/aa/bb/file.png">
            </div>
            <footer class="post-card__footer">
              <time datetime="2026-06-27 06:10:39"></time>
              <div>2 attachments<br>212 favorites</div>
            </footer>
          </article>
        "#;
        let posts = PawchiveClient::parse_popular_posts(html).unwrap();
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].title, "Freaky Horse");
        assert_eq!(posts[0].attachment_count, Some(2));
        assert_eq!(posts[0].favorite_count, Some(212));
        assert_eq!(
            posts[0].file.as_ref().unwrap().path.as_deref(),
            Some("/aa/bb/file.png")
        );
    }

    #[tokio::test]
    #[ignore = "requires live pawchive.pw"]
    async fn live_public_contracts() {
        let client = PawchiveClient::new(AppSettings::default()).unwrap();
        assert!(!client.fetch_recent_posts(None, 0).await.unwrap().is_empty());
        assert!(!client
            .fetch_popular_posts("day", None, 0)
            .await
            .unwrap()
            .is_empty());
        assert_eq!(
            client
                .fetch_creator_profile("patreon", "3340149")
                .await
                .unwrap()
                .id,
            "3340149"
        );
        assert!(!client
            .fetch_creator_posts("patreon", "3340149", None, 0)
            .await
            .unwrap()
            .is_empty());
        client
            .fetch_announcements("patreon", "8693043")
            .await
            .unwrap();
        client.fetch_fancards("fanbox", "3316400").await.unwrap();
        client
            .fetch_creator_links("patreon", "3340149")
            .await
            .unwrap();
        assert_eq!(
            client
                .fetch_post("patreon", "3340149", "142680139")
                .await
                .unwrap()
                .id,
            "142680139"
        );
        assert!(!client
            .fetch_post_comments("fanbox", "6570768", "1836570")
            .await
            .unwrap()
            .is_empty());
        assert!(!client
            .is_post_flagged("fanbox", "6570768", "1836570")
            .await
            .unwrap());
        assert!(!client.app_version().await.unwrap().trim().is_empty());
    }

    #[tokio::test]
    #[ignore = "requires live pawchive.pw"]
    async fn live_popular_contract() {
        let client = PawchiveClient::new(AppSettings::default()).unwrap();
        for period in ["day", "week", "month"] {
            let posts = client.fetch_popular_posts(period, None, 0).await.unwrap();
            assert!(!posts.is_empty(), "{period} popular feed is empty");
            assert!(
                posts.iter().all(|post| post.favorite_count.is_some()),
                "{period} popular feed is missing favorite counts"
            );
        }
    }
}
