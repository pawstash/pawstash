use crate::config::settings::ProxyMode;
use crate::db::downloads::DownloadRepository;
use crate::downloader::{DownloadControl, DownloadRunError, DownloadTask, Interruption};
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use tauri::Emitter;
use uuid::Uuid;

pub struct Aria2cManager;

impl Aria2cManager {
    pub fn is_installed() -> bool {
        which_aria2c().is_some()
    }

    pub async fn download(
        task: DownloadTask,
        repository: Arc<DownloadRepository>,
        control: Arc<DownloadControl>,
        app_handle: &tauri::AppHandle,
    ) -> Result<u64, DownloadRunError> {
        let aria2_path = which_aria2c().ok_or_else(|| {
            DownloadRunError::Failed("aria2c binary not found on system PATH".into())
        })?;

        let temp_path = Path::new(&task.temp_path);
        let temp_dir = temp_path
            .parent()
            .ok_or_else(|| DownloadRunError::Failed("Invalid temporary path".into()))?;
        let temp_name = temp_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| DownloadRunError::Failed("Invalid temporary filename".into()))?;
        tokio::fs::create_dir_all(temp_dir)
            .await
            .map_err(|error| DownloadRunError::Failed(error.to_string()))?;

        let mut cmd = tokio::process::Command::new(aria2_path);
        let connections = task.connections.clamp(1, 32).to_string();
        let rpc_listener = std::net::TcpListener::bind(("127.0.0.1", 0))
            .map_err(|error| DownloadRunError::Failed(error.to_string()))?;
        let rpc_port = rpc_listener
            .local_addr()
            .map_err(|error| DownloadRunError::Failed(error.to_string()))?
            .port();
        drop(rpc_listener);
        let rpc_secret = Uuid::new_v4().simple().to_string();
        let gid = Uuid::new_v4().simple().to_string()[..16].to_string();
        cmd.arg("-x")
            .arg(&connections)
            .arg("-s")
            .arg(&connections)
            .arg("-k")
            .arg("1M")
            .arg("--allow-overwrite=true")
            .arg("--auto-file-renaming=false")
            .arg("--continue=true")
            .arg("--summary-interval=0")
            .arg("--enable-rpc=true")
            .arg("--rpc-listen-all=false")
            .arg(format!("--rpc-listen-port={rpc_port}"))
            .arg(format!("--rpc-secret={rpc_secret}"))
            .arg(format!("--gid={gid}"))
            .arg("-d")
            .arg(temp_dir)
            .arg("-o")
            .arg(temp_name);

        cmd.arg("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36");
        cmd.arg("--header=Accept: */*");

        if let Some(referer) = super::derive_download_referer(&task.url) {
            cmd.arg(format!("--header=Referer: {referer}"));
        }

        if let Some(cookie) = &task.session_cookie {
            if !cookie.trim().is_empty() {
                let cookie_header = if cookie.contains('=') {
                    format!("Cookie: {}", cookie)
                } else {
                    format!("Cookie: session={}", cookie)
                };
                cmd.arg(format!("--header={}", cookie_header));
            }
        }

        match task.proxy_mode {
            ProxyMode::None => clear_proxy_environment(&mut cmd),
            ProxyMode::System => apply_system_proxy_environment(&mut cmd),
            ProxyMode::Custom => {
                clear_proxy_environment(&mut cmd);
                if task.proxy_url.trim().is_empty() {
                    return Err(DownloadRunError::Failed(
                        "Custom proxy URL is required".to_string(),
                    ));
                }
                cmd.arg(format!("--all-proxy={}", task.proxy_url.trim()));
                if !task.proxy_username.is_empty() {
                    cmd.arg(format!("--all-proxy-user={}", task.proxy_username));
                    cmd.arg(format!("--all-proxy-passwd={}", task.proxy_password));
                }
                if task.proxy_bypass_local {
                    cmd.arg("--no-proxy=localhost,127.0.0.1,::1");
                }
            }
        }

