pub mod announcements;
pub mod updater;
pub mod window_effects;

use crate::api::models::*;
use crate::api::pawchive::PawchiveClient;
use crate::api::provider::{
    FavoritesSyncResult, ProviderAuthSchema, ProviderCapabilities, ProviderConfig, ProviderHealth,
};
use crate::api::provider_manager::ProviderManager;
use crate::config::settings::{AppSettings, ConfigManager};
use crate::db::content::{CacheStats, ContentRepository, CreatorsPageResult, CreatorsQuery};
use crate::db::downloads::DownloadJob;
use crate::db::library::{
    LibraryCollection, LibraryPostIdentity, LibraryRepository, LibrarySaveResult,
};
use crate::db::subscriptions::{Subscription, SubscriptionInput};
use crate::downloader::aria2c::Aria2cManager;
use crate::downloader::manager::DownloadManager;
use crate::downloader::native::NativeDownloader;
use crate::downloader::{DownloadRunError, DownloadTask};
use crate::media_info::MediaInfo;
use crate::smart_links::{parse_external_creator_link, parse_external_post_link};
use crate::subscriptions::SubscriptionManager;
use crate::sync::client::SyncDevice;
use crate::sync::manager::{SyncManager, SyncStatus};
use std::collections::HashMap;

use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{AppHandle, Emitter, Manager, State};
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
    pub axum_token: String,
    pub provider_manager: Arc<ProviderManager>,
    pub pawchive_client: Arc<PawchiveClient>,
    pub content: Arc<ContentRepository>,
    pub library: Arc<LibraryRepository>,
    pub download_manager: Arc<DownloadManager>,
    pub subscription_manager: Arc<SubscriptionManager>,
    pub sync_manager: Arc<SyncManager>,
    pub config_manager: Arc<ConfigManager>,
    pub archives: Arc<crate::archive::ArchiveRegistry>,
    pub content_index: Arc<crate::db::content_index::ContentIndexRepository>,
}

