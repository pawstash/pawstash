use crate::db::storage::open_database;
use crate::sync::secrets::SecretStore;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

const SETTINGS_PREFIX: &str = "setting.";
const PAWCHIVE_SESSION_SECRET: &str = "pawchive-session";
const PROXY_PASSWORD_SECRET: &str = "proxy-password";
const PROVIDER_SESSION_SECRET_PREFIX: &str = "provider-session:";

fn primary_provider_id() -> String {
    crate::api::providers::ProviderManager::default_configs()
        .into_iter()
        .next()
        .map(|provider| provider.id)
        .unwrap_or_default()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    None,
    #[default]
    System,
    Custom,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GridAspectRatio {
    #[default]
    Square,
    Portrait,
    Landscape,
    Widescreen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub download_dir: String,
    pub cache_max_mb: u64,
    pub api_domain: String,
    pub file_domain: String,
    pub image_domain: String,
    pub session_cookie: String,
    pub pawchive_username: String,
    pub theme: String,
    pub use_aria2c: bool,
    pub aria2_connections: u32,
    pub proxy_mode: ProxyMode,
    pub proxy_url: String,
    pub proxy_username: String,
    pub proxy_password: String,
    pub proxy_bypass_local: bool,
    pub grid_scale: u32,
    pub grid_aspect_ratio: GridAspectRatio,
    pub dynamic_accent: bool,
    pub sticky_header: bool,
    pub layout_mode: String,
    #[serde(default)]
    pub ui_scale: u32,
    pub sync_enabled: bool,
    pub sync_auto: bool,
    pub sync_on_change: bool,
    pub sync_pawchive_session: bool,
    pub sync_pull_interval_seconds: u32,
    pub sync_push_interval_seconds: u32,
    pub toast_position: String,
    pub auto_check_updates: bool,
    pub include_prereleases: bool,
    pub scroll_edge_mask: bool,
    pub titlebar_style: String,
    pub download_group_by_creator: bool,
    pub download_creator_folder_template: String,
    pub download_group_by_post: bool,
    pub download_post_folder_template: String,
    pub download_filename_template: String,
    pub download_save_metadata: bool,
    pub download_metadata_format: String,
    pub download_max_concurrent: u32,
    #[serde(default = "default_true")]
    pub download_auto_retry: bool,
    #[serde(default = "default_download_auto_retry_max")]
    pub download_auto_retry_max: u32,
    pub panic_button_enabled: bool,
    pub panic_button_shortcut: String,
    pub providers: Vec<crate::api::provider::ProviderConfig>,
    pub smart_merge_attachments: bool,
    pub pawchive_hide_ai: bool,
    pub persist_in_app_favorites_locally: bool,
    #[serde(default)]
    pub disable_blur_placeholders: bool,
    #[serde(default = "default_card_view_mode")]
    pub card_view_mode: String,
    #[serde(default = "default_notifications_enabled")]
    pub notifications_enabled: bool,
    #[serde(default = "default_true")]
    pub notifications_download_completed: bool,
    #[serde(default = "default_true")]
    pub notifications_download_progress: bool,
    #[serde(default = "default_true")]
    pub notifications_show_preview: bool,
    #[serde(default)]
    pub notifications_sound: bool,
    // Empty or 0 means the built-in default, shown as a placeholder.
    #[serde(default)]
    pub network_api_user_agent: String,
    #[serde(default)]
    pub network_browser_user_agent: String,
    #[serde(default)]
    pub network_timeout_secs: u64,
    #[serde(default)]
    pub network_connect_timeout_secs: u64,
    #[serde(default)]
    pub provider_deadline_secs: u64,
    #[serde(default = "default_window_background_color")]
    pub window_background_color: String,
    #[serde(default)]
    pub linux_transparent_window: bool,
    #[serde(default)]
    pub cloud_timeout_secs: u64,
    #[serde(default)]
    pub cloud_max_redirects: u64,
    #[serde(default = "default_true")]
    pub cloud_scraping_enabled: bool,
    #[serde(default = "default_true")]
    pub content_indexing_enabled: bool,
    #[serde(default)]
    pub cloud_links_in_comments: bool,
    #[serde(default = "default_comment_cloud_source")]
    pub cloud_links_in_comments_from: String,
    #[serde(default)]
    pub cloud_user_agent: String,
    #[serde(default)]
    pub cloud_proxy_url: String,
}

fn default_true() -> bool {
    true
}

fn default_comment_cloud_source() -> String {
    "all".to_string()
}

fn default_notifications_enabled() -> bool {
    cfg!(target_os = "android")
}

fn default_download_auto_retry_max() -> u32 {
    3
}

fn default_window_background_color() -> String {
    "#0c0e14".to_string()
}

/// Forced opaque: a translucent clear colour on an opaque window renders as garbage.
pub fn parse_window_background_color(value: &str) -> Option<tauri::utils::config::Color> {
    value
        .trim()
        .parse::<tauri::utils::config::Color>()
        .ok()
        .map(|color| tauri::utils::config::Color(color.0, color.1, color.2, 255))
}

fn default_card_view_mode() -> String {
    "detailed".to_string()
}

fn legacy_domain_value(endpoint: Option<&str>) -> String {
    endpoint
        .unwrap_or_default()
        .trim()
        .trim_end_matches('/')
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        #[cfg(target_os = "android")]
        let download_dir = crate::db::storage::android_default_download_dir()
            .to_string_lossy()
            .to_string();

        #[cfg(not(target_os = "android"))]
        let download_dir = dirs::download_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("Pawstash")
            .to_string_lossy()
            .to_string();

        let default_provider = crate::api::provider_manager::ProviderManager::default_configs()
            .into_iter()
            .next();

        Self {
            download_dir,
            cache_max_mb: 128,
            api_domain: legacy_domain_value(
                default_provider
                    .as_ref()
                    .map(|provider| provider.api_url.as_str()),
            ),
            file_domain: legacy_domain_value(
                default_provider
                    .as_ref()
                    .and_then(|provider| provider.file_url.as_deref()),
            ),
            image_domain: legacy_domain_value(
                default_provider
                    .as_ref()
                    .and_then(|provider| provider.image_url.as_deref()),
            ),
            session_cookie: String::new(),
            pawchive_username: String::new(),
            theme: "glass".to_string(),
            use_aria2c: true,
            aria2_connections: 16,
            proxy_mode: ProxyMode::System,
            proxy_url: String::new(),
            proxy_username: String::new(),
            proxy_password: String::new(),
            proxy_bypass_local: true,
            grid_scale: 100,
            grid_aspect_ratio: GridAspectRatio::Square,
            dynamic_accent: true,
            sticky_header: true,
            layout_mode: "auto".to_string(),
            ui_scale: 0,
            sync_enabled: true,
            sync_auto: true,
            sync_on_change: true,
            sync_pawchive_session: false,
            sync_pull_interval_seconds: 300,
            sync_push_interval_seconds: 60,
            toast_position: "auto".to_string(),
            auto_check_updates: true,
            include_prereleases: false,
            scroll_edge_mask: true,
            titlebar_style: "auto".to_string(),
            download_group_by_creator: true,
            download_creator_folder_template: "{creator}".to_string(),
            download_group_by_post: false,
            download_post_folder_template: "{post_title}".to_string(),
            download_filename_template: "{post_title} - {filename}".to_string(),
            download_save_metadata: false,
            download_metadata_format: "txt".to_string(),
            download_max_concurrent: 3,
            download_auto_retry: true,
            download_auto_retry_max: default_download_auto_retry_max(),
            panic_button_enabled: true,
            panic_button_shortcut: "H".to_string(),
            providers: crate::api::provider_manager::ProviderManager::default_configs(),
            smart_merge_attachments: true,
            pawchive_hide_ai: false,
            persist_in_app_favorites_locally: true,
            disable_blur_placeholders: false,
            card_view_mode: "detailed".to_string(),
            notifications_enabled: cfg!(target_os = "android"),
            notifications_download_completed: true,
            notifications_download_progress: true,
            notifications_show_preview: true,
            notifications_sound: false,
            network_api_user_agent: String::new(),
            network_browser_user_agent: String::new(),
            network_timeout_secs: 0,
            network_connect_timeout_secs: 0,
            provider_deadline_secs: 0,
            window_background_color: default_window_background_color(),
            linux_transparent_window: false,
            cloud_timeout_secs: 0,
            cloud_max_redirects: 0,
            cloud_scraping_enabled: true,
            content_indexing_enabled: true,
            cloud_links_in_comments: false,
            cloud_links_in_comments_from: default_comment_cloud_source(),
            cloud_user_agent: String::new(),
            cloud_proxy_url: String::new(),
        }
    }
}

