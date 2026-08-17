pub mod updater;
pub mod window_effects;

use crate::api::models::*;
use crate::api::pawchive::PawchiveClient;
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
use crate::smart_links::parse_external_post_link;
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
    pub post_id: String,
    pub platform: String,
    pub source: String,
}

pub struct AppState {
    pub axum_port: Arc<AtomicU16>,
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
    if let Err(error) = state.config_manager.save(&settings) {
        let _ = state.pawchive_client.update_settings(previous).await;
        return Err(error);
    }
    let _ = state.content.set_cache_limit_mb(settings.cache_max_mb);
    Ok(())
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
        .pawchive_client
        .fetch_account_favorites(Some("artist"))
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
    let cookie = state.pawchive_client.login(&username, &password).await?;
    let previous = state.config_manager.load()?;
    let mut settings = previous.clone();
    settings.session_cookie = cookie;
    settings.pawchive_username = username.trim().to_string();
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
    let _ = state.pawchive_client.logout().await;
    let mut settings = state.config_manager.load()?;
    settings.session_cookie.clear();
    settings.pawchive_username.clear();
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
    match state.pawchive_client.fetch_creators().await {
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
        .pawchive_client
        .fetch_creator_posts(&service, &user_id, None, offset)
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
        .pawchive_client
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
        .pawchive_client
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
        .pawchive_client
        .fetch_creator_posts(&service, &creator_id, query.as_deref(), offset)
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
        .pawchive_client
        .fetch_creator_profile(&service, &creator_id)
        .await
    {
        Ok(profile) => {
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
        .pawchive_client
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
        .pawchive_client
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
pub async fn fetch_post(
    service: String,
    creator_id: String,
    post_id: String,
    state: State<'_, AppState>,
) -> Result<PawchivePost, String> {
    match state
        .pawchive_client
        .fetch_post(&service, &creator_id, &post_id)
        .await
    {
        Ok(post) => {
            state.content.save_posts(std::slice::from_ref(&post))?;
            Ok(post)
        }
        Err(error) => state
            .content
            .get_post(&service, &creator_id, &post_id)?
            .ok_or(error),
    }
}

#[tauri::command]
pub async fn resolve_external_post_link(
    url: String,
    current_service: Option<String>,
    current_creator_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<ResolvedPostLink>, String> {
    let parsed = if let Some(parsed) = parse_external_post_link(&url) {
        parsed
    } else {
        let Some(expanded_url) = state.pawchive_client.expand_short_link(&url).await? else {
            return Ok(None);
        };
        let Some(parsed) = parse_external_post_link(&expanded_url) else {
            return Ok(None);
        };
        parsed
    };
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
            post_id,
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
        if let Ok(post) = state
            .pawchive_client
            .fetch_post(&parsed.service, &creator_id, &parsed.post_id)
            .await
        {
            state.content.save_posts(std::slice::from_ref(&post))?;
            return Ok(Some(ResolvedPostLink {
                service: post.service,
                creator_id: post.user,
                post_id: post.id,
                platform: parsed.service,
                source: "remote".into(),
            }));
        }
    }

    if let Ok(Some((service, creator_id, post_id))) = state
        .pawchive_client
        .resolve_post_identity(&parsed.service, &parsed.post_id)
        .await
    {
        return Ok(Some(ResolvedPostLink {
            platform: parsed.service,
            service,
            creator_id,
            post_id,
            source: "remote".into(),
        }));
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
            .pawchive_client
            .fetch_account_favorites(Some(kind))
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
            return Ok(remote_items);
        }
    }

    Ok(local_favorites)
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
            .pawchive_client
            .set_post_favorite(&service, &creator_id, &post_id, favorite)
            .await;
    }

    if favorite {
        let post = match state
            .pawchive_client
            .fetch_post(&service, &creator_id, &post_id)
            .await
        {
            Ok(post) => post,
            Err(_) => state
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
            .pawchive_client
            .set_creator_favorite(&service, &creator_id, favorite)
            .await;
    }

    if favorite {
        let profile = match state
            .pawchive_client
            .fetch_creator_profile(&service, &creator_id)
            .await
        {
            Ok(profile) => profile,
            Err(_) => state
                .content
                .get_creator(&service, &creator_id)?
                .ok_or_else(|| "Creator is not cached".to_string())?,
        };
        state.content.save_creator(&profile)?;
    }
    state.content.set_pin(
        "creator",
        &service,
        &creator_id,
        None,
        "favorite",
        &account,
        favorite,
    )?;
    if !favorite && !account.is_empty() {
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
        .pawchive_client
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
    state.pawchive_client.search_hash(&file_hash).await
}

#[tauri::command]
pub async fn flag_post(
    service: String,
    creator_id: String,
    post_id: String,
    state: State<'_, AppState>,
) -> Result<ApiActionResult, String> {
    state
        .pawchive_client
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
        .pawchive_client
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
        .pawchive_client
        .fetch_post_revisions(&service, &creator_id, &post_id)
        .await
    {
        Ok(items) => {
            state.content.save_document(
                "post_revisions",
                &service,
                &creator_id,
                &post_id,
                &items,
            )?;
            Ok(items)
        }
        Err(error) => state
            .content
            .load_document("post_revisions", &service, &creator_id, &post_id)?
            .ok_or(error),
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
        .pawchive_client
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
    state.pawchive_client.app_version().await
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
    app_handle: tauri::AppHandle,
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
    state.download_manager.enqueue(
        post.service,
        post.user,
        post.id,
        media_id,
        url,
        filename,
        settings,
        app_handle,
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
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<DownloadJob, String> {
    state
        .download_manager
        .resume(download_id, state.config_manager.load()?, app_handle)
}

#[tauri::command]
pub fn retry_download(
    download_id: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<DownloadJob, String> {
    state
        .download_manager
        .retry(download_id, state.config_manager.load()?, app_handle)
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
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Only HTTP and HTTPS links can be opened".to_string());
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
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = safe_url;
        Err("Unsupported operating system".to_string())
    }
}

#[tauri::command]
pub fn open_downloads_folder(state: State<'_, AppState>) -> Result<(), String> {
    let settings = state.config_manager.load()?;
    let folder = std::path::PathBuf::from(settings.download_dir);
    std::fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    let folder = std::fs::canonicalize(folder).map_err(|error| error.to_string())?;

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer.exe")
            .arg(&folder)
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&folder)
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&folder)
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = folder;
        Err("Opening the downloads folder is unsupported on this operating system".to_string())
    }
}