#[tauri::command]
pub async fn list_post_inner_formats(
    state: State<'_, AppState>,
) -> Result<Vec<crate::db::content_index::IndexedFormats>, String> {
    let repository = state.content_index.clone();
    tokio::task::spawn_blocking(move || repository.list())
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MediaServerInfo {
    pub port: u16,
    pub token: String,
}

#[tauri::command]
pub fn store_custom_background(
    app: tauri::AppHandle,
    source_path: String,
    kind: String,
) -> Result<String, String> {
    let allowed_extensions: &[&str] = match kind.as_str() {
        "image" => &["png", "jpg", "jpeg", "webp", "gif", "avif"],
        "video" => &["mp4", "webm", "m4v", "mov"],
        _ => return Err("Unsupported custom background type".to_string()),
    };
    let source = std::path::Path::new(&source_path);
    if !source.is_file() {
        return Err("Selected background file does not exist".to_string());
    }
    let _extension = source
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|value| allowed_extensions.contains(&value.as_str()))
        .ok_or_else(|| "Unsupported custom background file format".to_string())?;

    if let Ok(dir) = app.path().app_data_dir().map(|d| d.join("background")) {
        if dir.is_dir() {
            let _ = remove_custom_background_files(&dir, &kind);
        }
    }

    let clean_path = dunce::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
    Ok(clean_path.to_string_lossy().to_string())
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
    if let Ok(dir) = app.path().app_data_dir().map(|d| d.join("background")) {
        if dir.is_dir() {
            let _ = remove_custom_background_files(&dir, &kind);
        }
    }
    Ok(())
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
pub fn get_axum_port(state: State<'_, AppState>) -> MediaServerInfo {
    MediaServerInfo {
        port: state.axum_port.load(Ordering::Acquire),
        token: state.axum_token.clone(),
    }
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
    let session_cookie = settings.resolve_cookie_for_url(&url);
    let proxy = crate::net::proxy_for_url(&settings, &url);
    let task = DownloadTask {
        id: "size-probe".to_string(),
        url,
        output_dir: String::new(),
        temp_path: String::new(),
        final_path: String::new(),
        filename: String::new(),
        session_cookie,
        proxy_mode: proxy.mode,
        proxy_url: proxy.url,
        proxy_username: proxy.username,
        proxy_password: proxy.password,
        proxy_bypass_local: proxy.bypass_local,
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
pub async fn probe_download_sizes(
    urls: Vec<String>,
    state: State<'_, AppState>,
) -> Result<std::collections::HashMap<String, u64>, String> {
    if urls.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let settings = state.config_manager.load()?;
    let first_url = urls.first().cloned().unwrap_or_default();
    let proxy = crate::net::proxy_for_url(&settings, &first_url);
    let task_template = DownloadTask {
        id: "size-probe-batch".to_string(),
        session_cookie: settings.resolve_cookie_for_url(&first_url),
        url: first_url,
        output_dir: String::new(),
        temp_path: String::new(),
        final_path: String::new(),
        filename: String::new(),
        proxy_mode: proxy.mode,
        proxy_url: proxy.url,
        proxy_username: proxy.username,
        proxy_password: proxy.password,
        proxy_bypass_local: proxy.bypass_local,
        connections: settings.aria2_connections.clamp(1, 32),
    };
    Ok(NativeDownloader::probe_total_sizes_batch(&urls, &task_template).await)
}

fn media_info_cache() -> &'static Mutex<HashMap<String, MediaInfo>> {
    static CACHE: OnceLock<Mutex<HashMap<String, MediaInfo>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn media_probe_task(url: &str, settings: &AppSettings) -> DownloadTask {
    let proxy = crate::net::proxy_for_url(settings, url);
    DownloadTask {
        id: "media-info-probe".to_string(),
        url: url.to_string(),
        output_dir: String::new(),
        temp_path: String::new(),
        final_path: String::new(),
        filename: String::new(),
        session_cookie: settings.resolve_cookie_for_url(url),
        proxy_mode: proxy.mode,
        proxy_url: proxy.url,
        proxy_username: proxy.username,
        proxy_password: proxy.password,
        proxy_bypass_local: proxy.bypass_local,
        connections: 1,
    }
}

#[tauri::command]
pub async fn probe_media_info(
    url: String,
    state: State<'_, AppState>,
) -> Result<Option<MediaInfo>, String> {
    let parsed = reqwest::Url::parse(&url).map_err(|error| error.to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Ok(None);
    }
    if let Ok(cache) = media_info_cache().lock() {
        if let Some(info) = cache.get(&url) {
            return Ok(Some(info.clone()));
        }
    }

    let settings = state.config_manager.load()?;
    let task = media_probe_task(&url, &settings);
    let head = NativeDownloader::probe_bytes(&task, "bytes=0-131071", 128 * 1024)
        .await
        .map_err(|error| match error {
            DownloadRunError::Failed(message) => message,
            DownloadRunError::Interrupted(_) => "Media info probe was interrupted".to_string(),
        })?;
    let mut info = crate::media_info::from_bytes(&head);
    if info.is_none() && head.get(4..8) == Some(b"ftyp") {
        if let Ok(tail) = NativeDownloader::probe_bytes(&task, "bytes=-524288", 512 * 1024).await {
            info = crate::media_info::from_mp4_tail(&tail);
        }
    }

    if let Some(ref info) = info {
        if let Ok(mut cache) = media_info_cache().lock() {
            if cache.len() >= 1024 {
                cache.clear();
            }
            cache.insert(url, info.clone());
        }
    }
    Ok(info)
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let mut settings = state.config_manager.load()?;
    settings.session_cookie.clear();
    settings.proxy_password.clear();
    for provider in &mut settings.providers {
        provider.session_cookie.clear();
    }
    Ok(settings)
}

#[tauri::command]
pub fn get_default_settings() -> AppSettings {
    AppSettings::default()
}

/// Synchronous commands run on the main thread; cache scans there freeze the window.
async fn with_content<T: Send + 'static>(
    state: &State<'_, AppState>,
    work: impl FnOnce(&ContentRepository) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let content = state.content.clone();
    tokio::task::spawn_blocking(move || work(&content))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn set_app_hidden(hidden: bool) {
    crate::db::compaction::set_app_hidden(hidden);
}

#[tauri::command]
pub async fn get_cache_stats(state: State<'_, AppState>) -> Result<CacheStats, String> {
    with_content(&state, |content| content.cache_stats()).await
}

#[tauri::command]
pub async fn clear_content_cache(state: State<'_, AppState>) -> Result<CacheStats, String> {
    with_content(&state, |content| content.clear_cached_images()).await
}

#[tauri::command]
pub async fn clear_all_content_cache(state: State<'_, AppState>) -> Result<CacheStats, String> {
    with_content(&state, |content| content.clear_all_cache()).await
}

#[tauri::command]
pub async fn wipe_all_data(state: State<'_, AppState>) -> Result<CacheStats, String> {
    state.download_manager.cancel_all();
    state.archives.clear();
    with_content(&state, |content| content.wipe_all_data()).await
}

#[tauri::command]
pub async fn save_settings(
    mut settings: AppSettings,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let previous = state.config_manager.load()?;
    // Backend-owned: the frontend's copy is stale after any theme change.
    settings.window_background_color = previous.window_background_color.clone();
    settings.ui_scale = previous.ui_scale;
    if settings.session_cookie.is_empty() {
        settings.session_cookie = previous.session_cookie.clone();
    }
    // The frontend never receives the stored password, so blank means "unchanged" while the username is the same.
    if settings.proxy_password.is_empty()
        && !settings.proxy_username.is_empty()
        && settings.proxy_username == previous.proxy_username
    {
        settings.proxy_password = previous.proxy_password.clone();
    }
    // Owned by `save_providers` and login; the frontend's copy may predate the last edit.
    settings.providers = previous.providers.clone();
    settings.normalize();
    ProviderManager::validate_configs(&settings.providers)?;
    crate::net::validate_proxy_url(&settings.cloud_proxy_url)?;
    if settings.proxy_mode == crate::config::settings::ProxyMode::Custom {
        crate::net::validate_proxy_url(&settings.proxy_url)?;
    }
    // Publish first: the clients rebuilt below read from it.
    crate::net::apply_settings(&settings);
    if let Err(error) = state
        .pawchive_client
        .update_settings(settings.clone())
        .await
    {
        crate::net::apply_settings(&previous);
        return Err(error);
    }
    if let Err(error) = state.config_manager.save(&settings) {
        crate::net::apply_settings(&previous);
        state.pawchive_client.update_settings(previous).await?;
        return Err(error);
    }
    if let Err(error) = state
        .provider_manager
        .update_providers(settings.providers.clone())
        .await
    {
        crate::net::apply_settings(&previous);
        state.config_manager.save(&previous)?;
        state
            .pawchive_client
            .update_settings(previous.clone())
            .await?;
        state
            .provider_manager
            .update_providers(previous.providers)
            .await?;
        return Err(error);
    }
    state.content.set_cache_limit_mb(settings.cache_max_mb)?;
    state.download_manager.notify_scheduler();
    let _ = state.sync_manager.set_enabled(settings.sync_enabled);
    let _ = app_handle.emit(
        "restart-pending",
        crate::window_setup::pending_restart(&settings),
    );
    Ok(())
}

#[tauri::command]
pub fn restart_app(app_handle: AppHandle) -> Result<(), String> {
    if !cfg!(desktop) {
        return Err("Restarting is not supported on this platform".into());
    }
    app_handle.request_restart();
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct NetworkDefaultsInfo {
    pub api_user_agent: String,
    pub browser_user_agent: String,
    pub request_timeout_secs: u64,
    pub connect_timeout_secs: u64,
    pub provider_deadline_secs: u64,
    pub cloud_timeout_secs: u64,
    pub cloud_max_redirects: u64,
}

#[tauri::command]
pub fn get_network_defaults() -> NetworkDefaultsInfo {
    NetworkDefaultsInfo {
        api_user_agent: crate::net::DEFAULT_API_USER_AGENT.to_string(),
        browser_user_agent: crate::net::DEFAULT_BROWSER_USER_AGENT.to_string(),
        request_timeout_secs: crate::net::DEFAULT_REQUEST_TIMEOUT_SECS,
        connect_timeout_secs: crate::net::DEFAULT_CONNECT_TIMEOUT_SECS,
        provider_deadline_secs: crate::net::DEFAULT_PROVIDER_DEADLINE_SECS,
        cloud_timeout_secs: crate::cloud::DEFAULT_CLOUD_TIMEOUT_SECS,
        cloud_max_redirects: crate::cloud::DEFAULT_CLOUD_MAX_REDIRECTS,
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WebviewProxyStatus {
    pub supported: bool,
    pub active: bool,
    pub needed: bool,
}

#[tauri::command]
pub fn get_webview_proxy_status(state: State<'_, AppState>) -> Result<WebviewProxyStatus, String> {
    let settings = state.config_manager.load()?;
    Ok(WebviewProxyStatus {
        supported: crate::webview_proxy::SUPPORTED,
        active: crate::webview_proxy::is_active(),
        needed: crate::webview_proxy::needed(&settings),
    })
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderNetworkDefaults {
    pub max_retries: u64,
    pub min_interval_ms: u64,
}

#[tauri::command]
pub async fn get_provider_network_defaults(
    state: State<'_, AppState>,
) -> Result<HashMap<String, ProviderNetworkDefaults>, String> {
    Ok(state
        .provider_manager
        .get_provider_configs()
        .await
        .iter()
        .map(|config| {
            let queue = crate::api::providers::manager::default_queue_config_for(config);
            (
                config.id.clone(),
                ProviderNetworkDefaults {
                    max_retries: queue.max_retries as u64,
                    min_interval_ms: queue.min_interval.as_millis() as u64,
                },
            )
        })
        .collect())
}

#[tauri::command]
pub async fn list_providers(state: State<'_, AppState>) -> Result<Vec<ProviderConfig>, String> {
    let mut providers = state.provider_manager.get_provider_configs().await;
    for provider in &mut providers {
        provider.session_cookie.clear();
    }
    Ok(providers)
}

#[tauri::command]
pub async fn save_providers(
    providers: Vec<ProviderConfig>,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let previous = state.config_manager.load()?;
    let mut settings = previous.clone();
    settings.providers = providers;
    for provider in &mut settings.providers {
        if provider.session_cookie.is_empty() {
            if let Some(previous_provider) = previous.providers.iter().find(|p| p.id == provider.id)
            {
                provider.session_cookie = previous_provider.session_cookie.clone();
            }
        }
    }
    settings.normalize();
    ProviderManager::validate_configs(&settings.providers)?;
    crate::net::validate_proxy_url(&settings.cloud_proxy_url)?;
    if settings.proxy_mode == crate::config::settings::ProxyMode::Custom {
        crate::net::validate_proxy_url(&settings.proxy_url)?;
    }
    crate::net::apply_settings(&settings);
    if let Err(error) = state.config_manager.save(&settings) {
        crate::net::apply_settings(&previous);
        return Err(error);
    }
    if let Err(error) = state
        .provider_manager
        .update_providers(settings.providers.clone())
        .await
    {
        crate::net::apply_settings(&previous);
        state.config_manager.save(&previous)?;
        state
            .provider_manager
            .update_providers(previous.providers)
            .await?;
        return Err(error);
    }
    let _ = app_handle.emit(
        "restart-pending",
        crate::window_setup::pending_restart(&settings),
    );
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
    let any_auth = settings
        .providers
        .iter()
        .find(|p| p.enabled && !p.session_cookie.trim().is_empty());

    if let Some(p) = any_auth {
        Ok(AccountSession {
            authenticated: true,
            username: if p.username.trim().is_empty() {
                None
            } else {
                Some(p.username.clone())
            },
        })
    } else {
        Ok(AccountSession {
            authenticated: false,
            username: None,
        })
    }
}

#[tauri::command]
pub async fn get_provider_auth_schema(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<ProviderAuthSchema, String> {
    state
        .provider_manager
        .get_provider_auth_schema(&provider_id)
        .await
}

#[tauri::command]
pub async fn get_provider_capabilities(
    state: State<'_, AppState>,
) -> Result<HashMap<String, ProviderCapabilities>, String> {
    Ok(state.provider_manager.get_provider_capabilities().await)
}

#[tauri::command]
pub async fn get_active_capabilities(
    state: State<'_, AppState>,
) -> Result<ProviderCapabilities, String> {
    Ok(state.provider_manager.get_active_capabilities().await)
}

#[tauri::command]
pub async fn save_provider_session(
    provider_id: String,
    cookie: String,
    username: Option<String>,
    state: State<'_, AppState>,
) -> Result<AccountSession, String> {
    let previous_providers = state.provider_manager.get_provider_configs().await;
    state
        .provider_manager
        .save_provider_session(&provider_id, &cookie, username.as_deref())
        .await?;

    let mut settings = state.config_manager.load()?;
    if let Some(p) = settings.providers.iter_mut().find(|p| p.id == provider_id) {
        p.session_cookie = cookie.clone();
        if let Some(ref u) = username {
            p.username = u.trim().to_string();
        }
    }
    if let Err(error) = state.config_manager.save(&settings) {
        state
            .provider_manager
            .update_providers(previous_providers)
            .await?;
        return Err(error);
    }

    Ok(AccountSession {
        authenticated: true,
        username: state
            .provider_manager
            .get_provider_by_id(&provider_id)
            .await
            .map(|p| p.config().username),
    })
}

#[tauri::command]
pub async fn login_provider(
    provider_id: String,
    credentials: HashMap<String, String>,
    state: State<'_, AppState>,
) -> Result<AccountSession, String> {
    let previous_providers = state.provider_manager.get_provider_configs().await;
    let username = state
        .provider_manager
        .login_provider_with_credentials(&provider_id, &credentials)
        .await?;

    let mut settings = state.config_manager.load()?;
    if let Some(p) = settings.providers.iter_mut().find(|p| p.id == provider_id) {
        if let Some(prov) = state
            .provider_manager
            .get_provider_by_id(&provider_id)
            .await
        {
            p.session_cookie = prov.config().session_cookie;
            p.username = prov.config().username;
        }
    }
    if let Err(error) = state.config_manager.save(&settings) {
        state
            .provider_manager
            .update_providers(previous_providers)
            .await?;
        return Err(error);
    }

    Ok(AccountSession {
        authenticated: true,
        username: if username.is_empty() {
            None
        } else {
            Some(username)
        },
    })
}

#[tauri::command]
pub async fn logout_provider_session(
    provider_id: String,
    remove_session_favorites: bool,
    state: State<'_, AppState>,
) -> Result<AccountSession, String> {
    state
        .provider_manager
        .logout_provider_session(&provider_id, remove_session_favorites, &state.content)
        .await?;

    let mut settings = state.config_manager.load()?;
    if let Some(p) = settings.providers.iter_mut().find(|p| p.id == provider_id) {
        p.session_cookie.clear();
        p.username.clear();
    }
    state.config_manager.clear_provider_session(&provider_id)?;
    state.config_manager.save(&settings)?;

    Ok(AccountSession {
        authenticated: false,
        username: None,
    })
}

#[tauri::command]
pub async fn sync_provider_favorites(
    provider_id: String,
    direction: String,
    state: State<'_, AppState>,
) -> Result<FavoritesSyncResult, String> {
    state
        .provider_manager
        .sync_provider_favorites(&provider_id, &direction, &state.content)
        .await
}

fn persist_post_list(
    content: std::sync::Arc<crate::db::content::ContentRepository>,
    list_key: String,
    offset: u32,
    posts: Vec<Post>,
) {
    tokio::task::spawn_blocking(move || {
        if let Err(error) = content.save_post_list(&list_key, offset, &posts) {
            tracing::warn!("Failed to cache post list '{list_key}' at offset {offset}: {error}");
        }
    });
}

fn emit_partial_posts(
    app_handle: &AppHandle,
    stream_id: &Option<String>,
    provider_id: &str,
    posts: &[Post],
) {
    let Some(stream_id) = stream_id else { return };
    if posts.is_empty() {
        return;
    }
    let _ = app_handle.emit(
        "posts-partial",
        serde_json::json!({
            "stream_id": stream_id,
            "provider_id": provider_id,
            "posts": posts,
        }),
    );
}

async fn emit_cached_posts(
    app_handle: &AppHandle,
    stream_id: &Option<String>,
    content: Arc<ContentRepository>,
    list_key: String,
    offset: u32,
) {
    if stream_id.is_none() {
        return;
    }
    let cached = tokio::task::spawn_blocking(move || content.load_post_list(&list_key, offset))
        .await
        .ok()
        .and_then(|result| result.ok())
        .unwrap_or_default();
    emit_partial_posts(app_handle, stream_id, "cache", &cached);
}

async fn enrich_posts(
    posts: &mut [Post],
    manager: &crate::api::providers::manager::ProviderManager,
) {
    futures_util::future::join_all(posts.iter_mut().map(|post| manager.enrich_post(post))).await;
}

#[tauri::command]
pub async fn list_creators_page(
    query: CreatorsQuery,
    state: State<'_, AppState>,
) -> Result<CreatorsPageResult, String> {
    let content = state.content.clone();
    let mut page = tokio::task::spawn_blocking(move || content.query_creators(&query))
        .await
        .map_err(|e| e.to_string())??;
    futures_util::future::join_all(
        page.items
            .iter_mut()
            .map(|item| state.provider_manager.enrich_creator(item)),
    )
    .await;
    Ok(page)
}

#[tauri::command]
pub async fn list_creator_services(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let content = state.content.clone();
    tokio::task::spawn_blocking(move || content.list_creator_services())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_creator_names(
    state: State<'_, AppState>,
) -> Result<HashMap<String, String>, String> {
    let content = state.content.clone();
    tokio::task::spawn_blocking(move || content.list_creator_names())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_creator_name(
    service: String,
    creator_id: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let content = state.content.clone();
    tokio::task::spawn_blocking(move || content.get_creator_name(&service, &creator_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn sync_creators(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<u64, String> {
    let creators = state.provider_manager.fetch_creators().await?;
    let count = creators.len() as u64;
    let content = state.content.clone();
    tokio::task::spawn_blocking(move || {
        let _ = content.save_creators(&creators);
    })
    .await
    .map_err(|e| e.to_string())?;

    let _ = app_handle.emit("creators-updated", count);
    Ok(count)
}

#[tauri::command]
pub async fn fetch_creators(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<Creator>, String> {
    let content_for_read = state.content.clone();
    let cached = tokio::task::spawn_blocking(move || content_for_read.list_creators())
        .await
        .map_err(|e| e.to_string())??;
    if !cached.is_empty() {
        let provider_manager = state.provider_manager.clone();
        let content = state.content.clone();
        let handle = app_handle.clone();
        tokio::spawn(async move {
            if let Ok(creators) = provider_manager.fetch_creators().await {
                let count = creators.len() as u64;
                let _ = tokio::task::spawn_blocking(move || {
                    let _ = content.save_creators(&creators);
                })
                .await;
                let _ = handle.emit("creators-updated", count);
            }
        });
        return Ok(cached);
    }

    let creators = state.provider_manager.fetch_creators().await?;
    let content = state.content.clone();
    let to_save = creators.clone();
    let _ = tokio::task::spawn_blocking(move || {
        let _ = content.save_creators(&to_save);
    })
    .await;
    Ok(creators)
}

#[tauri::command]
pub async fn fetch_recent_posts(
    query: Option<String>,
    offset: u32,
    stream_id: Option<String>,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<Vec<Post>, String> {
    let list_key = format!("recent:{}", query.as_deref().unwrap_or(""));
    emit_cached_posts(
        &app_handle,
        &stream_id,
        state.content.clone(),
        list_key.clone(),
        offset,
    )
    .await;
    match state
        .provider_manager
        .fetch_recent_posts_streaming(query.as_deref(), offset, |provider_id, posts| {
            emit_partial_posts(&app_handle, &stream_id, provider_id, posts);
        })
        .await
    {
        Ok(mut posts) => {
            enrich_posts(&mut posts, &state.provider_manager).await;
            persist_post_list(state.content.clone(), list_key, offset, posts.clone());
            Ok(posts)
        }
        Err(error) => {
            let content = state.content.clone();
            let lk = list_key.clone();
            let q_empty = query.as_deref().unwrap_or("").is_empty();
            let cached = tokio::task::spawn_blocking(move || -> Result<Vec<Post>, String> {
                let mut posts = content.load_post_list(&lk, offset)?;
                if posts.is_empty() && q_empty {
                    posts = content.list_recent_posts(offset, 50)?;
                }
                Ok(posts)
            })
            .await
            .map_err(|e| e.to_string())??;

            if cached.is_empty() {
                Err(error)
            } else {
                let mut cached = cached;
                enrich_posts(&mut cached, &state.provider_manager).await;
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
    stream_id: Option<String>,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<Vec<Post>, String> {
    let list_key = format!("popular:{period}:{}", date.as_deref().unwrap_or(""));
    emit_cached_posts(
        &app_handle,
        &stream_id,
        state.content.clone(),
        list_key.clone(),
        offset,
    )
    .await;
    match state
        .provider_manager
        .fetch_popular_posts_streaming(&period, date.as_deref(), offset, |provider_id, posts| {
            emit_partial_posts(&app_handle, &stream_id, provider_id, posts);
        })
        .await
    {
        Ok(mut posts) => {
            enrich_posts(&mut posts, &state.provider_manager).await;
            persist_post_list(state.content.clone(), list_key, offset, posts.clone());
            Ok(posts)
        }
        Err(error) => {
            let content = state.content.clone();
            let lk = list_key.clone();
            let cached = tokio::task::spawn_blocking(move || -> Result<Vec<Post>, String> {
                content.load_post_list(&lk, offset)
            })
            .await
            .map_err(|e| e.to_string())??;

            if cached.is_empty() {
                Err(error)
            } else {
                let mut cached = cached;
                enrich_posts(&mut cached, &state.provider_manager).await;
                Ok(cached)
            }
        }
    }
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn fetch_creator_posts(
    service: String,
    creator_id: String,
    query: Option<String>,
    offset: u32,
    provider_id: Option<String>,
    force_refresh: Option<bool>,
    stream_id: Option<String>,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<Vec<Post>, String> {
    let prov_key = provider_id.as_deref().unwrap_or("auto");
    let list_key = format!(
        "creator:{service}:{creator_id}:{prov_key}:{}",
        query.as_deref().unwrap_or("")
    );
    let force = force_refresh.unwrap_or(false);

    if !force {
        let content_for_cache = state.content.clone();
        let list_key_for_cache = list_key.clone();
        let cached_opt = tokio::task::spawn_blocking(move || {
            let age_secs = content_for_cache
                .get_post_list_age_secs(&list_key_for_cache, offset)
                .ok()
                .flatten();
            if let Some(age) = age_secs {
                if age < 900 {
                    return content_for_cache
                        .load_post_list(&list_key_for_cache, offset)
                        .ok();
                }
            }
            None
        })
        .await
        .ok()
        .flatten();

        if let Some(mut cached) = cached_opt {
            if !cached.is_empty() {
                enrich_posts(&mut cached, &state.provider_manager).await;
                return Ok(cached);
            }
        }
    }

    emit_cached_posts(
        &app_handle,
        &stream_id,
        state.content.clone(),
        list_key.clone(),
        offset,
    )
    .await;

    let result = state
        .provider_manager
        .fetch_posts_streaming(
            &service,
            &creator_id,
            offset,
            query.as_deref(),
            provider_id.as_deref(),
            |provider_id, posts| {
                emit_partial_posts(&app_handle, &stream_id, provider_id, posts);
            },
        )
        .await;
    match result {
        Ok(mut posts) => {
            enrich_posts(&mut posts, &state.provider_manager).await;
            persist_post_list(state.content.clone(), list_key, offset, posts.clone());
            Ok(posts)
        }
        Err(error) => {
            let content = state.content.clone();
            let lk = list_key.clone();
            let s = service.clone();
            let c = creator_id.clone();
            let q_empty = query.as_deref().unwrap_or("").is_empty();
            let is_auto = prov_key == "auto";

            let cached = tokio::task::spawn_blocking(move || -> Result<Vec<Post>, String> {
                let mut posts = content.load_post_list(&lk, offset)?;
                if posts.is_empty() && q_empty && is_auto {
                    posts = content.list_creator_posts(&s, &c, offset, 50)?;
                }
                Ok(posts)
            })
            .await
            .map_err(|e| e.to_string())??;

            if cached.is_empty() {
                Err(error)
            } else {
                let mut cached = cached;
                enrich_posts(&mut cached, &state.provider_manager).await;
                Ok(cached)
            }
        }
    }
}

#[tauri::command]
pub async fn fetch_creator_profile(
    service: String,
    creator_id: String,
    provider_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<CreatorProfile, String> {
    match state
        .provider_manager
        .fetch_creator_profile(&service, &creator_id, provider_id.as_deref())
        .await
    {
        Ok(creator) => {
            let mut profile = CreatorProfile {
                id: creator.id.clone(),
                name: creator.name.clone(),
                service: creator.service.clone(),
                public_id: creator.public_id,
                relation_id: creator.relation_id,
                indexed: creator.indexed.map(serde_json::Value::from),
                updated: creator.updated.map(serde_json::Value::from),
                favorited: creator.favorited,
                ever_imported: creator.ever_imported,
                avatar_url: creator.avatar_url,
                avatar_path: creator.avatar_path,
                banner_url: creator.banner_url,
                banner_path: creator.banner_path,
                page_url: creator.page_url,
                extra: creator.extra,
            };
            let explicit_provider = provider_id
                .as_deref()
                .filter(|pid| *pid != crate::db::content::AUTO_PROVIDER_ID);
            if let Some(pid) = explicit_provider {
                profile.extra.insert(
                    "provider_id".to_string(),
                    serde_json::Value::String(pid.to_string()),
                );
            }
            state
                .provider_manager
                .enrich_creator_profile(&mut profile)
                .await;
            if explicit_provider.is_none() {
                state.content.save_creator(&profile)?;
            }
            Ok(profile)
        }
        Err(error) => {
            let prov_key = provider_id.as_deref().unwrap_or("auto");
            if prov_key == "auto" {
                if let Ok(Some(mut cached)) = state.content.get_creator(&service, &creator_id) {
                    state
                        .provider_manager
                        .enrich_creator_profile(&mut cached)
                        .await;
                    return Ok(cached);
                }
            }
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn fetch_announcements(
    service: String,
    creator_id: String,
    provider_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Announcement>, String> {
    let prov_key = provider_id.as_deref().unwrap_or("auto");
    match state
        .provider_manager
        .fetch_announcements(&service, &creator_id, provider_id.as_deref())
        .await
    {
        Ok(items) => {
            state.content.save_document(
                "announcements",
                &service,
                &creator_id,
                prov_key,
                &items,
            )?;
            Ok(items)
        }
        Err(error) => state
            .content
            .load_document("announcements", &service, &creator_id, prov_key)?
            .ok_or(error),
    }
}

#[tauri::command]
pub async fn fetch_fancards(
    service: String,
    creator_id: String,
    provider_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Fancard>, String> {
    let prov_key = provider_id.as_deref().unwrap_or("auto");
    match state
        .provider_manager
        .fetch_fancards(&service, &creator_id, provider_id.as_deref())
        .await
    {
        Ok(items) => {
            state
                .content
                .save_document("fancards", &service, &creator_id, prov_key, &items)?;
            Ok(items)
        }
        Err(error) => {
            let mut cached: Vec<Fancard> = state
                .content
                .load_document("fancards", &service, &creator_id, prov_key)?
                .ok_or(error)?;
            state
                .provider_manager
                .enrich_fancards(&service, &mut cached, provider_id.as_deref())
                .await;
            Ok(cached)
        }
    }
}

#[tauri::command]
pub async fn fetch_creator_links(
    service: String,
    creator_id: String,
    provider_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<CreatorProfile>, String> {
    let prov_key = provider_id.as_deref().unwrap_or("auto");
    match state
        .provider_manager
        .fetch_creator_links(&service, &creator_id, provider_id.as_deref())
        .await
    {
        Ok(mut items) => {
            state.content.save_document(
                "creator_links",
                &service,
                &creator_id,
                prov_key,
                &items,
            )?;
            for item in &mut items {
                state.provider_manager.enrich_creator_profile(item).await;
            }
            Ok(items)
        }
        Err(error) => {
            let mut cached: Vec<CreatorProfile> = state
                .content
                .load_document("creator_links", &service, &creator_id, prov_key)?
                .ok_or(error)?;
            for item in &mut cached {
                state.provider_manager.enrich_creator_profile(item).await;
            }
            Ok(cached)
        }
    }
}

#[tauri::command]
pub async fn fetch_similar_creators(
    service: String,
    creator_id: String,
    provider_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<CreatorProfile>, String> {
    let prov_key = provider_id.as_deref().unwrap_or("auto");
    match state
        .provider_manager
        .fetch_similar_creators(&service, &creator_id, provider_id.as_deref())
        .await
    {
        Ok(mut items) => {
            state.content.save_document(
                "similar_creators",
                &service,
                &creator_id,
                prov_key,
                &items,
            )?;
            for item in &mut items {
                state.provider_manager.enrich_creator_profile(item).await;
            }
            Ok(items)
        }
        Err(error) => {
            let mut cached: Vec<CreatorProfile> = state
                .content
                .load_document("similar_creators", &service, &creator_id, prov_key)?
                .ok_or(error)?;
            for item in &mut cached {
                state.provider_manager.enrich_creator_profile(item).await;
            }
            Ok(cached)
        }
    }
}

#[tauri::command]
pub async fn fetch_creator_tags(
    service: String,
    creator_id: String,
    provider_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let prov_key = provider_id.as_deref().unwrap_or("auto");
    match state
        .provider_manager
        .fetch_creator_tags(&service, &creator_id, provider_id.as_deref())
        .await
    {
        Ok(items) => {
            if !items.is_empty() {
                let _ = state.content.save_document(
                    "creator_tags",
                    &service,
                    &creator_id,
                    prov_key,
                    &items,
                );
            }
            Ok(items)
        }
        Err(error) => state
            .content
            .load_document("creator_tags", &service, &creator_id, prov_key)?
            .ok_or(error),
    }
}

#[tauri::command]
pub async fn fetch_post(
    service: String,
    creator_id: String,
    post_id: String,
    provider_id: Option<String>,
    prefetch: Option<bool>,
    state: State<'_, AppState>,
) -> Result<Post, String> {
    let prov_key = provider_id.as_deref().unwrap_or("auto");
    let fetch =
        state
            .provider_manager
            .fetch_post(&service, &creator_id, &post_id, provider_id.as_deref());
    let result = if prefetch.unwrap_or(false) {
        crate::api::providers::queue::in_background(fetch).await
    } else {
        fetch.await
    };
    match result {
        Ok(Some(mut reconciled)) => {
            enrich_posts(
                std::slice::from_mut(&mut reconciled.post),
                &state.provider_manager,
            )
            .await;
            let content = state.content.clone();
            let to_save = reconciled.post.clone();
            let revisions = reconciled.revisions.clone();
            let s = service.clone();
            let c = creator_id.clone();
            let p = post_id.clone();
            let p_key = prov_key.to_string();
            tokio::task::spawn_blocking(move || {
                let _ = content.save_posts(std::slice::from_ref(&to_save));
                for rev in &revisions {
                    let pid = rev
                        .post
                        .extra
                        .get("provider_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&p_key);
                    let _ = content.save_post_revisions(&s, &c, &p, pid, std::slice::from_ref(rev));
                }
            })
            .await
            .map_err(|e| e.to_string())?;

            if !reconciled.revisions.is_empty() {
                for rev in &mut reconciled.revisions {
                    enrich_posts(std::slice::from_mut(&mut rev.post), &state.provider_manager)
                        .await;
                }
            }
            Ok(reconciled.post)
        }
        Ok(None) => {
            let content = state.content.clone();
            let s = service.clone();
            let c = creator_id.clone();
            let p = post_id.clone();
            let cached = tokio::task::spawn_blocking(move || content.get_post(&s, &c, &p))
                .await
                .map_err(|e| e.to_string())?;

            if let Ok(Some(mut post)) = cached {
                let matches_provider = match provider_id.as_deref() {
                    Some("auto") | None => true,
                    Some(pid) => post
                        .extra
                        .get("provider_id")
                        .and_then(|v| v.as_str())
                        .is_some_and(|p| p.eq_ignore_ascii_case(pid)),
                };
                if matches_provider {
                    enrich_posts(std::slice::from_mut(&mut post), &state.provider_manager).await;
                    return Ok(post);
                }
            }
            Err(format!("Post not found on provider '{prov_key}'"))
        }
        Err(error) => {
            let content = state.content.clone();
            let s = service.clone();
            let c = creator_id.clone();
            let p = post_id.clone();
            let cached = tokio::task::spawn_blocking(move || content.get_post(&s, &c, &p))
                .await
                .map_err(|e| e.to_string())?;

            if let Ok(Some(mut post)) = cached {
                let matches_provider = match provider_id.as_deref() {
                    Some("auto") | None => true,
                    Some(pid) => post
                        .extra
                        .get("provider_id")
                        .and_then(|v| v.as_str())
                        .is_some_and(|p| p.eq_ignore_ascii_case(pid)),
                };
                if matches_provider {
                    enrich_posts(std::slice::from_mut(&mut post), &state.provider_manager).await;
                    return Ok(post);
                }
            }
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn get_cached_post(
    service: String,
    creator_id: String,
    post_id: String,
    provider_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<Post>, String> {
    let content = state.content.clone();
    let lookup = (service.clone(), creator_id.clone(), post_id.clone());
    let mut post_opt =
        tokio::task::spawn_blocking(move || content.get_post(&lookup.0, &lookup.1, &lookup.2))
            .await
            .map_err(|e| e.to_string())??;
    if let Some(ref mut post) = post_opt {
        if let Some(ref pid) = provider_id {
            if pid != "auto" {
                let prov_id = post.extra.get("provider_id").and_then(|v| v.as_str());
                let matches_prov = prov_id.is_some_and(|p| p.eq_ignore_ascii_case(pid));
                if !matches_prov {
                    if let Ok(revs) =
                        state
                            .content
                            .load_post_revisions(&service, &creator_id, &post_id)
                    {
                        if let Some(matching_rev) = revs.into_iter().find(|r| {
                            let rev_prov = r.post.extra.get("provider_id").and_then(|v| v.as_str());
                            rev_prov.is_some_and(|p| p.eq_ignore_ascii_case(pid))
                        }) {
                            *post = matching_rev.post;
                        } else {
                            return Ok(None);
                        }
                    } else {
                        return Ok(None);
                    }
                }
            }
        }
        enrich_posts(std::slice::from_mut(post), &state.provider_manager).await;
    }
    Ok(post_opt)
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
                .fetch_post(&parsed.service, &creator_id, &parsed.post_id, None)
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
            .fetch_creator_profile(&creator_link.service, &creator_link.creator_hint, None)
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

async fn enrich_favorite_artist(state: &State<'_, AppState>, item: &mut Favorite) {
    let srv = item.service.as_deref().unwrap_or("").to_lowercase();
    let id = item.id.to_lowercase();
    if srv.is_empty() || id.is_empty() {
        return;
    }
    let mut creator = Creator {
        id: id.clone(),
        name: item.name.clone().unwrap_or_else(|| id.clone()),
        service: srv.clone(),
        public_id: None,
        relation_id: None,
        updated: None,
        indexed: None,
        favorited: None,
        ever_imported: None,
        avatar_url: None,
        avatar_path: None,
        banner_url: None,
        banner_path: None,
        page_url: None,
        extra: item.extra.clone(),
    };
    match state.content.get_creator(&srv, &id) {
        Ok(Some(cached)) => {
            if !cached.name.is_empty() && cached.name != id {
                item.name = Some(cached.name.clone());
                creator.name = cached.name;
            }
            creator.avatar_url = cached.avatar_url;
            creator.avatar_path = cached.avatar_path;
            creator.banner_url = cached.banner_url;
            creator.banner_path = cached.banner_path;
            creator.page_url = cached.page_url;
            let favorite_hint = crate::db::content::creator_provider_hint(
                creator.extra.get("provider_id"),
                creator.extra.get("provider_ids"),
            );
            if favorite_hint.is_none() {
                for key in ["provider_id", "provider_ids"] {
                    if let Some(value) = cached.extra.get(key) {
                        creator.extra.insert(key.to_string(), value.clone());
                    }
                }
            }
        }
        Ok(None) => {}
        Err(error) => {
            tracing::warn!(%error, service = %srv, creator_id = %id, "Cached creator lookup failed while enriching a favorite");
        }
    }
    state.provider_manager.enrich_creator(&mut creator).await;
    if (item.name.is_none() || item.name.as_deref() == Some(&item.id))
        && creator.name != id
        && !creator.name.is_empty()
    {
        item.name = Some(creator.name);
    }
    if let Some(av) = creator.avatar_url {
        item.extra
            .insert("avatar_url".to_string(), serde_json::Value::String(av));
    }
    if let Some(av_path) = creator.avatar_path {
        item.extra.insert(
            "avatar_path".to_string(),
            serde_json::Value::String(av_path),
        );
    }
    if let Some(bn) = creator.banner_url {
        item.extra
            .insert("banner_url".to_string(), serde_json::Value::String(bn));
    }
    if let Some(bn_path) = creator.banner_path {
        item.extra.insert(
            "banner_path".to_string(),
            serde_json::Value::String(bn_path),
        );
    }
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

    let mut local_favorites = state
        .content
        .list_favorites(kind, &account)
        .unwrap_or_default();

    if settings.persist_in_app_favorites_locally && !account.is_empty() {
        let guest_favorites = state.content.list_favorites(kind, "").unwrap_or_default();
        let mut existing_keys: std::collections::HashSet<(String, String)> = local_favorites
            .iter()
            .map(|f| {
                (
                    f.service.as_deref().unwrap_or("").to_lowercase(),
                    f.id.to_lowercase(),
                )
            })
            .collect();
        for guest_fav in guest_favorites {
            let key = (
                guest_fav.service.as_deref().unwrap_or("").to_lowercase(),
                guest_fav.id.to_lowercase(),
            );
            if existing_keys.insert(key) {
                local_favorites.push(guest_fav);
            }
        }
    }

    if is_authenticated {
        if let Ok(remote_items) = state
            .provider_manager
            .fetch_account_favorites(None, Some(kind))
            .await
        {
            for fav in &remote_items {
                if kind == "post" {
                    let user_id = fav
                        .extra
                        .get("user")
                        .or_else(|| fav.extra.get("user_id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let title = fav
                        .name
                        .as_deref()
                        .or_else(|| fav.extra.get("title").and_then(|v| v.as_str()))
                        .unwrap_or("");
                    let srv = fav.service.as_deref().unwrap_or("");
                    if !srv.is_empty() {
                        let cached_post = if !user_id.is_empty() {
                            state.content.get_post(srv, user_id, &fav.id).ok().flatten()
                        } else {
                            None
                        }
                        .or_else(|| {
                            if let Ok(Some((s, c, p))) = state.content.find_post_identity(
                                srv,
                                &fav.id,
                                if user_id.is_empty() {
                                    None
                                } else {
                                    Some(user_id)
                                },
                            ) {
                                state.content.get_post(&s, &c, &p).ok().flatten()
                            } else {
                                None
                            }
                        });

                        let post = if let Some(mut existing) = cached_post {
                            if !title.is_empty() && existing.title.is_empty() {
                                existing.title = title.to_string();
                            }
                            existing
                        } else if !user_id.is_empty() {
                            let mut p = Post {
                                id: fav.id.clone(),
                                user: user_id.to_string(),
                                service: srv.to_string(),
                                title: title.to_string(),
                                content: None,
                                substring: None,
                                published: fav.indexed.clone(),
                                added: fav.last_imported.clone(),
                                edited: fav.updated.clone(),
                                embed: None,
                                shared_file: None,
                                attachments: None,
                                file: None,
                                poll: None,
                                captions: None,
                                tags: None,
                                origin: None,
                                preview_state: None,
                                has_full: None,
                                detail_fetched: None,
                                next: None,
                                prev: None,
                                favorite_count: None,
                                attachment_count: None,
                                thumbnail_url: None,
                                media_url: None,
                                page_url: None,
                                preview_path: None,
                                cloud_urls: Vec::new(),
                                extra: fav.extra.clone(),
                            };
                            p.clean_extra();
                            p
                        } else {
                            continue;
                        };
                        let _ = state.content.pin_post(&post, "favorite", &account);
                    }
                } else {
                    let srv = fav.service.as_deref().unwrap_or("");
                    let raw_name = fav
                        .name
                        .as_deref()
                        .filter(|n| !n.trim().is_empty() && *n != fav.id);
                    let mut resolved_name = raw_name.map(str::to_string);
                    if resolved_name.is_none() {
                        if let Ok(Some(existing)) = state.content.get_creator(srv, &fav.id) {
                            if !existing.name.is_empty() && existing.name != fav.id {
                                resolved_name = Some(existing.name);
                            }
                        }
                    }
                    if !srv.is_empty() {
                        let name_to_save = resolved_name.clone().unwrap_or_else(|| fav.id.clone());
                        let mut extra = fav.extra.clone();
                        let known_keys = [
                            "id",
                            "name",
                            "service",
                            "public_id",
                            "relation_id",
                            "indexed",
                            "updated",
                            "favorited",
                            "ever_imported",
                            "extra",
                        ];
                        for k in &known_keys {
                            extra.remove(*k);
                        }
                        let existing = state.content.get_creator(srv, &fav.id).ok().flatten();
                        let mut profile = if let Some(mut prev) = existing {
                            if let Some(ref n) = resolved_name {
                                prev.name = n.clone();
                            }
                            prev
                        } else {
                            CreatorProfile {
                                id: fav.id.clone(),
                                name: name_to_save,
                                service: srv.to_string(),
                                public_id: None,
                                relation_id: None,
                                indexed: fav.indexed.clone().map(serde_json::Value::String),
                                updated: fav.updated.clone().map(serde_json::Value::String),
                                favorited: None,
                                ever_imported: None,
                                avatar_url: None,
                                avatar_path: None,
                                banner_url: None,
                                banner_path: None,
                                page_url: None,
                                extra,
                            }
                        };
                        state
                            .provider_manager
                            .enrich_creator_profile(&mut profile)
                            .await;
                        let _ = state.content.save_creator(&profile);
                        let _ = state
                            .content
                            .set_pin("creator", srv, &fav.id, None, "favorite", &account, true);
                    }
                }
            }

            let mut local_map = std::collections::HashMap::new();
            for loc in local_favorites {
                let srv = loc.service.as_deref().unwrap_or("").to_lowercase();
                let id = loc.id.to_lowercase();
                local_map.insert((srv, id), loc);
            }

            let mut merged = Vec::new();
            let mut seen_keys = std::collections::HashSet::new();

            for mut item in remote_items {
                let srv = item.service.as_deref().unwrap_or("").to_lowercase();
                let id = item.id.to_lowercase();
                if seen_keys.insert((srv.clone(), id.clone())) {
                    if let Some(local_entry) = local_map.get(&(srv.clone(), id.clone())) {
                        if let Some(faved_at) = local_entry.extra.get("faved_at") {
                            item.extra.insert("faved_at".to_string(), faved_at.clone());
                        }
                        if (item.name.is_none() || item.name.as_deref() == Some(&item.id))
                            && local_entry.name.is_some()
                            && local_entry.name.as_deref() != Some(&local_entry.id)
                        {
                            item.name = local_entry.name.clone();
                        }
                    }
                    if kind == "artist" {
                        enrich_favorite_artist(&state, &mut item).await;
                    } else if kind == "post" {
                        let user_id = item
                            .extra
                            .get("user")
                            .or_else(|| item.extra.get("user_id"))
                            .and_then(|v| v.as_str());

                        let cached_post = if let Some(uid) = user_id {
                            state.content.get_post(&srv, uid, &id).ok().flatten()
                        } else {
                            None
                        }
                        .or_else(|| {
                            if let Ok(Some((s, c, p))) =
                                state.content.find_post_identity(&srv, &id, user_id)
                            {
                                state.content.get_post(&s, &c, &p).ok().flatten()
                            } else {
                                None
                            }
                        });

                        if let Some(post) = cached_post {
                            if (item.name.is_none() || item.name.as_deref() == Some(&item.id))
                                && !post.title.is_empty()
                            {
                                item.name = Some(post.title.clone());
                            }
                            if let Some(file) = &post.file {
                                if let Ok(v) = serde_json::to_value(file) {
                                    item.extra.insert("file".to_string(), v);
                                }
                            }
                            if let Some(attachments) = &post.attachments {
                                if let Ok(v) = serde_json::to_value(attachments) {
                                    item.extra.insert("attachments".to_string(), v);
                                }
                            }
                            if let Some(thumb) = &post.thumbnail_url {
                                item.extra.insert(
                                    "thumbnail_url".to_string(),
                                    serde_json::Value::String(thumb.clone()),
                                );
                            }
                            if let Some(media) = &post.media_url {
                                item.extra.insert(
                                    "media_url".to_string(),
                                    serde_json::Value::String(media.clone()),
                                );
                            }
                            if let Some(prev) = &post.preview_path {
                                item.extra.insert(
                                    "preview_path".to_string(),
                                    serde_json::Value::String(prev.clone()),
                                );
                            }
                            if let Some(local_prev) = post.extra.get("local_preview_path") {
                                item.extra
                                    .insert("local_preview_path".to_string(), local_prev.clone());
                            }
                            if !post.user.is_empty() && !item.extra.contains_key("user") {
                                item.extra.insert(
                                    "user".to_string(),
                                    serde_json::Value::String(post.user.clone()),
                                );
                            }
                        }
                    }
                    merged.push(item);
                }
            }

            if settings.persist_in_app_favorites_locally || kind == "post" {
                for (key, loc) in &local_map {
                    if seen_keys.insert((key.0.clone(), key.1.clone())) {
                        let mut loc_item = loc.clone();
                        if kind == "artist" {
                            enrich_favorite_artist(&state, &mut loc_item).await;
                        }
                        merged.push(loc_item);
                    }
                }
            } else {
                for key in local_map.keys() {
                    if !seen_keys.contains(key) {
                        let _ = state.content.set_pin(
                            if kind == "post" { "post" } else { "creator" },
                            &key.0,
                            &key.1,
                            if kind == "post" { Some(&key.1) } else { None },
                            "favorite",
                            &account,
                            false,
                        );
                    }
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
        if kind == "artist" {
            enrich_favorite_artist(&state, item).await;
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
        let post = if let Ok(Some(cached_post)) =
            state.content.get_post(&service, &creator_id, &post_id)
        {
            cached_post
        } else {
            match state
                .provider_manager
                .fetch_post(&service, &creator_id, &post_id, None)
                .await
            {
                Ok(Some(reconciled)) => reconciled.post,
                _ => state
                    .content
                    .get_post(&service, &creator_id, &post_id)?
                    .ok_or_else(|| "Post is not cached".to_string())?,
            }
        };
        state.content.pin_post(&post, "favorite", &account)?;
        if settings.persist_in_app_favorites_locally && !account.is_empty() {
            let _ = state.content.pin_post(&post, "favorite", "");
        }
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
        if let Err(error) = state
            .provider_manager
            .set_creator_favorite(&service, &creator_id, favorite)
            .await
        {
            tracing::warn!(%error, %service, %creator_id, "Remote creator favorite failed; keeping the local favorite");
        }
    }

    if favorite {
        let profile = match state
            .provider_manager
            .fetch_creator_profile(&service, &creator_id, None)
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
                favorited: creator.favorited,
                ever_imported: creator.ever_imported,
                avatar_url: creator.avatar_url,
                avatar_path: creator.avatar_path,
                banner_url: creator.banner_url,
                banner_path: creator.banner_path,
                page_url: creator.page_url,
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
                        favorited: None,
                        ever_imported: None,
                        avatar_url: None,
                        avatar_path: None,
                        banner_url: None,
                        banner_path: None,
                        page_url: None,
                        extra: Default::default(),
                    }
                }
            }
        };
        if let Err(error) = state.content.save_creator(&profile) {
            tracing::warn!(%error, %service, %creator_id, "Failed to cache favorited creator profile");
        }
        state.content.set_pin(
            "creator",
            &service,
            &creator_id,
            None,
            "favorite",
            &account,
            true,
        )?;
        if settings.persist_in_app_favorites_locally && !account.is_empty() {
            let _ =
                state
                    .content
                    .set_pin("creator", &service, &creator_id, None, "favorite", "", true);
        }
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
pub async fn get_creator_artwork_path(
    service: String,
    creator_id: String,
    artwork_kind: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let path_opt = state
        .content
        .artwork_path(&service, &creator_id, &artwork_kind)?;
    if let Some(path_str) = path_opt {
        if std::path::Path::new(&path_str).is_file() {
            return Ok(Some(path_str));
        }
    }
    Ok(None)
}

#[tauri::command]
pub async fn revalidate_creator_artwork(
    service: String,
    creator_id: String,
    artwork_kind: String,
    provider_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let prov_key = provider_id.as_deref();
    let url = if artwork_kind == "banner" {
        state
            .provider_manager
            .resolve_banner_url(&service, &creator_id, prov_key)
            .await
    } else {
        state
            .provider_manager
            .resolve_avatar_url(&service, &creator_id, prov_key)
            .await
    };
    if url.is_empty() {
        return Ok(None);
    }
    let settings = state.config_manager.load()?;
    let client = crate::net::shared_client(&settings)?;
    let res = state
        .content
        .revalidate_creator_artwork(
            &service,
            &creator_id,
            &artwork_kind,
            &url,
            prov_key,
            &client,
        )
        .await?;
    if res.is_some() {
        state.provider_manager.invalidate_artwork_index().await;
    }
    Ok(res.map(|p| p.to_string_lossy().into_owned()))
}

#[tauri::command]
pub async fn resolve_creator_url(
    service: String,
    creator_id: String,
    provider_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    Ok(state
        .provider_manager
        .resolve_creator_url(&service, &creator_id, provider_id.as_deref())
        .await)
}

#[tauri::command]
pub async fn resolve_post_url(
    service: String,
    creator_id: String,
    post_id: String,
    provider_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    Ok(state
        .provider_manager
        .resolve_post_url(&service, &creator_id, &post_id, provider_id.as_deref())
        .await)
}

#[tauri::command]
pub async fn resolve_creator_avatar_url(
    service: String,
    creator_id: String,
    provider_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    Ok(state
        .provider_manager
        .resolve_avatar_url(&service, &creator_id, provider_id.as_deref())
        .await)
}

#[tauri::command]
pub async fn resolve_creator_banner_url(
    service: String,
    creator_id: String,
    provider_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    Ok(state
        .provider_manager
        .resolve_banner_url(&service, &creator_id, provider_id.as_deref())
        .await)
}

#[tauri::command]
pub async fn get_video_thumbnail(
    key: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    state.content.thumbnail_data_url(&key)
}

#[tauri::command]
pub async fn get_thumbnail_path(
    key: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let content = state.content.clone();
    tokio::task::spawn_blocking(move || content.thumbnail_path(&key))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn store_video_thumbnail(
    key: String,
    data_url: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let path = state.content.store_thumbnail_data_url(&key, &data_url)?;
    Ok(path.to_string_lossy().to_string())
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
            for item in &items {
                let provider_id = item
                    .post
                    .extra
                    .get("provider_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let _ = state.content.save_post_revisions(
                    &service,
                    &creator_id,
                    &post_id,
                    provider_id,
                    std::slice::from_ref(item),
                );
            }
            Ok(items)
        }
        _ => {
            let mut revisions = state
                .content
                .load_post_revisions(&service, &creator_id, &post_id)
                .unwrap_or_default();
            for revision in &mut revisions {
                state.provider_manager.enrich_post(&mut revision.post).await;
            }
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
pub async fn search_posts(query: String, state: State<'_, AppState>) -> Result<Vec<Post>, String> {
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
    color: Option<String>,
    state: State<'_, AppState>,
) -> Result<LibraryCollection, String> {
    state.library.create_stash(&name, color.as_deref())
}

#[tauri::command]
pub fn update_library_stash(
    collection_id: String,
    name: Option<String>,
    color: Option<Option<String>>,
    state: State<'_, AppState>,
) -> Result<LibraryCollection, String> {
    state.library.update_stash(
        &collection_id,
        name.as_deref(),
        color.as_ref().map(|c| c.as_deref()),
    )
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
    post: Post,
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
pub async fn list_saved_post_identities(
    state: State<'_, AppState>,
) -> Result<Vec<LibraryPostIdentity>, String> {
    let library = state.library.clone();
    tokio::task::spawn_blocking(move || library.list_saved_post_identities())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_post_stash_memberships(
    state: State<'_, AppState>,
) -> Result<Vec<crate::db::library::PostStashMembership>, String> {
    let library = state.library.clone();
    tokio::task::spawn_blocking(move || library.list_post_stash_memberships())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_library_posts(
    collection_id: Option<String>,
    offset: u32,
    limit: u32,
    state: State<'_, AppState>,
) -> Result<Vec<Post>, String> {
    let library = state.library.clone();
    tokio::task::spawn_blocking(move || library.list_posts(collection_id.as_deref(), offset, limit))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn start_download(
    post: Post,
    media_id: String,
    url: String,
    filename: String,
    _app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<DownloadJob, String> {
    let settings = state.config_manager.load()?;
    state.content.pin_post(&post, "download", "")?;

    let bg_content = state.content.clone();
    let bg_provider = state.provider_manager.clone();
    let bg_post = post.clone();
    let bg_settings = settings.clone();
    tokio::spawn(async move {
        if let Ok(creator) = bg_provider
            .fetch_creator_profile(&bg_post.service, &bg_post.user, None)
            .await
        {
            let profile = CreatorProfile {
                id: creator.id,
                name: creator.name,
                service: creator.service,
                public_id: creator.public_id,
                relation_id: creator.relation_id,
                indexed: creator.indexed.map(serde_json::Value::from),
                updated: creator.updated.map(serde_json::Value::from),
                favorited: creator.favorited,
                ever_imported: creator.ever_imported,
                avatar_url: creator.avatar_url,
                avatar_path: creator.avatar_path,
                banner_url: creator.banner_url,
                banner_path: creator.banner_path,
                page_url: creator.page_url,
                extra: creator.extra,
            };
            let _ = bg_content.save_creator(&profile);
        }
        for kind in ["avatar", "banner"] {
            if bg_content
                .artwork_path(&bg_post.service, &bg_post.user, kind)
                .ok()
                .flatten()
                .is_none()
            {
                if let Ok(data) = bg_provider
                    .fetch_creator_artwork_data_url(&bg_post.service, &bg_post.user, kind)
                    .await
                {
                    let _ = bg_content.store_artwork_data_url(
                        &bg_post.service,
                        &bg_post.user,
                        kind,
                        &data,
                    );
                    bg_provider.invalidate_artwork_index().await;
                }
            }
        }
        if let Some(file) = bg_post
            .file
            .as_ref()
            .filter(|file| file.path.is_some())
            .or_else(|| {
                bg_post
                    .attachments
                    .as_ref()
                    .and_then(|items| items.iter().find(|file| file.path.is_some()))
            })
            .and_then(|file| file.path.as_deref())
        {
            let prov_id = bg_post.extra.get("provider_id").and_then(|v| v.as_str());
            let preview_url = bg_provider
                .resolve_thumbnail_url(&bg_post.service, file, prov_id)
                .await;
            if let Ok(client) = crate::net::shared_client(&bg_settings) {
                let _ = bg_content
                    .cache_post_preview(&bg_post, &preview_url, &client)
                    .await;
            }
        }
    });

    let creator_name =
        crate::downloader::template::resolve_creator_name(&state.content, &post, None);

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
        let target_dir = crate::downloader::template::resolve_target_dir(&root, &settings, &ctx);
        std::fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
        let source_url = state
            .provider_manager
            .resolve_post_url(&post.service, &post.user, &post.id, None)
            .await;
        let raw_json = state
            .content
            .get_post_raw_json(&post.service, &post.user, &post.id)
            .ok()
            .flatten();
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
            source_url: Some(source_url),
            raw_json: raw_json.as_deref(),
        };
        crate::downloader::metadata::save_post_metadata(&target_dir, &meta, &settings)?;
    }

    let download_url = crate::server::absolute_local_media_url(
        url,
        state.axum_port.load(Ordering::Acquire),
        &state.axum_token,
    )?;

    state.download_manager.enqueue(
        post.service,
        post.user,
        creator_name,
        post.id,
        Some(post.title),
        post.published,
        media_id,
        download_url,
        filename,
        index,
    )
}

#[tauri::command]
pub async fn list_downloads(state: State<'_, AppState>) -> Result<Vec<DownloadJob>, String> {
    let mut jobs = state.download_manager.list()?;
    for job in &mut jobs {
        let post = match state
            .content
            .get_post(&job.service, &job.creator_id, &job.post_id)
        {
            Ok(Some(p)) => p,
            _ => continue,
        };
        let prov_id = post.extra.get("provider_id").and_then(|v| v.as_str());

        if job.file_preview_url.is_none() {
            let target_media_path = if !job.media_id.is_empty() && job.media_id.starts_with('/') {
                Some(job.media_id.as_str())
            } else if let Some(att) = post.attachments.as_ref().and_then(|items| {
                items.iter().find(|a| {
                    a.path.as_deref() == Some(&job.media_id)
                        || a.name.as_deref() == Some(&job.filename)
                        || a.url.as_deref() == Some(&job.url)
                })
            }) {
                att.path.as_deref()
            } else if let Some(f) = post.file.as_ref() {
                if f.path.as_deref() == Some(&job.media_id)
                    || f.name.as_deref() == Some(&job.filename)
                    || f.url.as_deref() == Some(&job.url)
                {
                    f.path.as_deref()
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(p) = target_media_path {
                job.file_preview_url = Some(
                    state
                        .provider_manager
                        .resolve_thumbnail_url(&job.service, p, prov_id)
                        .await,
                );
            }
        }

        if job.post_preview_url.is_none() {
            let post_cover_path = post
                .file
                .as_ref()
                .filter(|file| file.path.is_some())
                .or_else(|| {
                    post.attachments
                        .as_ref()
                        .and_then(|items| items.iter().find(|file| file.path.is_some()))
                })
                .and_then(|file| file.path.as_deref());
            if let Some(path) = post_cover_path {
                job.post_preview_url = Some(
                    state
                        .provider_manager
                        .resolve_thumbnail_url(&job.service, path, prov_id)
                        .await,
                );
            }
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
pub async fn remove_download(
    download_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    state.download_manager.remove(&download_id).await
}

#[tauri::command]
pub fn pause_all_downloads(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.download_manager.pause_all(&app_handle)
}

#[tauri::command]
pub fn resume_all_downloads(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.download_manager.resume_all(&app_handle)
}

#[tauri::command]
pub fn cancel_all_downloads(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.download_manager.cancel_all_with_events(&app_handle);
    Ok(())
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
        .provider_manager
        .fetch_creator_profile(&input.service, &input.creator_id, None)
        .await
    {
        state.content.save_creators(&[profile])?;
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
    remove_posts: Option<bool>,
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

        if remove_posts.unwrap_or(false) {
            let _ = state.library.remove_creator_posts_from_stash(
                Some(&subscription.destination_collection_id),
                &subscription.service,
                &subscription.creator_id,
            );
        }
    }
    let result = state
        .subscription_manager
        .repository()
        .delete(&subscription_id);
    state.sync_manager.trigger_sync_on_change(app_handle);
    result
}

#[tauri::command]
pub fn clear_subscription_posts(
    subscription_id: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<u64, String> {
    let subscription = state
        .subscription_manager
        .repository()
        .get(&subscription_id)?
        .ok_or_else(|| "Subscription not found".to_string())?;

    let count = state.library.remove_creator_posts_from_stash(
        Some(&subscription.destination_collection_id),
        &subscription.service,
        &subscription.creator_id,
    )?;

    state.sync_manager.trigger_sync_on_change(app_handle);
    Ok(count)
}

#[tauri::command]
pub fn count_subscription_posts(
    subscription_id: String,
    state: State<'_, AppState>,
) -> Result<u64, String> {
    let subscription = state
        .subscription_manager
        .repository()
        .get(&subscription_id)?
        .ok_or_else(|| "Subscription not found".to_string())?;

    state.library.count_creator_posts_in_stash(
        &subscription.destination_collection_id,
        &subscription.service,
        &subscription.creator_id,
    )
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
pub async fn test_sync_connection(
    server_url: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let settings = state.config_manager.load()?;
    let client = crate::sync::client::SyncHttpClient::new(&server_url, &settings)?;
    client.ping().await.map(|_| true)
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
pub async fn open_in_browser(url: String) -> Result<(), String> {
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
        crate::linux_open::open(safe_url).await
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
pub async fn open_downloads_folder(state: State<'_, AppState>) -> Result<(), String> {
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
        crate::linux_open::open(folder).await
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
pub async fn open_download_file(file_path: String) -> Result<(), String> {
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
        crate::linux_open::open(path).await
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
pub async fn show_in_folder(path: String) -> Result<(), String> {
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
        crate::linux_open::open(folder).await
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
static FILE_PICKER_TX: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<Option<String>>>> =
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
#[no_mangle]
pub extern "C" fn Java_app_pawstash_client_MainActivity_onFilePicked(
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

    if let Ok(mut lock) = FILE_PICKER_TX.lock() {
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
        let payload_str = json.to_string_lossy().to_string();
        if let Ok(lock) = APP_HANDLE.read() {
            if let Some(handle) = lock.as_ref() {
                use tauri::Emitter;
                if payload_str.starts_with('{') {
                    let _ = handle.emit("open-post-deep-link", payload_str);
                } else {
                    let _ = handle.emit("deep-link:opened", payload_str);
                }
            }
        }
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_app_pawstash_client_DownloadForegroundService_onNotificationAction(
    mut env: jni::JNIEnv,
    _class: jni::objects::JClass,
    action_jstr: jni::objects::JString,
) {
    if let Ok(action) = env.get_string(&action_jstr) {
        let action_str = action.to_string_lossy().to_string();
        tracing::info!("Received download notification action from Android: {action_str}");
        if let Ok(lock) = APP_HANDLE.read() {
            if let Some(handle) = lock.as_ref() {
                use tauri::Manager;
                if let Some(state) = handle.try_state::<crate::AppState>() {
                    match action_str.as_str() {
                        "pause" => {
                            let _ = state.download_manager.pause_all(handle);
                        }
                        "resume" => {
                            let _ = state.download_manager.resume_all(handle);
                        }
                        "cancel" => {
                            state.download_manager.cancel_all_with_events(handle);
                        }
                        _ => {}
                    }
                }
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

#[cfg(target_os = "android")]
fn launch_file_picker_android(kind: &str) -> Result<(), String> {
    with_android_context(|env, context| {
        let kind_jstr = env
            .new_string(kind)
            .map_err(|e| format!("Failed to create Java string: {e}"))?;
        env.call_method(
            context,
            "launchFilePicker",
            "(Ljava/lang/String;)V",
            &[jni::objects::JValue::Object(&kind_jstr)],
        )
        .map_err(|e| format!("Failed to launch native file picker: {e}"))?;
        Ok(())
    })
}

#[tauri::command]
pub fn open_app_links_settings() -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        with_android_context(|env, context| {
            env.call_method(context, "openAppLinksSettings", "()V", &[])
                .map_err(|e| format!("Failed to open Android app links settings: {e}"))?;
            Ok(())
        })
    }
    #[cfg(not(target_os = "android"))]
    {
        Err("App links settings are only supported on Android".to_string())
    }
}

#[tauri::command]
pub fn get_system_accent_color() -> Result<Option<String>, String> {
    #[cfg(target_os = "android")]
    {
        with_android_context(|env, context| {
            let res = env
                .call_method(context, "getSystemMonetColors", "()Ljava/lang/String;", &[])
                .map_err(|e| format!("Failed to call getSystemMonetColors: {e}"))?;
            let jstr = res.l().map_err(|e| e.to_string())?;
            if jstr.is_null() {
                return Ok(None);
            }
            let s: String = env
                .get_string(&jni::objects::JString::from(jstr))
                .map_err(|e| e.to_string())?
                .into();
            if s.is_empty() {
                Ok(None)
            } else {
                Ok(Some(s))
            }
        })
    }
    #[cfg(not(target_os = "android"))]
    {
        Ok(None)
    }
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
pub async fn pick_file(kind: String) -> Result<Option<String>, String> {
    #[cfg(target_os = "android")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let mut lock = FILE_PICKER_TX
                .lock()
                .map_err(|e| format!("Lock error: {e}"))?;
            *lock = Some(tx);
        }

        launch_file_picker_android(&kind)?;

        match rx.await {
            Ok(path) => Ok(path),
            Err(_) => Ok(None),
        }
    }

    #[cfg(not(target_os = "android"))]
    {
        let _ = kind;
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

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct CloudLinkContext {
    pub service: String,
    pub creator_id: String,
    #[serde(default)]
    pub post_text: Option<String>,
}

fn locked_link_id(url: &str) -> String {
    use sha2::Digest;
    format!("link:{:x}", sha2::Sha256::digest(url.as_bytes()))
}

async fn decrypt_with_candidates(
    url: &str,
    candidates: Vec<String>,
) -> Result<Option<(String, String)>, String> {
    let url = url.to_string();
    tokio::task::spawn_blocking(move || {
        for password in candidates {
            match crate::cloud::mega::decrypt_protected_link(&url, &password) {
                Ok(unlocked) => return Ok(Some((unlocked, password))),
                Err(crate::cloud::mega::ProtectedLinkError::WrongPassword) => continue,
                Err(crate::cloud::mega::ProtectedLinkError::Malformed) => {
                    return Err("This MEGA link is damaged or incomplete".to_string())
                }
                Err(crate::cloud::mega::ProtectedLinkError::Unsupported) => {
                    return Err(
                        "This kind of password-protected MEGA link isn't supported".to_string()
                    )
                }
            }
        }
        Ok(None)
    })
    .await
    .map_err(|e| format!("MEGA password check failed: {e}"))?
}

async fn resolve_and_cache(
    state: &AppState,
    url: &str,
    target: &str,
) -> Result<crate::cloud::CloudFolderResult, String> {
    let settings = state.config_manager.load()?;
    if !settings.cloud_scraping_enabled {
        return Err("Cloud link scraping is disabled in settings".to_string());
    }
    let result = crate::cloud::CloudResolver::new(&settings)?
        .resolve(target)
        .await?;
    if let Err(error) = state
        .content
        .save_document("cloud_folder", url, "", "", &result)
    {
        tracing::warn!(%error, "Failed to cache resolved cloud link");
    }
    Ok(result)
}

#[tauri::command]
pub async fn resolve_cloud_link(
    url: String,
    context: Option<CloudLinkContext>,
    state: State<'_, AppState>,
) -> Result<crate::cloud::CloudFolderResult, String> {
    if let Ok(Some(mut cached)) = state
        .content
        .load_document::<crate::cloud::CloudFolderResult>("cloud_folder", &url, "", "")
    {
        if cached.provider == "dropbox" && cached.nodes.iter().any(|n| n.is_folder) {
            return Err(
                "Dropbox folders are not supported; open the link in a browser".to_string(),
            );
        }
        let cached_empty_drive = cached.provider == "googledrive"
            && cached.nodes.len() == 1
            && cached.nodes[0].is_folder;
        let has_missing_sizes = cached
            .nodes
            .iter()
            .any(|n| !n.is_folder && (n.size.is_none() || n.size == Some(0)));
        let has_files = cached.nodes.iter().any(|n| !n.is_folder);
        if !cached_empty_drive && has_files && !has_missing_sizes {
            crate::cloud::canonicalize_cloud_result(&mut cached);
            return Ok(cached);
        }
    }

    let passwords = state.archives.passwords();
    let link_id = locked_link_id(&url);
    let context = context.unwrap_or_default();
    let target = if crate::cloud::mega::is_protected_link(&url) {
        let mut candidates: Vec<String> = Vec::new();
        candidates.extend(passwords.password_for_archive(&link_id)?);
        candidates.extend(passwords.creator_passwords(&context.service, &context.creator_id)?);
        if let Some(text) = context.post_text.as_deref() {
            candidates.extend(crate::archive::extract_password_candidates(text));
        }
        let mut seen = std::collections::HashSet::new();
        candidates.retain(|candidate| seen.insert(candidate.clone()));
        let Some((unlocked, password)) = decrypt_with_candidates(&url, candidates).await? else {
            return Err("password_required".to_string());
        };
        passwords.remember(
            &link_id,
            &context.service,
            &context.creator_id,
            &password,
            &url,
        )?;
        unlocked
    } else if crate::cloud::mega::is_keyless_link(&url) {
        match passwords.password_for_archive(&link_id)? {
            Some(key) => format!("{url}#{key}"),
            None => return Err("key_required".to_string()),
        }
    } else {
        url.clone()
    };

    match resolve_and_cache(&state, &url, &target).await {
        Ok(result) => Ok(result),
        Err(error) => {
            if let Ok(Some(mut cached)) = state
                .content
                .load_document::<crate::cloud::CloudFolderResult>("cloud_folder", &url, "", "")
            {
                crate::cloud::canonicalize_cloud_result(&mut cached);
                return Ok(cached);
            }
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn unlock_cloud_link(
    url: String,
    secret: String,
    context: Option<CloudLinkContext>,
    state: State<'_, AppState>,
) -> Result<crate::cloud::CloudFolderResult, String> {
    let secret = secret.trim().to_string();
    let context = context.unwrap_or_default();
    let link_id = locked_link_id(&url);
    let passwords = state.archives.passwords();

    if crate::cloud::mega::is_protected_link(&url) {
        let Some((unlocked, _)) = decrypt_with_candidates(&url, vec![secret.clone()]).await? else {
            return Err("wrong_password".to_string());
        };
        let result = resolve_and_cache(&state, &url, &unlocked).await?;
        passwords.remember(
            &link_id,
            &context.service,
            &context.creator_id,
            &secret,
            &url,
        )?;
        return Ok(result);
    }

    if crate::cloud::mega::is_keyless_link(&url) {
        let expected_len = if url.contains("/folder/") { 16 } else { 32 };
        let key = secret.trim_start_matches('#').trim_end_matches('=');
        let decoded =
            base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, key);
        if decoded.map(|bytes| bytes.len()) != Ok(expected_len) {
            return Err(crate::cloud::mega::WRONG_KEY.to_string());
        }
        let result = resolve_and_cache(&state, &url, &format!("{url}#{key}")).await?;
        passwords.remember(&link_id, "", "", key, &url)?;
        return Ok(result);
    }

    Err("This link is not locked".to_string())
}

#[tauri::command]
pub fn password_hints(text: String) -> Vec<String> {
    crate::archive::extract_password_candidates(&text)
}

#[tauri::command]
pub async fn open_archive(
    request: crate::archive::ArchiveOpenRequest,
    state: State<'_, AppState>,
) -> Result<crate::archive::ArchiveOpenResult, String> {
    state.archives.open(request).await
}

#[tauri::command]
pub async fn unlock_archive(
    archive_id: String,
    password: String,
    state: State<'_, AppState>,
) -> Result<crate::archive::ArchiveOpenResult, String> {
    state.archives.unlock(&archive_id, &password).await
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
pub async fn open_logs_folder() -> Result<(), String> {
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
        crate::linux_open::open(folder).await
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

#[tauri::command]
pub async fn resolve_deep_link(
    url: String,
    state: State<'_, AppState>,
) -> Result<crate::smart_links::DeepLinkTarget, String> {
    let configs = state.provider_manager.get_provider_configs().await;
    crate::smart_links::parse_deep_link(&url, &configs)
}

fn system_text_scale() -> f64 {
    #[cfg(target_os = "windows")]
    let scale = windows_registry::CURRENT_USER
        .open(r"Software\Microsoft\Accessibility")
        .and_then(|key| key.get_u32("TextScaleFactor"))
        .map_or(1.0, |percent| f64::from(percent) / 100.0);
    #[cfg(target_os = "android")]
    let scale = with_android_context(|env, activity| {
        let resources = env
            .call_method(
                activity,
                "getResources",
                "()Landroid/content/res/Resources;",
                &[],
            )
            .and_then(|value| value.l())
            .map_err(|e| format!("getResources failed: {e}"))?;
        let configuration = env
            .call_method(
                &resources,
                "getConfiguration",
                "()Landroid/content/res/Configuration;",
                &[],
            )
            .and_then(|value| value.l())
            .map_err(|e| format!("getConfiguration failed: {e}"))?;
        env.get_field(&configuration, "fontScale", "F")
            .and_then(|value| value.f())
            .map(f64::from)
            .map_err(|e| format!("fontScale failed: {e}"))
    })
    .unwrap_or(1.0);
    #[cfg(not(any(target_os = "windows", target_os = "android")))]
    let scale: f64 = 1.0;
    scale.clamp(0.5, 3.0)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UiScale {
    pub percent: u32,
    pub system: f64,
}

pub fn apply_ui_scale(app: &AppHandle, settings: &AppSettings) -> UiScale {
    let system = system_text_scale();
    let scale = match settings.ui_scale {
        0 => system,
        percent => f64::from(percent) / 100.0,
    };
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = window.set_zoom(scale) {
            tracing::warn!(%error, scale, "Could not apply the interface scale");
        }
    }
    UiScale {
        percent: settings.ui_scale,
        system,
    }
}

#[tauri::command]
pub fn refresh_ui_scale(app: AppHandle, state: State<'_, AppState>) -> Result<UiScale, String> {
    let settings = state.config_manager.load()?;
    Ok(apply_ui_scale(&app, &settings))
}

#[tauri::command]
pub fn set_ui_scale(
    percent: u32,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<UiScale, String> {
    let mut settings = state.config_manager.load()?;
    settings.ui_scale = percent;
    settings.normalize();
    state.config_manager.save(&settings)?;
    Ok(apply_ui_scale(&app, &settings))
}

/// Back at the root backgrounds the task instead of finishing the activity, as Android 12+ launchers do.
#[tauri::command]
pub fn android_minimize() -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        with_android_context(|env, activity| {
            env.call_method(activity, "moveTaskToBack", "(Z)Z", &[true.into()])
                .map(|_| ())
                .map_err(|e| format!("moveTaskToBack failed: {e}"))
        })
    }
    #[cfg(not(target_os = "android"))]
    Ok(())
}
