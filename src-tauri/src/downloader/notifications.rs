#[cfg(target_os = "android")]
use crate::commands::with_android_context;
use crate::config::settings::AppSettings;
use crate::db::downloads::DownloadJob;

pub fn update_download_notification(
    active_count: i32,
    total_count: i32,
    downloaded_bytes: u64,
    total_bytes: u64,
    speed_bytes_per_sec: u64,
    current_filename: &str,
    app_handle: Option<&tauri::AppHandle>,
) {
    #[cfg(target_os = "android")]
    {
        let _ = app_handle;
        let filename_str = current_filename.to_string();
        let res = with_android_context(|env, context| {
            let filename_jstr = env.new_string(&filename_str).map_err(|e| e.to_string())?;
            let class = env.get_object_class(context).map_err(|e| e.to_string())?;
            env.call_static_method(
                &class,
                "updateDownloadNotification",
                "(IIJJJLjava/lang/String;)V",
                &[
                    jni::objects::JValue::Int(active_count),
                    jni::objects::JValue::Int(total_count),
                    jni::objects::JValue::Long(downloaded_bytes as i64),
                    jni::objects::JValue::Long(total_bytes as i64),
                    jni::objects::JValue::Long(speed_bytes_per_sec as i64),
                    jni::objects::JValue::Object(&filename_jstr),
                ],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        });
        if let Err(e) = res {
            tracing::error!("update_download_notification error: {e}");
        }
    }

    #[cfg(not(target_os = "android"))]
    {
        let _ = (total_count, speed_bytes_per_sec, current_filename);
        if let Some(app) = app_handle {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let percent = if total_bytes > 0 {
                    ((downloaded_bytes as f64 / total_bytes as f64) * 100.0).clamp(0.0, 100.0)
                        as u64
                } else {
                    0
                };
                let _ = window.set_progress_bar(tauri::window::ProgressBarState {
                    status: if active_count > 0 {
                        Some(tauri::window::ProgressBarStatus::Normal)
                    } else {
                        Some(tauri::window::ProgressBarStatus::None)
                    },
                    progress: Some(percent),
                });
            }
        }
    }
}

pub fn update_download_paused_notification(
    paused_count: i32,
    app_handle: Option<&tauri::AppHandle>,
) {
    #[cfg(target_os = "android")]
    {
        let _ = app_handle;
        let res = with_android_context(|env, context| {
            let class = env.get_object_class(context).map_err(|e| e.to_string())?;
            env.call_static_method(
                &class,
                "updateDownloadPausedNotification",
                "(I)V",
                &[jni::objects::JValue::Int(paused_count)],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        });
        if let Err(e) = res {
            tracing::error!("update_download_paused_notification error: {e}");
        }
    }

    #[cfg(not(target_os = "android"))]
    {
        let _ = paused_count;
        if let Some(app) = app_handle {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_progress_bar(tauri::window::ProgressBarState {
                    status: Some(tauri::window::ProgressBarStatus::Paused),
                    progress: None,
                });
            }
        }
    }
}

