use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tokio::io::AsyncWriteExt;

const GITHUB_REPO: &str = "pawstash/pawstash";
const USER_AGENT: &str = concat!(
    "Pawstash/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/pawstash/pawstash)"
);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub prerelease: bool,
    pub published_at: Option<String>,
    pub html_url: String,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub is_prerelease: bool,
    pub release_name: String,
    pub release_notes: String,
    pub published_at: String,
    pub release_url: String,
    pub download_url: Option<String>,
    pub asset_name: Option<String>,
    pub asset_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProgressPayload {
    pub downloaded: u64,
    pub total: u64,
    pub percentage: f64,
    pub speed_bytes_per_sec: u64,
}

#[tauri::command]
pub async fn check_for_updates(include_prereleases: bool) -> Result<UpdateInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases");

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let response = client
        .get(&url)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch releases from GitHub: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API returned status: {}", response.status()));
    }

    let releases: Vec<GitHubRelease> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse GitHub releases response: {e}"))?;

    for release in releases {
        if release.prerelease && !include_prereleases {
            continue;
        }

        let latest_tag = release.tag_name.trim_start_matches('v');
        if is_version_newer(latest_tag, &current_version) {
            let (asset_url, asset_name, asset_size) = find_platform_asset(&release.assets);

            return Ok(UpdateInfo {
                available: true,
                current_version: current_version.clone(),
                latest_version: latest_tag.to_string(),
                is_prerelease: release.prerelease,
                release_name: release.name.unwrap_or_else(|| release.tag_name.clone()),
                release_notes: release.body.unwrap_or_default(),
                published_at: release.published_at.unwrap_or_default(),
                release_url: release.html_url,
                download_url: asset_url,
                asset_name,
                asset_size,
            });
        }
    }

    Ok(UpdateInfo {
        available: false,
        current_version: current_version.clone(),
        latest_version: current_version,
        is_prerelease: false,
        release_name: String::new(),
        release_notes: String::new(),
        published_at: String::new(),
        release_url: format!("https://github.com/{GITHUB_REPO}/releases"),
        download_url: None,
        asset_name: None,
        asset_size: None,
    })
}

