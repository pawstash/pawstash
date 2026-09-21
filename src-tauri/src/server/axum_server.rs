use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use cipher::StreamCipher;
use ctr::cipher::KeyIvInit;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, UNIX_EPOCH};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio_util::io::ReaderStream;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

pub struct MediaServer {
    pub port: u16,
}

const PROXY_HOST_TTL: Duration = Duration::from_secs(60);

const FILE_STREAM_CHUNK: usize = 256 * 1024;

struct ResolvedRoots {
    download_dir: String,
    roots: Vec<PathBuf>,
}

struct CachedClients {
    fingerprint: String,
    manual_redirect: reqwest::Client,
    follow_redirect: reqwest::Client,
}

pub struct MediaServerState {
    pub allowed_roots: Vec<PathBuf>,
    pub config_manager: Arc<crate::config::ConfigManager>,
    pub token: String,
    proxy_client: tokio::sync::RwLock<Option<CachedClients>>,
    proxy_hosts: tokio::sync::Mutex<HashMap<String, Instant>>,
    resolved_roots: tokio::sync::RwLock<Option<ResolvedRoots>>,
}

impl MediaServerState {
    fn new(
        allowed_roots: Vec<PathBuf>,
        config_manager: Arc<crate::config::ConfigManager>,
        token: String,
    ) -> Self {
        Self {
            allowed_roots,
            config_manager,
            token,
            proxy_client: tokio::sync::RwLock::new(None),
            proxy_hosts: tokio::sync::Mutex::new(HashMap::new()),
            resolved_roots: tokio::sync::RwLock::new(None),
        }
    }

    fn proxy_fingerprint(settings: &crate::config::AppSettings) -> String {
        format!(
            "{:?}\u{1}{}\u{1}{}\u{1}{}\u{1}{}",
            settings.proxy_mode,
            settings.proxy_url.trim(),
            settings.proxy_username,
            settings.proxy_password,
            settings.proxy_bypass_local
        )
    }

    async fn upstream_clients(
        &self,
        settings: &crate::config::AppSettings,
    ) -> Result<(reqwest::Client, reqwest::Client), String> {
        let fingerprint = Self::proxy_fingerprint(settings);
        if let Some(cached) = self.proxy_client.read().await.as_ref() {
            if cached.fingerprint == fingerprint {
                return Ok((
                    cached.manual_redirect.clone(),
                    cached.follow_redirect.clone(),
                ));
            }
        }

        let base = || -> Result<reqwest::ClientBuilder, String> {
            Ok(crate::net::builder_with_proxy(settings)?
                .no_gzip()
                .no_brotli()
                .no_deflate()
                .timeout(Duration::from_secs(60))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36"))
        };

        let manual_redirect = base()?
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| e.to_string())?;
        let follow_redirect = base()?
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| e.to_string())?;

        *self.proxy_client.write().await = Some(CachedClients {
            fingerprint,
            manual_redirect: manual_redirect.clone(),
            follow_redirect: follow_redirect.clone(),
        });
        Ok((manual_redirect, follow_redirect))
    }

    async fn host_recently_validated(&self, host: &str) -> bool {
        let mut hosts = self.proxy_hosts.lock().await;
        match hosts.get(host) {
            Some(expiry) if *expiry > Instant::now() => true,
            Some(_) => {
                hosts.remove(host);
                false
            }
            None => false,
        }
    }

    async fn remember_validated_host(&self, host: &str) {
        let mut hosts = self.proxy_hosts.lock().await;
        if hosts.len() > 256 {
            let now = Instant::now();
            hosts.retain(|_, expiry| *expiry > now);
            if hosts.len() > 256 {
                hosts.clear();
            }
        }
        hosts.insert(host.to_string(), Instant::now() + PROXY_HOST_TTL);
    }

    async fn allowed_roots_for(&self, download_dir: &str) -> Vec<PathBuf> {
        if let Some(cached) = self.resolved_roots.read().await.as_ref() {
            if cached.download_dir == download_dir {
                return cached.roots.clone();
            }
        }

        let mut candidates = self.allowed_roots.clone();
        let user_dir = PathBuf::from(download_dir);
        if !candidates.contains(&user_dir) {
            candidates.push(user_dir);
        }
        #[cfg(target_os = "android")]
        {
            if let Ok(ensured) =
                crate::downloader::manager::DownloadManager::ensure_download_root(download_dir)
            {
                if !candidates.contains(&ensured) {
                    candidates.push(ensured);
                }
            }
        }

        let roots = tokio::task::spawn_blocking(move || {
            candidates
                .iter()
                .filter_map(|root| dunce::canonicalize(root).ok())
                .collect::<Vec<PathBuf>>()
        })
        .await
        .unwrap_or_default();

        *self.resolved_roots.write().await = Some(ResolvedRoots {
            download_dir: download_dir.to_string(),
            roots: roots.clone(),
        });
        roots
    }
}

