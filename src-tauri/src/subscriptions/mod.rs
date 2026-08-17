use crate::api::models::{Attachment, PawchivePost};
use crate::api::pawchive::PawchiveClient;
use crate::config::settings::{AppSettings, ConfigManager};
use crate::db::content::ContentRepository;
use crate::db::library::LibraryRepository;
use crate::db::subscriptions::{Subscription, SubscriptionRepository};
use crate::downloader::manager::DownloadManager;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Emitter;

pub struct SubscriptionManager {
    repository: Arc<SubscriptionRepository>,
    client: Arc<PawchiveClient>,
    library: Arc<LibraryRepository>,
    content: Arc<ContentRepository>,
    downloads: Arc<DownloadManager>,
    config: Arc<ConfigManager>,
    running: Mutex<HashSet<String>>,
}

impl SubscriptionManager {
    pub fn new(
        repository: Arc<SubscriptionRepository>,
        client: Arc<PawchiveClient>,
        library: Arc<LibraryRepository>,
        content: Arc<ContentRepository>,
        downloads: Arc<DownloadManager>,
        config: Arc<ConfigManager>,
    ) -> Self {
        Self {
            repository,
            client,
            library,
            content,
            downloads,
            config,
            running: Mutex::new(HashSet::new()),
        }
    }

    pub fn repository(&self) -> &SubscriptionRepository {
        &self.repository
    }
    pub fn list(&self) -> Result<Vec<Subscription>, String> {
        self.repository.list()
    }

    pub fn start(self: &Arc<Self>, app_handle: tauri::AppHandle) {
        let manager = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                if let Ok(due) = manager.repository.due() {
                    for subscription in due {
                        let worker = manager.clone();
                        let handle = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = worker.refresh(subscription.id, false, handle).await;
                        });
                    }
                }
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });
    }

    pub async fn refresh(
        self: &Arc<Self>,
        id: String,
        initial: bool,
        app_handle: tauri::AppHandle,
    ) -> Result<Subscription, String> {
        {
            let mut running = self.running.lock().map_err(|e| e.to_string())?;
            if !running.insert(id.clone()) {
                return self
                    .repository
                    .get(&id)?
                    .ok_or_else(|| "Subscription not found".to_string());
            }
        }
        let result = self.refresh_inner(&id, initial, &app_handle).await;
        self.running.lock().map_err(|e| e.to_string())?.remove(&id);
        match result {
            Ok(updated) => {
                let _ = app_handle.emit("subscription-updated", &updated);
                Ok(updated)
            }
            Err(error) => {
                let failed = self.repository.mark_failure(&id, &error)?;
                let _ = app_handle.emit("subscription-updated", &failed);
                Err(error)
            }
        }
    }

    async fn refresh_inner(
        self: &Arc<Self>,
        id: &str,
        initial: bool,
        app_handle: &tauri::AppHandle,
    ) -> Result<Subscription, String> {
        let subscription = self
            .repository
            .get(id)?
            .ok_or_else(|| "Subscription not found".to_string())?;
        if !subscription.enabled {
            return Ok(subscription);
        }
        let max_pages = if initial {
            if subscription.initial_import == "all" {
                100
            } else {
                1
            }
        } else {
            4
        };
        let seed_only = initial && subscription.initial_import == "none";
        let settings = self.config.load()?;
        for page in 0..max_pages {
            let posts = self
                .client
                .fetch_creator_posts(
                    &subscription.service,
                    &subscription.creator_id,
                    None,
                    page * 50,
                )
                .await?;
            if posts.is_empty() {
                break;
            }
            let identities: Vec<String> = posts.iter().map(Self::post_identity).collect();
            let seen = self.repository.seen(id, &identities)?;
            let all_seen = identities.iter().all(|identity| seen.contains(identity));
            if !seed_only {
                for (post, identity) in posts.iter().zip(identities.iter()) {
                    if seen.contains(identity) {
                        continue;
                    }
                    self.content.pin_post(post, "subscription", "")?;
                    self.library
                        .save_post(post, Some(&subscription.destination_collection_id))?;
                    if subscription.auto_download {
                        self.enqueue_media(
                            post,
                            &subscription.download_scope,
                            &settings,
                            app_handle,
                        );
                    }
                }
            }
            self.repository.mark_seen(id, &identities)?;
            if posts.len() < 50 || (!initial && all_seen) {
                break;
            }
        }
        self.repository.mark_success(id)
    }

    fn post_identity(post: &PawchivePost) -> String {
        format!("{}:{}:{}", post.service, post.user, post.id)
    }

    fn enqueue_media(
        self: &Arc<Self>,
        post: &PawchivePost,
        scope: &str,
        settings: &AppSettings,
        app_handle: &tauri::AppHandle,
    ) {
        let mut files: Vec<&Attachment> = Vec::new();
        if let Some(file) = post.file.as_ref().filter(|file| file.path.is_some()) {
            files.push(file);
        }
        if scope == "all" {
            for attachment in post.attachments.as_deref().unwrap_or_default() {
                if attachment.path.is_some()
                    && !files.iter().any(|item| item.path == attachment.path)
                {
                    files.push(attachment);
                }
            }
        }
        for (index, file) in files.into_iter().enumerate() {
            let Some(path) = file.path.as_deref() else {
                continue;
            };
            let base = file.server.clone().unwrap_or_else(|| {
                if settings.file_domain.starts_with("http://")
                    || settings.file_domain.starts_with("https://")
                {
                    settings.file_domain.clone()
                } else {
                    format!("https://{}", settings.file_domain)
                }
            });
            let url = format!("{}/data{}", base.trim_end_matches('/'), path);
            let filename = file
                .name
                .clone()
                .unwrap_or_else(|| format!("{}_{}", post.id, index + 1));
            let _ = self.downloads.enqueue(
                post.service.clone(),
                post.user.clone(),
                post.id.clone(),
                path.to_string(),
                url,
                filename,
                settings.clone(),
                app_handle.clone(),
            );
        }
    }
}