fn get_update_temp_dir() -> Result<std::path::PathBuf, String> {
    #[cfg(target_os = "android")]
    {
        let ctx = ndk_context::android_context();
        let vm_ptr = ctx.vm();
        let context_ptr = ctx.context();
        if vm_ptr.is_null() || context_ptr.is_null() {
            return Ok(std::env::temp_dir());
        }
        let vm = unsafe { jni::JavaVM::from_raw(vm_ptr.cast()).map_err(|e| e.to_string())? };
        let mut env = vm
            .attach_current_thread_as_daemon()
            .map_err(|e| e.to_string())?;
        let context = unsafe { jni::objects::JObject::from_raw(context_ptr.cast()) };

        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
        }

        match env.call_method(&context, "getCacheDir", "()Ljava/io/File;", &[]) {
            Ok(val) => {
                if let Ok(file_obj) = val.l() {
                    if !file_obj.is_null() {
                        if let Ok(path_val) = env.call_method(
                            &file_obj,
                            "getAbsolutePath",
                            "()Ljava/lang/String;",
                            &[],
                        ) {
                            if let Ok(path_obj) = path_val.l() {
                                let jstr: jni::objects::JString = path_obj.into();
                                if let Ok(path_str) = env.get_string(&jstr) {
                                    return Ok(std::path::PathBuf::from(
                                        path_str.to_string_lossy().to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {
                let _ = env.exception_clear();
            }
        }

        Ok(std::env::temp_dir())
    }

    #[cfg(not(target_os = "android"))]
    {
        Ok(std::env::temp_dir())
    }
}

#[tauri::command]
pub async fn download_and_install_update(
    app_handle: tauri::AppHandle,
    download_url: String,
    asset_name: String,
) -> Result<(), String> {
    let parsed_url =
        reqwest::Url::parse(&download_url).map_err(|e| format!("Invalid download URL: {e}"))?;
    if parsed_url.scheme() != "https" {
        return Err("Only HTTPS download URLs are permitted".to_string());
    }

    let temp_dir = get_update_temp_dir()?;
    let safe_name = if asset_name.trim().is_empty() {
        "pawstash-update".to_string()
    } else {
        std::path::Path::new(&asset_name)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "pawstash-update".to_string())
    };
    let target_path = temp_dir.join(&safe_name);

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("HTTP Client error: {e}"))?;

    let response = client
        .get(parsed_url)
        .send()
        .await
        .map_err(|e| format!("Download request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Download failed with HTTP status: {}",
            response.status()
        ));
    }

    let total_bytes = response.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(&target_path)
        .await
        .map_err(|e| format!("Failed to create temp update file: {e}"))?;

    let mut stream = response.bytes_stream();
    let mut downloaded_bytes: u64 = 0;
    let mut last_emit = std::time::Instant::now();
    let mut last_bytes: u64 = 0;

    while let Some(chunk_res) = futures_util::StreamExt::next(&mut stream).await {
        let chunk = chunk_res.map_err(|e| format!("Error during stream chunk read: {e}"))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write chunk: {e}"))?;

        downloaded_bytes += chunk.len() as u64;

        if last_emit.elapsed().as_millis() >= 100
            || (total_bytes > 0 && downloaded_bytes >= total_bytes)
        {
            let elapsed_sec = last_emit.elapsed().as_secs_f64();
            let speed = if elapsed_sec > 0.0 {
                ((downloaded_bytes.saturating_sub(last_bytes)) as f64 / elapsed_sec) as u64
            } else {
                0
            };

            let percentage = if total_bytes > 0 {
                ((downloaded_bytes as f64 / total_bytes as f64) * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            };

            let _ = app_handle.emit(
                "update-download-progress",
                UpdateProgressPayload {
                    downloaded: downloaded_bytes,
                    total: total_bytes,
                    percentage,
                    speed_bytes_per_sec: speed,
                },
            );

            last_emit = std::time::Instant::now();
            last_bytes = downloaded_bytes;
        }
    }

    file.flush()
        .await
        .map_err(|e| format!("Failed to flush downloaded file: {e}"))?;
    drop(file);

    let meta = tokio::fs::metadata(&target_path)
        .await
        .map_err(|e| format!("Downloaded file missing: {e}"))?;
    if meta.len() == 0 {
        return Err("Downloaded update file is empty (0 bytes)".to_string());
    }

    #[cfg(target_os = "android")]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&target_path, std::fs::Permissions::from_mode(0o644));
    }

    launch_installer_and_exit(app_handle, &target_path)?;

    Ok(())
}

fn launch_installer_and_exit(
    app_handle: tauri::AppHandle,
    target_path: &std::path::Path,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let ext = target_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "exe" {
            std::process::Command::new(target_path)
                .spawn()
                .map_err(|e| format!("Failed to launch Windows installer: {e}"))?;
            app_handle.exit(0);
            Ok(())
        } else if ext == "msi" {
            std::process::Command::new("msiexec")
                .args(["/i", &target_path.to_string_lossy()])
                .spawn()
                .map_err(|e| format!("Failed to launch MSI installer: {e}"))?;
            app_handle.exit(0);
            Ok(())
        } else {
            Err(format!(
                "Unsupported Windows update asset extension: .{ext}"
            ))
        }
    }

    #[cfg(target_os = "android")]
    {
        let _ = app_handle;
        install_package_android(target_path)?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let _ = app_handle;
        std::process::Command::new("open")
            .arg(target_path)
            .spawn()
            .map_err(|e| format!("Failed to open macOS installer: {e}"))?;
        return Ok(());
    }

    #[cfg(all(target_os = "linux", not(target_os = "android")))]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(target_path, std::fs::Permissions::from_mode(0o755));

        let ext = target_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "appimage" {
            std::process::Command::new(target_path)
                .spawn()
                .map_err(|e| format!("Failed to launch AppImage: {e}"))?;
            app_handle.exit(0);
            return Ok(());
        } else {
            std::process::Command::new("xdg-open")
                .arg(target_path)
                .spawn()
                .map_err(|e| format!("Failed to open Linux package: {e}"))?;
            return Ok(());
        }
    }

    #[cfg(not(any(
        target_os = "windows",
        target_os = "android",
        target_os = "macos",
        target_os = "linux"
    )))]
    {
        let _ = app_handle;
        let _ = target_path;
        Err("Unsupported operating system for auto-install".to_string())
    }
}

