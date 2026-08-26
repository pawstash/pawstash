use crate::config::settings::{AppSettings, ConfigManager, ProxyMode};
use crate::db::downloads::{DownloadJob, DownloadRepository, NewDownloadJob};
use crate::downloader::aria2c::Aria2cManager;
use crate::downloader::native::NativeDownloader;
use crate::downloader::template::{
    resolve_creator_folder, resolve_filename, resolve_post_folder, TemplateContext,
};
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
    config: Arc<ConfigManager>,
    active: Mutex<HashMap<String, Arc<DownloadControl>>>,
    notify: Arc<tokio::sync::Notify>,
}

impl DownloadManager {
    pub fn new(repository: Arc<DownloadRepository>, config: Arc<ConfigManager>) -> Self {
        Self {
            repository,
            config,
            active: Mutex::new(HashMap::new()),
            notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    pub fn notify_scheduler(&self) {
        self.notify.notify_waiters();
    }

    pub fn list(&self) -> Result<Vec<DownloadJob>, String> {
        self.repository.list()
    }

    pub fn start(self: &Arc<Self>, app_handle: tauri::AppHandle) {
        let manager = self.clone();
        tauri::async_runtime::spawn(async move {
            if let Ok(ids) = manager.repository.recover_interrupted() {
                if !ids.is_empty() {
                    manager.notify.notify_waiters();
                }
            }

            loop {
                tokio::select! {
                    _ = manager.notify.notified() => {},
                    _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {},
                }

                manager.schedule_next(&app_handle).await;
            }
        });
    }

    async fn schedule_next(self: &Arc<Self>, app_handle: &tauri::AppHandle) {
        let settings = match self.config.load() {
            Ok(s) => s,
            Err(_) => return,
        };
        let max_concurrent = settings.download_max_concurrent.clamp(1, 10) as usize;

        let active_count = match self.active.lock() {
            Ok(guard) => guard.len(),
            Err(_) => return,
        };

        if active_count >= max_concurrent {
            return;
        }

        let available_slots = max_concurrent - active_count;
        let queued_ids = match self.repository.next_queued_jobs(available_slots) {
            Ok(ids) => ids,
            Err(_) => return,
        };

        for id in queued_ids {
            let control = Arc::new(DownloadControl::new());
            {
                let mut active = match self.active.lock() {
                    Ok(g) => g,
                    Err(_) => continue,
                };
                if active.len() >= max_concurrent {
                    break;
                }
                if active.contains_key(&id) {
                    continue;
                }
                active.insert(id.clone(), control.clone());
            }

            let manager = self.clone();
            let settings = settings.clone();
            let app_handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                manager.run(id, settings, control, app_handle).await;
            });
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn enqueue(
        self: &Arc<Self>,
        service: String,
        creator_id: String,
        creator_name: Option<String>,
        post_id: String,
        post_title: Option<String>,
        published: Option<String>,
        media_id: String,
        url: String,
        filename: String,
        index: usize,
    ) -> Result<DownloadJob, String> {
        let settings = self.config.load()?;
        let parsed = reqwest::Url::parse(&url).map_err(|error| error.to_string())?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err("Only HTTP and HTTPS downloads are supported".to_string());
        }
        let safe_filename = Path::new(&filename)
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("file")
            .to_string();

        let root = Self::ensure_download_root(&settings.download_dir)?;

        let c_name = creator_name.as_deref().unwrap_or("");
        let p_title = post_title.as_deref().unwrap_or("");

        let ctx = TemplateContext {
            service: &service,
            creator_id: &creator_id,
            creator_name: c_name,
            post_id: &post_id,
            post_title: p_title,
            published: published.as_deref(),
            original_filename: &safe_filename,
            index: if index == 0 { 1 } else { index },
            media_id: &media_id,
        };

        let mut target_dir = root.clone();
        if settings.download_group_by_creator {
            let creator_folder =
                resolve_creator_folder(&settings.download_creator_folder_template, &ctx);
            if !creator_folder.is_empty() {
                target_dir = target_dir.join(creator_folder);
            }
        }
        if settings.download_group_by_post {
            let post_folder = resolve_post_folder(&settings.download_post_folder_template, &ctx);
            if !post_folder.is_empty() {
                target_dir = target_dir.join(post_folder);
            }
        }

        std::fs::create_dir_all(&target_dir).map_err(|error| error.to_string())?;

        if settings.download_save_metadata {
            let meta = crate::downloader::metadata::PostMetadataExport {
                service: &service,
                creator_id: &creator_id,
                creator_name: c_name,
                post_id: &post_id,
                post_title: p_title,
                published: published.as_deref(),
                content: None,
                tags: None,
                origin_url: None,
            };
            let _ = crate::downloader::metadata::save_post_metadata(&target_dir, &meta, &settings);
        }

        let resolved_filename = resolve_filename(&settings.download_filename_template, &ctx);
        let final_path = self.unique_final_path(&target_dir, &resolved_filename)?;
        let id = Uuid::new_v4().to_string();
        let temp_dir = {
            #[cfg(target_os = "android")]
            {
                std::env::temp_dir().join("pawstash_temp")
            }
            #[cfg(not(target_os = "android"))]
            {
                root.join(".temp")
            }
        };
        let _ = std::fs::create_dir_all(&temp_dir);
        let temp_path = temp_dir.join(format!("{id}.part"));
        let custom_socks = settings.proxy_mode == ProxyMode::Custom
            && (settings.proxy_url.starts_with("socks5://")
                || settings.proxy_url.starts_with("socks5h://"));
        let engine = if settings.use_aria2c && Aria2cManager::is_installed() && !custom_socks {
            "aria2c"
        } else {
            "native"
        };
        let logical_key = format!("{service}:{creator_id}:{post_id}:{media_id}");
        let mut job = self.repository.create_or_get(NewDownloadJob {
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
                .unwrap_or(&resolved_filename),
            output_dir: &root.to_string_lossy(),
            temp_path: &temp_path.to_string_lossy(),
            final_path: &final_path.to_string_lossy(),
            engine,
        })?;

        if matches!(
            job.status.as_str(),
            "paused" | "failed" | "cancelled" | "missing"
        ) {
            job = self.repository.retry(&job.id)?;
        }
        self.notify.notify_waiters();
        Ok(job)
    }

    pub fn pause(&self, id: &str) -> Result<DownloadJob, String> {
        let active_ctrl = {
            self.active
                .lock()
                .map_err(|error| error.to_string())?
                .get(id)
                .cloned()
        };
        let job = self.repository.update_status(id, "paused")?;
        if let Some(control) = active_ctrl {
            control.pause();
        }
        self.notify.notify_waiters();
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
            let _ = std::fs::remove_file(format!("{}.aria2", job.temp_path));
        }
        self.notify.notify_waiters();
        Ok(job)
    }