impl MediaServer {
    pub async fn start(
        allowed_roots: Vec<PathBuf>,
        config_manager: Arc<crate::config::ConfigManager>,
        token: String,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cors = CorsLayer::new()
            .allow_origin([
                HeaderValue::from_static("tauri://localhost"),
                HeaderValue::from_static("http://tauri.localhost"),
                HeaderValue::from_static("https://tauri.localhost"),
                HeaderValue::from_static("http://localhost:1420"),
                HeaderValue::from_static("http://127.0.0.1:1420"),
            ])
            .allow_methods([Method::GET, Method::HEAD])
            .allow_headers([
                header::RANGE,
                header::CONTENT_TYPE,
                header::ACCEPT,
                header::IF_NONE_MATCH,
                header::IF_MODIFIED_SINCE,
            ])
            .expose_headers([
                header::CONTENT_LENGTH,
                header::CONTENT_RANGE,
                header::ACCEPT_RANGES,
                header::ETAG,
                header::CACHE_CONTROL,
            ]);

        let state = Arc::new(MediaServerState::new(allowed_roots, config_manager, token));

        let app = Router::new()
            .route("/media/*file_path", get(serve_media_handler))
            .route("/cloud_stream/mega", get(serve_mega_stream_handler))
            .route(
                "/cloud_stream/proxy",
                get(serve_cloud_proxy_stream_handler).head(serve_cloud_proxy_stream_handler),
            )
            .with_state(state)
            .layer(cors);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let port = addr.port();

        info!("Embedded Axum Media Server listening on http://{}", addr);

        tokio::spawn(async move {
            if let Err(err) = axum::serve(listener, app).await {
                error!("Axum media server error: {}", err);
            }
        });

        Ok(MediaServer { port })
    }
}