pub struct ConfigManager {
    conn: Mutex<Connection>,
    cached: Mutex<Option<AppSettings>>,
}

impl ConfigManager {
    pub fn new() -> Result<Self, String> {
        let conn = open_database()?;

        let manager = Self {
            conn: Mutex::new(conn),
            cached: Mutex::new(None),
        };
        manager.initialize()?;
        Ok(manager)
    }

    fn initialize(&self) -> Result<(), String> {
        let has_settings = {
            let conn = self.conn.lock().map_err(|e| e.to_string())?;
            conn.query_row(
                "SELECT 1 FROM app_settings WHERE key = 'setting.api_domain'",
                [],
                |_| Ok(()),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .is_some()
        };

        if has_settings {
            Ok(())
        } else {
            self.save(&AppSettings::default())
        }
    }

    pub fn load(&self) -> Result<AppSettings, String> {
        if let Ok(guard) = self.cached.lock() {
            if let Some(cached) = guard.as_ref() {
                return Ok(cached.clone());
            }
        }

        let values = {
            let conn = self.conn.lock().map_err(|e| e.to_string())?;
            let mut statement = conn
                .prepare("SELECT key, value FROM app_settings WHERE key LIKE 'setting.%'")
                .map_err(|e| e.to_string())?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<std::collections::HashMap<_, _>, _>>()
                .map_err(|e| e.to_string())?
        };
        let mut settings = AppSettings::default();
        settings.apply_values(&values);
        settings.session_cookie = Self::load_secret_string(PAWCHIVE_SESSION_SECRET)?;
        settings.proxy_password = Self::load_secret_string(PROXY_PASSWORD_SECRET)?;
        let migrated_plaintext_secret = Self::hydrate_provider_secrets(&mut settings)?;
        let providers_before_normalize = settings.providers.clone();
        let renamed_provider_ids = settings.migrate_legacy_provider_ids();
        settings.normalize();

        if migrated_plaintext_secret || settings.providers != providers_before_normalize {
            self.save(&settings)?;
            // Otherwise the old secret loads into whichever provider gets the old id.
            for old_id in &renamed_provider_ids {
                let still_owned = settings.providers.iter().any(|p| {
                    p.id.eq_ignore_ascii_case(old_id) && !p.session_cookie.trim().is_empty()
                });
                if !still_owned {
                    SecretStore::delete_named(&Self::provider_secret_name(old_id))?;
                }
            }
            return Ok(settings);
        }

        if let Ok(mut guard) = self.cached.lock() {
            *guard = Some(settings.clone());
        }

        Ok(settings)
    }

    pub fn save(&self, settings: &AppSettings) -> Result<(), String> {
        if !settings.session_cookie.is_empty() {
            SecretStore::save_named(PAWCHIVE_SESSION_SECRET, settings.session_cookie.as_bytes())?;
        }
        if !settings.proxy_password.is_empty() {
            SecretStore::save_named(PROXY_PASSWORD_SECRET, settings.proxy_password.as_bytes())?;
        }
        for provider in &settings.providers {
            if !provider.session_cookie.trim().is_empty() {
                SecretStore::save_named(
                    &Self::provider_secret_name(&provider.id),
                    provider.session_cookie.as_bytes(),
                )?;
            }
        }

        let mut conn = self.conn.lock().map_err(|e| e.to_string())?;
        let transaction = conn.transaction().map_err(|e| e.to_string())?;
        for (key, value) in settings.values()? {
            transaction.execute(
                "INSERT INTO app_settings (key, value, updated_at) VALUES (?1, ?2, CURRENT_TIMESTAMP)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
                params![format!("{SETTINGS_PREFIX}{key}"), value],
            ).map_err(|e| e.to_string())?;
        }
        transaction
            .execute(
                "DELETE FROM app_settings WHERE key IN (?1, ?2)",
                params![
                    format!("{SETTINGS_PREFIX}session_cookie"),
                    format!("{SETTINGS_PREFIX}proxy_password")
                ],
            )
            .map_err(|e| e.to_string())?;
        transaction.commit().map_err(|e| e.to_string())?;

        if let Ok(mut guard) = self.cached.lock() {
            *guard = Some(settings.clone());
        }

        Ok(())
    }

    /// Own row, so a concurrent full save can't clobber it.
    pub fn save_window_background_color(&self, color: &str) -> Result<(), String> {
        {
            let conn = self.conn.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "INSERT INTO app_settings (key, value, updated_at) VALUES (?1, ?2, CURRENT_TIMESTAMP)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
                params![format!("{SETTINGS_PREFIX}window_background_color"), color],
            )
            .map_err(|e| e.to_string())?;
        }
        if let Ok(mut guard) = self.cached.lock() {
            if let Some(cached) = guard.as_mut() {
                cached.window_background_color = color.to_string();
            }
        }
        Ok(())
    }