    pub fn cancel_all(&self) {
        if let Ok(active) = self.active.lock() {
            for control in active.values() {
                control.cancel();
            }
        }
        let _ = self.repository.cancel_all_queued();
        self.notify.notify_waiters();
    }

    pub fn resume(&self, id: &str) -> Result<DownloadJob, String> {
        let current = self
            .repository
            .get(id)?
            .ok_or_else(|| "Download job not found".to_string())?;
        if current.status != "paused" {
            return Err("Only paused downloads can be resumed".to_string());
        }
        let job = self.repository.update_status(id, "queued")?;
        self.notify.notify_waiters();
        Ok(job)
    }

    pub fn retry(&self, id: &str) -> Result<DownloadJob, String> {
        let job = self.repository.retry(id)?;
        if job.status != "queued" {
            return Err("Only failed, cancelled, or missing downloads can be retried".to_string());
        }
        self.notify.notify_waiters();
        Ok(job)
    }

    pub fn remove(&self, id: &str) -> Result<bool, String> {
        let control = self
            .active
            .lock()
            .map_err(|error| error.to_string())?
            .remove(id);
        if let Some(ctrl) = control {
            ctrl.cancel();
        }

        let job = match self.repository.get(id)? {
            Some(j) => j,
            None => return Ok(false),
        };
        let output_root = std::fs::canonicalize(&job.output_dir).ok();

        if let Some(root) = output_root.as_deref() {
            if job.status == "completed" {
                let _ = Self::remove_download_file(root, Path::new(&job.final_path));
            } else {
                let temp_path = Path::new(&job.temp_path);
                if temp_path.exists() {
                    let _ = Self::remove_download_file(root, temp_path);
                }
                let aria2_path = PathBuf::from(format!("{}.aria2", job.temp_path));
                if aria2_path.exists() {
                    let _ = Self::remove_download_file(root, &aria2_path);
                }
            }
        } else {
            let _ = std::fs::remove_file(&job.temp_path);
            let _ = std::fs::remove_file(format!("{}.aria2", job.temp_path));
            if job.status == "completed" {
                let _ = std::fs::remove_file(&job.final_path);
            }
        }

        let changed = self.repository.remove(id)?;
        if changed {
            if let Some(sha256) = job.sha256.as_deref() {
                if let Some(relative_path) = self.repository.take_orphan_blob(sha256)? {
                    if let Some(root) = output_root.as_deref() {
                        let _ = Self::remove_download_file(root, &root.join(relative_path));
                    }
                }
            }
        }
        self.notify.notify_waiters();
        Ok(changed)
    }

    fn hide_folder(path: &Path) {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Storage::FileSystem::{
                SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN,
            };

            let wide: Vec<u16> = path
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            unsafe {
                SetFileAttributesW(wide.as_ptr(), FILE_ATTRIBUTE_HIDDEN);
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = path;
        }
    }