#[cfg(target_os = "android")]
fn install_package_android(apk_path: &std::path::Path) -> Result<(), String> {
    let ctx = ndk_context::android_context();
    let vm_ptr = ctx.vm();
    let context_ptr = ctx.context();
    if vm_ptr.is_null() || context_ptr.is_null() {
        return Err("Android context not initialized".to_string());
    }
    let vm = unsafe { jni::JavaVM::from_raw(vm_ptr.cast()).map_err(|e| e.to_string())? };
    let mut env = vm
        .attach_current_thread_as_daemon()
        .map_err(|e| e.to_string())?;
    let context = unsafe { jni::objects::JObject::from_raw(context_ptr.cast()) };

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    let file_class = match env.find_class("java/io/File") {
        Ok(c) => c,
        Err(e) => {
            let _ = env.exception_clear();
            return Err(format!("Find File class: {e}"));
        }
    };

    let apk_path_str = match env.new_string(apk_path.to_string_lossy()) {
        Ok(s) => s,
        Err(e) => {
            let _ = env.exception_clear();
            return Err(format!("APK path string: {e}"));
        }
    };

    let file_obj = match env.new_object(
        &file_class,
        "(Ljava/lang/String;)V",
        &[jni::objects::JValue::Object(&apk_path_str)],
    ) {
        Ok(o) => o,
        Err(e) => {
            let _ = env.exception_clear();
            return Err(format!("New File: {e}"));
        }
    };

    let file_provider_class = match env.find_class("androidx/core/content/FileProvider") {
        Ok(c) => c,
        Err(e) => {
            let _ = env.exception_clear();
            return Err(format!("Find FileProvider class: {e}"));
        }
    };

    let package_name =
        if let Ok(val) = env.call_method(&context, "getPackageName", "()Ljava/lang/String;", &[]) {
            if let Ok(obj) = val.l() {
                let jstr: jni::objects::JString = obj.into();
                let parsed = env
                    .get_string(&jstr)
                    .map(|s| s.to_string_lossy().to_string());
                parsed.unwrap_or_else(|_| "app.pawstash.client".to_string())
            } else {
                "app.pawstash.client".to_string()
            }
        } else {
            let _ = env.exception_clear();
            "app.pawstash.client".to_string()
        };
    let authority_str = format!("{package_name}.fileprovider");
    let authority_jstring = match env.new_string(&authority_str) {
        Ok(s) => s,
        Err(e) => {
            let _ = env.exception_clear();
            return Err(format!("Authority string: {e}"));
        }
    };

    let uri_obj = match env.call_static_method(
        &file_provider_class,
        "getUriForFile",
        "(Landroid/content/Context;Ljava/lang/String;Ljava/io/File;)Landroid/net/Uri;",
        &[
            jni::objects::JValue::Object(&context),
            jni::objects::JValue::Object(&authority_jstring),
            jni::objects::JValue::Object(&file_obj),
        ],
    ) {
        Ok(val) => match val.l() {
            Ok(obj) => obj,
            Err(e) => {
                let _ = env.exception_clear();
                return Err(format!("Get Uri obj from FileProvider: {e}"));
            }
        },
        Err(e) => {
            let _ = env.exception_clear();
            return Err(format!("FileProvider.getUriForFile: {e}"));
        }
    };

    let intent_class = match env.find_class("android/content/Intent") {
        Ok(c) => c,
        Err(e) => {
            let _ = env.exception_clear();
            return Err(format!("Find Intent: {e}"));
        }
    };

    let action_view = match env.new_string("android.intent.action.VIEW") {
        Ok(s) => s,
        Err(e) => {
            let _ = env.exception_clear();
            return Err(e.to_string());
        }
    };

    let intent_obj = match env.new_object(
        &intent_class,
        "(Ljava/lang/String;)V",
        &[jni::objects::JValue::Object(&action_view)],
    ) {
        Ok(o) => o,
        Err(e) => {
            let _ = env.exception_clear();
            return Err(format!("New Intent: {e}"));
        }
    };

    let mime_str = match env.new_string("application/vnd.android.package-archive") {
        Ok(s) => s,
        Err(e) => {
            let _ = env.exception_clear();
            return Err(e.to_string());
        }
    };

    if let Err(e) = env.call_method(
        &intent_obj,
        "setDataAndType",
        "(Landroid/net/Uri;Ljava/lang/String;)Landroid/content/Intent;",
        &[
            jni::objects::JValue::Object(&uri_obj),
            jni::objects::JValue::Object(&mime_str),
        ],
    ) {
        let _ = env.exception_clear();
        return Err(format!("setDataAndType: {e}"));
    }

    let flags: i32 = 1 | 0x10000000;
    if let Err(e) = env.call_method(
        &intent_obj,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[jni::objects::JValue::Int(flags)],
    ) {
        let _ = env.exception_clear();
        return Err(format!("addFlags: {e}"));
    }

    if let Err(e) = env.call_method(
        &context,
        "startActivity",
        "(Landroid/content/Intent;)V",
        &[jni::objects::JValue::Object(&intent_obj)],
    ) {
        let _ = env.exception_clear();
        return Err(format!("startActivity: {e}"));
    }

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    Ok(())
}