    pub fn clear_session_cookie(&self) -> Result<(), String> {
        let primary_provider_id = primary_provider_id();
        if let Ok(mut guard) = self.cached.lock() {
            if let Some(cached) = guard.as_mut() {
                cached.session_cookie.clear();
                if let Some(provider) = cached
                    .providers
                    .iter_mut()
                    .find(|provider| provider.id == primary_provider_id)
                {
                    provider.session_cookie.clear();
                }
            }
        }
        SecretStore::delete_named(PAWCHIVE_SESSION_SECRET)?;
        SecretStore::delete_named(&Self::provider_secret_name(&primary_provider_id))
    }

    pub fn clear_provider_session(&self, provider_id: &str) -> Result<(), String> {
        let primary_provider_id = primary_provider_id();
        if let Ok(mut guard) = self.cached.lock() {
            if let Some(cached) = guard.as_mut() {
                if let Some(provider) = cached.providers.iter_mut().find(|p| p.id == provider_id) {
                    provider.session_cookie.clear();
                }
                if provider_id == primary_provider_id {
                    cached.session_cookie.clear();
                }
            }
        }
        SecretStore::delete_named(&Self::provider_secret_name(provider_id))?;
        if provider_id == primary_provider_id {
            SecretStore::delete_named(PAWCHIVE_SESSION_SECRET)?;
        }
        Ok(())
    }

    fn load_secret_string(name: &str) -> Result<String, String> {
        SecretStore::load_named(name)?
            .map(|value| String::from_utf8(value).map_err(|e| e.to_string()))
            .transpose()
            .map(|value| value.unwrap_or_default())
    }

    fn provider_secret_name(provider_id: &str) -> String {
        format!(
            "{PROVIDER_SESSION_SECRET_PREFIX}{}",
            provider_id.trim().to_lowercase()
        )
    }

    fn hydrate_provider_secrets(settings: &mut AppSettings) -> Result<bool, String> {
        let mut migrated_plaintext_secret = false;
        for provider in &mut settings.providers {
            let secret_name = Self::provider_secret_name(&provider.id);
            if provider.session_cookie.trim().is_empty() {
                provider.session_cookie = Self::load_secret_string(&secret_name)?;
            } else {
                SecretStore::save_named(&secret_name, provider.session_cookie.as_bytes())?;
                migrated_plaintext_secret = true;
            }
        }
        Ok(migrated_plaintext_secret)
    }
}