    pub fn ensure_download_root(preferred: &str) -> Result<PathBuf, String> {
        #[cfg(target_os = "android")]
        {
            let candidates = [
                PathBuf::from(preferred),
                PathBuf::from("/storage/emulated/0/Download/Pawstash"),
                PathBuf::from("/storage/emulated/0/Download"),
                PathBuf::from(
                    "/storage/emulated/0/Android/data/app.pawstash.client/files/Download",
                ),
                PathBuf::from("/data/user/0/app.pawstash.client/files/Pawstash/Downloads"),
                PathBuf::from("/data/data/app.pawstash.client/files/Pawstash/Downloads"),
            ];

            for candidate in candidates {
                if candidate.as_os_str().is_empty() {
                    continue;
                }
                if std::fs::create_dir_all(&candidate).is_ok() {
                    let test_file = candidate.join(".write_test");
                    if std::fs::write(&test_file, b"ok").is_ok() {
                        let _ = std::fs::remove_file(&test_file);
                        return Ok(candidate);
                    }
                }
            }
            return Err("Unable to access any writable download directory on Android".to_string());
        }

        #[cfg(not(target_os = "android"))]
        {
            let root = PathBuf::from(preferred);
            let temp = root.join(".temp");
            let media = root.join(".media");
            if std::fs::create_dir_all(&temp).is_ok() && std::fs::create_dir_all(&media).is_ok() {
                Self::hide_folder(&temp);
                Self::hide_folder(&media);
                return Ok(root);
            }

            std::fs::create_dir_all(&temp).map_err(|error| error.to_string())?;
            std::fs::create_dir_all(&media).map_err(|error| error.to_string())?;
            Self::hide_folder(&temp);
            Self::hide_folder(&media);
            Ok(root)
        }
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

    async fn run(
        self: Arc<Self>,
        id: String,
        settings: AppSettings,
        control: Arc<DownloadControl>,
        app_handle: tauri::AppHandle,
    ) {
        match self.run_inner(&id, settings, control, &app_handle).await {
            Err(DownloadRunError::Failed(message)) => {
                eprintln!("[Pawstash Downloader] Job {id} failed: {message}");
                tracing::error!(id = %id, error = %message, "Download job failed");
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
            if active.is_empty() {
                crate::downloader::notifications::stop_download_service();
            }
        }
        self.notify.notify_waiters();
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

        if let Ok((active, total_queued, downloaded_total, expected_total, speed_total)) =
            self.repository.queue_progress_stats()
        {
            crate::downloader::notifications::update_download_notification(
                active.max(1),
                total_queued.max(1),
                downloaded_total,
                expected_total,
                speed_total,
                &job.filename,
            );
        }

        let effective_url = if job.url.starts_with("https://pawchive.pw/data/") {
            job.url.replacen(
                "https://pawchive.pw/data/",
                "https://file.pawchive.pw/data/",
                1,
            )
        } else if job.url.starts_with("http://pawchive.pw/data/") {
            job.url.replacen(
                "http://pawchive.pw/data/",
                "https://file.pawchive.pw/data/",
                1,
            )
        } else {
            job.url.clone()
        };

        let task = DownloadTask {
            id: job.id.clone(),
            url: effective_url,
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
        let final_path = Path::new(&job.final_path);
        if let Some(parent) = final_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|error| {
                DownloadRunError::Failed(format!("Failed to create destination folder: {error}"))
            })?;
        }

        if recovered_final_size.is_none()
            && tokio::fs::rename(&job.temp_path, final_path).await.is_err()
        {
            tokio::fs::copy(&job.temp_path, final_path)
                .await
                .map_err(|error| {
                    DownloadRunError::Failed(format!("Failed to write final file: {error}"))
                })?;
            let _ = tokio::fs::remove_file(&job.temp_path).await;
        }

        let relative_blob = PathBuf::from(".media").join(&sha256[0..2]).join(&sha256);
        #[cfg(not(target_os = "android"))]
        {
            if let Ok(root) = Self::ensure_download_root(&settings.download_dir) {
                let blob_path = root.join(&relative_blob);
                if let Some(parent) = blob_path.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                if !blob_path.exists() {
                    let _ = tokio::fs::hard_link(final_path, &blob_path).await;
                }
            }
        }
        if let Some(interruption) = control.interruption() {
            return Err(DownloadRunError::Interrupted(interruption));
        }
        let completed = self
            .repository
            .mark_completed(id, &sha256, measured_size, &relative_blob.to_string_lossy())
            .map_err(DownloadRunError::Failed)?;
        Self::notify_system_media_scan(&job.final_path);
        let _ = app_handle.emit("download-job-updated", completed.clone());
        crate::downloader::notifications::notify_download_completed(
            &completed.service,
            &completed.creator_id,
            &completed.post_id,
            &completed.filename,
            &completed.post_title,
            1,
        );
        Ok(())
    }

    fn notify_system_media_scan(path: &str) {
        #[cfg(target_os = "android")]
        {
            let path_str = path.to_string();
            let _ = crate::commands::with_android_context(|env, context| {
                if let Ok(path_jstr) = env.new_string(&path_str) {
                    if let Ok(class) = env.get_object_class(context) {
                        let _ = env.call_static_method(
                            &class,
                            "scanMediaFile",
                            "(Ljava/lang/String;)V",
                            &[jni::objects::JValue::Object(&path_jstr)],
                        );
                    }
                }
                Ok(())
            });
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = path;
        }
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