async fn serve_media_handler(
    State(state): State<Arc<MediaServerState>>,
    Path(file_path): Path<String>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    require_token(&state, &auth.token)?;
    let decoded_path = urlencoding::decode(&file_path)
        .map(|s| s.into_owned())
        .unwrap_or(file_path);

    #[cfg(not(windows))]
    let path_str = if !decoded_path.starts_with('/') {
        format!("/{decoded_path}")
    } else {
        decoded_path
    };
    #[cfg(windows)]
    let path_str = {
        let trimmed = decoded_path.trim_start_matches('/');
        if trimmed.len() >= 2 && trimmed.chars().nth(1) == Some(':') {
            trimmed.to_string()
        } else {
            decoded_path
        }
    };

    let path = PathBuf::from(&path_str);
    let target = tokio::task::spawn_blocking(move || dunce::canonicalize(&path))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|_| (StatusCode::NOT_FOUND, "File not found".to_string()))?;

    let settings = state
        .config_manager
        .load()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let allowed_roots = state.allowed_roots_for(&settings.download_dir).await;
    let allowed = allowed_roots.iter().any(|root| target.starts_with(root));

    if !allowed {
        let is_media_extension = target
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| {
                matches!(
                    e.to_ascii_lowercase().as_str(),
                    "png"
                        | "jpg"
                        | "jpeg"
                        | "webp"
                        | "gif"
                        | "avif"
                        | "mp4"
                        | "webm"
                        | "m4v"
                        | "mov"
                )
            })
            .unwrap_or(false);

        if !is_media_extension {
            tracing::warn!("Local media access denied for target: {:?}", target);
            return Err((StatusCode::NOT_FOUND, "File not found".to_string()));
        }
    }

    let file_metadata = tokio::fs::metadata(&target)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "File not found".to_string()))?;
    if !file_metadata.is_file() {
        return Err((StatusCode::NOT_FOUND, "File not found".to_string()));
    }
    let file_size = file_metadata.len();

    let mime_type = mime_guess::from_path(&target)
        .first_or_octet_stream()
        .to_string();

    let etag = file_etag(&file_metadata);
    let cache_headers = |res_headers: &mut HeaderMap| {
        res_headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, max-age=86400"),
        );
        if let Some(ref tag) = etag {
            if let Ok(val) = HeaderValue::from_str(tag) {
                res_headers.insert(header::ETAG, val);
            }
        }
    };

    if let Some(ref tag) = etag {
        let matches = headers
            .get(header::IF_NONE_MATCH)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| {
                value == "*" || value.split(',').any(|candidate| candidate.trim() == tag)
            });
        if matches {
            let mut response = StatusCode::NOT_MODIFIED.into_response();
            cache_headers(response.headers_mut());
            return Ok(response);
        }
    }

    if let Some(range_header) = headers.get(header::RANGE) {
        if let Ok(range_str) = range_header.to_str() {
            if let Some(range) = parse_range(range_str, file_size) {
                let (start, end) = range;
                let chunk_size = end - start + 1;

                let mut file = File::open(&target)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

                file.seek(SeekFrom::Start(start))
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

                let limited_file = file.take(chunk_size);
                let stream = ReaderStream::with_capacity(limited_file, FILE_STREAM_CHUNK);

                let body = Body::from_stream(stream);

                let mut response = (StatusCode::PARTIAL_CONTENT, body).into_response();
                let res_headers = response.headers_mut();

                if let Ok(val) = HeaderValue::from_str(&mime_type) {
                    res_headers.insert(header::CONTENT_TYPE, val);
                }
                res_headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
                if let Ok(val) =
                    HeaderValue::from_str(&format!("bytes {}-{}/{}", start, end, file_size))
                {
                    res_headers.insert(header::CONTENT_RANGE, val);
                }
                res_headers.insert(header::CONTENT_LENGTH, HeaderValue::from(chunk_size));
                cache_headers(res_headers);
                return Ok(response);
            }
        }
    }

    let file = File::open(&target)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let stream = ReaderStream::with_capacity(file, FILE_STREAM_CHUNK);
    let body = Body::from_stream(stream);

    let mut response = (StatusCode::OK, body).into_response();
    let res_headers = response.headers_mut();

    if let Ok(val) = HeaderValue::from_str(&mime_type) {
        res_headers.insert(header::CONTENT_TYPE, val);
    }
    res_headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    res_headers.insert(header::CONTENT_LENGTH, HeaderValue::from(file_size));
    cache_headers(res_headers);
    Ok(response)
}

fn file_etag(metadata: &std::fs::Metadata) -> Option<String> {
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis();
    Some(format!("\"{:x}-{:x}\"", metadata.len(), modified))
}

#[derive(Debug, Deserialize)]
struct AuthQuery {
    token: String,
}

fn require_token(state: &MediaServerState, provided: &str) -> Result<(), (StatusCode, String)> {
    if provided.as_bytes() == state.token.as_bytes() {
        Ok(())
    } else {
        Err((StatusCode::NOT_FOUND, "Not found".to_string()))
    }
}

#[derive(Debug, Deserialize)]
struct MegaStreamQuery {
    token: String,
    folder_id: Option<String>,
    file_id: Option<String>,
    node_id: Option<String>,
    key: String,
    name: Option<String>,
}

fn mega_decode_key_param(raw: &str) -> Result<Vec<u8>, String> {
    let mut clean = raw.trim().replace('-', "+").replace('_', "/");
    while !clean.len().is_multiple_of(4) {
        clean.push('=');
    }
    base64::engine::general_purpose::STANDARD
        .decode(&clean)
        .or_else(|_| URL_SAFE_NO_PAD.decode(raw))
        .map_err(|e| format!("Base64 decode failed: {e}"))
}