impl AppSettings {
    fn values(&self) -> Result<Vec<(&'static str, String)>, String> {
        let providers_json = serde_json::to_string(&self.providers).map_err(|e| e.to_string())?;
        Ok(vec![
            ("download_dir", self.download_dir.clone()),
            ("cache_max_mb", self.cache_max_mb.to_string()),
            ("api_domain", self.api_domain.clone()),
            ("file_domain", self.file_domain.clone()),
            ("image_domain", self.image_domain.clone()),
            ("pawchive_username", self.pawchive_username.clone()),
            ("theme", self.theme.clone()),
            ("use_aria2c", self.use_aria2c.to_string()),
            ("aria2_connections", self.aria2_connections.to_string()),
            (
                "proxy_mode",
                match self.proxy_mode {
                    ProxyMode::None => "none",
                    ProxyMode::System => "system",
                    ProxyMode::Custom => "custom",
                }
                .to_string(),
            ),
            ("proxy_url", self.proxy_url.clone()),
            ("proxy_username", self.proxy_username.clone()),
            ("proxy_bypass_local", self.proxy_bypass_local.to_string()),
            ("grid_scale", self.grid_scale.to_string()),
            (
                "grid_aspect_ratio",
                match self.grid_aspect_ratio {
                    GridAspectRatio::Square => "square",
                    GridAspectRatio::Portrait => "portrait",
                    GridAspectRatio::Landscape => "landscape",
                    GridAspectRatio::Widescreen => "widescreen",
                }
                .to_string(),
            ),
            ("dynamic_accent", self.dynamic_accent.to_string()),
            ("sticky_header", self.sticky_header.to_string()),
            ("layout_mode", self.layout_mode.clone()),
            ("ui_scale", self.ui_scale.to_string()),
            ("sync_enabled", self.sync_enabled.to_string()),
            ("sync_auto", self.sync_auto.to_string()),
            ("sync_on_change", self.sync_on_change.to_string()),
            (
                "sync_pawchive_session",
                self.sync_pawchive_session.to_string(),
            ),
            (
                "sync_pull_interval_seconds",
                self.sync_pull_interval_seconds.to_string(),
            ),
            (
                "sync_push_interval_seconds",
                self.sync_push_interval_seconds.to_string(),
            ),
            ("toast_position", self.toast_position.clone()),
            ("auto_check_updates", self.auto_check_updates.to_string()),
            ("include_prereleases", self.include_prereleases.to_string()),
            ("scroll_edge_mask", self.scroll_edge_mask.to_string()),
            (
                "network_api_user_agent",
                self.network_api_user_agent.clone(),
            ),
            (
                "network_browser_user_agent",
                self.network_browser_user_agent.clone(),
            ),
            (
                "network_timeout_secs",
                self.network_timeout_secs.to_string(),
            ),
            (
                "network_connect_timeout_secs",
                self.network_connect_timeout_secs.to_string(),
            ),
            (
                "provider_deadline_secs",
                self.provider_deadline_secs.to_string(),
            ),
            (
                "linux_transparent_window",
                self.linux_transparent_window.to_string(),
            ),
            ("cloud_timeout_secs", self.cloud_timeout_secs.to_string()),
            ("cloud_max_redirects", self.cloud_max_redirects.to_string()),
            (
                "cloud_scraping_enabled",
                self.cloud_scraping_enabled.to_string(),
            ),
            (
                "content_indexing_enabled",
                self.content_indexing_enabled.to_string(),
            ),
            (
                "cloud_links_in_comments",
                self.cloud_links_in_comments.to_string(),
            ),
            (
                "cloud_links_in_comments_from",
                self.cloud_links_in_comments_from.clone(),
            ),
            ("cloud_user_agent", self.cloud_user_agent.clone()),
            ("cloud_proxy_url", self.cloud_proxy_url.clone()),
            ("titlebar_style", self.titlebar_style.clone()),
            (
                "download_group_by_creator",
                self.download_group_by_creator.to_string(),
            ),
            (
                "download_creator_folder_template",
                self.download_creator_folder_template.clone(),
            ),
            (
                "download_group_by_post",
                self.download_group_by_post.to_string(),
            ),
            (
                "download_post_folder_template",
                self.download_post_folder_template.clone(),
            ),
            (
                "download_filename_template",
                self.download_filename_template.clone(),
            ),
            (
                "download_save_metadata",
                self.download_save_metadata.to_string(),
            ),
            (
                "download_metadata_format",
                self.download_metadata_format.clone(),
            ),
            (
                "download_max_concurrent",
                self.download_max_concurrent.to_string(),
            ),
            ("download_auto_retry", self.download_auto_retry.to_string()),
            (
                "download_auto_retry_max",
                self.download_auto_retry_max.to_string(),
            ),
            (
                "panic_button_enabled",
                self.panic_button_enabled.to_string(),
            ),
            ("panic_button_shortcut", self.panic_button_shortcut.clone()),
            ("providers_json", providers_json),
            (
                "smart_merge_attachments",
                self.smart_merge_attachments.to_string(),
            ),
            ("pawchive_hide_ai", self.pawchive_hide_ai.to_string()),
            (
                "persist_in_app_favorites_locally",
                self.persist_in_app_favorites_locally.to_string(),
            ),
            (
                "disable_blur_placeholders",
                self.disable_blur_placeholders.to_string(),
            ),
            ("card_view_mode", self.card_view_mode.clone()),
            (
                "notifications_enabled",
                self.notifications_enabled.to_string(),
            ),
            (
                "notifications_download_completed",
                self.notifications_download_completed.to_string(),
            ),
            (
                "notifications_download_progress",
                self.notifications_download_progress.to_string(),
            ),
            (
                "notifications_show_preview",
                self.notifications_show_preview.to_string(),
            ),
            ("notifications_sound", self.notifications_sound.to_string()),
        ])
    }

