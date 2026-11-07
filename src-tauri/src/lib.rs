pub mod api;
pub mod archive;
pub mod cloud;
pub mod commands;
pub mod config;
pub mod content_index;
pub mod db;
pub mod downloader;
pub mod linux_open;
pub mod logging;
pub mod media_info;
pub mod net;
pub mod server;
pub mod smart_links;
pub mod subscriptions;
pub mod sync;
pub mod webview_proxy;
pub mod window_setup;

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
#[allow(unused_imports)]
use tauri::{Emitter, Listener, Manager};

#[cfg(target_os = "linux")]
fn setup_platform() {
    // Disabling DMA-BUF drops WebKitGTK's accelerated path. Only NVIDIA's explicit
    // sync breaks on Wayland, so only that is turned off.
    if std::env::var_os("__NV_DISABLE_EXPLICIT_SYNC").is_none()
        && (std::path::Path::new("/proc/driver/nvidia").exists()
            || std::path::Path::new("/sys/module/nvidia").exists())
    {
        std::env::set_var("__NV_DISABLE_EXPLICIT_SYNC", "1");
    }
}

#[cfg(not(target_os = "linux"))]
fn setup_platform() {}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    setup_platform();

    logging::init_logging();
    let config_mgr = ConfigManager::new().expect("Failed to initialize SQLite settings");
    let settings = config_mgr.load().expect("Failed to load settings");
    net::apply_settings(&settings);

    let mut context = tauri::generate_context!();
    if let Some(listener) = window_setup::apply(context.config_mut(), &settings) {
        tauri::async_runtime::spawn(webview_proxy::serve(listener));
    }

    let content = Arc::new(
        ContentRepository::new(settings.cache_max_mb)
            .expect("Failed to initialize content repository"),
    );
    let _ = content.set_cache_limit_mb(settings.cache_max_mb);
    let library = Arc::new(LibraryRepository::new().expect("Failed to initialize local library"));
    let content_index = Arc::new(
        db::content_index::ContentIndexRepository::new()
            .expect("Failed to initialize the content index"),
    );

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
        Some(content.clone()),
    ));
    let subscription_repository = Arc::new(
        SubscriptionRepository::new().expect("Failed to initialize creator subscriptions"),
    );
    let axum_port = Arc::new(AtomicU16::new(0));
    let axum_token = uuid::Uuid::new_v4().simple().to_string();
    let subscription_manager = Arc::new(SubscriptionManager::new(
        subscription_repository,
        provider_manager.clone(),
        library.clone(),
        content.clone(),
        download_manager.clone(),
        config_manager.clone(),
        axum_port.clone(),
        axum_token.clone(),
    ));
    let sync_repository = Arc::new(SyncRepository::new().expect("Failed to initialize sync state"));
    let sync_manager = Arc::new(SyncManager::new(sync_repository, config_manager.clone()));
    let upstream = Arc::new(server::axum_server::UpstreamGateway::new(
        config_manager.clone(),
    ));
    let archives = Arc::new(
        archive::ArchiveRegistry::new(
            upstream.clone(),
            content.clone(),
            Arc::new(
                db::archives::ArchivePasswordRepository::new()
                    .expect("Failed to initialize archive passwords"),
            ),
            axum_port.clone(),
            axum_token.clone(),
        )
        .expect("Failed to initialize archive support"),
    );

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_deep_link::init());

    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
            if let Some(url) = argv.into_iter().nth(1) {
                if !url.trim().is_empty() {
                    let _ = app.emit("deep-link:opened", url);
                }
            }
        }));

    builder
        .setup(move |app| {
            commands::apply_ui_scale(app.handle(), &settings);
            let window_handle = app.handle().clone();
            let wh = window_handle.clone();
            app.listen("frontend-ready", move |_| {
                if let Some(window) = wh.get_webview_window("main") {
                    let _ = window.show();
                    #[cfg(desktop)]
                    {
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                    }
                }
            });

            #[cfg(desktop)]
            {
                let wh_fallback = window_handle.clone();
                tauri::async_runtime::spawn(async move {
                    #[cfg(target_os = "linux")]
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                    #[cfg(target_os = "macos")]
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                    #[cfg(target_os = "windows")]
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                    #[cfg(not(any(
                        target_os = "linux",
                        target_os = "macos",
                        target_os = "windows"
                    )))]
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

                    if let Some(window) = wh_fallback.get_webview_window("main") {
                        if !window.is_visible().unwrap_or(false) {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                });
            }

            let server_port = axum_port.clone();
            let media_download_dir = settings.download_dir.clone();
            let server_config_manager = config_manager.clone();
            let server_token = axum_token.clone();
            let server_archives = archives.clone();
            tauri::async_runtime::spawn(async move {
                match tokio::task::spawn_blocking(archive::clear_extraction_cache).await {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => tracing::warn!(%error, "Archive cache cleanup failed"),
                    Err(error) => tracing::warn!(%error, "Archive cache cleanup task failed"),
                }
                let mut roots = vec![
                    std::path::PathBuf::from(&media_download_dir),
                    db::storage::content_cache_path(),
                ];
                if let Ok(ensured) =
                    crate::downloader::manager::DownloadManager::ensure_download_root(
                        &media_download_dir,
                    )
                {
                    if !roots.contains(&ensured) {
                        roots.push(ensured);
                    }
                }
                #[cfg(target_os = "android")]
                {
                    let pkg = crate::db::storage::android_package_name();
                    roots.push(crate::db::storage::android_default_download_dir());
                    roots.push(
                        crate::db::storage::android_external_storage()
                            .join(format!("Android/data/{pkg}/files/Download")),
                    );
                    roots.push(crate::db::storage::data_root().join("Downloads"));
                    // /data/user/0 is a bind mount, so paths saved by older builds don't canonicalise to it.
                    roots.push(std::path::PathBuf::from(format!(
                        "/data/data/{pkg}/files/Pawstash/Downloads",
                    )));
                    roots.push(std::path::PathBuf::from("/sdcard/Download/Pawstash"));
                }
                if let Ok(server) = MediaServer::start(
                    roots,
                    server_config_manager,
                    server_token,
                    upstream,
                    server_archives,
                )
                .await
                {
                    server_port.store(server.port, Ordering::Release);
                }
            });

            download_manager.start(app.handle().clone());
            subscription_manager.start(app.handle().clone());
            sync_manager.start(app.handle().clone());

            #[cfg(desktop)]
            {
                let _ = setup_desktop_tray(app);
                use tauri_plugin_deep_link::DeepLinkExt;
                let _ = app.deep_link().register("pawstash");
                let app_handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    for url in event.urls() {
                        let _ = app_handle.emit("deep-link:opened", url.to_string());
                    }
                });
            }

            #[cfg(target_os = "android")]
            commands::set_android_app_handle(app.handle().clone());

            app.manage(AppState {
                axum_port,
                axum_token,
                provider_manager: provider_manager.clone(),
                pawchive_client: pawchive_client.clone(),
                content: content.clone(),
                library: library.clone(),
                download_manager: download_manager.clone(),
                subscription_manager: subscription_manager.clone(),
                sync_manager: sync_manager.clone(),
                config_manager: config_manager.clone(),
                archives: archives.clone(),
                content_index: content_index.clone(),
            });
            db::compaction::spawn(app.handle().clone());
            content_index::spawn(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_system_accent_color,
            get_pending_deep_link,
            set_app_hidden,
            open_app_links_settings,
            get_axum_port,
            list_post_inner_formats,
            refresh_ui_scale,
            set_ui_scale,
            check_aria2c_installed,
            probe_download_size,
            probe_download_sizes,
            probe_media_info,
            get_settings,
            get_default_settings,
            get_network_defaults,
            get_provider_network_defaults,
            get_webview_proxy_status,
            restart_app,
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
            pick_file,
            save_settings,
            list_providers,
            save_providers,
            test_provider_connection,
            get_provider_auth_schema,
            get_provider_capabilities,
            get_active_capabilities,
            save_provider_session,
            login_provider,
            logout_provider_session,
            sync_provider_favorites,
            get_account_session,
            fetch_creators,
            list_creators_page,
            list_creator_services,
            list_creator_names,
            get_creator_name,
            sync_creators,
            fetch_recent_posts,
            fetch_popular_posts,
            fetch_creator_posts,
            fetch_creator_profile,
            fetch_announcements,
            fetch_fancards,
            fetch_creator_links,
            fetch_similar_creators,
            fetch_creator_tags,
            fetch_post,
            get_cached_post,
            resolve_external_post_link,
            resolve_cloud_link,
            unlock_cloud_link,
            password_hints,
            open_archive,
            unlock_archive,
            fetch_account_favorites,
            set_post_favorite,
            set_creator_favorite,
            fetch_creator_artwork_data_url,
            get_creator_artwork_path,
            revalidate_creator_artwork,
            resolve_creator_url,
            resolve_post_url,
            resolve_creator_avatar_url,
            resolve_creator_banner_url,
            get_video_thumbnail,
            get_thumbnail_path,
            store_video_thumbnail,
            search_hash,
            flag_post,
            is_post_flagged,
            fetch_post_revisions,
            fetch_post_comments,
            get_pawchive_app_version,
            search_posts,
            list_library_collections,
            create_library_stash,
            update_library_stash,
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
            pause_all_downloads,
            resume_all_downloads,
            cancel_all_downloads,
            list_subscriptions,
            upsert_subscription,
            set_subscription_enabled,
            refresh_subscription,
            delete_subscription,
            clear_subscription_posts,
            count_subscription_posts,
            get_sync_status,
            create_sync_account,
            connect_sync_account,
            unlock_sync,
            lock_sync,
            disconnect_sync,
            test_sync_connection,
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
            commands::updater::cancel_update_download,
            commands::announcements::fetch_app_notices,
            commands::android_minimize,
            commands::window_effects::set_window_effect,
            commands::window_effects::set_window_background_color,
            show_main_window,
            hide_to_tray,
            update_panic_key,
            update_boss_key,
            log_message,
            get_debug_log_path,
            read_recent_logs,
            open_logs_folder,
            clear_logs,
            resolve_deep_link
        ])
        .run(context)
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