pub fn notify_download_completed(
    app_handle: &tauri::AppHandle,
    settings: &AppSettings,
    completed: &DownloadJob,
) {
    if !settings.notifications_enabled || !settings.notifications_download_completed {
        return;
    }

    #[cfg(target_os = "android")]
    {
        let _ = app_handle;
        let service_str = completed.service.clone();
        let creator_str = completed.creator_id.clone();
        let creator_name_str = completed.creator_name.clone();
        let post_str = completed.post_id.clone();
        let filename_str = completed.filename.clone();
        let title_str = completed.post_title.clone();
        let final_path_str = completed.final_path.clone();
        let preview_path_str = completed.post_preview_path.clone().unwrap_or_default();
        let sound = settings.notifications_sound;
        let show_preview = settings.notifications_show_preview;

        let _ = with_android_context(|env, context| {
            let service_jstr = env.new_string(&service_str).map_err(|e| e.to_string())?;
            let creator_jstr = env.new_string(&creator_str).map_err(|e| e.to_string())?;
            let creator_name_jstr = env
                .new_string(&creator_name_str)
                .map_err(|e| e.to_string())?;
            let post_jstr = env.new_string(&post_str).map_err(|e| e.to_string())?;
            let filename_jstr = env.new_string(&filename_str).map_err(|e| e.to_string())?;
            let title_jstr = env.new_string(&title_str).map_err(|e| e.to_string())?;
            let final_path_jstr = env.new_string(&final_path_str).map_err(|e| e.to_string())?;
            let preview_path_jstr = env
                .new_string(&preview_path_str)
                .map_err(|e| e.to_string())?;
            let class = env.get_object_class(context).map_err(|e| e.to_string())?;

            env.call_static_method(
                &class,
                "notifyDownloadCompleted",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;ZZ)V",
                &[
                    jni::objects::JValue::Object(&service_jstr),
                    jni::objects::JValue::Object(&creator_jstr),
                    jni::objects::JValue::Object(&creator_name_jstr),
                    jni::objects::JValue::Object(&post_jstr),
                    jni::objects::JValue::Object(&filename_jstr),
                    jni::objects::JValue::Object(&title_jstr),
                    jni::objects::JValue::Object(&final_path_jstr),
                    jni::objects::JValue::Object(&preview_path_jstr),
                    jni::objects::JValue::Bool(sound as u8),
                    jni::objects::JValue::Bool(show_preview as u8),
                ],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        });
    }

    #[cfg(not(target_os = "android"))]
    {
        use tauri::Manager;
        use tauri_plugin_notification::NotificationExt;

        if let Some(window) = app_handle.get_webview_window("main") {
            let is_focused = window.is_focused().unwrap_or(false);
            let is_minimized = window.is_minimized().unwrap_or(false);
            let is_visible = window.is_visible().unwrap_or(true);

            if is_focused && !is_minimized && is_visible {
                tracing::info!(
                    "Main window is focused and visible; suppressing desktop OS notification"
                );
                return;
            }
        }

        let display_title = if !completed.creator_name.trim().is_empty() {
            let item_title = if !completed.post_title.trim().is_empty() {
                &completed.post_title
            } else {
                &completed.filename
            };
            format!("{} • {}", completed.creator_name, item_title)
        } else if !completed.post_title.trim().is_empty() {
            completed.post_title.clone()
        } else {
            "Download Complete".to_string()
        };

        let body = format!("Downloaded: {}", completed.filename);
        tracing::info!(title = %display_title, "Dispatching desktop OS notification");

        let mut builder = app_handle
            .notification()
            .builder()
            .title(display_title)
            .body(body);

        if !settings.notifications_sound {
            builder = builder.silent();
        }

        if settings.notifications_show_preview {
            let preview_candidate = completed
                .post_preview_path
                .as_deref()
                .or(Some(&completed.final_path))
                .filter(|p| {
                    let lower = p.to_lowercase();
                    lower.ends_with(".png")
                        || lower.ends_with(".jpg")
                        || lower.ends_with(".jpeg")
                        || lower.ends_with(".webp")
                        || lower.ends_with(".gif")
                });
            if let Some(img_path) = preview_candidate {
                if std::path::Path::new(img_path).exists() {
                    builder = builder.icon(img_path);
                }
            }
        }

        if let Err(e) = builder.show() {
            tracing::error!("Desktop notification error: {e}");
        }
    }
}

pub fn stop_download_service(app_handle: Option<&tauri::AppHandle>) {
    #[cfg(target_os = "android")]
    {
        let _ = app_handle;
        let _ = with_android_context(|env, context| {
            let class = env.get_object_class(context).map_err(|e| e.to_string())?;
            env.call_static_method(&class, "stopDownloadNotification", "()V", &[])
                .map_err(|e| e.to_string())?;
            Ok(())
        });
    }

    #[cfg(not(target_os = "android"))]
    {
        if let Some(app) = app_handle {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_progress_bar(tauri::window::ProgressBarState {
                    status: Some(tauri::window::ProgressBarStatus::None),
                    progress: None,
                });
            }
        }
    }
}

pub fn get_pending_deep_link() -> Option<String> {
    #[cfg(target_os = "android")]
    {
        with_android_context(|env, context| {
            let class = env.get_object_class(context).map_err(|e| e.to_string())?;
            let result = env
                .call_static_method(&class, "getPendingDeepLink", "()Ljava/lang/String;", &[])
                .map_err(|e| e.to_string())?;
            let obj = result.l().map_err(|e| e.to_string())?;
            if obj.is_null() {
                Ok(None)
            } else {
                let jstr: jni::objects::JString = obj.into();
                let rust_str = env.get_string(&jstr).map_err(|e| e.to_string())?;
                Ok(Some(rust_str.to_string_lossy().to_string()))
            }
        })
        .unwrap_or(None)
    }

    #[cfg(not(target_os = "android"))]
    {
        None
    }
}