    fn apply_values(&mut self, values: &std::collections::HashMap<String, String>) {
        let get = |key: &str| values.get(&format!("{SETTINGS_PREFIX}{key}"));
        macro_rules! string {
            ($field:ident) => {
                if let Some(value) = get(stringify!($field)) {
                    self.$field = value.clone();
                }
            };
        }
        string!(download_dir);
        #[cfg(target_os = "android")]
        if self.download_dir.contains("/data/data/app.pawstash") {
            self.download_dir = crate::db::storage::android_default_download_dir()
                .to_string_lossy()
                .to_string();
        }
        string!(api_domain);
        string!(file_domain);
        string!(image_domain);
        string!(pawchive_username);
        string!(theme);
        string!(proxy_url);
        string!(proxy_username);
        string!(layout_mode);
        string!(toast_position);
        string!(titlebar_style);
        string!(network_api_user_agent);
        string!(network_browser_user_agent);
        string!(window_background_color);
        string!(cloud_user_agent);
        string!(cloud_proxy_url);
        string!(download_creator_folder_template);
        string!(download_post_folder_template);
        string!(download_filename_template);
        string!(download_metadata_format);
        string!(card_view_mode);
        string!(cloud_links_in_comments_from);
        if let Some(value) = get("providers_json") {
            if let Ok(providers) =
                serde_json::from_str::<Vec<crate::api::provider::ProviderConfig>>(value)
            {
                if !providers.is_empty() {
                    self.providers = providers;
                }
            }
        }
        if let Some(value) = get("smart_merge_attachments").and_then(|v| v.parse().ok()) {
            self.smart_merge_attachments = value;
        }
        if let Some(value) = get("pawchive_hide_ai").and_then(|v| v.parse().ok()) {
            self.pawchive_hide_ai = value;
        }
        if let Some(value) = get("persist_in_app_favorites_locally").and_then(|v| v.parse().ok()) {
            self.persist_in_app_favorites_locally = value;
        }
        if let Some(value) = get("disable_blur_placeholders").and_then(|v| v.parse().ok()) {
            self.disable_blur_placeholders = value;
        }
        if let Some(value) = get("notifications_enabled").and_then(|v| v.parse().ok()) {
            self.notifications_enabled = value;
        }
        if let Some(value) = get("notifications_download_completed").and_then(|v| v.parse().ok()) {
            self.notifications_download_completed = value;
        }
        if let Some(value) = get("notifications_download_progress").and_then(|v| v.parse().ok()) {
            self.notifications_download_progress = value;
        }
        if let Some(value) = get("notifications_show_preview").and_then(|v| v.parse().ok()) {
            self.notifications_show_preview = value;
        }
        if let Some(value) = get("notifications_sound").and_then(|v| v.parse().ok()) {
            self.notifications_sound = value;
        }
        if let Some(value) = get("panic_button_shortcut").or_else(|| get("boss_key_shortcut")) {
            if value == "Alt+X" {
                self.panic_button_shortcut = "H".to_string();
            } else {
                self.panic_button_shortcut = value.clone();
            }
        }
        if let Some(value) = get("download_save_metadata").and_then(|v| v.parse().ok()) {
            self.download_save_metadata = value;
        }
        if let Some(value) = get("download_max_concurrent").and_then(|v| v.parse().ok()) {
            self.download_max_concurrent = value;
        }
        if let Some(value) = get("download_auto_retry").and_then(|v| v.parse().ok()) {
            self.download_auto_retry = value;
        }
        if let Some(value) = get("download_auto_retry_max").and_then(|v| v.parse().ok()) {
            self.download_auto_retry_max = value;
        }
        if let Some(value) = get("panic_button_enabled")
            .or_else(|| get("boss_key_enabled"))
            .and_then(|v| v.parse().ok())
        {
            self.panic_button_enabled = value;
        }
        if let Some(value) = get("download_group_by_creator").and_then(|v| v.parse().ok()) {
            self.download_group_by_creator = value;
        }
        if let Some(value) = get("download_group_by_post").and_then(|v| v.parse().ok()) {
            self.download_group_by_post = value;
        }
        if let Some(value) = get("sync_enabled").and_then(|v| v.parse().ok()) {
            self.sync_enabled = value;
        }
        if let Some(value) = get("sync_auto").and_then(|v| v.parse().ok()) {
            self.sync_auto = value;
        }
        if let Some(value) = get("sync_on_change").and_then(|v| v.parse().ok()) {
            self.sync_on_change = value;
        }
        if let Some(value) = get("sync_pawchive_session").and_then(|v| v.parse().ok()) {
            self.sync_pawchive_session = value;
        }
        if let Some(value) = get("sync_pull_interval_seconds").and_then(|v| v.parse().ok()) {
            self.sync_pull_interval_seconds = value;
        }
        if let Some(value) = get("sync_push_interval_seconds").and_then(|v| v.parse().ok()) {
            self.sync_push_interval_seconds = value;
        }
        if let Some(value) = get("auto_check_updates").and_then(|v| v.parse().ok()) {
            self.auto_check_updates = value;
        }
        if let Some(value) = get("include_prereleases").and_then(|v| v.parse().ok()) {
            self.include_prereleases = value;
        }
        if let Some(value) = get("scroll_edge_mask").and_then(|v| v.parse().ok()) {
            self.scroll_edge_mask = value;
        }
        if let Some(value) = get("network_timeout_secs").and_then(|v| v.parse().ok()) {
            self.network_timeout_secs = value;
        }
        if let Some(value) = get("network_connect_timeout_secs").and_then(|v| v.parse().ok()) {
            self.network_connect_timeout_secs = value;
        }
        if let Some(value) = get("provider_deadline_secs").and_then(|v| v.parse().ok()) {
            self.provider_deadline_secs = value;
        }
        if let Some(value) = get("linux_transparent_window").and_then(|v| v.parse().ok()) {
            self.linux_transparent_window = value;
        }
        if let Some(value) = get("cloud_timeout_secs").and_then(|v| v.parse().ok()) {
            self.cloud_timeout_secs = value;
        }
        if let Some(value) = get("cloud_max_redirects").and_then(|v| v.parse().ok()) {
            self.cloud_max_redirects = value;
        }
        if let Some(value) = get("ui_scale").and_then(|v| v.parse().ok()) {
            self.ui_scale = value;
        }
        if let Some(value) = get("cloud_scraping_enabled").and_then(|v| v.parse().ok()) {
            self.cloud_scraping_enabled = value;
        }
        if let Some(value) = get("content_indexing_enabled").and_then(|v| v.parse().ok()) {
            self.content_indexing_enabled = value;
        }
        if let Some(value) = get("cloud_links_in_comments").and_then(|v| v.parse().ok()) {
            self.cloud_links_in_comments = value;
        }
        if let Some(value) = get("cache_max_mb").and_then(|v| v.parse().ok()) {
            self.cache_max_mb = value;
        }
        if let Some(value) = get("use_aria2c").and_then(|v| v.parse().ok()) {
            self.use_aria2c = value;
        }
        if let Some(value) = get("aria2_connections").and_then(|v| v.parse().ok()) {
            self.aria2_connections = value;
        }
        if let Some(value) = get("proxy_bypass_local").and_then(|v| v.parse().ok()) {
            self.proxy_bypass_local = value;
        }
        if let Some(value) = get("grid_scale").and_then(|v| v.parse().ok()) {
            self.grid_scale = value;
        }
        if let Some(value) = get("dynamic_accent").and_then(|v| v.parse().ok()) {
            self.dynamic_accent = value;
        }
        if let Some(value) = get("sticky_header").and_then(|v| v.parse().ok()) {
            self.sticky_header = value;
        }
        if let Some(value) = get("proxy_mode") {
            self.proxy_mode = match value.as_str() {
                "system" => ProxyMode::System,
                "custom" => ProxyMode::Custom,
                _ => ProxyMode::None,
            };
        }
        if let Some(value) = get("grid_aspect_ratio") {
            self.grid_aspect_ratio = match value.as_str() {
                "portrait" => GridAspectRatio::Portrait,
                "landscape" => GridAspectRatio::Landscape,
                "widescreen" => GridAspectRatio::Widescreen,
                _ => GridAspectRatio::Square,
            };
        }
    }

