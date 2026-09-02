use reqwest::Url;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "payload")]
pub enum DeepLinkTarget {
    #[serde(rename = "post")]
    Post {
        provider_id: String,
        service: String,
        creator_id: String,
        post_id: String,
    },
    #[serde(rename = "creator")]
    Creator {
        provider_id: String,
        service: String,
        creator_id: String,
    },
    #[serde(rename = "search")]
    Search {
        provider_id: Option<String>,
        query: String,
    },
}

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

    if (host_matches(&host, "cum.st") || host.contains("pawchive"))
        && segments.len() >= 5
        && matches!(segments[1], "user" | "server" | "channel")
        && segments[3] == "post"
    {
        let service = clean_segment(segments[0])?;
        let creator_id = clean_segment(segments[2])?;
        let post_id = clean_segment(segments[4])?;
        return Some(ExternalPostLink {
            service,
            post_id,
            creator_hint: Some(creator_id),
        });
    }

    if host_matches(&host, "cum.st")
        && segments.len() >= 5
        && segments[0] == "creators"
        && segments[3] == "post"
    {
        let service = clean_segment(segments[1])?;
        let creator_id = clean_segment(segments[2])?;
        let post_id = clean_segment(segments[4])?;
        return Some(ExternalPostLink {
            service,
            post_id,
            creator_hint: Some(creator_id),
        });
    }

    if (host_matches(&host, "discord.com") || host_matches(&host, "discordapp.com"))
        && segments.len() >= 4
        && segments[0] == "channels"
    {
        let server_id = clean_segment(segments[1]);
        let post_id = clean_segment(segments[3])?;
        return Some(ExternalPostLink {
            service: "discord".into(),
            post_id,
            creator_hint: server_id,
        });
    }

    if host_matches(&host, "onlyfans.com") {
        let post_id = segment_after(&segments, "posts")
            .or_else(|| segments.first().and_then(|s| numeric_suffix(s)))?;
        return Some(ExternalPostLink {
            service: "onlyfans".into(),
            post_id,
            creator_hint: None,
        });
    }

    if host_matches(&host, "fansly.com") {
        let post_id =
            segment_after(&segments, "post").or_else(|| segment_after(&segments, "posts"))?;
        return Some(ExternalPostLink {
            service: "fansly".into(),
            post_id,
            creator_hint: None,
        });
    }

    if host_matches(&host, "candfans.jp") {
        let post_id =
            segment_after(&segments, "posts").or_else(|| segment_after(&segments, "post"))?;
        return Some(ExternalPostLink {
            service: "candfans".into(),
            post_id,
            creator_hint: segments.first().and_then(|s| clean_segment(s)),
        });
    }

    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalCreatorLink {
    pub service: String,
    pub creator_hint: String,
}

