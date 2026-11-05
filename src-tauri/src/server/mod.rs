pub mod axum_server;
pub mod thumbnails;

pub fn absolute_local_media_url(url: String, port: u16, token: &str) -> Result<String, String> {
    if !url.starts_with('/') {
        return Ok(url);
    }
    if port == 0 {
        return Err("Local media server is not running yet; cannot fetch proxied media".into());
    }
    let separator = if url.contains('?') { '&' } else { '?' };
    Ok(format!(
        "http://127.0.0.1:{port}{url}{separator}token={token}"
    ))
}

#[cfg(test)]
mod tests {
    use super::absolute_local_media_url;

    #[test]
    fn rewrites_only_relative_media_paths() {
        assert_eq!(
            absolute_local_media_url("/cloud_stream/proxy?url=x".into(), 8080, "t").unwrap(),
            "http://127.0.0.1:8080/cloud_stream/proxy?url=x&token=t"
        );
        assert_eq!(
            absolute_local_media_url("/cloud_stream/mega".into(), 8080, "t").unwrap(),
            "http://127.0.0.1:8080/cloud_stream/mega?token=t"
        );
        assert_eq!(
            absolute_local_media_url("https://cdn.example/a.png".into(), 0, "t").unwrap(),
            "https://cdn.example/a.png"
        );
        assert!(absolute_local_media_url("/cloud_stream/mega".into(), 0, "t").is_err());
    }
}