    pub fn normalize(&mut self) {
        if let Some(absolute) = absolute_download_dir(&self.download_dir) {
            self.download_dir = absolute;
        }
        self.cache_max_mb = self.cache_max_mb.clamp(64, 2048);
        self.grid_scale = self.grid_scale.clamp(60, 160);
        self.download_max_concurrent = self.download_max_concurrent.clamp(1, 10);
        self.download_auto_retry_max = self.download_auto_retry_max.clamp(1, 10);
        if self.ui_scale != 0 {
            self.ui_scale = self.ui_scale.clamp(50, 300);
        }
        if !matches!(self.layout_mode.as_str(), "auto" | "mobile" | "desktop") {
            self.layout_mode = "auto".to_string();
        }
        if !matches!(self.titlebar_style.as_str(), "auto" | "windows" | "macos") {
            self.titlebar_style = "auto".to_string();
        }
        if !matches!(
            self.toast_position.as_str(),
            "auto"
                | "top-left"
                | "top-center"
                | "top-right"
                | "bottom-left"
                | "bottom-center"
                | "bottom-right"
        ) {
            self.toast_position = "auto".to_string();
        }
        if !matches!(self.card_view_mode.as_str(), "detailed" | "lite") {
            self.card_view_mode = "detailed".to_string();
        }
        if self.network_timeout_secs != 0 {
            self.network_timeout_secs = self.network_timeout_secs.clamp(5, 600);
        }
        if self.network_connect_timeout_secs != 0 {
            self.network_connect_timeout_secs = self.network_connect_timeout_secs.clamp(1, 120);
        }
        if self.provider_deadline_secs != 0 {
            self.provider_deadline_secs = self.provider_deadline_secs.clamp(1, 120);
        }
        if self.cloud_timeout_secs != 0 {
            self.cloud_timeout_secs = self.cloud_timeout_secs.clamp(5, 600);
        }
        if self.cloud_max_redirects != 0 {
            self.cloud_max_redirects = self.cloud_max_redirects.clamp(1, 30);
        }
        if parse_window_background_color(&self.window_background_color).is_none() {
            self.window_background_color = default_window_background_color();
        }
        if self.providers.is_empty() {
            self.providers = crate::api::provider_manager::ProviderManager::default_configs();
        }
        self.migrate_legacy_provider_ids();
        let default_configs = crate::api::provider_manager::ProviderManager::default_configs();
        for p in &mut self.providers {
            if p.timeout_secs != 0 {
                p.timeout_secs = p.timeout_secs.clamp(5, 600);
            }
            if p.max_retries != 0 {
                p.max_retries = p.max_retries.clamp(1, 10);
            }
            if p.min_interval_ms != 0 {
                p.min_interval_ms = p.min_interval_ms.clamp(10, 5000);
            }
        }
        for p in &mut self.providers {
            if let Some(def) = default_configs
                .iter()
                .find(|d| d.id.eq_ignore_ascii_case(&p.id))
            {
                if p.name.trim().is_empty() {
                    p.name = def.name.clone();
                }
                if p.services.is_empty() {
                    p.services = def.services.clone();
                } else {
                    for s in &def.services {
                        if !p
                            .services
                            .iter()
                            .any(|existing| existing.eq_ignore_ascii_case(s))
                        {
                            p.services.push(s.clone());
                        }
                    }
                }
                if p.file_prefix.is_none() {
                    p.file_prefix = def.file_prefix.clone();
                }
                if p.image_prefix.is_none() {
                    p.image_prefix = def.image_prefix.clone();
                }

                let api_matches_default =
                    normalized_origin(&p.api_url) == normalized_origin(&def.api_url);

                if !p.is_custom && api_matches_default {
                    if endpoint_is_missing_or_api_origin(p.file_url.as_deref(), &p.api_url) {
                        p.file_url = def.file_url.clone();
                    }
                    if endpoint_is_missing_or_api_origin(p.image_url.as_deref(), &p.api_url) {
                        p.image_url = def.image_url.clone();
                    }
                }
            }
        }
        for def in default_configs {
            if !self
                .providers
                .iter()
                .any(|p| p.id.eq_ignore_ascii_case(&def.id))
            {
                self.providers.push(def);
            }
        }
        let primary_provider_id = primary_provider_id();
        if let Some(pawchive) = self
            .providers
            .iter_mut()
            .find(|provider| provider.id == primary_provider_id)
        {
            if !self.session_cookie.is_empty() {
                if pawchive.session_cookie.is_empty() {
                    pawchive.session_cookie = self.session_cookie.clone();
                }
                if pawchive.username.is_empty() {
                    pawchive.username = self.pawchive_username.clone();
                }
            } else if !pawchive.session_cookie.is_empty() {
                self.session_cookie = pawchive.session_cookie.clone();
                if self.pawchive_username.is_empty() {
                    self.pawchive_username = pawchive.username.clone();
                }
            }
        }
    }

    /// v26.8.3 to v26.8.12 saved OnlyHaven under `coomer`. Returns the renamed-away ids.
    pub fn migrate_legacy_provider_ids(&mut self) -> Vec<String> {
        let defaults = crate::api::provider_manager::ProviderManager::default_configs();
        let mut renamed = Vec::new();
        for provider in &mut self.providers {
            if provider.is_custom {
                continue;
            }
            let Some(host) = url_host(&provider.api_url) else {
                continue;
            };
            let Some(def) = defaults.iter().find(|def| {
                url_host(&def.api_url).is_some_and(|def_host| host_within(&host, &def_host))
            }) else {
                continue;
            };
            if provider.id.eq_ignore_ascii_case(&def.id) {
                continue;
            }
            renamed.push(std::mem::replace(&mut provider.id, def.id.clone()));
            provider.name = def.name.clone();
            provider.services = def.services.clone();
            provider.file_prefix = def.file_prefix.clone();
            provider.image_prefix = def.image_prefix.clone();
            let def_host = url_host(&def.api_url).unwrap_or_default();
            let foreign = |url: &Option<String>| {
                !url.as_deref()
                    .and_then(url_host)
                    .is_some_and(|h| host_within(&h, &def_host))
            };
            if foreign(&provider.file_url) {
                provider.file_url = def.file_url.clone();
            }
            if foreign(&provider.image_url) {
                provider.image_url = def.image_url.clone();
            }
        }

        let mut kept: Vec<crate::api::provider::ProviderConfig> =
            Vec::with_capacity(self.providers.len());
        for provider in self.providers.drain(..) {
            match kept
                .iter_mut()
                .find(|existing| existing.id.eq_ignore_ascii_case(&provider.id))
            {
                Some(existing) => {
                    existing.enabled |= provider.enabled;
                    if existing.session_cookie.trim().is_empty()
                        && !provider.session_cookie.trim().is_empty()
                    {
                        existing.session_cookie = provider.session_cookie;
                        existing.username = provider.username;
                    }
                }
                None => kept.push(provider),
            }
        }
        self.providers = kept;
        renamed
    }