pub fn parse_external_creator_link(raw: &str) -> Option<ExternalCreatorLink> {
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

    if (host_matches(&host, "cum.st") || host.contains("pawchive"))
        && segments.len() >= 3
        && matches!(segments[1], "user" | "server" | "channel")
    {
        let service = clean_segment(segments[0])?;
        let creator_id = clean_segment(segments[2])?;
        return Some(ExternalCreatorLink {
            service,
            creator_hint: creator_id,
        });
    }

    if host_matches(&host, "cum.st") && segments.len() >= 3 && segments[0] == "creators" {
        let service = clean_segment(segments[1])?;
        let creator_id = clean_segment(segments[2])?;
        return Some(ExternalCreatorLink {
            service,
            creator_hint: creator_id,
        });
    }

    if host_matches(&host, "patreon.com") {
        if let Some((_, u)) = url.query_pairs().find(|(k, _)| k == "u") {
            if let Some(creator_id) = clean_segment(&u) {
                return Some(ExternalCreatorLink {
                    service: "patreon".into(),
                    creator_hint: creator_id,
                });
            }
        }
        if let Some(first) = segments.first() {
            let user =
                if (*first == "m" || *first == "c" || *first == "user" || *first == "creators")
                    && segments.len() > 1
                {
                    segments[1]
                } else {
                    *first
                };
            let ignore_routes = [
                "home",
                "search",
                "explore",
                "settings",
                "messages",
                "notifications",
                "login",
                "signup",
                "posts",
                "about",
                "policy",
                "legal",
                "join",
                "bePatron",
                "create",
                "dashboard",
                "insights",
                "payouts",
                "guidelines",
            ];
            if !ignore_routes.contains(&user) {
                if let Some(creator_hint) = clean_segment(user) {
                    return Some(ExternalCreatorLink {
                        service: "patreon".into(),
                        creator_hint,
                    });
                }
            }
        }
    }

    if host_matches(&host, "fanbox.cc") {
        if host != "fanbox.cc" && host != "www.fanbox.cc" {
            if let Some(sub) = host.strip_suffix(".fanbox.cc") {
                if sub != "www" && sub != "api" && sub != "official" {
                    if let Some(creator_hint) = clean_segment(sub) {
                        return Some(ExternalCreatorLink {
                            service: "fanbox".into(),
                            creator_hint,
                        });
                    }
                }
            }
        }
        if let Some(first) = segments.first() {
            if let Some(at_user) = first.strip_prefix('@') {
                if let Some(creator_hint) = clean_segment(at_user) {
                    return Some(ExternalCreatorLink {
                        service: "fanbox".into(),
                        creator_hint,
                    });
                }
            } else if *first == "creator" && segments.len() > 1 {
                if let Some(creator_hint) = clean_segment(segments[1]) {
                    return Some(ExternalCreatorLink {
                        service: "fanbox".into(),
                        creator_hint,
                    });
                }
            }
        }
    }

    if host_matches(&host, "fantia.jp") {
        if let Some(id) = segment_after(&segments, "fanclubs") {
            return Some(ExternalCreatorLink {
                service: "fantia".into(),
                creator_hint: id,
            });
        }
        if let Some(first) = segments.first() {
            let ignore_routes = [
                "posts",
                "mypage",
                "sessions",
                "search",
                "help",
                "terms",
                "privacy",
                "commissions",
            ];
            if !ignore_routes.contains(first) {
                if let Some(creator_hint) = clean_segment(first) {
                    return Some(ExternalCreatorLink {
                        service: "fantia".into(),
                        creator_hint,
                    });
                }
            }
        }
    }

    if host_matches(&host, "boosty.to") {
        if let Some(first) = segments.first() {
            let ignore_routes = ["app", "search", "settings", "messages", "feed", "posts"];
            if !ignore_routes.contains(first) {
                if let Some(creator_hint) = clean_segment(first) {
                    return Some(ExternalCreatorLink {
                        service: "boosty".into(),
                        creator_hint,
                    });
                }
            }
        }
    }

    if host_matches(&host, "subscribestar.com") || host_matches(&host, "subscribestar.adult") {
        if let Some(first) = segments.first() {
            let ignore_routes = [
                "posts", "feed", "search", "settings", "messages", "terms", "privacy", "about",
            ];
            if !ignore_routes.contains(first) {
                if let Some(creator_hint) = clean_segment(first) {
                    return Some(ExternalCreatorLink {
                        service: "subscribestar".into(),
                        creator_hint,
                    });
                }
            }
        }
    }

    if host_matches(&host, "afdian.com") || host_matches(&host, "afdian.net") {
        if let Some(user) = segment_after(&segments, "a").or_else(|| segment_after(&segments, "u"))
        {
            return Some(ExternalCreatorLink {
                service: "afdian".into(),
                creator_hint: user,
            });
        }
    }

    if host_matches(&host, "onlyfans.com") {
        if let Some(first) = segments.first() {
            let ignore_routes = [
                "my",
                "settings",
                "notifications",
                "chats",
                "posts",
                "about",
                "help",
            ];
            if !ignore_routes.contains(first) {
                if let Some(creator_hint) = clean_segment(first) {
                    return Some(ExternalCreatorLink {
                        service: "onlyfans".into(),
                        creator_hint,
                    });
                }
            }
        }
    }

    if host_matches(&host, "fansly.com") {
        if let Some(first) = segments.first() {
            let ignore_routes = [
                "post",
                "posts",
                "feed",
                "explore",
                "settings",
                "notifications",
                "messages",
            ];
            if !ignore_routes.contains(first) {
                if let Some(creator_hint) = clean_segment(first) {
                    return Some(ExternalCreatorLink {
                        service: "fansly".into(),
                        creator_hint,
                    });
                }
            }
        }
    }

    if host_matches(&host, "candfans.jp") {
        if let Some(id) = segment_after(&segments, "user") {
            return Some(ExternalCreatorLink {
                service: "candfans".into(),
                creator_hint: id,
            });
        }
        if let Some(first) = segments.first() {
            let ignore_routes = ["posts", "search", "settings", "login", "register"];
            if !ignore_routes.contains(first) {
                if let Some(creator_hint) = clean_segment(first) {
                    return Some(ExternalCreatorLink {
                        service: "candfans".into(),
                        creator_hint,
                    });
                }
            }
        }
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
    if segments.len() < 5
        || !matches!(segments[1], "user" | "server" | "channel")
        || segments[3] != "post"
    {
        return None;
    }
    let service = clean_segment(segments[0])?;
    let creator_id = clean_segment(segments[2])?;
    let post_id = clean_segment(segments[4])?;
    (service.eq_ignore_ascii_case(expected_service) && post_id == expected_post_id)
        .then_some((service, creator_id, post_id))
}

fn extract_domain_root(url_or_host: &str) -> Option<String> {
    let host = if let Ok(parsed) = Url::parse(url_or_host) {
        parsed.host_str()?.to_string()
    } else {
        url_or_host
            .split('/')
            .next()?
            .split(':')
            .next()?
            .to_string()
    };
    let clean = host
        .trim_start_matches("www.")
        .trim_start_matches("api.")
        .to_ascii_lowercase();
    let parts: Vec<&str> = clean.split('.').collect();
    if parts.len() > 2 {
        Some(parts[parts.len() - 2..].join("."))
    } else {
        Some(clean)
    }
}

fn matches_provider_domain(
    host: &str,
    provider: &crate::api::providers::traits::ProviderConfig,
) -> bool {
    let target_clean = host.trim_start_matches("www.").to_ascii_lowercase();
    let candidate_urls = std::iter::once(&provider.api_url).chain(provider.fallback_urls.iter());

    for cand in candidate_urls {
        if let Some(cand_root) = extract_domain_root(cand) {
            if target_clean == cand_root || target_clean.ends_with(&format!(".{cand_root}")) {
                return true;
            }
        }
    }
    false
}

fn find_provider_for_service(
    service: &str,
    configured_providers: &[crate::api::providers::traits::ProviderConfig],
) -> String {
    let srv_clean = service.trim().to_ascii_lowercase();
    configured_providers
        .iter()
        .find(|p| {
            p.enabled
                && p.services
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(&srv_clean))
        })
        .or_else(|| {
            configured_providers.iter().find(|p| {
                p.services
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(&srv_clean))
            })
        })
        .map(|p| p.id.clone())
        .unwrap_or_else(|| {
            if srv_clean == "onlyfans" || srv_clean == "fansly" {
                "onlyhaven".to_string()
            } else {
                "pawchive".to_string()
            }
        })
}

