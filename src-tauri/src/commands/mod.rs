pub mod updater;
pub mod window_effects;

use crate::api::models::*;
use crate::api::pawchive::PawchiveClient;
use crate::api::provider::{ProviderConfig, ProviderHealth};
use crate::api::provider_manager::ProviderManager;
use crate::config::settings::{AppSettings, ConfigManager};
use crate::db::content::{CacheStats, ContentRepository};
use crate::db::downloads::DownloadJob;
use crate::db::library::{
    LibraryCollection, LibraryPostIdentity, LibraryRepository, LibrarySaveResult,
};
use crate::db::subscriptions::{Subscription, SubscriptionInput};
use crate::downloader::aria2c::Aria2cManager;
use crate::downloader::manager::DownloadManager;
use crate::downloader::native::NativeDownloader;
use crate::downloader::{DownloadRunError, DownloadTask};
use crate::smart_links::{parse_external_creator_link, parse_external_post_link};
use crate::subscriptions::SubscriptionManager;
use crate::sync::client::SyncDevice;
use crate::sync::manager::{SyncManager, SyncStatus};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResolvedPostLink {
    pub service: String,
    pub creator_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_id: Option<String>,
    #[serde(default)]
    pub link_type: String,
    pub platform: String,
    pub source: String,
}

pub struct AppState {
    pub axum_port: Arc<AtomicU16>,
    pub provider_manager: Arc<ProviderManager>,
    pub pawchive_client: Arc<PawchiveClient>,
    pub content: Arc<ContentRepository>,
    pub library: Arc<LibraryRepository>,
    pub download_manager: Arc<DownloadManager>,
    pub subscription_manager: Arc<SubscriptionManager>,
    pub sync_manager: Arc<SyncManager>,
    pub config_manager: Arc<ConfigManager>,
}

#[tauri::command]
pub fn store_custom_background(
    app: tauri::AppHandle,
    source_path: String,
    kind: String,
) -> Result<String, String> {
    let allowed_extensions: &[&str] = match kind.as_str() {
        "image" => &["png", "jpg", "jpeg", "webp", "gif", "avif"],
        "video" => &["mp4", "webm"],
        _ => return Err("Unsupported custom background type".to_string()),
    };
    let source = std::path::Path::new(&source_path);
    if !source.is_file() {
        return Err("Selected background file does not exist".to_string());
    }
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|value| allowed_extensions.contains(&value.as_str()))
        .ok_or_else(|| "Unsupported custom background file format".to_string())?;
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("background");
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let destination = directory.join(format!("custom-{kind}.{extension}"));
    let importing = directory.join(format!(".importing-{kind}"));
    std::fs::copy(source, &importing).map_err(|error| error.to_string())?;
    remove_custom_background_files(&directory, &kind)?;
    std::fs::rename(&importing, &destination).map_err(|error| error.to_string())?;
    Ok(destination.to_string_lossy().to_string())
}

#[tauri::command]
pub fn store_custom_background_bytes(
    app: tauri::AppHandle,
    data_base64: String,
    extension: String,
    kind: String,
) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let allowed_extensions: &[&str] = match kind.as_str() {
        "image" => &["png", "jpg", "jpeg", "webp", "gif", "avif"],
        "video" => &["mp4", "webm"],
        _ => return Err("Unsupported custom background type".to_string()),
    };
    let ext_clean = extension
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase();
    if !allowed_extensions.contains(&ext_clean.as_str()) {
        return Err("Unsupported custom background file format".to_string());
    }

    let clean_b64 = if let Some(idx) = data_base64.find(";base64,") {
        &data_base64[idx + 8..]
    } else {
        &data_base64
    };

    let bytes = STANDARD
        .decode(clean_b64.trim())
        .map_err(|e| format!("Invalid base64 payload: {e}"))?;

    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("background");
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let destination = directory.join(format!("custom-{kind}.{ext_clean}"));
    let importing = directory.join(format!(".importing-{kind}"));
    std::fs::write(&importing, &bytes).map_err(|error| error.to_string())?;
    remove_custom_background_files(&directory, &kind)?;
    std::fs::rename(&importing, &destination).map_err(|error| error.to_string())?;
    Ok(destination.to_string_lossy().to_string())
}

#[tauri::command]
pub fn clear_custom_background(app: tauri::AppHandle, kind: String) -> Result<(), String> {
    if kind != "image" && kind != "video" {
        return Err("Unsupported custom background type".to_string());
    }
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("background");
    remove_custom_background_files(&directory, &kind)
}