        cmd.arg(&task.url);
        cmd.stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);

        let mut child = cmd.spawn().map_err(|error| {
            DownloadRunError::Failed(format!("Failed to start aria2c: {error}"))
        })?;
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(150));
        let rpc_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(500))
            .build()
            .map_err(|error| DownloadRunError::Failed(error.to_string()))?;
        let rpc_url = format!("http://127.0.0.1:{rpc_port}/jsonrpc");

        loop {
            tokio::select! {
                status = child.wait() => {
                    let status = status.map_err(|error| DownloadRunError::Failed(error.to_string()))?;
                    if !status.success() {
                        return Err(DownloadRunError::Failed(format!("aria2c exited with {status}")));
                    }
                    let size = tokio::fs::metadata(temp_path)
                        .await
                        .map_err(|error| DownloadRunError::Failed(error.to_string()))?
                        .len();
                    return Ok(size);
                }
                _ = interval.tick() => {
                    if let Some(interruption) = control.interruption() {
                        let _ = child.kill().await;
                        if interruption == Interruption::Cancel {
                            let _ = tokio::fs::remove_file(temp_path).await;
                            let _ = tokio::fs::remove_file(format!("{}.aria2", task.temp_path)).await;
                        }
                        return Err(DownloadRunError::Interrupted(interruption));
                    }
                    let payload = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": "pawstash-progress",
                        "method": "aria2.tellStatus",
                        "params": [
                            format!("token:{rpc_secret}"),
                            gid,
                            ["status", "completedLength", "totalLength", "downloadSpeed"]
                        ]
                    });
                    if let Ok(response) = rpc_client.post(&rpc_url).json(&payload).send().await {
                        if let Ok(body) = response.json::<serde_json::Value>().await {
                            if let Some(result) = body.get("result") {
                                let parse = |field: &str| {
                                    result.get(field)
                                        .and_then(serde_json::Value::as_str)
                                        .and_then(|value| value.parse::<u64>().ok())
                                        .unwrap_or(0)
                                };
                                let bytes = parse("completedLength");
                                let total = parse("totalLength");
                                let speed = parse("downloadSpeed");
                                if let Ok(job) = repository.update_progress(&task.id, bytes, total, speed) {
                                    let _ = app_handle.emit("download-job-updated", job);
                                    if let Ok((active, total_queued, downloaded_total, expected_total, speed_total)) =
                                        repository.queue_progress_stats()
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
                                if result.get("status").and_then(serde_json::Value::as_str) == Some("complete") {
                                    let shutdown = serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "id": "pawstash-shutdown",
                                        "method": "aria2.shutdown",
                                        "params": [format!("token:{rpc_secret}")]
                                    });
                                    let _ = rpc_client.post(&rpc_url).json(&shutdown).send().await;
                                    match tokio::time::timeout(
                                        std::time::Duration::from_secs(2),
                                        child.wait(),
                                    ).await {
                                        Ok(Ok(status)) if status.success() => {}
                                        _ => {
                                            let _ = child.kill().await;
                                            let _ = child.wait().await;
                                        }
                                    }
                                    let size = tokio::fs::metadata(temp_path)
                                        .await
                                        .map_err(|error| DownloadRunError::Failed(error.to_string()))?
                                        .len();
                                    return Ok(size);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn clear_proxy_environment(cmd: &mut tokio::process::Command) {
    for key in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NO_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "no_proxy",
    ] {
        cmd.env_remove(key);
    }
}

fn apply_system_proxy_environment(cmd: &mut tokio::process::Command) {
    for (lower, upper) in [
        ("http_proxy", "HTTP_PROXY"),
        ("https_proxy", "HTTPS_PROXY"),
        ("all_proxy", "ALL_PROXY"),
        ("no_proxy", "NO_PROXY"),
    ] {
        if std::env::var_os(lower).is_none() {
            if let Some(value) = std::env::var_os(upper) {
                cmd.env(lower, value);
            }
        }
    }
}

fn which_aria2c() -> Option<String> {
    if let Ok(path) = std::env::var("PATH") {
        for p in std::env::split_paths(&path) {
            let exe = p.join("aria2c.exe");
            if exe.exists() {
                return Some(exe.to_string_lossy().to_string());
            }
            let bin = p.join("aria2c");
            if bin.exists() {
                return Some(bin.to_string_lossy().to_string());
            }
        }
    }
    None
}
