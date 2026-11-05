use axum::{
    body::Body,
    extract::{Query, State},
    http::{header, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::{Arc, OnceLock};
use tokio::sync::Semaphore;

use super::axum_server::{require_token, MediaServerState};

const MAX_SOURCE_BYTES: u64 = 32 * 1024 * 1024;
const JPEG_QUALITY: u8 = 82;
const MIN_WIDTH: u32 = 48;
const MAX_WIDTH: u32 = 1600;
const MAX_PARALLEL_RESIZES: usize = 3;

fn resize_slots() -> &'static Semaphore {
    static SLOTS: OnceLock<Semaphore> = OnceLock::new();
    SLOTS.get_or_init(|| Semaphore::new(MAX_PARALLEL_RESIZES))
}

#[derive(Deserialize)]
pub struct ThumbQuery {
    u: String,
    w: Option<u32>,
    token: Option<String>,
}

pub async fn serve_thumbnail_handler(
    State(state): State<Arc<MediaServerState>>,
    Query(query): Query<ThumbQuery>,
) -> Response {
    match build_thumbnail(state, query).await {
        Ok((bytes, cached)) => {
            let mut response = Response::new(Body::from(bytes));
            let headers = response.headers_mut();
            headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/jpeg"));
            headers.insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
            headers.insert(
                "x-pawstash-cache",
                HeaderValue::from_static(if cached { "hit" } else { "miss" }),
            );
            response
        }
        Err((status, message)) => (status, message).into_response(),
    }
}

async fn build_thumbnail(
    state: Arc<MediaServerState>,
    query: ThumbQuery,
) -> Result<(Vec<u8>, bool), (StatusCode, String)> {
    require_token(&state, query.token.as_deref().unwrap_or_default())?;

    let source = query.u.trim();
    if !(source.starts_with("https://") || source.starts_with("http://")) {
        return Err((StatusCode::BAD_REQUEST, "u must be an http(s) URL".into()));
    }
    let width = query.w.unwrap_or(400).clamp(MIN_WIDTH, MAX_WIDTH);
    let key = cache_key(source, width);
    let dir = state.thumb_cache_dir.join(&key[0..2]);
    let path = dir.join(format!("{key}.jpg"));

    if let Some(bytes) = read_cached(&path).await {
        return Ok((bytes, true));
    }

    let _permit = resize_slots().acquire().await.map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "resize queue closed".to_string(),
        )
    })?;

    if let Some(bytes) = read_cached(&path).await {
        return Ok((bytes, true));
    }

    let (response, _) = state
        .upstream
        .send(Method::GET, source, None)
        .await
        .map_err(|(status, message)| (status, format!("thumbnail upstream: {message}")))?;

    let status = response.status();
    if !status.is_success() {
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("thumbnail upstream returned {status}"),
        ));
    }
    if response
        .content_length()
        .is_some_and(|len| len > MAX_SOURCE_BYTES)
    {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            "source image is too large".into(),
        ));
    }
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    if !content_type.is_empty() && !content_type.starts_with("image/") {
        return Err((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            format!("upstream returned {content_type}"),
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|error| (StatusCode::BAD_GATEWAY, error.to_string()))?;
    if bytes.len() as u64 > MAX_SOURCE_BYTES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            "source image is too large".into(),
        ));
    }

    let resized = tokio::task::spawn_blocking(move || resize_to_jpeg(&bytes, width))
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))??;

    if let Err(error) = tokio::fs::create_dir_all(&dir).await {
        tracing::debug!("Thumbnail cache dir unavailable: {error}");
    } else {
        let temporary = dir.join(format!("{key}.{}.tmp", uuid::Uuid::new_v4().simple()));
        if tokio::fs::write(&temporary, &resized).await.is_ok()
            && tokio::fs::rename(&temporary, &path).await.is_err()
        {
            let _ = tokio::fs::remove_file(&temporary).await;
        }
    }

    Ok((resized, false))
}

async fn read_cached(path: &std::path::Path) -> Option<Vec<u8>> {
    match tokio::fs::read(path).await {
        Ok(bytes) if !bytes.is_empty() => Some(bytes),
        _ => None,
    }
}

fn cache_key(source: &str, width: u32) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher.update(b"|");
    hasher.update(width.to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}

fn resize_to_jpeg(bytes: &[u8], width: u32) -> Result<Vec<u8>, (StatusCode, String)> {
    let image = image::load_from_memory(bytes).map_err(|error| {
        (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            format!("cannot decode source image: {error}"),
        )
    })?;

    let resized = if image.width() > width {
        let height =
            ((image.height() as u64 * width as u64) / image.width().max(1) as u64).max(1) as u32;
        image.resize_exact(width, height, image::imageops::FilterType::Triangle)
    } else {
        image
    };

    let rgb = resized.to_rgb8();
    let mut out = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY);
    encoder
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("cannot encode thumbnail: {error}"),
            )
        })?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::cache_key;

    #[test]
    fn cache_key_is_width_specific() {
        let small = cache_key("https://example.test/a.webp", 400);
        let large = cache_key("https://example.test/a.webp", 800);
        assert_eq!(small.len(), 64);
        assert_ne!(small, large);
    }
}
