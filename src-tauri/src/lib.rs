pub mod api;
pub mod cloud;
pub mod commands;
pub mod config;
pub mod db;
pub mod downloader;
pub mod logging;
pub mod server;
pub mod smart_links;
pub mod subscriptions;
pub mod sync;

use api::pawchive::PawchiveClient;
use api::provider_manager::ProviderManager;
use commands::*;
use config::settings::ConfigManager;
use db::content::ContentRepository;
use db::downloads::DownloadRepository;
use db::library::LibraryRepository;
use db::subscriptions::SubscriptionRepository;
use downloader::manager::DownloadManager;
use server::axum_server::MediaServer;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use subscriptions::SubscriptionManager;
use sync::manager::SyncManager;
use sync::repository::SyncRepository;
use tauri::{Listener, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::init_logging();
    let config_mgr = ConfigManager::new().expect("Failed to initialize SQLite settings");
    let settings = config_mgr.load().expect("Failed to load settings");

    let content = Arc::new(
        ContentRepository::new(settings.cache_max_mb)
            .expect("Failed to initialize content repository"),
    );
    let _ = content.set_cache_limit_mb(settings.cache_max_mb);
    let library = Arc::new(LibraryRepository::new().expect("Failed to initialize local library"));

    let provider_manager = Arc::new(ProviderManager::new(settings.providers.clone()));
    let pawchive_client = Arc::new(
        PawchiveClient::new(settings.clone()).expect("Failed to initialize Pawchive HTTP client"),
    );
    let config_manager = Arc::new(config_mgr);
    let download_repository =
        Arc::new(DownloadRepository::new().expect("Failed to initialize download queue"));
    let download_manager = Arc::new(DownloadManager::new(
        download_repository,
        config_manager.clone(),
    ));
    let subscription_repository = Arc::new(
        SubscriptionRepository::new().expect("Failed to initialize creator subscriptions"),
    );
    let subscription_manager = Arc::new(SubscriptionManager::new(
        subscription_repository,
        provider_manager.clone(),
        library.clone(),
        content.clone(),
        download_manager.clone(),
        config_manager.clone(),
    ));
    let sync_repository = Arc::new(SyncRepository::new().expect("Failed to initialize sync state"));
    let sync_manager = Arc::new(SyncManager::new(sync_repository, config_manager.clone()));

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init());

    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());

    builder
        .setup(move |app| {
            // Do not reveal an uninitialized WebView: on Windows it otherwise
            // appears as a large gray surface while Vite is still serving the UI.
            let window_handle = app.handle().clone();
            app.listen("frontend-ready", move |_| {
                if let Some(window) = window_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            });

            let axum_port = Arc::new(AtomicU16::new(0));
            let server_port = axum_port.clone();
            let media_download_dir = settings.download_dir.clone();
            tauri::async_runtime::spawn(async move {
                let roots = vec![
                    std::path::PathBuf::from(media_download_dir),
                    db::storage::content_cache_path(),
                ];
                if let Ok(server) = MediaServer::start(roots).await {
                    server_port.store(server.port, Ordering::Release);
                }
            });

            download_manager.start(app.handle().clone());
            subscription_manager.start(app.handle().clone());
            sync_manager.start(app.handle().clone());

            #[cfg(desktop)]
            {
                let _ = setup_desktop_tray(app);
            }

            #[cfg(target_os = "android")]
            commands::set_android_app_handle(app.handle().clone());

            app.manage(AppState {
                axum_port,
                provider_manager: provider_manager.clone(),
                pawchive_client: pawchive_client.clone(),
                content: content.clone(),
                library: library.clone(),
                download_manager: download_manager.clone(),
                subscription_manager: subscription_manager.clone(),
                sync_manager: sync_manager.clone(),
                config_manager: config_manager.clone(),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_pending_deep_link,
            get_axum_port,
            check_aria2c_installed,
            probe_download_size,
            get_settings,
            get_default_settings,
            get_cache_stats,
            clear_content_cache,
            clear_all_content_cache,
            wipe_all_data,
            store_custom_background,
            store_custom_background_bytes,
            clear_custom_background,
            open_in_browser,
            open_downloads_folder,
            open_download_file,
            show_in_folder,
            pick_folder,
            save_settings,
            list_providers,
            save_providers,
            test_provider_connection,
            get_account_session,
            login_account,
            logout_account,
            fetch_creators,
            fetch_posts,
            fetch_recent_posts,
            fetch_popular_posts,
            fetch_creator_posts,
            fetch_creator_profile,
            fetch_announcements,
            fetch_fancards,
            fetch_creator_links,
            fetch_similar_creators,
            fetch_post,
            get_cached_post,
            resolve_external_post_link,
            resolve_cloud_link,
            fetch_account_favorites,
            set_post_favorite,
            set_creator_favorite,
            fetch_creator_artwork_data_url,
            search_hash,
            flag_post,
            is_post_flagged,
            fetch_post_revisions,
            fetch_post_comments,
            get_pawchive_app_version,
            search_posts,
            list_library_collections,
            create_library_stash,
            delete_library_stash,
            rename_library_stash,
            reorder_library_stashes,
            clear_library_stash,
            remove_library_post_from_stash,
            list_post_collections,
            save_library_post,
            remove_library_post,
            list_saved_post_identities,
            list_post_stash_memberships,
            list_library_posts,
            start_download,
            list_downloads,
            pause_download,
            cancel_download,
            resume_download,
            retry_download,
            remove_download,
            list_subscriptions,
            upsert_subscription,
            set_subscription_enabled,
            refresh_subscription,
            delete_subscription,
            get_sync_status,
            create_sync_account,
            connect_sync_account,
            unlock_sync,
            lock_sync,
            disconnect_sync,
            change_sync_password,
            list_sync_devices,
            revoke_sync_device,
            get_sync_recovery_kit,
            copy_sync_recovery_kit,
            recover_sync_account,
            run_sync,
            resolve_sync_conflict,
            set_sync_enabled,
            commands::updater::check_for_updates,
            commands::updater::download_and_install_update,
            commands::window_effects::set_window_effect,
            hide_to_tray,
            update_panic_key,
            update_boss_key,
            log_message,
            get_debug_log_path,
            read_recent_logs,
            open_logs_folder,
            clear_logs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(desktop)]
fn setup_desktop_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    };

    let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    if let Some(icon) = app.default_window_icon().cloned() {
        let _tray = TrayIconBuilder::with_id("pawstash-tray")
            .icon(icon)
            .menu(&menu)
            .show_menu_on_left_click(false)
            .tooltip("Pawstash")
            .on_menu_event(move |app_handle, event| match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                    }
                }
                "quit" => {
                    app_handle.exit(0);
                }
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    let app = tray.app_handle();
                    if let Some(window) = app.get_webview_window("main") {
                        if window.is_visible().unwrap_or(false) {
                            let _ = window.hide();
                        } else {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                }
            })
            .build(app)?;
    }

    Ok(())
}
