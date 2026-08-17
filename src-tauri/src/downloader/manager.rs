use crate::config::settings::{AppSettings, ProxyMode};
use crate::db::downloads::{DownloadJob, DownloadRepository, NewDownloadJob};
use crate::downloader::aria2c::Aria2cManager;
use crate::downloader::native::NativeDownloader;
use crate::downloader::{DownloadControl, DownloadRunError, DownloadTask, Interruption};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

pub struct DownloadManager {
    repository: Arc<DownloadRepository>,
    active: Mutex<HashMap<String, Arc<DownloadControl>>>,
}

impl DownloadManager {
    pub fn new(repository: Arc<DownloadRepository>) -> Self {
        Self {
            repository,
            active: Mutex::new(HashMap::new()),
        }
    }

    pub fn list(&self) -> Result<Vec<DownloadJob>, String> {
        self.repository.list()
    }

    pub fn recover(self: &Arc<Self>, settings: AppSettings, app_handle: tauri::AppHandle) {
        if let Ok(ids) = self.repository.recover_interrupted() {
            for id in ids {
                let _ = self.start_existing(id, settings.clone(), app_handle.clone());
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn enqueue(
        self: &Arc<Self>,
        service: String,
        creator_id: String,
        post_id: String,
        media_id: String,
        url: String,
        filename: String,
        settings: AppSettings,
        app_handle: tauri::AppHandle,
    ) -> Result<DownloadJob, String> {
        let parsed = reqwest::Url::parse(&url).map_err(|error| error.to_string())?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err("Only HTTP and HTTPS downloads are supported".to_string());
        }
        let safe_filename = Path::new(&filename)
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| "Invalid download filename".to_string())?
            .to_string();

        let root = Self::ensure_download_root(&settings.download_dir)?;
        let final_path = self.unique_final_path(&root, &safe_filename)?;
        let id = Uuid::new_v4().to_string();
        let temp_path = root.join(".temp").join(format!("{id}.part"));
        let custom_socks = settings.proxy_mode == ProxyMode::Custom
            && (settings.proxy_url.starts_with("socks5://")
                || settings.proxy_url.starts_with("socks5h://"));
        let engine = if settings.use_aria2c && Aria2cManager::is_installed() && !custom_socks {
            "aria2c"
        } else {
            "native"
        };
        let logical_key = format!("{service}:{creator_id}:{post_id}:{media_id}");
        let job = self.repository.create_or_get(NewDownloadJob {
            id: &id,
            logical_key: &logical_key,
            service: &service,
            creator_id: &creator_id,
            post_id: &post_id,
            media_id: &media_id,
            url: &url,
            filename: final_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&safe_filename),
            output_dir: &root.to_string_lossy(),
            temp_path: &temp_path.to_string_lossy(),
            final_path: &final_path.to_string_lossy(),
            engine,
        })?;
        if matches!(
            job.status.as_str(),
            "queued" | "paused" | "failed" | "cancelled" | "missing"
        ) {
            self.start_existing(job.id.clone(), settings, app_handle)?;
        }
        Ok(job)
    }

    pub fn pause(&self, id: &str) -> Result<DownloadJob, String> {
        let control = self
            .active
            .lock()
            .map_err(|error| error.to_string())?
            .get(id)
            .cloned()
            .ok_or_else(|| "Download is not active".to_string())?;
        let job = self.repository.update_status(id, "paused")?;
        control.pause();
        Ok(job)
    }

    pub fn cancel(&self, id: &str) -> Result<DownloadJob, String> {
        let current = self
            .repository
            .get(id)?
            .ok_or_else(|| "Download job not found".to_string())?;
        if matches!(current.status.as_str(), "completed" | "cancelled") {
            return Err("Completed or cancelled downloads cannot be cancelled".to_string());
        }
        let job = self.repository.update_status(id, "cancelled")?;
        if let Some(control) = self
            .active
            .lock()
            .map_err(|error| error.to_string())?
            .get(id)
            .cloned()
        {
            control.cancel();
        } else {
            let _ = std::fs::remove_file(&job.temp_path);
        }
        Ok(job)
    }