async fn serve_mega_stream_handler(
    State(state): State<Arc<MediaServerState>>,
    Query(params): Query<MegaStreamQuery>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    require_token(&state, &params.token)?;
    let key_bytes = mega_decode_key_param(&params.key).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let (cipher_key, nonce) = if key_bytes.len() >= 32 {
        let mut k = [0u8; 16];
        for i in 0..16 {
            k[i] = key_bytes[i] ^ key_bytes[i + 16];
        }
        let mut iv_nonce = [0u8; 8];
        iv_nonce.copy_from_slice(&key_bytes[16..24]);
        (k, iv_nonce)
    } else if key_bytes.len() >= 24 {
        let mut k = [0u8; 16];
        k.copy_from_slice(&key_bytes[..16]);
        let mut iv_nonce = [0u8; 8];
        iv_nonce.copy_from_slice(&key_bytes[16..24]);
        (k, iv_nonce)
    } else if key_bytes.len() >= 16 {
        let mut k = [0u8; 16];
        k.copy_from_slice(&key_bytes[..16]);
        (k, [0u8; 8])
    } else {
        return Err((StatusCode::BAD_REQUEST, "Invalid key length".to_string()));
    };

    let settings = state
        .config_manager
        .load()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let (_, client) = state
        .upstream_clients(&settings)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let payload = if let Some(ref nid) = params.node_id {
        json!([{"a": "g", "g": 1, "n": nid}])
    } else if let Some(ref fid) = params.file_id {
        json!([{"a": "g", "g": 1, "p": fid}])
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            "Missing node_id or file_id".to_string(),
        ));
    };

    let api_url = crate::cloud::mega::api_url(params.folder_id.as_deref());

    let api_resp = client
        .post(&api_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let api_vals: Vec<serde_json::Value> = api_resp
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let first = api_vals
        .into_iter()
        .next()
        .ok_or_else(|| (StatusCode::BAD_GATEWAY, "Empty MEGA response".to_string()))?;
    let g_url = first.get("g").and_then(|v| v.as_str()).ok_or_else(|| {
        (
            StatusCode::BAD_GATEWAY,
            "Missing stream URL from MEGA".to_string(),
        )
    })?;
    let total_size = first.get("s").and_then(|v| v.as_u64()).unwrap_or(0);

    let filename = params.name.unwrap_or_else(|| "file.mp4".to_string());
    let mime = mime_guess::from_path(&filename)
        .first_or_octet_stream()
        .to_string();

    let (range_start, range_end) = if let Some(range_header) = headers.get(header::RANGE) {
        if let Ok(range_str) = range_header.to_str() {
            if let Some(r) = parse_range(range_str, total_size) {
                r
            } else {
                (0, total_size.saturating_sub(1))
            }
        } else {
            (0, total_size.saturating_sub(1))
        }
    } else {
        (0, total_size.saturating_sub(1))
    };

    let chunk_size = if total_size > 0 {
        range_end - range_start + 1
    } else {
        0
    };

    let block_index = range_start / 16;
    let block_aligned_start = block_index * 16;
    let skip_bytes = (range_start % 16) as usize;

    let mut mega_req = client.get(g_url);
    if total_size > 0 {
        mega_req = mega_req.header(
            header::RANGE,
            format!("bytes={block_aligned_start}-{range_end}"),
        );
    }

    let mega_resp = mega_req
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let mut iv = [0u8; 16];
    iv[0..8].copy_from_slice(&nonce);
    iv[8..16].copy_from_slice(&block_index.to_be_bytes());

    let byte_stream = mega_resp.bytes_stream();
    let decrypted_stream = async_stream::stream! {
        let mut cipher = ctr::Ctr64BE::<aes::Aes128>::new((&cipher_key).into(), (&iv).into());
        let mut pending_skip = skip_bytes;

        for await chunk_res in byte_stream {
            match chunk_res {
                Ok(bytes) => {
                    let mut data = bytes.to_vec();
                    cipher.apply_keystream(&mut data);
                    if pending_skip > 0 {
                        if data.len() <= pending_skip {
                            pending_skip -= data.len();
                            continue;
                        } else {
                            let yield_data = data[pending_skip..].to_vec();
                            pending_skip = 0;
                            yield Ok::<_, std::io::Error>(bytes::Bytes::from(yield_data));
                        }
                    } else {
                        yield Ok::<_, std::io::Error>(bytes::Bytes::from(data));
                    }
                }
                Err(e) => {
                    yield Err(std::io::Error::other(e.to_string()));
                }
            }
        }
    };

    let body = Body::from_stream(decrypted_stream);
    let mut response = if headers.contains_key(header::RANGE) && total_size > 0 {
        let mut resp = (StatusCode::PARTIAL_CONTENT, body).into_response();
        if let Ok(val) =
            HeaderValue::from_str(&format!("bytes {range_start}-{range_end}/{total_size}"))
        {
            resp.headers_mut().insert(header::CONTENT_RANGE, val);
        }
        resp
    } else {
        (StatusCode::OK, body).into_response()
    };

    let res_headers = response.headers_mut();
    if let Ok(val) = HeaderValue::from_str(&mime) {
        res_headers.insert(header::CONTENT_TYPE, val);
    }
    res_headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    if chunk_size > 0 {
        res_headers.insert(header::CONTENT_LENGTH, HeaderValue::from(chunk_size));
    }

    Ok(response)
}

