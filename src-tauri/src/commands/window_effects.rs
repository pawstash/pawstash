#[cfg(any(target_os = "windows", target_os = "macos"))]
use tauri::utils::config::{Color, WindowEffectsConfig};
#[cfg(any(target_os = "windows", target_os = "macos"))]
use tauri::window::Effect;
use tauri::AppHandle;
#[cfg(any(target_os = "windows", target_os = "macos"))]
use tauri::Manager;

#[tauri::command]
pub async fn set_window_effect(app: AppHandle, effect_type: String) -> Result<(), String> {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    if let Some(window) = app.get_webview_window("main") {
        window
            .set_effects(None::<WindowEffectsConfig>)
            .map_err(|error| error.to_string())?;

        if effect_type == "none" {
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        {
            let effect = match effect_type.as_str() {
                "blur" => Effect::Blur,
                "mica" => Effect::Mica,
                "mica-dark" => Effect::MicaDark,
                "mica-light" => Effect::MicaLight,
                "tabbed" => Effect::Tabbed,
                "tabbed-dark" => Effect::TabbedDark,
                "tabbed-light" => Effect::TabbedLight,
                "acrylic" => Effect::Acrylic,
                other => return Err(format!("Window effect {other} is not available on Windows")),
            };

            let color = match effect_type.as_str() {
                "acrylic" => Some(Color(19, 19, 19, 163)),
                "mica" | "mica-dark" => None,
                _ => Some(Color(19, 19, 19, 163)),
            };

            let effects_config = WindowEffectsConfig {
                effects: vec![effect],
                state: None,
                radius: None,
                color,
            };

            window
                .set_effects(Some(effects_config))
                .map_err(|error| error.to_string())?;
        }

        #[cfg(target_os = "macos")]
        {
            let effect = match effect_type.as_str() {
                "sidebar" => Effect::Sidebar,
                "under-window" | "vibrancy" => Effect::UnderWindowBackground,
                other => return Err(format!("Window effect {other} is not available on macOS")),
            };

            let effects_config = WindowEffectsConfig {
                effects: vec![effect],
                state: None,
                radius: Some(14.0),
                color: None,
            };

            window
                .set_effects(Some(effects_config))
                .map_err(|error| error.to_string())?;
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let _ = (app, effect_type);

    Ok(())
}