    pub fn cancel_all(&self) {
        if let Ok(active) = self.active.lock() {
            for control in active.values() {
                control.cancel();
            }
        }
    }

    pub fn resume(
        self: &Arc<Self>,
        id: String,
        settings: AppSettings,
        app_handle: tauri::AppHandle,
    ) -> Result<DownloadJob, String> {
        let current = self
            .repository
            .get(&id)?
            .ok_or_else(|| "Download job not found".to_string())?;
        if current.status != "paused" {
            return Err("Only paused downloads can be resumed".to_string());
        }
        let job = self.repository.update_status(&id, "queued")?;
        self.start_existing(id, settings, app_handle)?;
        Ok(job)
    }

    pub fn retry(
        self: &Arc<Self>,
        id: String,
        settings: AppSettings,
        app_handle: tauri::AppHandle,
    ) -> Result<DownloadJob, String> {
        let job = self.repository.retry(&id)?;
        if job.status != "queued" {
            return Err("Only failed, cancelled, or missing downloads can be retried".to_string());
        }
        self.start_existing(id, settings, app_handle)?;
        Ok(job)
    }

    pub fn remove(&self, id: &str) -> Result<bool, String> {
        if self
            .active
            .lock()
            .map_err(|error| error.to_string())?
            .contains_key(id)
        {
            return Err("Stop the download before removing it".to_string());
        }
        let job = self
            .repository
            .get(id)?
            .ok_or_else(|| "Download job not found".to_string())?;
        let output_root =
            std::fs::canonicalize(&job.output_dir).map_err(|error| error.to_string())?;

        if job.status == "completed" {
            Self::remove_download_file(&output_root, Path::new(&job.final_path))?;
        } else if Path::new(&job.temp_path).exists() {
            Self::remove_download_file(&output_root, Path::new(&job.temp_path))?;
        }

        let changed = self.repository.remove(id)?;
        if changed {
            if let Some(sha256) = job.sha256.as_deref() {
                if let Some(relative_path) = self.repository.take_orphan_blob(sha256)? {
                    Self::remove_download_file(&output_root, &output_root.join(relative_path))?;
                }
            }
        }
        Ok(changed)
    }

    fn ensure_download_root(preferred: &str) -> Result<PathBuf, String> {
        let root = PathBuf::from(preferred);
        if std::fs::create_dir_all(root.join(".temp")).is_ok()
            && std::fs::create_dir_all(root.join(".media")).is_ok()
        {
            return Ok(root);
        }

        #[cfg(target_os = "android")]
        {
            let fallbacks = [
                PathBuf::from("/storage/emulated/0/Download/Pawstash"),
                PathBuf::from(
                    "/storage/emulated/0/Android/data/app.pawstash.client/files/Download",
                ),
                PathBuf::from("/data/data/app.pawstash.client/files/Pawstash/Downloads"),
            ];

            for fb in fallbacks {
                if std::fs::create_dir_all(fb.join(".temp")).is_ok()
                    && std::fs::create_dir_all(fb.join(".media")).is_ok()
                {
                    return Ok(fb);
                }
            }
        }

        std::fs::create_dir_all(root.join(".temp")).map_err(|error| error.to_string())?;
        std::fs::create_dir_all(root.join(".media")).map_err(|error| error.to_string())?;
        Ok(root)
    }

    fn remove_download_file(output_root: &Path, path: &Path) -> Result<(), String> {
        if !path.exists() {
            return Ok(());
        }
        let canonical = std::fs::canonicalize(path).map_err(|error| error.to_string())?;
        if canonical == output_root || !canonical.starts_with(output_root) || !canonical.is_file() {
            return Err("Refusing to delete a path outside the download directory".to_string());
        }
        std::fs::remove_file(canonical).map_err(|error| error.to_string())
    }

    fn start_existing(
        self: &Arc<Self>,
        id: String,
        settings: AppSettings,
        app_handle: tauri::AppHandle,
    ) -> Result<(), String> {
        let control = Arc::new(DownloadControl::new());
        {
            let mut active = self.active.lock().map_err(|error| error.to_string())?;
            if active.contains_key(&id) {
                return Ok(());
            }
            active.insert(id.clone(), control.clone());
        }
        let manager = self.clone();
        tauri::async_runtime::spawn(async move {
            manager.run(id, settings, control, app_handle).await;
        });
        Ok(())
    }

