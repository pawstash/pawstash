use reqwest::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalPostLink {
    pub service: String,
    pub post_id: String,
    pub creator_hint: Option<String>,
}

fn host_matches(host: &str, root: &str) -> bool {
    host == root || host.ends_with(&format!(".{root}"))
}

pub fn is_known_shortener_url(url: &Url) -> bool {
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host.trim_start_matches("www.").to_ascii_lowercase();
    matches!(
        host.as_str(),
        "bit.ly"
            | "buff.ly"
            | "cutt.ly"
            | "goo.gl"
            | "is.gd"
            | "lnkd.in"
            | "ow.ly"
            | "rb.gy"
            | "rebrand.ly"
            | "shorturl.at"
            | "t.co"
            | "tiny.one"
            | "tinyurl.com"
            | "v.gd"
            | "x.gd"
    )
}

fn clean_segment(value: &str) -> Option<String> {
    let value = value.trim_matches('/').trim();
    (!value.is_empty()
        && value.len() <= 160
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        }))
    .then(|| value.to_string())
}

fn numeric_suffix(value: &str) -> Option<String> {
    value
        .rsplit('-')
        .next()
        .filter(|suffix| {
            !suffix.is_empty() && suffix.chars().all(|character| character.is_ascii_digit())
        })
        .map(str::to_string)
}

fn segment_after(segments: &[&str], marker: &str) -> Option<String> {
    let marker_index = segments.iter().position(|segment| *segment == marker)?;
    clean_segment(segments.get(marker_index + 1)?)
}

pub fn parse_external_post_link(raw: &str) -> Option<ExternalPostLink> {
    let url = Url::parse(raw).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let host = url
        .host_str()?
        .trim_start_matches("www.")
        .to_ascii_lowercase();
    let segments: Vec<&str> = url
        .path_segments()?
        .filter(|segment| !segment.is_empty())
        .collect();

    if host_matches(&host, "patreon.com") {
        let raw_id = segment_after(&segments, "posts")?;
        return Some(ExternalPostLink {
            service: "patreon".into(),
            post_id: numeric_suffix(&raw_id)?,
            creator_hint: None,
        });
    }

    if host_matches(&host, "fanbox.cc") {
        return Some(ExternalPostLink {
            service: "fanbox".into(),
            post_id: segment_after(&segments, "posts")?,
            creator_hint: host.strip_suffix(".fanbox.cc").and_then(clean_segment),
        });
    }

    if host_matches(&host, "fantia.jp") {
        return Some(ExternalPostLink {
            service: "fantia".into(),
            post_id: segment_after(&segments, "posts")?,
            creator_hint: None,
        });
    }

    if host_matches(&host, "subscribestar.com") || host_matches(&host, "subscribestar.adult") {
        return Some(ExternalPostLink {
            service: "subscribestar".into(),
            post_id: segment_after(&segments, "posts")?,
            creator_hint: segments.first().and_then(|segment| clean_segment(segment)),
        });
    }

    if host_matches(&host, "boosty.to") {
        return Some(ExternalPostLink {
            service: "boosty".into(),
            post_id: segment_after(&segments, "posts")?,
            creator_hint: segments.first().and_then(|segment| clean_segment(segment)),
        });
    }

    if host_matches(&host, "afdian.com") || host_matches(&host, "afdian.net") {
        return Some(ExternalPostLink {
            service: "afdian".into(),
            post_id: segment_after(&segments, "p")?,
            creator_hint: None,
        });
    }

    None
}

pub fn parse_pawchive_post_url(
    url: &Url,
    expected_service: &str,
    expected_post_id: &str,
) -> Option<(String, String, String)> {
    let segments: Vec<&str> = url
        .path_segments()?
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.len() < 5 || segments[1] != "user" || segments[3] != "post" {
        return None;
    }
    let service = clean_segment(segments[0])?;
    let creator_id = clean_segment(segments[2])?;
    let post_id = clean_segment(segments[4])?;
    (service.eq_ignore_ascii_case(expected_service) && post_id == expected_post_id)
        .then_some((service, creator_id, post_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_external_post_urls() {
        let patreon = parse_external_post_link(
            "https://www.patreon.com/posts/my-update-130382350?utm_source=x",
        )
        .unwrap();
        assert_eq!(patreon.service, "patreon");
        assert_eq!(patreon.post_id, "130382350");

        let fanbox =
            parse_external_post_link("https://artist-name.fanbox.cc/posts/12035562").unwrap();
        assert_eq!(fanbox.service, "fanbox");
        assert_eq!(fanbox.post_id, "12035562");
        assert_eq!(fanbox.creator_hint.as_deref(), Some("artist-name"));

        let boosty = parse_external_post_link("https://boosty.to/creator/posts/abc_123").unwrap();
        assert_eq!(boosty.creator_hint.as_deref(), Some("creator"));
        assert_eq!(boosty.post_id, "abc_123");
    }

    #[test]
    fn rejects_non_post_and_unsafe_urls() {
        assert!(parse_external_post_link("javascript:alert(1)").is_none());
        assert!(parse_external_post_link("https://patreon.com/some-creator").is_none());
        assert!(parse_external_post_link("https://example.com/posts/123").is_none());
    }

    #[test]
    fn recognizes_only_explicit_shortener_hosts() {
        assert!(is_known_shortener_url(
            &Url::parse("https://bit.ly/example").unwrap()
        ));
        assert!(is_known_shortener_url(
            &Url::parse("https://t.co/example").unwrap()
        ));
        assert!(is_known_shortener_url(
            &Url::parse("https://x.gd/QESov").unwrap()
        ));
        assert!(!is_known_shortener_url(
            &Url::parse("https://bit.ly.attacker.test/example").unwrap()
        ));
        assert!(!is_known_shortener_url(
            &Url::parse("http://127.0.0.1/example").unwrap()
        ));
    }

    #[test]
    fn parses_full_pawchive_route() {
        let url = Url::parse("https://pawchive.st/patreon/user/981501/post/130382350").unwrap();
        assert_eq!(
            parse_pawchive_post_url(&url, "patreon", "130382350"),
            Some(("patreon".into(), "981501".into(), "130382350".into()))
        );
    }
}