fn remove_custom_background_files(directory: &std::path::Path, kind: &str) -> Result<(), String> {
    if !directory.is_dir() {
        return Ok(());
    }
    let prefix = format!("custom-{kind}.");
    for entry in std::fs::read_dir(directory).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        let matches = path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.starts_with(&prefix));
        if matches && path.is_file() {
            std::fs::remove_file(path).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn get_axum_port(state: State<'_, AppState>) -> u16 {
    state.axum_port.load(Ordering::Acquire)
}

#[tauri::command]
pub fn check_aria2c_installed() -> bool {
    Aria2cManager::is_installed()
}

#[tauri::command]
pub async fn probe_download_size(
    url: String,
    state: State<'_, AppState>,
) -> Result<Option<u64>, String> {
    let parsed = reqwest::Url::parse(&url).map_err(|error| error.to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Only HTTP and HTTPS media probes are supported".to_string());
    }
    let settings = state.config_manager.load()?;
    let task = DownloadTask {
        id: "size-probe".to_string(),
        url,
        output_dir: String::new(),
        temp_path: String::new(),
        final_path: String::new(),
        filename: String::new(),
        session_cookie: (!settings.session_cookie.is_empty()).then_some(settings.session_cookie),
        proxy_mode: settings.proxy_mode,
        proxy_url: settings.proxy_url,
        proxy_username: settings.proxy_username,
        proxy_password: settings.proxy_password,
        proxy_bypass_local: settings.proxy_bypass_local,
        connections: settings.aria2_connections.clamp(1, 32),
    };
    NativeDownloader::probe_total_size(&task)
        .await
        .map_err(|error| match error {
            DownloadRunError::Failed(message) => message,
            DownloadRunError::Interrupted(_) => "Media size probe was interrupted".to_string(),
        })
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let mut settings = state.config_manager.load()?;
    settings.session_cookie.clear();
    settings.proxy_password.clear();
    Ok(settings)
}

#[tauri::command]
pub fn get_default_settings() -> AppSettings {
    AppSettings::default()
}

#[tauri::command]
pub fn get_cache_stats(state: State<'_, AppState>) -> Result<CacheStats, String> {
    state.content.cache_stats()
}

#[tauri::command]
pub fn clear_content_cache(state: State<'_, AppState>) -> Result<CacheStats, String> {
    state.content.clear_cached_images()
}

#[tauri::command]
pub fn clear_all_content_cache(state: State<'_, AppState>) -> Result<CacheStats, String> {
    state.content.clear_all_cache()
}

#[tauri::command]
pub fn wipe_all_data(state: State<'_, AppState>) -> Result<CacheStats, String> {
    state.download_manager.cancel_all();
    state.content.wipe_all_data()
}

#[tauri::command]
pub async fn save_settings(
    mut settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let previous = state.config_manager.load()?;
    // Empty secret fields mean "unchanged": settings returned to Svelte are
    // deliberately redacted, while explicit non-empty replacements still work.
    if settings.session_cookie.is_empty() {
        settings.session_cookie = previous.session_cookie.clone();
    }
    if settings.proxy_password.is_empty() {
        settings.proxy_password = previous.proxy_password.clone();
    }
    state
        .pawchive_client
        .update_settings(settings.clone())
        .await?;
    let _ = state
        .provider_manager
        .update_providers(settings.providers.clone())
        .await;
    if let Err(error) = state.config_manager.save(&settings) {
        let _ = state.pawchive_client.update_settings(previous).await;
        return Err(error);
    }
    let _ = state.content.set_cache_limit_mb(settings.cache_max_mb);
    state.download_manager.notify_scheduler();
    Ok(())
}

#[tauri::command]
pub async fn list_providers(state: State<'_, AppState>) -> Result<Vec<ProviderConfig>, String> {
    Ok(state.provider_manager.get_provider_configs().await)
}

#[tauri::command]
pub async fn save_providers(
    providers: Vec<ProviderConfig>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut settings = state.config_manager.load()?;
    settings.providers = providers.clone();
    state.provider_manager.update_providers(providers).await?;
    state.config_manager.save(&settings)?;
    Ok(())
}

#[tauri::command]
pub async fn test_provider_connection(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<ProviderHealth, String> {
    state
        .provider_manager
        .test_provider_health(&provider_id)
        .await
}

#[tauri::command]
pub async fn get_account_session(state: State<'_, AppState>) -> Result<AccountSession, String> {
    let settings = state.config_manager.load()?;
    if settings.session_cookie.trim().is_empty() {
        return Ok(AccountSession {
            authenticated: false,
            username: None,
        });
    }
    match state
        .provider_manager
        .fetch_account_favorites(None, Some("artist"))
        .await
    {
        Ok(_) => Ok(AccountSession {
            authenticated: true,
            username: Some(settings.pawchive_username),
        }),
        Err(error) if error.contains("401") || error.contains("403") => Ok(AccountSession {
            authenticated: false,
            username: None,
        }),
        Err(_) => Ok(AccountSession {
            authenticated: true,
            username: Some(settings.pawchive_username),
        }),
    }
}

#[tauri::command]
pub async fn login_account(
    username: String,
    password: String,
    state: State<'_, AppState>,
) -> Result<AccountSession, String> {
    let cookie = state
        .provider_manager
        .login("pawchive", &username, &password)
        .await?;
    let previous = state.config_manager.load()?;
    let mut settings = previous.clone();
    settings.session_cookie = cookie.clone();
    settings.pawchive_username = username.trim().to_string();
    if let Some(pawchive) = settings.providers.iter_mut().find(|p| p.id == "pawchive") {
        pawchive.session_cookie = cookie;
        pawchive.username = username.trim().to_string();
    }
    let _ = state
        .provider_manager
        .update_providers(settings.providers.clone())
        .await;
    state
        .pawchive_client
        .update_settings(settings.clone())
        .await?;
    if let Err(error) = state.config_manager.save(&settings) {
        let _ = state.pawchive_client.update_settings(previous).await;
        return Err(error);
    }
    Ok(AccountSession {
        authenticated: true,
        username: Some(settings.pawchive_username),
    })
}

#[tauri::command]
pub async fn logout_account(state: State<'_, AppState>) -> Result<AccountSession, String> {
    let _ = state.provider_manager.logout("pawchive").await;
    let mut settings = state.config_manager.load()?;
    settings.session_cookie.clear();
    settings.pawchive_username.clear();
    if let Some(pawchive) = settings.providers.iter_mut().find(|p| p.id == "pawchive") {
        pawchive.session_cookie.clear();
        pawchive.username.clear();
    }
    let _ = state
        .provider_manager
        .update_providers(settings.providers.clone())
        .await;
    state
        .pawchive_client
        .update_settings(settings.clone())
        .await?;
    state.config_manager.clear_session_cookie()?;
    state.config_manager.save(&settings)?;
    Ok(AccountSession {
        authenticated: false,
        username: None,
    })
}

#[tauri::command]
pub async fn fetch_creators(state: State<'_, AppState>) -> Result<Vec<Creator>, String> {
    match state.provider_manager.fetch_creators().await {
        Ok(creators) => {
            state.content.save_creators(&creators)?;
            Ok(creators)
        }
        Err(error) => {
            let cached = state.content.list_creators()?;
            if cached.is_empty() {
                Err(error)
            } else {
                Ok(cached)
            }
        }
    }
}

#[tauri::command]
pub async fn fetch_posts(
    service: String,
    user_id: String,
    offset: u32,
    state: State<'_, AppState>,
) -> Result<Vec<PawchivePost>, String> {
    let list_key = format!("creator:{service}:{user_id}:");
    match state
        .provider_manager
        .fetch_posts(&service, &user_id, offset, None)
        .await
    {
        Ok(posts) => {
            state.content.save_post_list(&list_key, offset, &posts)?;
            Ok(posts)
        }
        Err(error) => {
            let cached = state.content.load_post_list(&list_key, offset)?;
            if cached.is_empty() {
                Err(error)
            } else {
                Ok(cached)
            }
        }
    }
}

#[tauri::command]
pub async fn fetch_recent_posts(
    query: Option<String>,
    offset: u32,
    state: State<'_, AppState>,
) -> Result<Vec<PawchivePost>, String> {
    let list_key = format!("recent:{}", query.as_deref().unwrap_or(""));
    match state
        .provider_manager
        .fetch_recent_posts(query.as_deref(), offset)
        .await
    {
        Ok(posts) => {
            state.content.save_post_list(&list_key, offset, &posts)?;
            Ok(posts)
        }
        Err(error) => {
            let mut cached = state.content.load_post_list(&list_key, offset)?;
            if cached.is_empty() && query.as_deref().unwrap_or("").is_empty() {
                cached = state.content.list_recent_posts(offset, 50)?;
            }
            if cached.is_empty() {
                Err(error)
            } else {
                Ok(cached)
            }
        }
    }
}

#[tauri::command]
pub async fn fetch_popular_posts(
    period: String,
    date: Option<String>,
    offset: u32,
    state: State<'_, AppState>,
) -> Result<Vec<PawchivePost>, String> {
    let list_key = format!("popular:{period}:{}", date.as_deref().unwrap_or(""));
    match state
        .provider_manager
        .fetch_popular_posts(&period, date.as_deref(), offset)
        .await
    {
        Ok(posts) => {
            state.content.save_post_list(&list_key, offset, &posts)?;
            Ok(posts)
        }
        Err(error) => {
            let cached = state.content.load_post_list(&list_key, offset)?;
            if cached.is_empty() {
                Err(error)
            } else {
                Ok(cached)
            }
        }
    }
}

#[tauri::command]
pub async fn fetch_creator_posts(
    service: String,
    creator_id: String,
    query: Option<String>,
    offset: u32,
    state: State<'_, AppState>,
) -> Result<Vec<PawchivePost>, String> {
    let list_key = format!(
        "creator:{service}:{creator_id}:{}",
        query.as_deref().unwrap_or("")
    );
    let result = state
        .provider_manager
        .fetch_posts(&service, &creator_id, offset, query.as_deref())
        .await;
    match result {
        Ok(posts) => {
            state.content.save_post_list(&list_key, offset, &posts)?;
            Ok(posts)
        }
        Err(error) => {
            let mut cached = state.content.load_post_list(&list_key, offset)?;
            if cached.is_empty() && query.as_deref().unwrap_or("").is_empty() {
                cached = state
                    .content
                    .list_creator_posts(&service, &creator_id, offset, 50)?;
            }
            if cached.is_empty() {
                Err(error)
            } else {
                Ok(cached)
            }
        }
    }
}

#[tauri::command]
pub async fn fetch_creator_profile(
    service: String,
    creator_id: String,
    state: State<'_, AppState>,
) -> Result<CreatorProfile, String> {
    match state
        .provider_manager
        .fetch_creator_profile(&service, &creator_id)
        .await
    {
        Ok(creator) => {
            let profile = CreatorProfile {
                id: creator.id.clone(),
                name: creator.name.clone(),
                service: creator.service.clone(),
                public_id: creator.public_id,
                relation_id: creator.relation_id,
                indexed: creator.indexed.map(serde_json::Value::from),
                updated: creator.updated.map(serde_json::Value::from),
                kemono_favorited: creator.kemono_favorited,
                ever_imported: creator.ever_imported,
                extra: creator.extra,
            };
            state.content.save_creator(&profile)?;
            Ok(profile)
        }
        Err(error) => state
            .content
            .get_creator(&service, &creator_id)?
            .ok_or(error),
    }
}

#[tauri::command]
pub async fn fetch_announcements(
    service: String,
    creator_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Announcement>, String> {
    match state
        .pawchive_client
        .fetch_announcements(&service, &creator_id)
        .await
    {
        Ok(items) => {
            state
                .content
                .save_document("announcements", &service, &creator_id, "", &items)?;
            Ok(items)
        }
        Err(error) => state
            .content
            .load_document("announcements", &service, &creator_id, "")?
            .ok_or(error),
    }
}

#[tauri::command]
pub async fn fetch_fancards(
    service: String,
    creator_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Fancard>, String> {
    match state
        .provider_manager
        .fetch_fancards(&service, &creator_id)
        .await
    {
        Ok(items) => {
            state
                .content
                .save_document("fancards", &service, &creator_id, "", &items)?;
            Ok(items)
        }
        Err(error) => state
            .content
            .load_document("fancards", &service, &creator_id, "")?
            .ok_or(error),
    }
}

#[tauri::command]
pub async fn fetch_creator_links(
    service: String,
    creator_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<CreatorProfile>, String> {
    match state
        .provider_manager
        .fetch_creator_links(&service, &creator_id)
        .await
    {
        Ok(items) => {
            state
                .content
                .save_document("creator_links", &service, &creator_id, "", &items)?;
            Ok(items)
        }
        Err(error) => state
            .content
            .load_document("creator_links", &service, &creator_id, "")?
            .ok_or(error),
    }
}

#[tauri::command]
pub async fn fetch_similar_creators(
    service: String,
    creator_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<CreatorProfile>, String> {
    match state
        .provider_manager
        .fetch_similar_creators(&service, &creator_id)
        .await
    {
        Ok(items) => {
            state
                .content
                .save_document("similar_creators", &service, &creator_id, "", &items)?;
            Ok(items)
        }
        Err(error) => state
            .content
            .load_document("similar_creators", &service, &creator_id, "")?
            .ok_or(error),
    }
}

#[tauri::command]
pub async fn fetch_creator_tags(
    service: String,
    creator_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    match state
        .provider_manager
        .fetch_creator_tags(&service, &creator_id)
        .await
    {
        Ok(items) => {
            if !items.is_empty() {
                let _ =
                    state
                        .content
                        .save_document("creator_tags", &service, &creator_id, "", &items);
            }
            Ok(items)
        }
        Err(error) => state
            .content
            .load_document("creator_tags", &service, &creator_id, "")?
            .ok_or(error),
    }
}

#[tauri::command]
pub async fn fetch_post(
    service: String,
    creator_id: String,
    post_id: String,
    state: State<'_, AppState>,
) -> Result<PawchivePost, String> {
    match state
        .provider_manager
        .fetch_post(&service, &creator_id, &post_id)
        .await
    {
        Ok(Some(reconciled)) => {
            state
                .content
                .save_posts(std::slice::from_ref(&reconciled.post))?;
            if !reconciled.revisions.is_empty() {
                let _ = state.content.save_post_revisions(
                    &service,
                    &creator_id,
                    &post_id,
                    reconciled
                        .available_providers
                        .first()
                        .map(String::as_str)
                        .unwrap_or("pawchive"),
                    &reconciled.revisions,
                );
            }
            Ok(reconciled.post)
        }
        Ok(None) => state
            .content
            .get_post(&service, &creator_id, &post_id)?
            .ok_or_else(|| "Post not found".to_string()),
        Err(error) => state
            .content
            .get_post(&service, &creator_id, &post_id)?
            .ok_or(error),
    }
}

#[tauri::command]
pub async fn get_cached_post(
    service: String,
    creator_id: String,
    post_id: String,
    state: State<'_, AppState>,
) -> Result<Option<PawchivePost>, String> {
    state.content.get_post(&service, &creator_id, &post_id)
}

#[tauri::command]
pub async fn resolve_external_post_link(
    url: String,
    current_service: Option<String>,
    current_creator_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<ResolvedPostLink>, String> {
    let mut expanded_opt: Option<String> = None;

    let post_parsed = if let Some(parsed) = parse_external_post_link(&url) {
        Some(parsed)
    } else if let Ok(Some(expanded_url)) = state.provider_manager.expand_short_link(&url).await {
        let res = parse_external_post_link(&expanded_url);
        expanded_opt = Some(expanded_url);
        res
    } else {
        None
    };

    if let Some(parsed) = post_parsed {
        let preferred_creator = current_service
            .as_deref()
            .filter(|service| service.eq_ignore_ascii_case(&parsed.service))
            .and(current_creator_id.as_deref());

        if let Some((service, creator_id, post_id)) =
            state
                .content
                .find_post_identity(&parsed.service, &parsed.post_id, preferred_creator)?
        {
            return Ok(Some(ResolvedPostLink {
                platform: parsed.service,
                service,
                creator_id,
                post_id: Some(post_id),
                link_type: "post".into(),
                source: "cache".into(),
            }));
        }

        let mut candidates = Vec::new();
        if let Some(creator_id) = preferred_creator {
            candidates.push(creator_id.to_string());
        }
        if let Some(hint) = parsed.creator_hint.as_deref() {
            if let Some(creator_id) = state.content.find_creator_by_alias(&parsed.service, hint)? {
                if !candidates.contains(&creator_id) {
                    candidates.push(creator_id);
                }
            }
            if !candidates.iter().any(|candidate| candidate == hint) {
                candidates.push(hint.to_string());
            }
        }

        for creator_id in candidates {
            if let Ok(Some(reconciled)) = state
                .provider_manager
                .fetch_post(&parsed.service, &creator_id, &parsed.post_id)
                .await
            {
                state
                    .content
                    .save_posts(std::slice::from_ref(&reconciled.post))?;
                return Ok(Some(ResolvedPostLink {
                    service: reconciled.post.service,
                    creator_id: reconciled.post.user,
                    post_id: Some(reconciled.post.id),
                    link_type: "post".into(),
                    platform: parsed.service,
                    source: "remote".into(),
                }));
            }
        }

        if let Ok(Some((service, creator_id, post_id))) = state
            .provider_manager
            .resolve_post_identity(&parsed.service, &parsed.post_id)
            .await
        {
            return Ok(Some(ResolvedPostLink {
                platform: parsed.service,
                service,
                creator_id,
                post_id: Some(post_id),
                link_type: "post".into(),
                source: "remote".into(),
            }));
        }
    }

    // Check for creator profile links
    let target_url = expanded_opt.as_deref().unwrap_or(&url);
    if let Some(creator_link) = parse_external_creator_link(target_url) {
        if let Some(creator_id) = state
            .content
            .find_creator_by_alias(&creator_link.service, &creator_link.creator_hint)?
        {
            return Ok(Some(ResolvedPostLink {
                service: creator_link.service.clone(),
                creator_id,
                post_id: None,
                link_type: "creator".into(),
                platform: creator_link.service,
                source: "cache".into(),
            }));
        }

        if let Ok(profile) = state
            .provider_manager
            .fetch_creator_profile(&creator_link.service, &creator_link.creator_hint)
            .await
        {
            let _ = state.content.save_creators(std::slice::from_ref(&profile));
            return Ok(Some(ResolvedPostLink {
                service: profile.service,
                creator_id: profile.id,
                post_id: None,
                link_type: "creator".into(),
                platform: creator_link.service,
                source: "remote".into(),
            }));
        }

        if let Ok(creators) = state.provider_manager.fetch_creators().await {
            let _ = state.content.save_creators(&creators);
            if let Some(creator_id) = state
                .content
                .find_creator_by_alias(&creator_link.service, &creator_link.creator_hint)?
            {
                return Ok(Some(ResolvedPostLink {
                    service: creator_link.service.clone(),
                    creator_id,
                    post_id: None,
                    link_type: "creator".into(),
                    platform: creator_link.service,
                    source: "remote".into(),
                }));
            }
        }
    }

    Ok(None)
}

#[tauri::command]
pub async fn fetch_account_favorites(
    favorite_type: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Favorite>, String> {
    let kind = favorite_type.as_deref().unwrap_or("post");
    if !matches!(kind, "post" | "artist") {
        return Err("favorite_type must be 'post' or 'artist'".to_string());
    }
    let settings = state.config_manager.load()?;
    let account = settings.pawchive_username;
    let is_authenticated = !settings.session_cookie.is_empty();

    let local_favorites = state
        .content
        .list_favorites(kind, &account)
        .unwrap_or_default();

    if is_authenticated {
        if let Ok(remote_items) = state
            .provider_manager
            .fetch_account_favorites(None, Some(kind))
            .await
        {
            // Pin remote favorites locally so they persist offline too
            for fav in &remote_items {
                if kind == "post" {
                    if let Ok(post) = serde_json::from_value::<PawchivePost>(
                        serde_json::to_value(fav).unwrap_or_default(),
                    ) {
                        let _ = state.content.pin_post(&post, "favorite", &account);
                    }
                } else if let Ok(creator) =
                    serde_json::from_value::<Creator>(serde_json::to_value(fav).unwrap_or_default())
                {
                    let _ = state.content.save_creators(std::slice::from_ref(&creator));
                    let _ = state.content.set_pin(
                        "creator",
                        &creator.service,
                        &creator.id,
                        None,
                        "favorite",
                        &account,
                        true,
                    );
                }
            }

            // Build map of local favorites with their faved_at timestamps from content_pins
            let mut local_map = std::collections::HashMap::new();
            for loc in local_favorites {
                let srv = loc.service.as_deref().unwrap_or("").to_lowercase();
                let id = loc.id.to_lowercase();
                local_map.insert((srv, id), loc);
            }

            let mut merged = Vec::new();
            let mut seen_keys = std::collections::HashSet::new();

            // First include all remote items, carrying over any local faved_at if already pinned
            for mut item in remote_items {
                let srv = item.service.as_deref().unwrap_or("").to_lowercase();
                let id = item.id.to_lowercase();
                if seen_keys.insert((srv.clone(), id.clone())) {
                    if let Some(local_entry) = local_map.get(&(srv, id)) {
                        if let Some(faved_at) = local_entry.extra.get("faved_at") {
                            item.extra.insert("faved_at".to_string(), faved_at.clone());
                        }
                    }
                    merged.push(item);
                }
            }

            // Then include all purely local favorites (not present on remote)
            for (key, local) in local_map {
                if seen_keys.insert(key) {
                    merged.push(local);
                }
            }

            return Ok(merged);
        }
    }

    let mut ordered_locals = local_favorites;
    let count = ordered_locals.len() as i64;
    for (idx, item) in ordered_locals.iter_mut().enumerate() {
        if item.faved_seq.is_none() {
            item.faved_seq = Some(count - idx as i64);
        }
    }

    Ok(ordered_locals)
}

#[tauri::command]
pub async fn set_post_favorite(
    service: String,
    creator_id: String,
    post_id: String,
    favorite: bool,
    state: State<'_, AppState>,
) -> Result<ApiActionResult, String> {
    let settings = state.config_manager.load()?;
    let account = settings.pawchive_username;
    let is_authenticated = !settings.session_cookie.is_empty();

    if is_authenticated {
        let _ = state
            .provider_manager
            .set_post_favorite(&service, &creator_id, &post_id, favorite)
            .await;
    }

    if favorite {
        let post = match state
            .provider_manager
            .fetch_post(&service, &creator_id, &post_id)
            .await
        {
            Ok(Some(reconciled)) => reconciled.post,
            _ => state
                .content
                .get_post(&service, &creator_id, &post_id)?
                .ok_or_else(|| "Post is not cached".to_string())?,
        };
        state.content.pin_post(&post, "favorite", &account)?;
    } else {
        state.content.set_pin(
            "post",
            &service,
            &creator_id,
            Some(&post_id),
            "favorite",
            &account,
            false,
        )?;
        if !account.is_empty() {
            let _ = state.content.set_pin(
                "post",
                &service,
                &creator_id,
                Some(&post_id),
                "favorite",
                "",
                false,
            );
        }
    }
    Ok(ApiActionResult {
        status: 200,
        success: true,
    })
}

#[tauri::command]
pub async fn set_creator_favorite(
    service: String,
    creator_id: String,
    favorite: bool,
    state: State<'_, AppState>,
) -> Result<ApiActionResult, String> {
    let settings = state.config_manager.load()?;
    let account = settings.pawchive_username;
    let is_authenticated = !settings.session_cookie.is_empty();

    if is_authenticated {
        let _ = state
            .provider_manager
            .set_creator_favorite(&service, &creator_id, favorite)
            .await;
    }

    if favorite {
        let profile = match state
            .provider_manager
            .fetch_creator_profile(&service, &creator_id)
            .await
        {
            Ok(creator) => CreatorProfile {
                id: creator.id,
                name: creator.name,
                service: creator.service,
                public_id: creator.public_id,
                relation_id: creator.relation_id,
                indexed: creator.indexed.map(|v| serde_json::Value::Number(v.into())),
                updated: creator.updated.map(|v| serde_json::Value::Number(v.into())),
                kemono_favorited: creator.kemono_favorited,
                ever_imported: creator.ever_imported,
                extra: creator.extra,
            },
            Err(_) => {
                if let Some(cached) = state.content.get_creator(&service, &creator_id)? {
                    cached
                } else {
                    CreatorProfile {
                        id: creator_id.clone(),
                        name: creator_id.clone(),
                        service: service.clone(),
                        public_id: None,
                        relation_id: None,
                        indexed: None,
                        updated: None,
                        kemono_favorited: None,
                        ever_imported: None,
                        extra: Default::default(),
                    }
                }
            }
        };
        let _ = state.content.save_creator(&profile);
        state.content.set_pin(
            "creator",
            &service,
            &creator_id,
            None,
            "favorite",
            &account,
            true,
        )?;
    } else {
        state.content.set_pin(
            "creator",
            &service,
            &creator_id,
            None,
            "favorite",
            &account,
            false,
        )?;
        if !account.is_empty() {
            let _ = state.content.set_pin(
                "creator",
                &service,
                &creator_id,
                None,
                "favorite",
                "",
                false,
            );
        }
    }
    Ok(ApiActionResult {
        status: 200,
        success: true,
    })
}

#[tauri::command]
pub async fn fetch_creator_artwork_data_url(
    service: String,
    creator_id: String,
    artwork_kind: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if let Some(cached) = state
        .content
        .artwork_data_url(&service, &creator_id, &artwork_kind)?
    {
        return Ok(cached);
    }
    let data = state
        .provider_manager
        .fetch_creator_artwork_data_url(&service, &creator_id, &artwork_kind)
        .await?;
    state
        .content
        .store_artwork_data_url(&service, &creator_id, &artwork_kind, &data)?;
    Ok(data)
}

#[tauri::command]
pub async fn search_hash(
    file_hash: String,
    state: State<'_, AppState>,
) -> Result<FileSearchResult, String> {
    if file_hash.len() != 64 || !file_hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("file_hash must be a 64-character SHA-256 hex string".to_string());
    }
    state.provider_manager.search_hash(&file_hash).await
}

#[tauri::command]
pub async fn flag_post(
    service: String,
    creator_id: String,
    post_id: String,
    state: State<'_, AppState>,
) -> Result<ApiActionResult, String> {
    state
        .provider_manager
        .flag_post(&service, &creator_id, &post_id)
        .await
}

#[tauri::command]
pub async fn is_post_flagged(
    service: String,
    creator_id: String,
    post_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    state
        .provider_manager
        .is_post_flagged(&service, &creator_id, &post_id)
        .await
}

#[tauri::command]
pub async fn fetch_post_revisions(
    service: String,
    creator_id: String,
    post_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<PostRevision>, String> {
    match state
        .provider_manager
        .fetch_post_revisions(&service, &creator_id, &post_id)
        .await
    {
        Ok(items) if !items.is_empty() => {
            let _ = state.content.save_post_revisions(
                &service,
                &creator_id,
                &post_id,
                "pawchive",
                &items,
            );
            Ok(items)
        }
        _ => {
            let revisions = state
                .content
                .load_post_revisions(&service, &creator_id, &post_id)
                .unwrap_or_default();
            Ok(revisions)
        }
    }
}

#[tauri::command]
pub async fn fetch_post_comments(
    service: String,
    creator_id: String,
    post_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Comment>, String> {
    match state
        .provider_manager
        .fetch_post_comments(&service, &creator_id, &post_id)
        .await
    {
        Ok(items) => {
            state.content.save_document(
                "post_comments",
                &service,
                &creator_id,
                &post_id,
                &items,
            )?;
            Ok(items)
        }
        Err(error) => state
            .content
            .load_document("post_comments", &service, &creator_id, &post_id)?
            .ok_or(error),
    }
}

#[tauri::command]
pub async fn get_pawchive_app_version(state: State<'_, AppState>) -> Result<String, String> {
    state.provider_manager.app_version().await
}

#[tauri::command]
pub async fn search_posts(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<PawchivePost>, String> {
    state.content.search_posts(&query)
}

#[tauri::command]
pub fn list_library_collections(
    state: State<'_, AppState>,
) -> Result<Vec<LibraryCollection>, String> {
    state.library.list_collections()
}

#[tauri::command]
pub fn create_library_stash(
    name: String,
    state: State<'_, AppState>,
) -> Result<LibraryCollection, String> {
    state.library.create_stash(&name)
}

#[tauri::command]
pub fn delete_library_stash(
    collection_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    state.library.delete_stash(&collection_id)
}

#[tauri::command]
pub fn rename_library_stash(
    collection_id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    state.library.rename_stash(&collection_id, &name)
}

#[tauri::command]
pub fn reorder_library_stashes(
    collection_ids: Vec<String>,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let result = state.library.reorder_stashes(&collection_ids);
    state.sync_manager.trigger_sync_on_change(app_handle);
    result
}

#[tauri::command]
pub fn clear_library_stash(
    collection_id: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<u64, String> {
    let result = state.library.clear_stash(&collection_id);
    state.sync_manager.trigger_sync_on_change(app_handle);
    result
}

#[tauri::command]
pub fn remove_library_post_from_stash(
    collection_id: String,
    service: String,
    creator_id: String,
    post_id: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let result =
        state
            .library
            .remove_post_from_stash(&collection_id, &service, &creator_id, &post_id);
    state.sync_manager.trigger_sync_on_change(app_handle);
    result
}

#[tauri::command]
pub fn list_post_collections(
    service: String,
    creator_id: String,
    post_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    state
        .library
        .list_post_collections(&service, &creator_id, &post_id)
}

#[tauri::command]
pub fn save_library_post(
    post: PawchivePost,
    collection_id: Option<String>,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<LibrarySaveResult, String> {
    let result = state.library.save_post(&post, collection_id.as_deref());
    state.sync_manager.trigger_sync_on_change(app_handle);
    result
}

#[tauri::command]
pub fn remove_library_post(
    service: String,
    creator_id: String,
    post_id: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let result = state.library.remove_post(&service, &creator_id, &post_id);
    state.sync_manager.trigger_sync_on_change(app_handle);
    result
}

#[tauri::command]
pub fn list_saved_post_identities(
    state: State<'_, AppState>,
) -> Result<Vec<LibraryPostIdentity>, String> {
    state.library.list_saved_post_identities()
}

#[tauri::command]
pub fn list_post_stash_memberships(
    state: State<'_, AppState>,
) -> Result<Vec<crate::db::library::PostStashMembership>, String> {
    state.library.list_post_stash_memberships()
}

#[tauri::command]
pub fn list_library_posts(
    collection_id: Option<String>,
    offset: u32,
    limit: u32,
    state: State<'_, AppState>,
) -> Result<Vec<PawchivePost>, String> {
    state
        .library
        .list_posts(collection_id.as_deref(), offset, limit)
}

#[tauri::command]
pub async fn start_download(
    post: PawchivePost,
    media_id: String,
    url: String,
    filename: String,
    _app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<DownloadJob, String> {
    let settings = state.config_manager.load()?;
    state.content.pin_post(&post, "download", "")?;
    if let Ok(profile) = state
        .pawchive_client
        .fetch_creator_profile(&post.service, &post.user)
        .await
    {
        let _ = state.content.save_creator(&profile);
    }
    for kind in ["avatar", "banner"] {
        if state
            .content
            .artwork_path(&post.service, &post.user, kind)?
            .is_none()
        {
            if let Ok(data) = state
                .pawchive_client
                .fetch_creator_artwork_data_url(&post.service, &post.user, kind)
                .await
            {
                let _ =
                    state
                        .content
                        .store_artwork_data_url(&post.service, &post.user, kind, &data);
            }
        }
    }
    if let Some(file) = post
        .file
        .as_ref()
        .filter(|file| file.path.is_some())
        .or_else(|| {
            post.attachments
                .as_ref()
                .and_then(|items| items.iter().find(|file| file.path.is_some()))
        })
        .and_then(|file| file.path.as_deref())
    {
        let base = if settings.image_domain.starts_with("http") {
            settings.image_domain.clone()
        } else {
            format!("https://{}", settings.image_domain)
        };
        let _ = state
            .content
            .cache_post_preview(
                &post,
                &format!("{}/thumbnail/data{}", base.trim_end_matches('/'), file),
            )
            .await;
    }
    let creator_name = state
        .content
        .get_creator(&post.service, &post.user)
        .ok()
        .flatten()
        .map(|c| c.name);

    let index = {
        let mut idx = 1;
        let mut found = false;
        if let Some(file) = &post.file {
            if file.path.as_deref() == Some(&media_id) {
                found = true;
            } else {
                idx += 1;
            }
        }
        if !found {
            if let Some(attachments) = &post.attachments {
                for att in attachments {
                    if att.path.as_deref() == Some(&media_id) {
                        found = true;
                        break;
                    }
                    idx += 1;
                }
            }
        }
        if found {
            idx
        } else {
            1
        }
    };

    if settings.download_save_metadata {
        let tags_vec: Option<Vec<String>> = post.tags.as_ref().and_then(|t| {
            if let Some(arr) = t.as_array() {
                Some(
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect(),
                )
            } else {
                t.as_str()
                    .map(|s| s.split(',').map(|v| v.trim().to_string()).collect())
            }
        });

        let root = crate::downloader::manager::DownloadManager::ensure_download_root(
            &settings.download_dir,
        )
        .unwrap_or_else(|_| std::path::PathBuf::from(&settings.download_dir));
        let c_name = creator_name.as_deref().unwrap_or("");
        let ctx = crate::downloader::template::TemplateContext {
            service: &post.service,
            creator_id: &post.user,
            creator_name: c_name,
            post_id: &post.id,
            post_title: &post.title,
            published: post.published.as_deref(),
            original_filename: &filename,
            index,
            media_id: &media_id,
        };
        let mut target_dir = root;
        if settings.download_group_by_creator {
            let creator_folder = crate::downloader::template::resolve_creator_folder(
                &settings.download_creator_folder_template,
                &ctx,
            );
            if !creator_folder.is_empty() {
                target_dir = target_dir.join(creator_folder);
            }
        }
        if settings.download_group_by_post {
            let post_folder = crate::downloader::template::resolve_post_folder(
                &settings.download_post_folder_template,
                &ctx,
            );
            if !post_folder.is_empty() {
                target_dir = target_dir.join(post_folder);
            }
        }
        let _ = std::fs::create_dir_all(&target_dir);
        let meta = crate::downloader::metadata::PostMetadataExport {
            service: &post.service,
            creator_id: &post.user,
            creator_name: c_name,
            post_id: &post.id,
            post_title: &post.title,
            published: post.published.as_deref(),
            content: post.content.as_deref(),
            tags: tags_vec.as_deref(),
            origin_url: post.origin.clone(),
        };
        let _ = crate::downloader::metadata::save_post_metadata(&target_dir, &meta, &settings);
    }

    state.download_manager.enqueue(
        post.service,
        post.user,
        creator_name,
        post.id,
        Some(post.title),
        post.published,
        media_id,
        url,
        filename,
        index,
    )
}

#[tauri::command]
pub fn list_downloads(state: State<'_, AppState>) -> Result<Vec<DownloadJob>, String> {
    let mut jobs = state.download_manager.list()?;
    let settings = state.config_manager.load()?;
    let image_base = if settings.image_domain.starts_with("http") {
        settings.image_domain
    } else {
        format!("https://{}", settings.image_domain)
    };
    for job in &mut jobs {
        let Some(post) = state
            .content
            .get_post(&job.service, &job.creator_id, &job.post_id)?
        else {
            continue;
        };
        let path = post
            .file
            .as_ref()
            .filter(|file| file.path.is_some())
            .or_else(|| {
                post.attachments
                    .as_ref()
                    .and_then(|items| items.iter().find(|file| file.path.is_some()))
            })
            .and_then(|file| file.path.as_deref());
        if let Some(path) = path {
            job.post_preview_url = Some(format!(
                "{}/thumbnail/data{}",
                image_base.trim_end_matches('/'),
                path
            ));
        }
    }
    Ok(jobs)
}

#[tauri::command]
pub fn pause_download(
    download_id: String,
    state: State<'_, AppState>,
) -> Result<DownloadJob, String> {
    state.download_manager.pause(&download_id)
}

#[tauri::command]
pub fn cancel_download(
    download_id: String,
    state: State<'_, AppState>,
) -> Result<DownloadJob, String> {
    state.download_manager.cancel(&download_id)
}

#[tauri::command]
pub fn resume_download(
    download_id: String,
    state: State<'_, AppState>,
) -> Result<DownloadJob, String> {
    state.download_manager.resume(&download_id)
}

#[tauri::command]
pub fn retry_download(
    download_id: String,
    state: State<'_, AppState>,
) -> Result<DownloadJob, String> {
    state.download_manager.retry(&download_id)
}

#[tauri::command]
pub fn remove_download(download_id: String, state: State<'_, AppState>) -> Result<bool, String> {
    state.download_manager.remove(&download_id)
}

#[tauri::command]
pub fn list_subscriptions(state: State<'_, AppState>) -> Result<Vec<Subscription>, String> {
    state.subscription_manager.list()
}

#[tauri::command]
pub async fn upsert_subscription(
    input: SubscriptionInput,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Subscription, String> {
    if let Ok(profile) = state
        .pawchive_client
        .fetch_creator_profile(&input.service, &input.creator_id)
        .await
    {
        state.content.save_creator(&profile)?;
    }
    state.content.set_pin(
        "creator",
        &input.service,
        &input.creator_id,
        None,
        "subscription",
        "",
        true,
    )?;
    let (subscription, created) = state.subscription_manager.repository().upsert(&input)?;
    state
        .sync_manager
        .trigger_sync_on_change(app_handle.clone());
    state
        .subscription_manager
        .clone()
        .refresh(subscription.id, created, app_handle)
        .await
}

#[tauri::command]
pub fn set_subscription_enabled(
    subscription_id: String,
    enabled: bool,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Subscription, String> {
    let result = state
        .subscription_manager
        .repository()
        .set_enabled(&subscription_id, enabled);
    state.sync_manager.trigger_sync_on_change(app_handle);
    result
}

#[tauri::command]
pub async fn refresh_subscription(
    subscription_id: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Subscription, String> {
    state
        .subscription_manager
        .clone()
        .refresh(subscription_id, false, app_handle)
        .await
}

#[tauri::command]
pub fn delete_subscription(
    subscription_id: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    if let Some(subscription) = state
        .subscription_manager
        .repository()
        .get(&subscription_id)?
    {
        state.content.set_pin(
            "creator",
            &subscription.service,
            &subscription.creator_id,
            None,
            "subscription",
            "",
            false,
        )?;
    }
    let result = state
        .subscription_manager
        .repository()
        .delete(&subscription_id);
    state.sync_manager.trigger_sync_on_change(app_handle);
    result
}

#[tauri::command]
pub fn get_sync_status(state: State<'_, AppState>) -> Result<SyncStatus, String> {
    state.sync_manager.status()
}

#[tauri::command]
pub async fn create_sync_account(
    server_url: String,
    account_id: String,
    master_password: String,
    device_name: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<SyncStatus, String> {
    state
        .sync_manager
        .clone()
        .create_account(
            server_url,
            account_id,
            master_password,
            device_name,
            app_handle,
        )
        .await
}

#[tauri::command]
pub async fn connect_sync_account(
    server_url: String,
    account_id: String,
    master_password: String,
    device_name: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<SyncStatus, String> {
    state
        .sync_manager
        .clone()
        .connect(
            server_url,
            account_id,
            master_password,
            device_name,
            app_handle,
        )
        .await
}

#[tauri::command]
pub fn unlock_sync(
    master_password: String,
    state: State<'_, AppState>,
) -> Result<SyncStatus, String> {
    state.sync_manager.unlock(&master_password)
}

#[tauri::command]
pub fn lock_sync(state: State<'_, AppState>) -> Result<SyncStatus, String> {
    state.sync_manager.lock()
}

#[tauri::command]
pub fn disconnect_sync(state: State<'_, AppState>) -> Result<SyncStatus, String> {
    state.sync_manager.disconnect()
}

#[tauri::command]
pub async fn change_sync_password(
    current_password: String,
    new_password: String,
    state: State<'_, AppState>,
) -> Result<SyncStatus, String> {
    state
        .sync_manager
        .change_password(&current_password, &new_password)
        .await
}

#[tauri::command]
pub async fn list_sync_devices(state: State<'_, AppState>) -> Result<Vec<SyncDevice>, String> {
    state.sync_manager.devices().await
}

#[tauri::command]
pub async fn revoke_sync_device(
    device_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<SyncDevice>, String> {
    state.sync_manager.revoke_device(&device_id).await
}

#[tauri::command]
pub fn get_sync_recovery_kit(state: State<'_, AppState>) -> Result<String, String> {
    state.sync_manager.recovery_kit()
}

#[tauri::command]
pub fn copy_sync_recovery_kit(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let kit = state.sync_manager.recovery_kit()?;
    app_handle
        .clipboard()
        .write_text(kit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn recover_sync_account(
    recovery_kit: String,
    new_password: String,
    device_name: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<SyncStatus, String> {
    state
        .sync_manager
        .clone()
        .recover(&recovery_kit, &new_password, &device_name, app_handle)
        .await
}

#[tauri::command]
pub async fn run_sync(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<SyncStatus, String> {
    state.sync_manager.clone().sync(app_handle).await
}

#[tauri::command]
pub async fn resolve_sync_conflict(
    resolution: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<SyncStatus, String> {
    match resolution.as_str() {
        "local" => state.sync_manager.clone().resolve_local(app_handle).await,
        "remote" => state.sync_manager.clone().resolve_remote(app_handle).await,
        _ => Err("resolution must be 'local' or 'remote'".to_string()),
    }
}

#[tauri::command]
pub fn set_sync_enabled(
    enabled: bool,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<SyncStatus, String> {
    let status = state.sync_manager.set_enabled(enabled)?;
    let _ = app_handle.emit("sync-status-updated", &status);
    Ok(status)
}

#[tauri::command]
pub fn open_in_browser(url: String) -> Result<(), String> {
    let parsed = reqwest::Url::parse(&url).map_err(|error| error.to_string())?;
    if !matches!(parsed.scheme(), "http" | "https" | "ms-windows-store") {
        return Err("Only HTTP, HTTPS, and Store links can be opened".to_string());
    }
    let safe_url = parsed.as_str();
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let operation = OsStr::new("open")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let target = OsStr::new(safe_url)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                target.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        if result as isize <= 32 {
            Err(format!(
                "Windows could not open the URL (ShellExecuteW code {})",
                result as isize
            ))
        } else {
            Ok(())
        }
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(safe_url)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(safe_url)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "android")]
    {
        with_android_context(|env, activity| {
            use jni::objects::JValue;
            let url_jstr = env.new_string(safe_url).map_err(|e| e.to_string())?;

            let uri_class = env
                .find_class("android/net/Uri")
                .map_err(|e| e.to_string())?;
            let uri_obj = env
                .call_static_method(
                    uri_class,
                    "parse",
                    "(Ljava/lang/String;)Landroid/net/Uri;",
                    &[JValue::Object(&url_jstr)],
                )
                .map_err(|e| e.to_string())?
                .l()
                .map_err(|e| e.to_string())?;

            let intent_class = env
                .find_class("android/content/Intent")
                .map_err(|e| e.to_string())?;
            let action_view = env
                .new_string("android.intent.action.VIEW")
                .map_err(|e| e.to_string())?;
            let intent_obj = env
                .new_object(
                    intent_class,
                    "(Ljava/lang/String;Landroid/net/Uri;)V",
                    &[JValue::Object(&action_view), JValue::Object(&uri_obj)],
                )
                .map_err(|e| e.to_string())?;

            env.call_method(
                activity,
                "startActivity",
                "(Landroid/content/Intent;)V",
                &[JValue::Object(&intent_obj)],
            )
            .map_err(|e| e.to_string())?;

            Ok(())
        })
    }
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "android"
    )))]
    {
        let _ = safe_url;
        Err("Unsupported operating system".to_string())
    }
}

#[tauri::command]
pub fn open_downloads_folder(state: State<'_, AppState>) -> Result<(), String> {
    let settings = state.config_manager.load()?;
    let folder = std::path::PathBuf::from(&settings.download_dir);
    let _ = std::fs::create_dir_all(&folder);

    #[cfg(target_os = "windows")]
    {
        let folder = std::fs::canonicalize(folder).map_err(|error| error.to_string())?;
        std::process::Command::new("explorer.exe")
            .arg(&folder)
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        let folder = std::fs::canonicalize(folder).map_err(|error| error.to_string())?;
        std::process::Command::new("open")
            .arg(&folder)
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        let folder = std::fs::canonicalize(folder).map_err(|error| error.to_string())?;
        std::process::Command::new("xdg-open")
            .arg(&folder)
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "android")]
    {
        let path_str = folder.to_string_lossy().to_string();
        with_android_context(|env, context| {
            let jstr = env
                .new_string(&path_str)
                .map_err(|e| format!("New string error: {e}"))?;
            env.call_method(
                context,
                "openFolderInFileManager",
                "(Ljava/lang/String;)V",
                &[jni::objects::JValue::Object(&jstr)],
            )
            .map_err(|e| format!("Failed to open folder in Android file manager: {e}"))?;
            Ok(())
        })
    }
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "android"
    )))]
    {
        let _ = folder;
        Err("Opening the downloads folder is unsupported on this operating system".to_string())
    }
}

#[tauri::command]
pub fn open_download_file(file_path: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(&file_path);
    if !path.exists() {
        return Err("File not found on device".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let canonical = std::fs::canonicalize(&path).unwrap_or(path);
        let path_str = canonical.to_string_lossy().to_string();
        let path_clean = path_str.trim_start_matches(r"\\?\");

        let operation = OsStr::new("open")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let target = OsStr::new(path_clean)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                target.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        if result as isize <= 32 {
            std::process::Command::new("cmd")
                .args(["/C", "start", "", path_clean])
                .spawn()
                .map_err(|e| format!("Failed to open file: {e}"))?;
        }
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "android")]
    {
        let path_str = path.to_string_lossy().to_string();
        with_android_context(|env, context| {
            let jstr = env
                .new_string(&path_str)
                .map_err(|e| format!("New string error: {e}"))?;
            env.call_method(
                context,
                "openFileInNativeViewer",
                "(Ljava/lang/String;)V",
                &[jni::objects::JValue::Object(&jstr)],
            )
            .map_err(|e| format!("Failed to open file in Android viewer: {e}"))?;
            Ok(())
        })
    }
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "android"
    )))]
    {
        let _ = path;
        Err("Opening files is unsupported on this operating system".to_string())
    }
}

#[tauri::command]
pub fn show_in_folder(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    if !p.exists() {
        return Err("Path not found on device".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        let canonical = std::fs::canonicalize(&p).unwrap_or_else(|_| p.clone());
        let path_str = canonical.to_string_lossy().to_string();
        let clean_path = path_str.trim_start_matches(r"\\?\").replace('/', "\\");

        if canonical.is_file() {
            std::process::Command::new("explorer.exe")
                .raw_arg(format!("/select,\"{clean_path}\""))
                .spawn()
                .map_err(|e| format!("Failed to reveal file in explorer: {e}"))?;
        } else {
            std::process::Command::new("explorer.exe")
                .raw_arg(format!("\"{clean_path}\""))
                .spawn()
                .map_err(|e| format!("Failed to open folder in explorer: {e}"))?;
        }
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        if p.is_file() {
            std::process::Command::new("open")
                .args(["-R", &path])
                .spawn()
                .map_err(|e| format!("Failed to reveal in Finder: {e}"))?;
        } else {
            std::process::Command::new("open")
                .arg(&path)
                .spawn()
                .map_err(|e| format!("Failed to open in Finder: {e}"))?;
        }
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        let folder = if p.is_file() {
            p.parent().unwrap_or(&p).to_string_lossy().to_string()
        } else {
            path
        };
        std::process::Command::new("xdg-open")
            .arg(&folder)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {e}"))?;
        Ok(())
    }
    #[cfg(target_os = "android")]
    {
        let folder = if p.is_file() {
            p.parent().unwrap_or(&p).to_string_lossy().to_string()
        } else {
            path
        };
        with_android_context(|env, context| {
            let jstr = env
                .new_string(&folder)
                .map_err(|e| format!("New string error: {e}"))?;
            env.call_method(
                context,
                "openFolderInFileManager",
                "(Ljava/lang/String;)V",
                &[jni::objects::JValue::Object(&jstr)],
            )
            .map_err(|e| format!("Failed to open folder in Android file manager: {e}"))?;
            Ok(())
        })
    }
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "android"
    )))]
    {
        let _ = p;
        Err("Showing in folder is unsupported on this operating system".to_string())
    }
}

#[cfg(target_os = "android")]
static FOLDER_PICKER_TX: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<Option<String>>>> =
    std::sync::Mutex::new(None);

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_app_pawstash_client_MainActivity_onFolderPicked(
    mut env: jni::JNIEnv,
    _class: jni::objects::JClass,
    path_jstr: jni::objects::JString,
) {
    let path: Option<String> = if !path_jstr.is_null() {
        env.get_string(&path_jstr)
            .ok()
            .map(|s| s.to_string_lossy().to_string())
    } else {
        None
    };

    if let Ok(mut lock) = FOLDER_PICKER_TX.lock() {
        if let Some(tx) = lock.take() {
            let _ = tx.send(path);
        }
    }
}

#[cfg(target_os = "android")]
static ANDROID_APP_CONTEXT: std::sync::RwLock<Option<(jni::JavaVM, jni::objects::GlobalRef)>> =
    std::sync::RwLock::new(None);

#[cfg(target_os = "android")]
static APP_HANDLE: std::sync::RwLock<Option<tauri::AppHandle>> = std::sync::RwLock::new(None);

#[cfg(target_os = "android")]
pub fn set_android_app_handle(handle: tauri::AppHandle) {
    if let Ok(mut lock) = APP_HANDLE.write() {
        *lock = Some(handle);
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_app_pawstash_client_MainActivity_onDeepLinkReceived(
    mut env: jni::JNIEnv,
    _class: jni::objects::JClass,
    json_jstr: jni::objects::JString,
) {
    if let Ok(json) = env.get_string(&json_jstr) {
        let json_str = json.to_string_lossy().to_string();
        if let Ok(lock) = APP_HANDLE.read() {
            if let Some(handle) = lock.as_ref() {
                use tauri::Emitter;
                let _ = handle.emit("open-post-deep-link", json_str);
            }
        }
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_app_pawstash_client_MainActivity_initAndroidContext(
    env: jni::JNIEnv,
    _class: jni::objects::JClass,
    activity: jni::objects::JObject,
) {
    if let Ok(vm) = env.get_java_vm() {
        if let Ok(global_ref) = env.new_global_ref(&activity) {
            if let Ok(mut lock) = ANDROID_APP_CONTEXT.write() {
                *lock = Some((vm, global_ref));
            }
        }
    }
}

#[cfg(target_os = "android")]
pub fn with_android_context<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce(&mut jni::JNIEnv, &jni::objects::JObject) -> Result<R, String>,
{
    let lock = ANDROID_APP_CONTEXT
        .read()
        .map_err(|e| format!("Lock error: {e}"))?;
    let (vm, context_ref) = lock
        .as_ref()
        .ok_or_else(|| "Android context not initialized yet".to_string())?;

    let mut env = vm
        .attach_current_thread_as_daemon()
        .map_err(|e| format!("JNI attach error: {e}"))?;

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }

    let res = f(&mut env, context_ref.as_obj());

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }

    res
}

#[cfg(target_os = "android")]
fn launch_folder_picker_android() -> Result<(), String> {
    with_android_context(|env, context| {
        env.call_method(context, "launchFolderPicker", "()V", &[])
            .map_err(|e| format!("Failed to launch native folder picker: {e}"))?;
        Ok(())
    })
}

#[tauri::command]
pub async fn pick_folder() -> Result<Option<String>, String> {
    #[cfg(target_os = "android")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let mut lock = FOLDER_PICKER_TX
                .lock()
                .map_err(|e| format!("Lock error: {e}"))?;
            *lock = Some(tx);
        }

        launch_folder_picker_android()?;

        match rx.await {
            Ok(path) => Ok(path),
            Err(_) => Ok(None),
        }
    }

    #[cfg(not(target_os = "android"))]
    {
        Ok(None)
    }
}

#[tauri::command]
pub async fn get_pending_deep_link() -> Result<Option<String>, String> {
    Ok(crate::downloader::notifications::get_pending_deep_link())
}

#[tauri::command]
pub fn show_main_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    #[cfg(desktop)]
    {
        use tauri::Manager;
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
    }
    let _ = app_handle;
    Ok(())
}

#[tauri::command]
pub fn hide_to_tray(app_handle: tauri::AppHandle) -> Result<(), String> {
    #[cfg(desktop)]
    {
        use tauri::Manager;
        for (_, window) in app_handle.webview_windows() {
            if window.is_fullscreen().unwrap_or(false) {
                let _ = window.set_fullscreen(false);
            }
            let _ = window.hide();
        }
    }
    let _ = app_handle;
    Ok(())
}

#[tauri::command]
pub fn update_panic_key(
    _shortcut: String,
    _enabled: bool,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    #[cfg(desktop)]
    {
        use tauri_plugin_global_shortcut::GlobalShortcutExt;
        let _ = app_handle.global_shortcut().unregister_all();
    }
    let _ = app_handle;
    Ok(())
}

#[tauri::command]
pub fn update_boss_key(
    shortcut: String,
    enabled: bool,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    update_panic_key(shortcut, enabled, app_handle)
}

#[tauri::command]
pub async fn resolve_cloud_link(
    url: String,
    state: State<'_, AppState>,
) -> Result<crate::cloud::CloudFolderResult, String> {
    if let Ok(Some(cached)) = state
        .content
        .load_document::<crate::cloud::CloudFolderResult>("cloud_folder", &url, "", "")
    {
        return Ok(cached);
    }

    let settings = state.config_manager.load().ok();
    let resolver = crate::cloud::CloudResolver::new(settings.as_ref());
    match resolver.resolve(&url).await {
        Ok(result) => {
            let _ = state
                .content
                .save_document("cloud_folder", &url, "", "", &result);
            Ok(result)
        }
        Err(error) => {
            if let Ok(Some(cached)) = state
                .content
                .load_document::<crate::cloud::CloudFolderResult>("cloud_folder", &url, "", "")
            {
                return Ok(cached);
            }
            Err(error)
        }
    }
}

#[tauri::command]
pub fn log_message(level: String, message: String, context: Option<serde_json::Value>) {
    let ctx_str = context.map(|c| c.to_string()).unwrap_or_default();
    match level.to_lowercase().as_str() {
        "error" => {
            if ctx_str.is_empty() {
                tracing::error!(target: "frontend", "{message}");
            } else {
                tracing::error!(target: "frontend", context = %ctx_str, "{message}");
            }
        }
        "warn" => {
            if ctx_str.is_empty() {
                tracing::warn!(target: "frontend", "{message}");
            } else {
                tracing::warn!(target: "frontend", context = %ctx_str, "{message}");
            }
        }
        "debug" => {
            if ctx_str.is_empty() {
                tracing::debug!(target: "frontend", "{message}");
            } else {
                tracing::debug!(target: "frontend", context = %ctx_str, "{message}");
            }
        }
        _ => {
            if ctx_str.is_empty() {
                tracing::info!(target: "frontend", "{message}");
            } else {
                tracing::info!(target: "frontend", context = %ctx_str, "{message}");
            }
        }
    }
}

#[tauri::command]
pub fn get_debug_log_path() -> Result<String, String> {
    Ok(crate::logging::log_file_path()
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
pub fn read_recent_logs(lines: Option<usize>) -> Result<String, String> {
    crate::logging::read_recent_logs(lines.unwrap_or(500))
}

#[tauri::command]
pub fn open_logs_folder() -> Result<(), String> {
    let folder = crate::logging::logs_dir();
    let _ = std::fs::create_dir_all(&folder);

    #[cfg(target_os = "windows")]
    {
        let folder = std::fs::canonicalize(folder).map_err(|error| error.to_string())?;
        std::process::Command::new("explorer.exe")
            .arg(&folder)
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        let folder = std::fs::canonicalize(folder).map_err(|error| error.to_string())?;
        std::process::Command::new("open")
            .arg(&folder)
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        let folder = std::fs::canonicalize(folder).map_err(|error| error.to_string())?;
        std::process::Command::new("xdg-open")
            .arg(&folder)
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "android")]
    {
        let path_str = folder.to_string_lossy().to_string();
        with_android_context(|env, context| {
            let jstr = env
                .new_string(&path_str)
                .map_err(|e| format!("New string error: {e}"))?;
            env.call_method(
                context,
                "openFolderInFileManager",
                "(Ljava/lang/String;)V",
                &[jni::objects::JValue::Object(&jstr)],
            )
            .map_err(|e| format!("Failed to open folder in Android file manager: {e}"))?;
            Ok(())
        })
    }
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "android"
    )))]
    {
        Err("Unsupported operating system".to_string())
    }
}

#[tauri::command]
pub fn clear_logs() -> Result<(), String> {
    crate::logging::clear_log_file()
}