fn find_platform_asset(assets: &[ReleaseAsset]) -> (Option<String>, Option<String>, Option<u64>) {
    #[cfg(target_os = "windows")]
    {
        // Priority 1: Setup installer
        if let Some(asset) = assets.iter().find(|a| {
            let name = a.name.to_lowercase();
            name.ends_with("-setup.exe")
                || name.ends_with("_setup.exe")
                || name == "pawstash-setup.exe"
        }) {
            return (
                Some(asset.browser_download_url.clone()),
                Some(asset.name.clone()),
                Some(asset.size),
            );
        }
        // Priority 2: MSI installer
        if let Some(asset) = assets
            .iter()
            .find(|a| a.name.to_lowercase().ends_with(".msi"))
        {
            return (
                Some(asset.browser_download_url.clone()),
                Some(asset.name.clone()),
                Some(asset.size),
            );
        }
        // Priority 3: Portable / standalone executable
        if let Some(asset) = assets
            .iter()
            .find(|a| a.name.to_lowercase().ends_with(".exe"))
        {
            return (
                Some(asset.browser_download_url.clone()),
                Some(asset.name.clone()),
                Some(asset.size),
            );
        }
    }

    #[cfg(target_os = "android")]
    {
        if let Some(asset) = assets
            .iter()
            .find(|a| a.name.to_lowercase().ends_with(".apk"))
        {
            return (
                Some(asset.browser_download_url.clone()),
                Some(asset.name.clone()),
                Some(asset.size),
            );
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(asset) = assets
            .iter()
            .find(|a| a.name.to_lowercase().ends_with(".dmg"))
        {
            return (
                Some(asset.browser_download_url.clone()),
                Some(asset.name.clone()),
                Some(asset.size),
            );
        }
        if let Some(asset) = assets.iter().find(|a| {
            a.name.to_lowercase().ends_with(".app.tar.gz")
                || a.name.to_lowercase().ends_with(".tar.gz")
        }) {
            return (
                Some(asset.browser_download_url.clone()),
                Some(asset.name.clone()),
                Some(asset.size),
            );
        }
    }

    #[cfg(all(target_os = "linux", not(target_os = "android")))]
    {
        if let Some(asset) = assets
            .iter()
            .find(|a| a.name.ends_with(".AppImage") || a.name.to_lowercase().ends_with(".appimage"))
        {
            return (
                Some(asset.browser_download_url.clone()),
                Some(asset.name.clone()),
                Some(asset.size),
            );
        }
        if let Some(asset) = assets
            .iter()
            .find(|a| a.name.to_lowercase().ends_with(".deb"))
        {
            return (
                Some(asset.browser_download_url.clone()),
                Some(asset.name.clone()),
                Some(asset.size),
            );
        }
    }

    (None, None, None)
}

fn parse_semver(tag: &str) -> (Vec<u64>, Option<String>) {
    let clean = tag.trim().trim_start_matches('v');
    let parts: Vec<&str> = clean.splitn(2, '-').collect();
    let nums: Vec<u64> = parts[0]
        .split('.')
        .filter_map(|s| s.parse::<u64>().ok())
        .collect();
    let pre = if parts.len() > 1 {
        Some(parts[1].to_string())
    } else {
        None
    };
    (nums, pre)
}

fn compare_prerelease_tokens(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<&str> = a.split('.').collect();
    let b_parts: Vec<&str> = b.split('.').collect();

    for (p1, p2) in a_parts.iter().zip(b_parts.iter()) {
        if p1 == p2 {
            continue;
        }
        if let (Ok(n1), Ok(n2)) = (p1.parse::<u64>(), p2.parse::<u64>()) {
            if n1 != n2 {
                return n1.cmp(&n2);
            }
        } else {
            let p1_prefix = p1.trim_end_matches(|c: char| c.is_ascii_digit());
            let p2_prefix = p2.trim_end_matches(|c: char| c.is_ascii_digit());
            if p1_prefix == p2_prefix {
                let n1 = p1[p1_prefix.len()..].parse::<u64>().ok();
                let n2 = p2[p2_prefix.len()..].parse::<u64>().ok();
                if let (Some(num1), Some(num2)) = (n1, n2) {
                    if num1 != num2 {
                        return num1.cmp(&num2);
                    }
                }
            }
            return p1.cmp(p2);
        }
    }
    a_parts.len().cmp(&b_parts.len())
}

fn is_version_newer(latest: &str, current: &str) -> bool {
    let (latest_nums, latest_pre) = parse_semver(latest);
    let (current_nums, current_pre) = parse_semver(current);

    let max_len = latest_nums.len().max(current_nums.len());
    for i in 0..max_len {
        let l = latest_nums.get(i).copied().unwrap_or(0);
        let c = current_nums.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }

    match (latest_pre, current_pre) {
        (None, Some(_)) => true,
        (Some(l), Some(c)) => compare_prerelease_tokens(&l, &c) == std::cmp::Ordering::Greater,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_version_newer("0.2.0", "0.1.0"));
        assert!(is_version_newer("0.1.1", "0.1.0"));
        assert!(is_version_newer("1.0.0", "0.9.9"));
        assert!(is_version_newer("0.1.0", "0.1.0-beta.1"));
        assert!(is_version_newer("26.8.1", "26.8.1-pre1"));
        assert!(is_version_newer("26.8.2", "26.8.1-pre1"));
        assert!(is_version_newer("v26.8.2", "26.8.1"));
        assert!(is_version_newer("26.8.1-pre2", "26.8.1-pre1"));
        assert!(is_version_newer("26.8.1-pre10", "26.8.1-pre2"));
        assert!(is_version_newer("26.8.1-beta.2", "26.8.1-beta.1"));
        assert!(is_version_newer("26.8.1-beta.10", "26.8.1-beta.2"));
        assert!(!is_version_newer("0.1.0", "0.1.0"));
        assert!(!is_version_newer("26.8.1-pre1", "26.8.1-pre1"));
        assert!(!is_version_newer("26.8.1-pre1", "26.8.1"));
        assert!(!is_version_newer("26.8.1-pre2", "26.8.1-pre10"));
        assert!(!is_version_newer("0.1.0", "0.2.0"));
        assert!(!is_version_newer("0.0.9", "0.1.0"));
    }

    #[test]
    fn test_platform_asset_resolution() {
        let assets = vec![
            ReleaseAsset {
                name: "Pawstash_26.8.1_x64-setup.exe".to_string(),
                browser_download_url: "https://github.com/pawstash/pawstash/releases/download/v26.8.1/Pawstash_26.8.1_x64-setup.exe".to_string(),
                size: 15_000_000,
                content_type: None,
            },
            ReleaseAsset {
                name: "pawstash_26.8.1_universal.apk".to_string(),
                browser_download_url: "https://github.com/pawstash/pawstash/releases/download/v26.8.1/pawstash_26.8.1_universal.apk".to_string(),
                size: 20_000_000,
                content_type: None,
            },
            ReleaseAsset {
                name: "Pawstash_26.8.1_universal.dmg".to_string(),
                browser_download_url: "https://github.com/pawstash/pawstash/releases/download/v26.8.1/Pawstash_26.8.1_universal.dmg".to_string(),
                size: 18_000_000,
                content_type: None,
            },
            ReleaseAsset {
                name: "pawstash_26.8.1_amd64.AppImage".to_string(),
                browser_download_url: "https://github.com/pawstash/pawstash/releases/download/v26.8.1/pawstash_26.8.1_amd64.AppImage".to_string(),
                size: 22_000_000,
                content_type: None,
            },
        ];

        let (url, name, size) = find_platform_asset(&assets);
        assert!(url.is_some());
        assert!(name.is_some());
        assert!(size.is_some());

        #[cfg(target_os = "windows")]
        assert!(name.unwrap().ends_with(".exe"));

        #[cfg(target_os = "android")]
        assert!(name.unwrap().ends_with(".apk"));

        #[cfg(target_os = "macos")]
        assert!(name.unwrap().ends_with(".dmg"));

        #[cfg(all(target_os = "linux", not(target_os = "android")))]
        assert!(name.unwrap().ends_with(".AppImage"));
    }
}