    async fn run(
        self: Arc<Self>,
        id: String,
        settings: AppSettings,
        control: Arc<DownloadControl>,
        app_handle: tauri::AppHandle,
    ) {
        match self.run_inner(&id, settings, control, &app_handle).await {
            Err(DownloadRunError::Failed(message)) => {
                if let Ok(job) = self
                    .repository
                    .mark_failed(&id, "download_failed", &message)
                {
                    let _ = app_handle.emit("download-job-updated", job);
                }
            }
            Err(DownloadRunError::Interrupted(interruption)) => {
                let status = match interruption {
                    Interruption::Pause => "paused",
                    Interruption::Cancel => "cancelled",
                };
                if let Ok(job) = self.repository.update_status(&id, status) {
                    let _ = app_handle.emit("download-job-updated", job);
                }
            }
            Ok(()) => {}
        }
        if let Ok(mut active) = self.active.lock() {
            active.remove(&id);
        }
    }

    async fn run_inner(
        &self,
        id: &str,
        settings: AppSettings,
        control: Arc<DownloadControl>,
        app_handle: &tauri::AppHandle,
    ) -> Result<(), DownloadRunError> {
        let job = self
            .repository
            .update_status(id, "resolving")
            .map_err(DownloadRunError::Failed)?;
        let _ = app_handle.emit("download-job-updated", job.clone());

        let task = DownloadTask {
            id: job.id.clone(),
            url: job.url.clone(),
            output_dir: job.output_dir.clone(),
            temp_path: job.temp_path.clone(),
            final_path: job.final_path.clone(),
            filename: job.filename.clone(),
            session_cookie: (!settings.session_cookie.is_empty())
                .then_some(settings.session_cookie),
            proxy_mode: settings.proxy_mode,
            proxy_url: settings.proxy_url,
            proxy_username: settings.proxy_username,
            proxy_password: settings.proxy_password,
            proxy_bypass_local: settings.proxy_bypass_local,
            connections: settings.aria2_connections.clamp(1, 32),
        };
        if job.total_bytes == 0 {
            let probe_task = task.clone();
            let repository = self.repository.clone();
            let probe_id = id.to_string();
            let probe_handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(Some(total)) = NativeDownloader::probe_total_size(&probe_task).await {
                    if let Ok(sized) = repository.update_total_size(&probe_id, total) {
                        let _ = probe_handle.emit("download-job-updated", sized);
                    }
                }
            });
        }
        let recovered_final_size = if !Path::new(&job.temp_path).exists()
            && Path::new(&job.final_path).exists()
            && job.total_bytes > 0
            && job.downloaded_bytes == job.total_bytes
        {
            tokio::fs::metadata(&job.final_path)
                .await
                .ok()
                .map(|metadata| metadata.len())
                .filter(|size| *size == job.total_bytes)
        } else {
            None
        };
        let size = if let Some(size) = recovered_final_size {
            size
        } else if job.engine == "aria2c" && Aria2cManager::is_installed() {
            Aria2cManager::download(task, self.repository.clone(), control.clone(), app_handle)
                .await?
        } else {
            NativeDownloader::download(task, self.repository.clone(), control.clone(), app_handle)
                .await?
        };

        if let Some(interruption) = control.interruption() {
            return Err(DownloadRunError::Interrupted(interruption));
        }

        let transferred = self
            .repository
            .update_progress(id, size, size, 0)
            .map_err(DownloadRunError::Failed)?;
        let _ = app_handle.emit("download-job-updated", transferred);
        let verifying = self
            .repository
            .update_status(id, "verifying")
            .map_err(DownloadRunError::Failed)?;
        let _ = app_handle.emit("download-job-updated", verifying);
        let verification_path = if recovered_final_size.is_some() {
            Path::new(&job.final_path)
        } else {
            Path::new(&job.temp_path)
        };
        let (sha256, measured_size) = Self::hash_file(verification_path).await?;
        if let Some(interruption) = control.interruption() {
            if interruption == Interruption::Cancel {
                tokio::fs::remove_file(&job.temp_path).await.ok();
            }
            return Err(DownloadRunError::Interrupted(interruption));
        }
        if measured_size != size {
            return Err(DownloadRunError::Failed(
                "Downloaded file size changed during verification".to_string(),
            ));
        }
        let relative_blob = PathBuf::from(".media").join(&sha256[0..2]).join(&sha256);
        let blob_path = Path::new(&job.output_dir).join(&relative_blob);
        if let Some(parent) = blob_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|error| DownloadRunError::Failed(error.to_string()))?;
        }
        if blob_path.exists() {
            let (existing_hash, existing_size) = Self::hash_file(&blob_path).await?;
            if existing_hash != sha256 || existing_size != measured_size {
                return Err(DownloadRunError::Failed(
                    "Content-addressed blob failed integrity verification".to_string(),
                ));
            }
            if recovered_final_size.is_none() {
                tokio::fs::remove_file(&job.temp_path).await.ok();
            }
        } else if recovered_final_size.is_some() {
            if tokio::fs::hard_link(&job.final_path, &blob_path)
                .await
                .is_err()
            {
                tokio::fs::copy(&job.final_path, &blob_path)
                    .await
                    .map_err(|error| DownloadRunError::Failed(error.to_string()))?;
            }
        } else {
            tokio::fs::rename(&job.temp_path, &blob_path)
                .await
                .map_err(|error| DownloadRunError::Failed(error.to_string()))?;
        }

        let final_path = Path::new(&job.final_path);
        if let Some(parent) = final_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|error| DownloadRunError::Failed(error.to_string()))?;
        }
        if final_path.exists() {
            if recovered_final_size.is_none() {
                let (existing_hash, existing_size) = Self::hash_file(final_path).await?;
                if existing_hash != sha256 || existing_size != measured_size {
                    return Err(DownloadRunError::Failed(
                        "The destination path was occupied by a different file".to_string(),
                    ));
                }
            }
        } else if tokio::fs::hard_link(&blob_path, final_path).await.is_err() {
            tokio::fs::copy(&blob_path, final_path)
                .await
                .map_err(|error| DownloadRunError::Failed(error.to_string()))?;
        }
        if let Some(interruption) = control.interruption() {
            return Err(DownloadRunError::Interrupted(interruption));
        }
        let completed = self
            .repository
            .mark_completed(id, &sha256, measured_size, &relative_blob.to_string_lossy())
            .map_err(DownloadRunError::Failed)?;
        let _ = app_handle.emit("download-job-updated", completed);
        Ok(())
    }

    async fn hash_file(path: &Path) -> Result<(String, u64), DownloadRunError> {
        let mut file = tokio::fs::File::open(path)
            .await
            .map_err(|error| DownloadRunError::Failed(error.to_string()))?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 1024 * 1024];
        let mut size = 0u64;
        loop {
            let read = file
                .read(&mut buffer)
                .await
                .map_err(|error| DownloadRunError::Failed(error.to_string()))?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
            size += read as u64;
        }
        Ok((format!("{:x}", hasher.finalize()), size))
    }

    fn unique_final_path(&self, root: &Path, filename: &str) -> Result<PathBuf, String> {
        let reserved: HashSet<PathBuf> = self
            .repository
            .list()?
            .into_iter()
            .map(|job| PathBuf::from(job.final_path))
            .collect();
        let requested = root.join(filename);
        if !requested.exists() && !reserved.contains(&requested) {
            return Ok(requested);
        }
        let path = Path::new(filename);
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("download");
        let extension = path.extension().and_then(|value| value.to_str());
        for suffix in 1..10_000 {
            let candidate_name = match extension {
                Some(extension) => format!("{stem} ({suffix}).{extension}"),
                None => format!("{stem} ({suffix})"),
            };
            let candidate = root.join(candidate_name);
            if !candidate.exists() && !reserved.contains(&candidate) {
                return Ok(candidate);
            }
        }
        Err("Could not allocate a unique download filename".to_string())
    }
}
