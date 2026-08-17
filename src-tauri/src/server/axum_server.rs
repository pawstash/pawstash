use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
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
    let path = PathBuf::from(file_path);
    let canonical = tokio::fs::canonicalize(&path)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "File not found".to_string()))?;
    let allowed = allowed_roots
        .iter()
        .filter_map(|root| std::fs::canonicalize(root).ok())
        .any(|root| canonical.starts_with(root));
    if !allowed || !canonical.is_file() {
        return Err((StatusCode::NOT_FOUND, "File not found".to_string()));
    }

    let file_metadata = tokio::fs::metadata(&canonical)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let file_size = file_metadata.len();

    let mime_type = mime_guess::from_path(&canonical)
        .first_or_octet_stream()
        .to_string();

    if let Some(range_header) = headers.get(header::RANGE) {
        if let Ok(range_str) = range_header.to_str() {
            if let Some(range) = parse_range(range_str, file_size) {
                let (start, end) = range;
                let chunk_size = end - start + 1;

                let mut file = File::open(&canonical)
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

    // Serve full file if no range header or range parsing failed
    let file = File::open(&canonical)
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
