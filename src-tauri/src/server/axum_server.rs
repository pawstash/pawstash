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
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio_util::io::ReaderStream;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

pub struct MediaServer {
    pub port: u16,
}

#[derive(Clone)]
pub struct MediaServerState {
    pub allowed_roots: Vec<PathBuf>,
    pub config_manager: Arc<crate::config::ConfigManager>,
    pub token: String,
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
            .allow_headers([header::RANGE, header::CONTENT_TYPE, header::ACCEPT])
            .expose_headers([
                header::CONTENT_LENGTH,
                header::CONTENT_RANGE,
                header::ACCEPT_RANGES,
            ]);

        let state = Arc::new(MediaServerState {
            allowed_roots,
            config_manager,
            token,
        });

        let app = Router::new()
            .route("/media/*file_path", get(serve_media_handler))
            .route("/cloud_stream/mega", get(serve_mega_stream_handler))
            .route(
                "/cloud_stream/proxy",
                get(serve_cloud_proxy_stream_handler).head(serve_cloud_proxy_stream_handler),
            )
            .with_state(state)
            .layer(cors);

        // Bind to dynamic loopback port
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
    let target = dunce::canonicalize(&path).map_err(|_| {
        tracing::warn!("Local media file not found: {:?}", path);
        (StatusCode::NOT_FOUND, "File not found".to_string())
    })?;
    if !target.is_file() {
        return Err((StatusCode::NOT_FOUND, "File not found".to_string()));
    }

    let mut allowed_roots = state.allowed_roots.clone();
    let settings = state
        .config_manager
        .load()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let user_dir = PathBuf::from(&settings.download_dir);
    if !allowed_roots.contains(&user_dir) {
        allowed_roots.push(user_dir);
    }
    let allowed = allowed_roots.iter().any(|root| {
        if let Ok(clean_root) = dunce::canonicalize(root) {
            target.starts_with(clean_root)
        } else {
            false
        }
    });

    if !allowed {
        tracing::warn!("Local media access denied for target: {:?}", target);
        return Err((StatusCode::NOT_FOUND, "File not found".to_string()));
    }

    let file_metadata = tokio::fs::metadata(&target)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let file_size = file_metadata.len();

    let mime_type = mime_guess::from_path(&target)
        .first_or_octet_stream()
        .to_string();

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
                let stream = ReaderStream::new(limited_file);

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
                return Ok(response);
            }
        }
    }

    let file = File::open(&target)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let mut response = (StatusCode::OK, body).into_response();
    let res_headers = response.headers_mut();

    if let Ok(val) = HeaderValue::from_str(&mime_type) {
        res_headers.insert(header::CONTENT_TYPE, val);
    }
    res_headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    res_headers.insert(header::CONTENT_LENGTH, HeaderValue::from(file_size));
    Ok(response)
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

    let client = reqwest::Client::builder()
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
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

    let api_url = if let Some(ref fid) = params.folder_id {
        format!("https://g.api.mega.co.nz/cs?n={fid}")
    } else {
        "https://g.api.mega.co.nz/cs".to_string()
    };

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
    let mut builder = reqwest::Client::builder()
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .timeout(std::time::Duration::from_secs(60))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36");

    match settings.proxy_mode {
        crate::config::ProxyMode::None => builder = builder.no_proxy(),
        crate::config::ProxyMode::System => {}
        crate::config::ProxyMode::Custom => {
            if !settings.proxy_url.trim().is_empty() {
                let mut proxy = reqwest::Proxy::all(settings.proxy_url.trim())
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                if !settings.proxy_username.is_empty() {
                    proxy = proxy.basic_auth(&settings.proxy_username, &settings.proxy_password);
                }
                builder = builder.proxy(proxy);
            }
        }
    }

    let client = builder
        .build()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let is_head = method == Method::HEAD;
    let normalized_url = crate::downloader::normalize_download_url(&params.url);
    let mut target_url = validate_proxy_target(&normalized_url).await?;
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
        target_url = validate_proxy_target(redirected.as_str()).await?;
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

    // If upstream returned an HTML page (e.g. Dropbox "File Deleted" or login page), fail
    if upstream_content_type.starts_with("text/html") {
        tracing::warn!(
            "Cloud proxy upstream returned HTML instead of media for target '{}' (file may be deleted)",
            target_url
        );
        return Err((
            StatusCode::NOT_FOUND,
            "Upstream returned HTML page instead of media stream".to_string(),
        ));
    }

    let body = if is_head {
        Body::empty()
    } else {
        Body::from_stream(upstream.bytes_stream())
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

    // Ensure valid streaming MIME type if upstream returned non-media or generic type (e.g. Dropbox returning application/json)
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

async fn validate_proxy_target(raw: &str) -> Result<reqwest::Url, (StatusCode, String)> {
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
    let host = url.host_str().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "Cloud proxy URL has no host".to_string(),
        )
    })?;
    let addresses = tokio::net::lookup_host((host, 443)).await.map_err(|e| {
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
        // Fake-IP (Clash, Mihomo, Sing-box TUN adapter) must be allowed
        assert!(is_public_ipv4(Ipv4Addr::new(198, 18, 0, 1)));
        assert!(is_public_ipv4(Ipv4Addr::new(198, 18, 0, 37)));
        assert!(is_public_ipv4(Ipv4Addr::new(198, 19, 255, 254)));

        // CGNAT (Carrier-Grade NAT, Tailscale) must be allowed
        assert!(is_public_ipv4(Ipv4Addr::new(100, 64, 0, 1)));
        assert!(is_public_ipv4(Ipv4Addr::new(100, 100, 100, 100)));

        // Public internet IPs must be allowed
        assert!(is_public_ipv4(Ipv4Addr::new(1, 1, 1, 1)));
        assert!(is_public_ipv4(Ipv4Addr::new(8, 8, 8, 8)));
    }

    #[test]
    fn test_is_public_ipv4_blocks_private_and_local() {
        // Loopback
        assert!(!is_public_ipv4(Ipv4Addr::new(127, 0, 0, 1)));
        // RFC 1918 Private LAN
        assert!(!is_public_ipv4(Ipv4Addr::new(192, 168, 1, 1)));
        assert!(!is_public_ipv4(Ipv4Addr::new(10, 0, 0, 1)));
        assert!(!is_public_ipv4(Ipv4Addr::new(172, 16, 0, 1)));
        // Link-local / Cloud metadata (AWS/GCP/Azure: 169.254.169.254)
        assert!(!is_public_ipv4(Ipv4Addr::new(169, 254, 169, 254)));
        // Broadcast and Multicast
        assert!(!is_public_ipv4(Ipv4Addr::new(255, 255, 255, 255)));
        assert!(!is_public_ipv4(Ipv4Addr::new(224, 0, 0, 1)));
        // Zero address
        assert!(!is_public_ipv4(Ipv4Addr::new(0, 0, 0, 0)));
    }

    #[tokio::test]
    async fn test_validate_proxy_target_dropbox() {
        let res = validate_proxy_target("https://www.dropbox.com/scl/fi/dzou30iaabzttgdkofk0s/KEI-FULL-VIDEO.mp4?rlkey=d69eq3s4u7ds888h9ia9qrhwn&st=vna292xr&raw=1").await;
        assert!(
            res.is_ok(),
            "Failed to validate proxy target: {:?}",
            res.err()
        );
    }
}
