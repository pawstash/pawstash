use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
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
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio_util::io::ReaderStream;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};

pub struct MediaServer {
    pub port: u16,
}

impl MediaServer {
    pub async fn start(
        allowed_roots: Vec<PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let app = Router::new()
            .route("/media/*file_path", get(serve_media_handler))
            .route("/cloud_stream/mega", get(serve_mega_stream_handler))
            .route("/health", get(|| async { "OK" }))
            .with_state(Arc::new(allowed_roots))
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
    State(allowed_roots): State<Arc<Vec<PathBuf>>>,
    Path(file_path): Path<String>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    #[cfg(not(windows))]
    let path_str = if !file_path.starts_with('/') {
        format!("/{file_path}")
    } else {
        file_path
    };
    #[cfg(windows)]
    let path_str = {
        let trimmed = file_path.trim_start_matches('/');
        if trimmed.len() >= 2 && trimmed.chars().nth(1) == Some(':') {
            trimmed.to_string()
        } else {
            file_path
        }
    };

    let path = PathBuf::from(&path_str);
    let canonical = tokio::fs::canonicalize(&path)
        .await
        .unwrap_or_else(|_| path.clone());

    if !path.is_file() && !canonical.is_file() {
        return Err((StatusCode::NOT_FOUND, "File not found".to_string()));
    }

    let target = if canonical.is_file() { canonical } else { path };

    #[cfg(target_os = "android")]
    let allowed = true;

    #[cfg(not(target_os = "android"))]
    let allowed = allowed_roots.is_empty() || allowed_roots.iter().any(|root| {
        if target.starts_with(root) {
            return true;
        }
        if let Ok(can_root) = std::fs::canonicalize(root) {
            if target.starts_with(&can_root) {
                return true;
            }
        }
        false
    });

    if !allowed {
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

                res_headers.insert(header::CONTENT_TYPE, mime_type.parse().unwrap());
                res_headers.insert(header::ACCEPT_RANGES, "bytes".parse().unwrap());
                res_headers.insert(
                    header::CONTENT_RANGE,
                    format!("bytes {}-{}/{}", start, end, file_size)
                        .parse()
                        .unwrap(),
                );
                res_headers.insert(
                    header::CONTENT_LENGTH,
                    chunk_size.to_string().parse().unwrap(),
                );

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

    res_headers.insert(header::CONTENT_TYPE, mime_type.parse().unwrap());
    res_headers.insert(header::ACCEPT_RANGES, "bytes".parse().unwrap());
    res_headers.insert(
        header::CONTENT_LENGTH,
        file_size.to_string().parse().unwrap(),
    );

    Ok(response)
}

#[derive(Debug, Deserialize)]
struct MegaStreamQuery {
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
    Query(params): Query<MegaStreamQuery>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
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

    let client = reqwest::Client::new();
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
        resp.headers_mut().insert(
            header::CONTENT_RANGE,
            format!("bytes {range_start}-{range_end}/{total_size}")
                .parse()
                .unwrap(),
        );
        resp
    } else {
        (StatusCode::OK, body).into_response()
    };

    let res_headers = response.headers_mut();
    res_headers.insert(header::CONTENT_TYPE, mime.parse().unwrap());
    res_headers.insert(header::ACCEPT_RANGES, "bytes".parse().unwrap());
    if chunk_size > 0 {
        res_headers.insert(
            header::CONTENT_LENGTH,
            chunk_size.to_string().parse().unwrap(),
        );
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
