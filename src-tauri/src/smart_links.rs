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

    if (host_matches(&host, "cum.st")
        || host_matches(&host, "coomer.su")
        || host_matches(&host, "coomer.party")
        || host_matches(&host, "kemono.su")
        || host_matches(&host, "kemono.party")
        || host_matches(&host, "pawchive.pw")
        || host_matches(&host, "pawchive.st"))
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

    if (host_matches(&host, "cum.st")
        || host_matches(&host, "coomer.su")
        || host_matches(&host, "coomer.party")
        || host_matches(&host, "kemono.su")
        || host_matches(&host, "kemono.party")
        || host_matches(&host, "pawchive.pw")
        || host_matches(&host, "pawchive.st"))
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
            let user = if (*first == "m" || *first == "c" || *first == "user" || *first == "creators") && segments.len() > 1 {
                segments[1]
            } else {
                *first
            };
            let ignore_routes = [
                "home", "search", "explore", "settings", "messages", "notifications",
                "login", "signup", "posts", "about", "policy", "legal", "join", "bePatron",
                "create", "dashboard", "insights", "payouts", "guidelines"
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
            let ignore_routes = ["posts", "mypage", "sessions", "search", "help", "terms", "privacy", "commissions"];
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
            let ignore_routes = ["posts", "feed", "search", "settings", "messages", "terms", "privacy", "about"];
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
        if let Some(user) = segment_after(&segments, "a").or_else(|| segment_after(&segments, "u")) {
            return Some(ExternalCreatorLink {
                service: "afdian".into(),
                creator_hint: user,
            });
        }
    }

    if host_matches(&host, "onlyfans.com") {
        if let Some(first) = segments.first() {
            let ignore_routes = ["my", "settings", "notifications", "chats", "posts", "about", "help"];
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
            let ignore_routes = ["post", "posts", "feed", "explore", "settings", "notifications", "messages"];
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
    if segments.len() < 5 || !matches!(segments[1], "user" | "server" | "channel") || segments[3] != "post" {
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
    fn parses_supported_external_creator_urls() {
        let patreon_user = parse_external_creator_link("https://www.patreon.com/dakkokujiro").unwrap();
        assert_eq!(patreon_user.service, "patreon");
        assert_eq!(patreon_user.creator_hint, "dakkokujiro");

        let patreon_id = parse_external_creator_link("https://patreon.com/user?u=58552278").unwrap();
        assert_eq!(patreon_id.service, "patreon");
        assert_eq!(patreon_id.creator_hint, "58552278");

        let fanbox_sub = parse_external_creator_link("https://dakkokujiro.fanbox.cc").unwrap();
        assert_eq!(fanbox_sub.service, "fanbox");
        assert_eq!(fanbox_sub.creator_hint, "dakkokujiro");

        let fanbox_at = parse_external_creator_link("https://fanbox.cc/@dakkokujiro").unwrap();
        assert_eq!(fanbox_at.service, "fanbox");
        assert_eq!(fanbox_at.creator_hint, "dakkokujiro");

        let pawchive_creator = parse_external_creator_link("https://pawchive.pw/fanbox/user/58552278").unwrap();
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
}
