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

    let temp_dir = std::env::temp_dir();
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
    let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
    let context = unsafe { jni::objects::JObject::from_raw(context_ptr.cast()) };

    let file_class = env
        .find_class("java/io/File")
        .map_err(|e| format!("Find File: {e}"))?;
    let apk_path_str = env
        .new_string(apk_path.to_string_lossy())
        .map_err(|e| e.to_string())?;
    let file_obj = env
        .new_object(
            &file_class,
            "(Ljava/lang/String;)V",
            &[jni::objects::JValue::Object(&apk_path_str)],
        )
        .map_err(|e| format!("New File: {e}"))?;

    let pkg_name_obj = env
        .call_method(&context, "getPackageName", "()Ljava/lang/String;", &[])
        .map_err(|e| format!("Get pkg name: {e}"))?
        .l()
        .map_err(|e| e.to_string())?;
    let pkg_name_jstr: jni::objects::JString = pkg_name_obj.into();
    let pkg_name: String = env
        .get_string(&pkg_name_jstr)
        .map_err(|e| e.to_string())?
        .into();
    let authority = format!("{}.fileprovider", pkg_name);
    let auth_str = env.new_string(authority).map_err(|e| e.to_string())?;

    let fp_class = env
        .find_class("androidx/core/content/FileProvider")
        .map_err(|e| format!("Find FileProvider: {e}"))?;
    let uri_obj = env
        .call_static_method(
            &fp_class,
            "getUriForFile",
            "(Landroid/content/Context;Ljava/lang/String;Ljava/io/File;)Landroid/net/Uri;",
            &[
                jni::objects::JValue::Object(&context),
                jni::objects::JValue::Object(&auth_str),
                jni::objects::JValue::Object(&file_obj),
            ],
        )
        .map_err(|e| format!("FileProvider.getUriForFile: {e}"))?
        .l()
        .map_err(|e| e.to_string())?;

    let intent_class = env
        .find_class("android/content/Intent")
        .map_err(|e| format!("Find Intent: {e}"))?;
    let action_view = env
        .new_string("android.intent.action.VIEW")
        .map_err(|e| e.to_string())?;
    let intent_obj = env
        .new_object(
            &intent_class,
            "(Ljava/lang/String;)V",
            &[jni::objects::JValue::Object(&action_view)],
        )
        .map_err(|e| format!("New Intent: {e}"))?;

    let mime_str = env
        .new_string("application/vnd.android.package-archive")
        .map_err(|e| e.to_string())?;
    env.call_method(
        &intent_obj,
        "setDataAndType",
        "(Landroid/net/Uri;Ljava/lang/String;)Landroid/content/Intent;",
        &[
            jni::objects::JValue::Object(&uri_obj),
            jni::objects::JValue::Object(&mime_str),
        ],
    )
    .map_err(|e| format!("setDataAndType: {e}"))?;

    let flags: i32 = 1 | 0x10000000;
    env.call_method(
        &intent_obj,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[jni::objects::JValue::Int(flags)],
    )
    .map_err(|e| format!("addFlags: {e}"))?;

    env.call_method(
        &context,
        "startActivity",
        "(Landroid/content/Intent;)V",
        &[jni::objects::JValue::Object(&intent_obj)],
    )
    .map_err(|e| format!("startActivity: {e}"))?;

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

    // Fallback: pick first available asset
    if let Some(first) = assets.first() {
        return (
            Some(first.browser_download_url.clone()),
            Some(first.name.clone()),
            Some(first.size),
        );
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