fn parse_range(range_str: &str, file_size: u64) -> Option<(u64, u64)> {
    if !range_str.starts_with("bytes=") {
        return None;
    }

    let range_val = &range_str[6..];
    let parts: Vec<&str> = range_val.split('-').collect();
    if parts.is_empty() {
        return None;
    }

    let start = parts[0].parse::<u64>().ok()?;
    let end = if parts.len() > 1 && !parts[1].is_empty() {
        parts[1].parse::<u64>().ok()?
    } else {
        file_size.saturating_sub(1)
    };

    if start > end || end >= file_size {
        return None;
    }

    Some((start, end))
}

#[derive(Deserialize)]
struct CloudProxyParams {
    token: String,
    url: String,
    name: Option<String>,
}

async fn serve_cloud_proxy_stream_handler(
    State(state): State<Arc<MediaServerState>>,
    method: Method,
    Query(params): Query<CloudProxyParams>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    require_token(&state, &params.token)?;
    let settings = state
        .config_manager
        .load()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let (client, _) = state
        .upstream_clients(&settings)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let is_head = method == Method::HEAD;
    let normalized_url = crate::downloader::normalize_download_url(&params.url);
    let mut target_url = validate_proxy_target(&state, &normalized_url).await?;
    let mut redirect_count = 0;
    let upstream = loop {
        let target_text = target_url.as_str();
        let mut req = if is_head {
            client.head(target_url.clone())
        } else {
            client.get(target_url.clone())
        };
        req = req.headers(crate::downloader::derive_download_headers(target_text));
        if let Some(referer_url) = crate::downloader::derive_download_referer(target_text) {
            if let Ok(ref_val) = header::HeaderValue::from_str(&referer_url) {
                req = req.header(header::REFERER, ref_val);
            }
        }
        let resolved_cookie = settings
            .resolve_cookie_for_url(target_text)
            .unwrap_or_default();
        if let Some(cookie_val) =
            crate::downloader::derive_download_cookie(target_text, &resolved_cookie)
        {
            if let Ok(val) = header::HeaderValue::from_str(&cookie_val) {
                req = req.header(header::COOKIE, val);
            }
        }
        if let Some(range) = headers.get(header::RANGE) {
            if let Ok(range_val) = range.to_str() {
                req = req.header(header::RANGE, range_val);
            }
        }
        let response = req.send().await.map_err(|e| {
            tracing::error!("Cloud proxy stream request failed: {e}");
            (
                StatusCode::BAD_GATEWAY,
                format!("Cloud upstream request failed: {e}"),
            )
        })?;
        if !response.status().is_redirection() {
            break response;
        }
        if redirect_count >= 5 {
            return Err((
                StatusCode::BAD_GATEWAY,
                "Cloud upstream exceeded the redirect limit".to_string(),
            ));
        }
        let location = response
            .headers()
            .get(header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                (
                    StatusCode::BAD_GATEWAY,
                    "Cloud upstream returned an invalid redirect".to_string(),
                )
            })?;
        let redirected = target_url.join(location).map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Cloud upstream returned an invalid redirect URL: {e}"),
            )
        })?;
        target_url = validate_proxy_target(&state, redirected.as_str()).await?;
        redirect_count += 1;
    };

    let status = upstream.status();
    if !status.is_success() && status != StatusCode::PARTIAL_CONTENT {
        tracing::warn!(
            "Cloud proxy upstream returned HTTP status {} for target '{}'",
            status,
            target_url
        );
        return Err((
            status,
            format!("Upstream media server returned error: {status}"),
        ));
    }

    let upstream_headers = upstream.headers().clone();
    let upstream_content_type = upstream_headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let final_upstream = if upstream_content_type.starts_with("text/html") {
        let html_bytes = upstream.bytes().await.unwrap_or_default();
        let html_str = String::from_utf8_lossy(&html_bytes);

        if let Some(confirmed_resp) = crate::cloud::follow_cloud_stream_html_warning(
            &client,
            &target_url,
            &upstream_headers,
            &html_str,
            is_head,
            headers.get(header::RANGE),
        )
        .await
        {
            confirmed_resp
        } else {
            tracing::warn!(
                "Cloud proxy upstream returned HTML instead of media for target '{}' (file may be deleted)",
                target_url
            );
            return Err((
                StatusCode::NOT_FOUND,
                "Upstream returned HTML page instead of media stream".to_string(),
            ));
        }
    } else {
        upstream
    };

    let status = final_upstream.status();
    let upstream_headers = final_upstream.headers().clone();
    let upstream_content_type = upstream_headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let body = if is_head {
        Body::empty()
    } else {
        Body::from_stream(final_upstream.bytes_stream())
    };

    let mut response = Response::builder()
        .status(status)
        .body(body)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let resp_headers = response.headers_mut();
    for (k, v) in upstream_headers.iter() {
        if k == header::CONTENT_TYPE
            || k == header::CONTENT_LENGTH
            || k == header::CONTENT_RANGE
            || k == header::ACCEPT_RANGES
        {
            resp_headers.insert(k.clone(), v.clone());
        }
    }

    let is_not_media = !upstream_content_type.starts_with("video/")
        && !upstream_content_type.starts_with("audio/")
        && !upstream_content_type.starts_with("image/");

    if is_not_media {
        let extracted_path =
            if let Some(name) = params.name.as_deref().filter(|s| !s.trim().is_empty()) {
                Some(name.to_string())
            } else {
                Some(target_url.path().to_string())
            };

        if let Some(path_str) = extracted_path {
            if let Some(mime) = mime_guess::from_path(&path_str).first() {
                if let Ok(val) = mime.as_ref().parse() {
                    resp_headers.insert(header::CONTENT_TYPE, val);
                }
            }
        }
    }

    if !resp_headers.contains_key(header::ACCEPT_RANGES) {
        if let Ok(val) = "bytes".parse() {
            resp_headers.insert(header::ACCEPT_RANGES, val);
        }
    }

    Ok(response)
}