pub fn parse_deep_link(
    raw: &str,
    configured_providers: &[crate::api::providers::traits::ProviderConfig],
) -> Result<DeepLinkTarget, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("Empty deep link URL".to_string());
    }

    let url = Url::parse(raw).map_err(|e| format!("Invalid deep link URL: {e}"))?;

    // 1. Custom Scheme: pawstash://
    if url.scheme().eq_ignore_ascii_case("pawstash") {
        let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
        let segments: Vec<&str> = url
            .path_segments()
            .map(|s| s.filter(|p| !p.is_empty()).collect())
            .unwrap_or_default();

        // 1.1 pawstash://open?url=...
        if host == "open" {
            let inner_url = url
                .query_pairs()
                .find(|(k, _)| k == "url")
                .map(|(_, v)| v.into_owned())
                .ok_or_else(|| "Missing 'url' parameter in pawstash://open".to_string())?;
            return parse_deep_link(&inner_url, configured_providers);
        }

        // 1.2 pawstash://search?q=...
        if host == "search" {
            let query = url
                .query_pairs()
                .find(|(k, _)| k == "q" || k == "query")
                .map(|(_, v)| v.into_owned())
                .unwrap_or_default();
            let provider_id = url
                .query_pairs()
                .find(|(k, _)| k == "provider" || k == "provider_id")
                .map(|(_, v)| v.into_owned());
            return Ok(DeepLinkTarget::Search { provider_id, query });
        }

        // 1.3 pawstash://post/{service}/{creator_id}/{post_id}
        if host == "post" || host == "posts" {
            if segments.len() >= 3 {
                let service = segments[0].to_string();
                let creator_id = segments[1].to_string();
                let post_id = segments[2].to_string();
                let provider_id = url
                    .query_pairs()
                    .find(|(k, _)| k == "provider" || k == "provider_id")
                    .map(|(_, v)| v.into_owned())
                    .unwrap_or_else(|| find_provider_for_service(&service, configured_providers));
                return Ok(DeepLinkTarget::Post {
                    provider_id,
                    service,
                    creator_id,
                    post_id,
                });
            } else if segments.len() == 2 {
                let service = segments[0].to_string();
                let post_id = segments[1].to_string();
                let provider_id = url
                    .query_pairs()
                    .find(|(k, _)| k == "provider" || k == "provider_id")
                    .map(|(_, v)| v.into_owned())
                    .unwrap_or_else(|| find_provider_for_service(&service, configured_providers));
                return Ok(DeepLinkTarget::Post {
                    provider_id,
                    service,
                    creator_id: String::new(),
                    post_id,
                });
            }
        }

        // 1.4 pawstash://creator/{service}/{creator_id}
        if (host == "creator" || host == "creators" || host == "user") && segments.len() >= 2 {
            let service = segments[0].to_string();
            let creator_id = segments[1].to_string();
            let provider_id = url
                .query_pairs()
                .find(|(k, _)| k == "provider" || k == "provider_id")
                .map(|(_, v)| v.into_owned())
                .unwrap_or_else(|| find_provider_for_service(&service, configured_providers));
            return Ok(DeepLinkTarget::Creator {
                provider_id,
                service,
                creator_id,
            });
        }

        // 1.5 pawstash://{provider_id}/post/{service}/{creator_id}/{post_id} or pawstash://{provider_id}/{service}/user/{creator_id}/post/{post_id}
        if let Some(prov) = configured_providers
            .iter()
            .find(|p| p.id.eq_ignore_ascii_case(&host))
        {
            if segments.len() >= 4 && (segments[0] == "post" || segments[0] == "posts") {
                return Ok(DeepLinkTarget::Post {
                    provider_id: prov.id.clone(),
                    service: segments[1].to_string(),
                    creator_id: segments[2].to_string(),
                    post_id: segments[3].to_string(),
                });
            }
            if segments.len() >= 5 && segments[1] == "user" && segments[3] == "post" {
                return Ok(DeepLinkTarget::Post {
                    provider_id: prov.id.clone(),
                    service: segments[0].to_string(),
                    creator_id: segments[2].to_string(),
                    post_id: segments[4].to_string(),
                });
            }
            if segments.len() >= 3 && segments[1] == "user" {
                return Ok(DeepLinkTarget::Creator {
                    provider_id: prov.id.clone(),
                    service: segments[0].to_string(),
                    creator_id: segments[2].to_string(),
                });
            }
        }

        return Err(format!("Unrecognized pawstash scheme format: {raw}"));
    }

    // 2. Direct Web URLs (HTTP / HTTPS)
    if matches!(url.scheme(), "http" | "https") {
        let host = url
            .host_str()
            .ok_or_else(|| "URL has no host".to_string())?
            .to_ascii_lowercase();
        let segments: Vec<&str> = url
            .path_segments()
            .map(|s| s.filter(|p| !p.is_empty()).collect())
            .unwrap_or_default();

        // 2.1 Check if host matches any configured provider's dynamic domain / mirror
        if let Some(prov) = configured_providers
            .iter()
            .find(|p| matches_provider_domain(&host, p))
        {
            // Case A: /{service}/user/{creator_id}/post/{post_id} or /{service}/server/{creator_id}/post/{post_id}
            if segments.len() >= 5
                && matches!(segments[1], "user" | "server" | "channel")
                && segments[3] == "post"
            {
                return Ok(DeepLinkTarget::Post {
                    provider_id: prov.id.clone(),
                    service: segments[0].to_string(),
                    creator_id: segments[2].to_string(),
                    post_id: segments[4].to_string(),
                });
            }

            // Case B: /posts/{service}/{creator_id}/{post_id} (OnlyHaven / generic posts)
            if segments.len() >= 4 && (segments[0] == "posts" || segments[0] == "post") {
                return Ok(DeepLinkTarget::Post {
                    provider_id: prov.id.clone(),
                    service: segments[1].to_string(),
                    creator_id: segments[2].to_string(),
                    post_id: segments[3].to_string(),
                });
            }

            // Case C: /{service}/user/{creator_id} or /creators/{service}/{creator_id}
            if segments.len() >= 3 && (segments[1] == "user" || segments[0] == "creators") {
                let service = if segments[0] == "creators" {
                    segments[1]
                } else {
                    segments[0]
                };
                let creator_id = segments[2];
                return Ok(DeepLinkTarget::Creator {
                    provider_id: prov.id.clone(),
                    service: service.to_string(),
                    creator_id: creator_id.to_string(),
                });
            }

            // Case D: /posts?q=... or /search?q=...
            if (segments.is_empty() || segments[0] == "posts" || segments[0] == "search")
                && (url.query_pairs().any(|(k, _)| k == "q" || k == "query"))
            {
                let query = url
                    .query_pairs()
                    .find(|(k, _)| k == "q" || k == "query")
                    .map(|(_, v)| v.into_owned())
                    .unwrap_or_default();
                return Ok(DeepLinkTarget::Search {
                    provider_id: Some(prov.id.clone()),
                    query,
                });
            }
        }

        // 2.2 Check external creator / post links (Patreon, Fanbox, Fantia, Boosty, OnlyFans, Fansly, etc.)
        if let Some(ext_post) = parse_external_post_link(raw) {
            let provider_id = find_provider_for_service(&ext_post.service, configured_providers);
            return Ok(DeepLinkTarget::Post {
                provider_id,
                service: ext_post.service,
                creator_id: ext_post.creator_hint.unwrap_or_default(),
                post_id: ext_post.post_id,
            });
        }

        if let Some(ext_creator) = parse_external_creator_link(raw) {
            let provider_id = find_provider_for_service(&ext_creator.service, configured_providers);
            return Ok(DeepLinkTarget::Creator {
                provider_id,
                service: ext_creator.service,
                creator_id: ext_creator.creator_hint,
            });
        }
    }

    Err(format!("Unsupported link format: {raw}"))
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
    fn parses_supported_external_creator_urls() {
        let patreon_user =
            parse_external_creator_link("https://www.patreon.com/dakkokujiro").unwrap();
        assert_eq!(patreon_user.service, "patreon");
        assert_eq!(patreon_user.creator_hint, "dakkokujiro");

        let patreon_id =
            parse_external_creator_link("https://patreon.com/user?u=58552278").unwrap();
        assert_eq!(patreon_id.service, "patreon");
        assert_eq!(patreon_id.creator_hint, "58552278");

        let fanbox_sub = parse_external_creator_link("https://dakkokujiro.fanbox.cc").unwrap();
        assert_eq!(fanbox_sub.service, "fanbox");
        assert_eq!(fanbox_sub.creator_hint, "dakkokujiro");

        let fanbox_at = parse_external_creator_link("https://fanbox.cc/@dakkokujiro").unwrap();
        assert_eq!(fanbox_at.service, "fanbox");
        assert_eq!(fanbox_at.creator_hint, "dakkokujiro");

        let pawchive_creator =
            parse_external_creator_link("https://pawchive.pw/fanbox/user/58552278").unwrap();
        assert_eq!(pawchive_creator.service, "fanbox");
        assert_eq!(pawchive_creator.creator_hint, "58552278");
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

    #[test]
    fn test_parse_deep_link_custom_scheme() {
        let providers = vec![
            crate::api::providers::traits::ProviderConfig {
                id: "pawchive".into(),
                name: "Pawchive".into(),
                enabled: true,
                api_url: "https://pawchive.pw".into(),
                fallback_urls: vec![],
                file_url: None,
                image_url: None,
                file_prefix: None,
                image_prefix: None,
                session_cookie: "".into(),
                username: "".into(),
                services: vec!["patreon".into(), "fanbox".into()],
                is_custom: false,
                priority: 0,
            },
            crate::api::providers::traits::ProviderConfig {
                id: "onlyhaven".into(),
                name: "OnlyHaven".into(),
                enabled: true,
                api_url: "https://cum.st".into(),
                fallback_urls: vec![],
                file_url: None,
                image_url: None,
                file_prefix: None,
                image_prefix: None,
                session_cookie: "".into(),
                username: "".into(),
                services: vec!["onlyfans".into(), "fansly".into()],
                is_custom: false,
                priority: 1,
            },
        ];

        // 1. Direct scheme post
        let res =
            parse_deep_link("pawstash://post/patreon/12516244/168022069", &providers).unwrap();
        assert_eq!(
            res,
            DeepLinkTarget::Post {
                provider_id: "pawchive".into(),
                service: "patreon".into(),
                creator_id: "12516244".into(),
                post_id: "168022069".into()
            }
        );

        // 2. Direct scheme creator
        let res = parse_deep_link("pawstash://creator/fanbox/51803217", &providers).unwrap();
        assert_eq!(
            res,
            DeepLinkTarget::Creator {
                provider_id: "pawchive".into(),
                service: "fanbox".into(),
                creator_id: "51803217".into()
            }
        );

        // 3. Direct scheme search
        let res = parse_deep_link("pawstash://search?q=art", &providers).unwrap();
        assert_eq!(
            res,
            DeepLinkTarget::Search {
                provider_id: None,
                query: "art".into()
            }
        );

        // 4. pawstash://open?url=...
        let res = parse_deep_link(
            "pawstash://open?url=https%3A%2F%2Fpawchive.pw%2Ffanbox%2Fuser%2F51803217%2Fpost%2F12531297",
            &providers,
        )
        .unwrap();
        assert_eq!(
            res,
            DeepLinkTarget::Post {
                provider_id: "pawchive".into(),
                service: "fanbox".into(),
                creator_id: "51803217".into(),
                post_id: "12531297".into()
            }
        );
    }

    #[test]
    fn test_parse_deep_link_dynamic_provider_domains() {
        let providers = vec![
            crate::api::providers::traits::ProviderConfig {
                id: "custom_pawchive".into(),
                name: "Custom Pawchive Mirror".into(),
                enabled: true,
                api_url: "https://mirror.pw".into(),
                fallback_urls: vec!["https://backup-mirror.net".into()],
                file_url: None,
                image_url: None,
                file_prefix: None,
                image_prefix: None,
                session_cookie: "".into(),
                username: "".into(),
                services: vec!["patreon".into(), "fanbox".into()],
                is_custom: true,
                priority: 0,
            },
            crate::api::providers::traits::ProviderConfig {
                id: "onlyhaven".into(),
                name: "OnlyHaven".into(),
                enabled: true,
                api_url: "https://cum.st".into(),
                fallback_urls: vec![],
                file_url: None,
                image_url: None,
                file_prefix: None,
                image_prefix: None,
                session_cookie: "".into(),
                username: "".into(),
                services: vec!["onlyfans".into(), "fansly".into()],
                is_custom: false,
                priority: 1,
            },
        ];

        // Matched against custom user-configured mirror
        let res = parse_deep_link(
            "https://backup-mirror.net/patreon/user/123/post/456",
            &providers,
        )
        .unwrap();
        assert_eq!(
            res,
            DeepLinkTarget::Post {
                provider_id: "custom_pawchive".into(),
                service: "patreon".into(),
                creator_id: "123".into(),
                post_id: "456".into()
            }
        );

        // Matched against OnlyHaven web post link
        let res = parse_deep_link(
            "https://cum.st/posts/onlyfans/14644822/2702157105",
            &providers,
        )
        .unwrap();
        assert_eq!(
            res,
            DeepLinkTarget::Post {
                provider_id: "onlyhaven".into(),
                service: "onlyfans".into(),
                creator_id: "14644822".into(),
                post_id: "2702157105".into()
            }
        );

        // Matched against OnlyHaven creator link
        let res = parse_deep_link("https://cum.st/creators/onlyfans/14644822", &providers).unwrap();
        assert_eq!(
            res,
            DeepLinkTarget::Creator {
                provider_id: "onlyhaven".into(),
                service: "onlyfans".into(),
                creator_id: "14644822".into()
            }
        );

        // Matched against external Patreon post link
        let res = parse_deep_link(
            "https://www.patreon.com/posts/my-post-130382350",
            &providers,
        )
        .unwrap();
        assert_eq!(
            res,
            DeepLinkTarget::Post {
                provider_id: "custom_pawchive".into(),
                service: "patreon".into(),
                creator_id: "".into(),
                post_id: "130382350".into()
            }
        );
    }
}
