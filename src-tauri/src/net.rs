use crate::config::settings::{AppSettings, ProxyMode};
use std::time::Duration;

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
pub const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(90);
pub const POOL_MAX_IDLE_PER_HOST: usize = 16;

pub fn builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .pool_idle_timeout(POOL_IDLE_TIMEOUT)
        .pool_max_idle_per_host(POOL_MAX_IDLE_PER_HOST)
        .tcp_keepalive(Duration::from_secs(60))
}

pub fn apply_proxy(
    mut builder: reqwest::ClientBuilder,
    settings: &AppSettings,
) -> Result<reqwest::ClientBuilder, String> {
    match settings.proxy_mode {
        ProxyMode::None => builder = builder.no_proxy(),
        ProxyMode::System => {}
        ProxyMode::Custom => {
            if settings.proxy_url.trim().is_empty() {
                return Ok(builder.no_proxy());
            }
            let mut proxy = reqwest::Proxy::all(settings.proxy_url.trim())
                .map_err(|e| format!("Invalid proxy URL: {e}"))?;
            if !settings.proxy_username.is_empty() {
                proxy = proxy.basic_auth(&settings.proxy_username, &settings.proxy_password);
            }
            if settings.proxy_bypass_local {
                proxy = proxy.no_proxy(reqwest::NoProxy::from_string("localhost,127.0.0.1,::1"));
            }
            builder = builder.proxy(proxy);
        }
    }
    Ok(builder)
}

pub fn builder_with_proxy(settings: &AppSettings) -> Result<reqwest::ClientBuilder, String> {
    apply_proxy(builder(), settings)
}

fn proxy_fingerprint(settings: &AppSettings) -> String {
    format!(
        "{:?}\u{1}{}\u{1}{}\u{1}{}\u{1}{}",
        settings.proxy_mode,
        settings.proxy_url.trim(),
        settings.proxy_username,
        settings.proxy_password,
        settings.proxy_bypass_local
    )
}

static SHARED_CLIENT: std::sync::OnceLock<std::sync::Mutex<Option<(String, reqwest::Client)>>> =
    std::sync::OnceLock::new();

pub fn shared_client(settings: &AppSettings) -> Result<reqwest::Client, String> {
    let fingerprint = proxy_fingerprint(settings);
    let cell = SHARED_CLIENT.get_or_init(|| std::sync::Mutex::new(None));

    if let Ok(guard) = cell.lock() {
        if let Some((cached, client)) = guard.as_ref() {
            if cached == &fingerprint {
                return Ok(client.clone());
            }
        }
    }

    let client = builder_with_proxy(settings)?
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|e| e.to_string())?;

    if let Ok(mut guard) = cell.lock() {
        *guard = Some((fingerprint, client.clone()));
    }
    Ok(client)
}

pub fn reset_shared_clients() {
    if let Some(cell) = SHARED_CLIENT.get() {
        if let Ok(mut guard) = cell.lock() {
            *guard = None;
        }
    }
    crate::downloader::native::NativeDownloader::reset_client_cache();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_none_disables_proxy() {
        let settings = AppSettings {
            proxy_mode: ProxyMode::None,
            ..AppSettings::default()
        };
        assert!(builder_with_proxy(&settings).is_ok());
    }

    #[test]
    fn empty_custom_proxy_falls_back_to_direct() {
        let settings = AppSettings {
            proxy_mode: ProxyMode::Custom,
            proxy_url: "   ".to_string(),
            ..AppSettings::default()
        };
        assert!(builder_with_proxy(&settings).is_ok());
    }

    #[test]
    fn malformed_custom_proxy_is_reported() {
        let settings = AppSettings {
            proxy_mode: ProxyMode::Custom,
            proxy_url: "not a url".to_string(),
            ..AppSettings::default()
        };
        assert!(builder_with_proxy(&settings).is_err());
    }

    #[test]
    fn base_builder_produces_a_client() {
        assert!(builder().build().is_ok());
    }

    #[test]
    fn shared_client_caches_by_proxy_fingerprint() {
        let settings = AppSettings {
            proxy_mode: ProxyMode::None,
            ..AppSettings::default()
        };
        shared_client(&settings).expect("client builds");

        let cached_fingerprint = SHARED_CLIENT.get().and_then(|cell| {
            cell.lock()
                .ok()
                .and_then(|g| g.as_ref().map(|(f, _)| f.clone()))
        });
        assert_eq!(
            cached_fingerprint.as_deref(),
            Some(proxy_fingerprint(&settings).as_str())
        );

        shared_client(&settings).expect("client is reused");
        let still_cached = SHARED_CLIENT.get().and_then(|cell| {
            cell.lock()
                .ok()
                .and_then(|g| g.as_ref().map(|(f, _)| f.clone()))
        });
        assert_eq!(still_cached, cached_fingerprint);
    }

    #[test]
    fn fingerprint_distinguishes_proxy_settings() {
        let direct = AppSettings {
            proxy_mode: ProxyMode::None,
            ..AppSettings::default()
        };
        let proxied = AppSettings {
            proxy_mode: ProxyMode::Custom,
            proxy_url: "http://127.0.0.1:8080".to_string(),
            ..AppSettings::default()
        };
        assert_ne!(proxy_fingerprint(&direct), proxy_fingerprint(&proxied));
    }

    #[test]
    fn fingerprint_is_stable_for_equal_settings() {
        let settings = AppSettings {
            proxy_mode: ProxyMode::Custom,
            proxy_url: "http://127.0.0.1:8080".to_string(),
            proxy_username: "user".to_string(),
            ..AppSettings::default()
        };
        assert_eq!(
            proxy_fingerprint(&settings),
            proxy_fingerprint(&settings.clone())
        );
    }
}