async fn validate_proxy_target(
    state: &MediaServerState,
    raw: &str,
) -> Result<reqwest::Url, (StatusCode, String)> {
    let url = reqwest::Url::parse(raw)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid cloud URL: {e}")))?;
    if url.scheme() != "https"
        || url.username() != ""
        || url.password().is_some()
        || url.port().is_some()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "Cloud proxy only accepts credential-free HTTPS URLs on port 443".to_string(),
        ));
    }
    let host = url
        .host_str()
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "Cloud proxy URL has no host".to_string(),
            )
        })?
        .to_ascii_lowercase();

    if state.host_recently_validated(&host).await {
        return Ok(url);
    }

    let addresses = tokio::net::lookup_host((host.as_str(), 443))
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Cloud host lookup failed: {e}"),
            )
        })?;
    let mut found = false;
    for address in addresses {
        found = true;
        if !is_public_ip(address.ip()) {
            return Err((
                StatusCode::FORBIDDEN,
                "Cloud proxy cannot access local or private network addresses".to_string(),
            ));
        }
    }
    if !found {
        return Err((
            StatusCode::BAD_GATEWAY,
            "Cloud host did not resolve to an address".to_string(),
        ));
    }
    state.remember_validated_host(&host).await;
    Ok(url)
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => {
            if let Some(mapped) = ip.to_ipv4_mapped() {
                return is_public_ipv4(mapped);
            }
            !ip.is_loopback()
                && !ip.is_unspecified()
                && !ip.is_multicast()
                && !ip.is_unique_local()
                && !ip.is_unicast_link_local()
                && !is_ipv6_documentation(ip)
        }
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    !ip.is_private()
        && !ip.is_loopback()
        && !ip.is_link_local()
        && !ip.is_broadcast()
        && !ip.is_documentation()
        && !ip.is_unspecified()
        && !ip.is_multicast()
        && octets[0] != 0
        && octets[0] < 240
}

