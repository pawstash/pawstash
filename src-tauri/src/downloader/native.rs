use crate::config::settings::ProxyMode;
use crate::db::downloads::DownloadRepository;
use crate::downloader::{DownloadControl, DownloadRunError, DownloadTask, Interruption};
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_RANGE, COOKIE, RANGE, USER_AGENT};
use reqwest::{Client, Response, StatusCode};
use std::path::Path;
use std::sync::Arc;
use tauri::Emitter;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

pub struct NativeDownloader;

impl NativeDownloader {
    pub async fn probe_total_size(task: &DownloadTask) -> Result<Option<u64>, DownloadRunError> {
        let client = Self::client(task)?;

        // 1. First try Range GET (bytes=0-0), which works reliably across CDNs that block or truncate HEAD requests
        if let Ok(headers) = Self::headers(task, Some("bytes=0-0")) {
            if let Ok(response) = client
                .get(&task.url)
                .headers(headers)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                if response.status() == StatusCode::PARTIAL_CONTENT || response.status().is_success() {
                    if let Some(total) = response
                        .headers()
                        .get(CONTENT_RANGE)
                        .and_then(|value| value.to_str().ok())
                        .and_then(|value| value.rsplit_once('/').map(|(_, total)| total))
                        .and_then(|total| total.parse::<u64>().ok())
                        .filter(|total| *total > 0)
                    {
                        return Ok(Some(total));
                    }
                    if let Some(len) = response.content_length().filter(|len| *len > 1) {
                        return Ok(Some(len));
                    }
                }
            }
        }

