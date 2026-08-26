use crate::db::storage::open_database;
use crate::sync::secrets::SecretStore;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

const SETTINGS_PREFIX: &str = "setting.";
const PAWCHIVE_SESSION_SECRET: &str = "pawchive-session";
const PROXY_PASSWORD_SECRET: &str = "proxy-password";

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
    pub panic_button_enabled: bool,
    pub panic_button_shortcut: String,
    pub providers: Vec<crate::api::provider::ProviderConfig>,
    pub smart_merge_attachments: bool,
    pub pawchive_hide_ai: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        #[cfg(target_os = "android")]
        let download_dir = PathBuf::from("/storage/emulated/0/Download/Pawstash")
            .to_string_lossy()
            .to_string();

        #[cfg(not(target_os = "android"))]
        let download_dir = dirs::download_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Pawstash")
            .to_string_lossy()
            .to_string();

        Self {
            download_dir,
            cache_max_mb: 128,
            api_domain: "pawchive.pw".to_string(),
            file_domain: "file.pawchive.pw".to_string(),
            image_domain: "img.pawchive.pw".to_string(),
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
            grid_scale: 85,
            grid_aspect_ratio: GridAspectRatio::Square,
            dynamic_accent: true,
            sticky_header: true,
            layout_mode: "auto".to_string(),
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
            panic_button_enabled: true,
            panic_button_shortcut: "H".to_string(),
            providers: crate::api::provider_manager::ProviderManager::default_configs(),
            smart_merge_attachments: true,
            pawchive_hide_ai: false,
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

        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut statement = conn
            .prepare("SELECT key, value FROM app_settings WHERE key LIKE 'setting.%'")
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;
        let values = rows
            .collect::<Result<std::collections::HashMap<_, _>, _>>()
            .map_err(|e| e.to_string())?;
        let mut settings = AppSettings::default();
        settings.apply_values(&values);
        settings.session_cookie = Self::load_secret_string(PAWCHIVE_SESSION_SECRET)?;
        settings.proxy_password = Self::load_secret_string(PROXY_PASSWORD_SECRET)?;
        settings.normalize();

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

        let mut conn = self.conn.lock().map_err(|e| e.to_string())?;
        let transaction = conn.transaction().map_err(|e| e.to_string())?;
        for (key, value) in settings.values() {
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

    pub fn clear_session_cookie(&self) -> Result<(), String> {
        if let Ok(mut guard) = self.cached.lock() {
            if let Some(cached) = guard.as_mut() {
                cached.session_cookie.clear();
            }
        }
        SecretStore::delete_named(PAWCHIVE_SESSION_SECRET)
    }

    fn load_secret_string(name: &str) -> Result<String, String> {
        SecretStore::load_named(name)?
            .map(|value| String::from_utf8(value).map_err(|e| e.to_string()))
            .transpose()
            .map(|value| value.unwrap_or_default())
    }
}

impl AppSettings {
    fn values(&self) -> Vec<(&'static str, String)> {
        vec![
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
            ("auto_check_updates", self.auto_check_updates.to_string()),
            ("include_prereleases", self.include_prereleases.to_string()),
            ("scroll_edge_mask", self.scroll_edge_mask.to_string()),
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
            (
                "panic_button_enabled",
                self.panic_button_enabled.to_string(),
            ),
            ("panic_button_shortcut", self.panic_button_shortcut.clone()),
            (
                "providers_json",
                serde_json::to_string(&self.providers).unwrap_or_else(|_| "[]".to_string()),
            ),
            (
                "smart_merge_attachments",
                self.smart_merge_attachments.to_string(),
            ),
            (
                "pawchive_hide_ai",
                self.pawchive_hide_ai.to_string(),
            ),
        ]
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
        if self
            .download_dir
            .contains("/data/data/app.pawstash.client/files")
        {
            self.download_dir = "/storage/emulated/0/Download/Pawstash".to_string();
        }
        string!(api_domain);
        string!(file_domain);
        string!(image_domain);
        string!(pawchive_username);
        string!(theme);
        string!(proxy_url);
        string!(proxy_username);
        string!(layout_mode);
        string!(titlebar_style);
        string!(download_creator_folder_template);
        string!(download_post_folder_template);
        string!(download_filename_template);
        string!(download_metadata_format);
        if let Some(value) = get("providers_json") {
            if let Ok(mut providers) =
                serde_json::from_str::<Vec<crate::api::provider::ProviderConfig>>(value)
            {
                if !providers.is_empty() {
                    for p in &mut providers {
                        if p.name.contains("Coomer") || p.name.contains("cum.st") {
                            p.name = "OnlyHaven".into();
                        }
                        if p.id == "coomer"
                            && (p.api_url.contains("coomer.su")
                                || p.api_url.contains("coomer.party"))
                        {
                            p.api_url = "https://cum.st".into();
                            p.name = "OnlyHaven".into();
                            if !p.fallback_urls.iter().any(|u| u.contains("coomer.su")) {
                                p.fallback_urls.push("https://coomer.su".into());
                            }
                        }
                    }
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

    fn normalize(&mut self) {
        self.cache_max_mb = self.cache_max_mb.clamp(64, 2048);
        self.grid_scale = self.grid_scale.clamp(60, 160);
        self.download_max_concurrent = self.download_max_concurrent.clamp(1, 10);
        if !matches!(self.layout_mode.as_str(), "auto" | "mobile" | "desktop") {
            self.layout_mode = "auto".to_string();
        }
        if !matches!(self.titlebar_style.as_str(), "auto" | "windows" | "macos") {
            self.titlebar_style = "auto".to_string();
        }
        if self.providers.is_empty() {
            self.providers = crate::api::provider_manager::ProviderManager::default_configs();
        }
        if !self.session_cookie.is_empty() {
            if let Some(pawchive) = self.providers.iter_mut().find(|p| p.id == "pawchive") {
                if pawchive.session_cookie.is_empty() {
                    pawchive.session_cookie = self.session_cookie.clone();
                }
                if pawchive.username.is_empty() {
                    pawchive.username = self.pawchive_username.clone();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_use_defaults() {
        let settings: AppSettings =
            serde_json::from_str(r#"{"api_domain":"mirror.example","download_dir":"downloads"}"#)
                .unwrap();
        assert_eq!(settings.api_domain, "mirror.example");
        assert_eq!(settings.file_domain, "file.pawchive.pw");
        assert!(settings.proxy_bypass_local);
        assert_eq!(settings.grid_scale, 85);
        assert_eq!(settings.grid_aspect_ratio, GridAspectRatio::Square);
    }

    #[test]
    fn grid_scale_is_normalized() {
        let mut settings = AppSettings {
            grid_scale: 999,
            ..AppSettings::default()
        };
        settings.normalize();
        assert_eq!(settings.grid_scale, 160);
    }

    #[test]
    fn cache_limit_is_normalized() {
        let mut settings = AppSettings {
            cache_max_mb: 1,
            ..AppSettings::default()
        };
        settings.normalize();
        assert_eq!(settings.cache_max_mb, 64);

        settings.cache_max_mb = u64::MAX;
        settings.normalize();
        assert_eq!(settings.cache_max_mb, 2048);
    }

    #[test]
    fn scalar_database_values_exclude_secrets() {
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
            .into_iter()
            .map(|(key, value)| (format!("{SETTINGS_PREFIX}{key}"), value))
            .collect();
        let mut restored = AppSettings::default();
        restored.apply_values(&values);
        assert_eq!(restored.api_domain, original.api_domain);
        assert_eq!(restored.proxy_mode, ProxyMode::System);
        assert!(restored.session_cookie.is_empty());
        assert_eq!(restored.grid_scale, 125);
        assert!(!restored.dynamic_accent);
        assert!(!restored.sticky_header);
        assert_eq!(restored.layout_mode, "mobile");
        assert!(!restored.sync_enabled);
        assert!(!restored.sync_auto);
        assert!(!restored.sync_on_change);
        assert!(restored.sync_pawchive_session);
        assert_eq!(restored.sync_pull_interval_seconds, 120);
        assert_eq!(restored.sync_push_interval_seconds, 30);
    }
}