fn is_ipv6_documentation(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    segments[0] == 0x2001 && segments[1] == 0x0db8
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_is_public_ipv4_allows_fakeip_and_cgnat() {
        assert!(is_public_ipv4(Ipv4Addr::new(198, 18, 0, 1)));
        assert!(is_public_ipv4(Ipv4Addr::new(198, 18, 0, 37)));
        assert!(is_public_ipv4(Ipv4Addr::new(198, 19, 255, 254)));

        assert!(is_public_ipv4(Ipv4Addr::new(100, 64, 0, 1)));
        assert!(is_public_ipv4(Ipv4Addr::new(100, 100, 100, 100)));

        assert!(is_public_ipv4(Ipv4Addr::new(1, 1, 1, 1)));
        assert!(is_public_ipv4(Ipv4Addr::new(8, 8, 8, 8)));
    }

    #[test]
    fn test_is_public_ipv4_blocks_private_and_local() {
        assert!(!is_public_ipv4(Ipv4Addr::new(127, 0, 0, 1)));
        assert!(!is_public_ipv4(Ipv4Addr::new(192, 168, 1, 1)));
        assert!(!is_public_ipv4(Ipv4Addr::new(10, 0, 0, 1)));
        assert!(!is_public_ipv4(Ipv4Addr::new(172, 16, 0, 1)));
        assert!(!is_public_ipv4(Ipv4Addr::new(169, 254, 169, 254)));
        assert!(!is_public_ipv4(Ipv4Addr::new(255, 255, 255, 255)));
        assert!(!is_public_ipv4(Ipv4Addr::new(224, 0, 0, 1)));
        assert!(!is_public_ipv4(Ipv4Addr::new(0, 0, 0, 0)));
    }

    fn test_state() -> Option<MediaServerState> {
        let config_manager = crate::config::ConfigManager::new().ok()?;
        Some(MediaServerState::new(
            Vec::new(),
            Arc::new(config_manager),
            "test-token".to_string(),
        ))
    }

    #[tokio::test]
    async fn test_validate_proxy_target_dropbox() {
        let Some(state) = test_state() else { return };
        let res = validate_proxy_target(&state, crate::cloud::dropbox::example_url()).await;

        match res {
            Ok(_) => {}
            Err((StatusCode::BAD_GATEWAY, message)) if message.contains("lookup failed") => {}
            Err(other) => panic!("Failed to validate proxy target: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_validated_host_is_cached_then_expires() {
        let Some(state) = test_state() else { return };
        assert!(!state.host_recently_validated("example.com").await);

        state.remember_validated_host("example.com").await;
        assert!(state.host_recently_validated("example.com").await);

        state
            .proxy_hosts
            .lock()
            .await
            .insert("example.com".to_string(), Instant::now());
        assert!(!state.host_recently_validated("example.com").await);
    }

    #[tokio::test]
    async fn test_private_targets_are_rejected_and_not_cached() {
        let Some(state) = test_state() else { return };
        let res = validate_proxy_target(&state, "https://localhost/file.mp4").await;
        assert!(res.is_err(), "loopback target must be rejected");
        assert!(!state.host_recently_validated("localhost").await);
    }

    #[test]
    fn test_file_etag_tracks_size_and_mtime() {
        let dir = std::env::temp_dir().join(format!("pawstash-etag-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("sample.bin");

        std::fs::write(&path, b"one").expect("write");
        let first = file_etag(&std::fs::metadata(&path).expect("metadata"));

        std::fs::write(&path, b"a longer body").expect("rewrite");
        let second = file_etag(&std::fs::metadata(&path).expect("metadata"));

        assert!(first.is_some() && second.is_some());
        assert_ne!(first, second, "etag must change when the file changes");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