    pub fn resolve_cookie_for_url(&self, url: &str) -> Option<String> {
        let parsed = reqwest::Url::parse(url).ok()?;
        let host = parsed.host_str()?.to_lowercase();

        for p in &self.providers {
            if p.session_cookie.trim().is_empty() {
                continue;
            }
            if let Ok(api_u) = reqwest::Url::parse(&p.api_url) {
                if let Some(api_h) = api_u.host_str() {
                    let domain = api_h.trim_start_matches("www.").trim_start_matches("api.");
                    if Self::host_matches(&host, api_h) || Self::host_matches(&host, domain) {
                        return Some(p.session_cookie.clone());
                    }
                }
            }
            for fb in &p.fallback_urls {
                if let Ok(fb_u) = reqwest::Url::parse(fb) {
                    if let Some(fb_h) = fb_u.host_str() {
                        let domain = fb_h.trim_start_matches("www.").trim_start_matches("api.");
                        if Self::host_matches(&host, fb_h) || Self::host_matches(&host, domain) {
                            return Some(p.session_cookie.clone());
                        }
                    }
                }
            }
            if let Some(ref file_u_str) = p.file_url {
                if let Ok(file_u) = reqwest::Url::parse(file_u_str) {
                    if let Some(file_h) = file_u.host_str() {
                        if Self::host_matches(&host, file_h) {
                            return Some(p.session_cookie.clone());
                        }
                    }
                }
            }
            if let Some(ref image_u_str) = p.image_url {
                if let Ok(image_u) = reqwest::Url::parse(image_u_str) {
                    if let Some(image_h) = image_u.host_str() {
                        if Self::host_matches(&host, image_h) {
                            return Some(p.session_cookie.clone());
                        }
                    }
                }
            }
        }
        None
    }

    fn host_matches(candidate: &str, allowed: &str) -> bool {
        let allowed = allowed.trim_end_matches('.').to_lowercase();
        candidate == allowed || candidate.ends_with(&format!(".{allowed}"))
    }

    pub fn get_provider_for_service(
        &self,
        service: &str,
    ) -> Option<&crate::api::providers::traits::ProviderConfig> {
        let clean = service.trim().to_ascii_lowercase();
        self.providers
            .iter()
            .find(|p| p.enabled && p.services.iter().any(|s| s.eq_ignore_ascii_case(&clean)))
            .or_else(|| {
                self.providers
                    .iter()
                    .find(|p| p.services.iter().any(|s| s.eq_ignore_ascii_case(&clean)))
            })
    }
}

fn url_host(value: &str) -> Option<String> {
    let url = reqwest::Url::parse(value.trim()).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    Some(
        host.strip_prefix("www.")
            .map(str::to_string)
            .unwrap_or(host),
    )
}

fn host_within(host: &str, domain: &str) -> bool {
    host == domain
        || host
            .strip_suffix(domain)
            .is_some_and(|prefix| prefix.ends_with('.'))
}

fn normalized_origin(value: &str) -> String {
    value
        .trim()
        .trim_end_matches('/')
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .to_ascii_lowercase()
}

fn endpoint_is_missing_or_api_origin(endpoint: Option<&str>, api_url: &str) -> bool {
    endpoint.is_none_or(|value| {
        value.trim().is_empty() || normalized_origin(value) == normalized_origin(api_url)
    })
}

