#[cfg(target_os = "android")]
use crate::commands::with_android_context;

pub fn update_download_notification(
    active_count: i32,
    total_count: i32,
    downloaded_bytes: u64,
    total_bytes: u64,
    speed_bytes_per_sec: u64,
    current_filename: &str,
) {
    #[cfg(target_os = "android")]
    {
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
        let _ = (
            active_count,
            total_count,
            downloaded_bytes,
            total_bytes,
            speed_bytes_per_sec,
            current_filename,
        );
    }
}

pub fn notify_download_completed(
    service: &str,
    creator_id: &str,
    post_id: &str,
    filename: &str,
    title: &str,
    media_count: i32,
) {
    #[cfg(target_os = "android")]
    {
        let service_str = service.to_string();
        let creator_str = creator_id.to_string();
        let post_str = post_id.to_string();
        let filename_str = filename.to_string();
        let title_str = title.to_string();

        let _ = with_android_context(|env, context| {
            let service_jstr = env.new_string(&service_str).map_err(|e| e.to_string())?;
            let creator_jstr = env.new_string(&creator_str).map_err(|e| e.to_string())?;
            let post_jstr = env.new_string(&post_str).map_err(|e| e.to_string())?;
            let filename_jstr = env.new_string(&filename_str).map_err(|e| e.to_string())?;
            let title_jstr = env.new_string(&title_str).map_err(|e| e.to_string())?;
            let class = env.get_object_class(context).map_err(|e| e.to_string())?;

            env.call_static_method(
                &class,
                "notifyDownloadCompleted",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;I)V",
                &[
                    jni::objects::JValue::Object(&service_jstr),
                    jni::objects::JValue::Object(&creator_jstr),
                    jni::objects::JValue::Object(&post_jstr),
                    jni::objects::JValue::Object(&filename_jstr),
                    jni::objects::JValue::Object(&title_jstr),
                    jni::objects::JValue::Int(media_count),
                ],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        });
    }

    #[cfg(not(target_os = "android"))]
    {
        let _ = (service, creator_id, post_id, filename, title, media_count);
    }
}

pub fn stop_download_service() {
    #[cfg(target_os = "android")]
    {
        let _ = with_android_context(|env, context| {
            let class = env.get_object_class(context).map_err(|e| e.to_string())?;
            env.call_static_method(&class, "stopDownloadNotification", "()V", &[])
                .map_err(|e| e.to_string())?;
            Ok(())
        });
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