        // 2. Fallback to HEAD request
        if let Ok(headers) = Self::headers(task, None) {
            if let Ok(response) = client
                .head(&task.url)
                .headers(headers)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                if response.status().is_success() {
                    if let Some(size) = response.content_length().filter(|size| *size > 0) {
                        return Ok(Some(size));
                    }
                }
            }
        }

        Ok(None)
    }

    pub async fn download(
        task: DownloadTask,
        repository: Arc<DownloadRepository>,
        control: Arc<DownloadControl>,
        app_handle: &tauri::AppHandle,
    ) -> Result<u64, DownloadRunError> {
        let temp_path = Path::new(&task.temp_path);
        if let Some(parent) = temp_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|error| DownloadRunError::Failed(error.to_string()))?;
        }

        let client = Self::client(&task)?;
        let mut existing_len = tokio::fs::metadata(temp_path)
            .await
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let mut response = Self::request(&client, &task, existing_len).await?;

        let valid_resume = existing_len > 0
            && response.status() == StatusCode::PARTIAL_CONTENT
            && response
                .headers()
                .get(CONTENT_RANGE)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.starts_with(&format!("bytes {existing_len}-")));
        if existing_len > 0 && !valid_resume {
            existing_len = 0;
            response = Self::request(&client, &task, 0).await?;
        }
        if !response.status().is_success() {
            return Err(DownloadRunError::Failed(format!(
                "Download failed HTTP {}",
                response.status()
            )));
        }

        let total_size = response.content_length().unwrap_or(0) + existing_len;
        let mut options = OpenOptions::new();
        options.create(true).write(true);
        if existing_len > 0 {
            options.append(true);
        } else {
            options.truncate(true);
        }
        let mut file = options
            .open(temp_path)
            .await
            .map_err(|error| DownloadRunError::Failed(error.to_string()))?;

        let mut stream = response.bytes_stream();
        let mut downloaded = existing_len;
        let mut checkpoint_bytes = existing_len;
        let mut checkpoint_time = std::time::Instant::now();

        // Publish the confirmed response size before reading the body so the UI
        // starts at the real initial position instead of learning the total at
        // the last checkpoint.
        if let Ok(job) = repository.update_progress(&task.id, downloaded, total_size, 0) {
            let _ = app_handle.emit("download-job-updated", job);
        }

        while let Some(chunk) = stream.next().await {
            if let Some(interruption) = control.interruption() {
                file.flush().await.ok();
                if interruption == Interruption::Cancel {
                    drop(file);
                    let _ = tokio::fs::remove_file(temp_path).await;
                }
                return Err(DownloadRunError::Interrupted(interruption));
            }

            let chunk = chunk.map_err(|error| DownloadRunError::Failed(error.to_string()))?;
            file.write_all(&chunk)
                .await
                .map_err(|error| DownloadRunError::Failed(error.to_string()))?;
            downloaded += chunk.len() as u64;

            if checkpoint_time.elapsed() >= std::time::Duration::from_millis(100) {
                let elapsed = checkpoint_time.elapsed().as_secs_f64().max(0.001);
                let speed = ((downloaded - checkpoint_bytes) as f64 / elapsed) as u64;
                checkpoint_bytes = downloaded;
                checkpoint_time = std::time::Instant::now();
                if let Ok(job) = repository.update_progress(&task.id, downloaded, total_size, speed)
                {
                    let _ = app_handle.emit("download-job-updated", job);
                    if let Ok((
                        active,
                        total_queued,
                        downloaded_total,
                        expected_total,
                        speed_total,
                    )) = repository.queue_progress_stats()
                    {
                        crate::downloader::notifications::update_download_notification(
                            active,
                            total_queued,
                            downloaded_total,
                            expected_total,
                            speed_total,
                            &task.filename,
                        );
                    }
                }
            }
        }

        file.flush()
            .await
            .map_err(|error| DownloadRunError::Failed(error.to_string()))?;
        file.sync_all()
            .await
            .map_err(|error| DownloadRunError::Failed(error.to_string()))?;
        if total_size > 0 && downloaded != total_size {
            return Err(DownloadRunError::Failed(format!(
                "Incomplete download: expected {total_size} bytes, received {downloaded}"
            )));
        }
        Ok(downloaded)
    }

    fn client(task: &DownloadTask) -> Result<Client, DownloadRunError> {
        let is_local = task.url.starts_with("http://127.0.0.1")
            || task.url.starts_with("http://localhost")
            || task.url.starts_with("http://[::1]")
            || task.url.starts_with("https://127.0.0.1")
            || task.url.starts_with("https://localhost")
            || task.url.starts_with("https://[::1]");
        let mut builder = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36")
            .redirect(reqwest::redirect::Policy::limited(10))
            .pool_idle_timeout(std::time::Duration::from_secs(30));
        match task.proxy_mode {
            ProxyMode::None => builder = builder.no_proxy(),
            ProxyMode::System => {}
            ProxyMode::Custom if !(task.proxy_bypass_local && is_local) => {
                if task.proxy_url.trim().is_empty() {
                    return Err(DownloadRunError::Failed(
                        "Custom proxy URL is required".to_string(),
                    ));
                }
                let mut proxy = reqwest::Proxy::all(task.proxy_url.trim()).map_err(|error| {
                    DownloadRunError::Failed(format!("Invalid proxy URL: {error}"))
                })?;
                if !task.proxy_username.is_empty() {
                    proxy = proxy.basic_auth(&task.proxy_username, &task.proxy_password);
                }
                builder = builder.proxy(proxy);
            }
            ProxyMode::Custom => builder = builder.no_proxy(),
        }
        builder
            .build()
            .map_err(|error| DownloadRunError::Failed(error.to_string()))
    }

    async fn request(
        client: &Client,
        task: &DownloadTask,
        existing_len: u64,
    ) -> Result<Response, DownloadRunError> {
        let range = (existing_len > 0).then(|| format!("bytes={existing_len}-"));
        let headers = Self::headers(task, range.as_deref())?;
        client
            .get(&task.url)
            .headers(headers)
            .send()
            .await
            .map_err(|error| {
                let mut details = error.to_string();
                let mut source = std::error::Error::source(&error);
                while let Some(s) = source {
                    details.push_str(&format!(" -> {s}"));
                    source = s.source();
                }
                DownloadRunError::Failed(details)
            })
    }

    fn headers(task: &DownloadTask, range: Option<&str>) -> Result<HeaderMap, DownloadRunError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            HeaderValue::from_static("*/*"),
        );
        headers.insert(
            reqwest::header::ACCEPT_ENCODING,
            HeaderValue::from_static("identity"),
        );

        if let Ok(parsed) = reqwest::Url::parse(&task.url) {
            let host = parsed.host_str().unwrap_or("");
            let is_kemono_or_pawchive = host.contains("kemono")
                || host.contains("coomer")
                || host.contains("cum.st")
                || host.contains("pawchive");

            let referer = if host.contains("kemono") {
                "https://kemono.cr/"
            } else if host.contains("coomer") || host.contains("cum.st") {
                "https://coomer.party/"
            } else if host.contains("pawchive") {
                "https://pawchive.pw/"
            } else if host.contains("dropbox") || host.contains("google") || host.contains("mega") || host.contains("pixeldrain") {
                ""
            } else if !host.is_empty() {
                let scheme = parsed.scheme();
                let owned = format!("{scheme}://{host}/");
                if let Ok(val) = HeaderValue::from_str(&owned) {
                    headers.insert(reqwest::header::REFERER, val);
                }
                ""
            } else {
                ""
            };
            if !referer.is_empty() {
                if let Ok(val) = HeaderValue::from_str(referer) {
                    headers.insert(reqwest::header::REFERER, val);
                }
            }

            // Only attach Pawchive/Kemono session cookie to Pawchive/Kemono endpoints
            if is_kemono_or_pawchive {
                if let Some(cookie) = &task.session_cookie {
                    if !cookie.trim().is_empty() {
                        let cookie = if cookie.contains('=') {
                            cookie.clone()
                        } else {
                            format!("session={cookie}")
                        };
                        let value = HeaderValue::from_str(&cookie)
                            .map_err(|error| DownloadRunError::Failed(error.to_string()))?;
                        headers.insert(COOKIE, value);
                    }
                }
            }
        }

        if let Some(range) = range {
            let value = HeaderValue::from_str(range)
                .map_err(|error| DownloadRunError::Failed(error.to_string()))?;
            headers.insert(RANGE, value);
        }
        Ok(headers)
    }
}