fn absolute_download_dir(raw: &str) -> Option<String> {
    let path = std::path::Path::new(raw.trim());
    if raw.trim().is_empty() || !path.is_relative() {
        return None;
    }
    #[cfg(target_os = "android")]
    let base = crate::db::storage::android_external_storage();
    #[cfg(not(target_os = "android"))]
    let base = dirs::home_dir()?;
    Some(base.join(path).to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::provider_manager::ProviderManager;

    fn normalized(mut settings: AppSettings) -> AppSettings {
        settings.normalize();
        settings
    }

    #[test]
    fn relative_download_dir_is_anchored_to_home() {
        let settings = normalized(AppSettings {
            download_dir: "Downloads/Pawstash".into(),
            ..AppSettings::default()
        });
        assert!(std::path::Path::new(&settings.download_dir).is_absolute());
        assert!(settings.download_dir.ends_with("Pawstash"));
        assert_eq!(
            normalized(settings.clone()).download_dir,
            settings.download_dir
        );
    }

    #[test]
    fn legacy_onlyhaven_saved_as_coomer_is_renamed_not_duplicated() {
        use crate::api::providers::onlyhaven::OnlyHavenProvider;
        let legacy = crate::api::provider::ProviderConfig {
            id: "coomer".to_string(),
            name: "OnlyHaven".to_string(),
            enabled: true,
            api_url: "https://cum.st".to_string(),
            file_prefix: Some("c1".to_string()),
            services: vec!["onlyfans".into(), "fansly".into(), "candfans".into()],
            session_cookie: "legacy-session".to_string(),
            ..Default::default()
        };
        let mut settings = AppSettings {
            providers: vec![ProviderManager::default_configs()[0].clone(), legacy],
            ..AppSettings::default()
        };

        assert_eq!(
            settings.migrate_legacy_provider_ids(),
            vec!["coomer".to_string()]
        );
        settings.normalize();

        let onlyhaven: Vec<_> = settings
            .providers
            .iter()
            .filter(|p| OnlyHavenProvider::matches_config(p))
            .collect();
        assert_eq!(onlyhaven.len(), 1);
        assert_eq!(onlyhaven[0].id, "onlyhaven");
        assert!(onlyhaven[0].enabled);
        assert_eq!(onlyhaven[0].session_cookie, "legacy-session");
        assert!(!onlyhaven[0].services.iter().any(|s| s == "candfans"));
        assert_ne!(onlyhaven[0].file_prefix.as_deref(), Some("c1"));

        let coomer: Vec<_> = settings
            .providers
            .iter()
            .filter(|p| p.id == "coomer")
            .collect();
        assert_eq!(coomer.len(), 1);
        assert_eq!(coomer[0].api_url, "https://coomer.st");
        assert!(coomer[0].session_cookie.is_empty());
        assert_eq!(
            settings
                .providers
                .iter()
                .filter(|p| p.name == "OnlyHaven")
                .count(),
            1
        );
    }

    #[test]
    fn provider_mirrors_keep_their_ids() {
        let mut coomer = ProviderManager::default_configs()
            .into_iter()
            .find(|p| p.id == "coomer")
            .unwrap();
        coomer.api_url = "https://coomer.su".to_string();
        let mut settings = AppSettings {
            providers: vec![coomer],
            ..AppSettings::default()
        };
        assert!(settings.migrate_legacy_provider_ids().is_empty());
        assert_eq!(settings.providers[0].id, "coomer");
    }

    #[test]
    fn normalize_keeps_every_choice_the_ui_offers() {
        for offered in [
            "auto",
            "top-left",
            "top-center",
            "top-right",
            "bottom-left",
            "bottom-center",
            "bottom-right",
        ] {
            let settings = AppSettings {
                toast_position: offered.to_string(),
                ..AppSettings::default()
            };
            assert_eq!(normalized(settings).toast_position, offered);
        }
        for offered in ["detailed", "lite"] {
            let settings = AppSettings {
                card_view_mode: offered.to_string(),
                ..AppSettings::default()
            };
            assert_eq!(normalized(settings).card_view_mode, offered);
        }
        let unknown = normalized(AppSettings {
            toast_position: "sideways".to_string(),
            card_view_mode: "compact".to_string(),
            ..AppSettings::default()
        });
        assert_eq!(unknown.toast_position, "auto");
        assert_eq!(unknown.card_view_mode, "detailed");
    }

    #[test]
    fn scalar_database_values_round_trip_without_secrets() {
        let original = AppSettings {
            api_domain: "mirror.example".to_string(),
            proxy_mode: ProxyMode::System,
            session_cookie: "session=secret".to_string(),
            grid_scale: 125,
            dynamic_accent: false,
            sticky_header: false,
            layout_mode: "mobile".to_string(),
            sync_enabled: false,
            sync_auto: false,
            sync_on_change: false,
            sync_pawchive_session: true,
            sync_pull_interval_seconds: 120,
            sync_push_interval_seconds: 30,
            ..AppSettings::default()
        };
        let values = original
            .values()
            .unwrap()
            .into_iter()
            .map(|(key, value)| (format!("{SETTINGS_PREFIX}{key}"), value))
            .collect();
        let mut restored = AppSettings::default();
        restored.apply_values(&values);
        let expected = AppSettings {
            session_cookie: String::new(),
            ..original
        };
        assert_eq!(
            serde_json::to_value(&restored).unwrap(),
            serde_json::to_value(&expected).unwrap()
        );
        assert!(!values[&format!("{SETTINGS_PREFIX}providers_json")].contains("session=secret"));
    }

    #[test]
    fn session_cookies_only_go_to_their_provider_hosts() {
        let mut settings = normalized(AppSettings::default());
        let mut file_urls = Vec::new();
        for (provider, cookie) in settings.providers.iter_mut().zip(["primary", "secondary"]) {
            provider.session_cookie = cookie.into();
            file_urls.push(format!(
                "{}/data/ab/cd/video.mp4",
                provider.file_url.as_deref().unwrap()
            ));
        }
        assert_eq!(
            settings.resolve_cookie_for_url(&file_urls[0]),
            Some("primary".into())
        );
        assert_eq!(
            settings.resolve_cookie_for_url(&file_urls[1]),
            Some("secondary".into())
        );

        let primary_host = reqwest::Url::parse(&settings.providers[0].api_url)
            .unwrap()
            .host_str()
            .unwrap()
            .to_string();
        for foreign in [
            format!("https://{primary_host}.evil.example/video.mp4"),
            "https://unrelated.example/video.mp4".to_string(),
        ] {
            assert_eq!(settings.resolve_cookie_for_url(&foreign), None, "{foreign}");
        }
    }

    #[test]
    fn normalize_repairs_builtin_media_origins_without_overriding_mirrors() {
        let default = ProviderManager::default_configs()
            .into_iter()
            .find(|provider| provider.file_url.is_some() && provider.image_url.is_some())
            .unwrap();
        let normalized_provider = |edit: &dyn Fn(&mut crate::api::provider::ProviderConfig)| {
            let mut settings = AppSettings::default();
            edit(
                settings
                    .providers
                    .iter_mut()
                    .find(|p| p.id == default.id)
                    .unwrap(),
            );
            normalized(settings)
                .providers
                .into_iter()
                .find(|p| p.id == default.id)
                .unwrap()
        };

        let repaired = normalized_provider(&|p| {
            p.file_url = Some(p.api_url.clone());
            p.image_url = None;
        });
        assert_eq!(repaired.file_url, default.file_url);
        assert_eq!(repaired.image_url, default.image_url);

        let mirror = normalized_provider(&|p| {
            p.api_url = "https://mirror.example".into();
            p.file_url = None;
            p.image_url = None;
        });
        assert_eq!(mirror.file_url, None);
        assert_eq!(mirror.image_url, None);
    }
}
