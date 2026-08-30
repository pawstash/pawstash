use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Attachment {
    pub name: Option<String>,
    pub path: Option<String>,
    pub server: Option<String>,
    pub size: Option<u64>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    #[serde(deserialize_with = "deserialize_flexible_id")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_flexible_id")]
    pub user: String,
    pub service: String,
    #[serde(default)]
    pub title: String,
    pub content: Option<String>,
    pub substring: Option<String>,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub published: Option<String>,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub added: Option<String>,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub edited: Option<String>,
    pub embed: Option<Value>,
    pub shared_file: Option<bool>,
    pub attachments: Option<Vec<Attachment>>,
    pub file: Option<Attachment>,
    pub poll: Option<Value>,
    pub captions: Option<Value>,
    pub tags: Option<Value>,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub origin: Option<String>,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub preview_state: Option<String>,
    pub has_full: Option<bool>,
    pub detail_fetched: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub next: Option<String>,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub prev: Option<String>,
    #[serde(
        default,
        alias = "favs",
        alias = "favorites",
        alias = "fav_count",
        alias = "favorite_count"
    )]
    pub favorite_count: Option<u64>,
    #[serde(default)]
    pub attachment_count: Option<u64>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl Post {
    pub fn clean_extra(&mut self) {
        let known_keys = [
            "id",
            "user",
            "service",
            "title",
            "content",
            "substring",
            "published",
            "added",
            "edited",
            "embed",
            "shared_file",
            "attachments",
            "file",
            "poll",
            "captions",
            "tags",
            "origin",
            "preview_state",
            "has_full",
            "detail_fetched",
            "next",
            "prev",
            "favorite_count",
            "favs",
            "favorites",
            "fav_count",
            "attachment_count",
            "extra",
        ];
        for k in &known_keys {
            self.extra.remove(*k);
        }
    }

    pub fn from_json_str(json_str: &str) -> Result<Self, String> {
        match serde_json::from_str::<Post>(json_str) {
            Ok(mut post) => {
                post.clean_extra();
                Ok(post)
            }
            Err(_) => {
                let mut val: serde_json::Value =
                    serde_json::from_str(json_str).map_err(|e| e.to_string())?;
                if let Some(obj) = val.as_object_mut() {
                    let id = obj
                        .get("id")
                        .and_then(|v| {
                            if v.is_string() {
                                v.as_str().map(|s| s.to_string())
                            } else {
                                v.as_i64().map(|n| n.to_string())
                            }
                        })
                        .unwrap_or_default();
                    let user = obj
                        .get("user")
                        .and_then(|v| {
                            if v.is_string() {
                                v.as_str().map(|s| s.to_string())
                            } else {
                                v.as_i64().map(|n| n.to_string())
                            }
                        })
                        .unwrap_or_default();
                    let service = obj
                        .get("service")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let title = obj
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let content = obj
                        .get("content")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let substring = obj
                        .get("substring")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let published = obj.get("published").and_then(|v| {
                        if v.is_string() {
                            v.as_str().map(|s| s.to_string())
                        } else {
                            v.as_i64().map(|n| n.to_string())
                        }
                    });
                    let added = obj.get("added").and_then(|v| {
                        if v.is_string() {
                            v.as_str().map(|s| s.to_string())
                        } else {
                            v.as_i64().map(|n| n.to_string())
                        }
                    });
                    let edited = obj.get("edited").and_then(|v| {
                        if v.is_string() {
                            v.as_str().map(|s| s.to_string())
                        } else {
                            v.as_i64().map(|n| n.to_string())
                        }
                    });
                    let embed = obj.get("embed").cloned();
                    let shared_file = obj.get("shared_file").and_then(|v| v.as_bool());
                    let attachments = obj
                        .get("attachments")
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    let file = obj
                        .get("file")
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    let poll = obj.get("poll").cloned();
                    let captions = obj.get("captions").cloned();
                    let tags = obj.get("tags").cloned();
                    let origin = obj
                        .get("origin")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let preview_state = obj
                        .get("preview_state")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let has_full = obj.get("has_full").and_then(|v| v.as_bool());
                    let detail_fetched = obj.get("detail_fetched").and_then(|v| v.as_bool());
                    let next = obj
                        .get("next")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let prev = obj
                        .get("prev")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let favorite_count = obj
                        .get("favorite_count")
                        .or_else(|| obj.get("favs"))
                        .or_else(|| obj.get("favorites"))
                        .and_then(|v| v.as_u64());
                    let attachment_count = obj.get("attachment_count").and_then(|v| v.as_u64());

                    let known_keys = [
                        "id",
                        "user",
                        "service",
                        "title",
                        "content",
                        "substring",
                        "published",
                        "added",
                        "edited",
                        "embed",
                        "shared_file",
                        "attachments",
                        "file",
                        "poll",
                        "captions",
                        "tags",
                        "origin",
                        "preview_state",
                        "has_full",
                        "detail_fetched",
                        "next",
                        "prev",
                        "favorite_count",
                        "favs",
                        "favorites",
                        "fav_count",
                        "attachment_count",
                        "extra",
                    ];
                    for k in &known_keys {
                        obj.remove(*k);
                    }
                    let extra: HashMap<String, serde_json::Value> =
                        obj.clone().into_iter().collect();

                    Ok(Post {
                        id,
                        user,
                        service,
                        title,
                        content,
                        substring,
                        published,
                        added,
                        edited,
                        embed,
                        shared_file,
                        attachments,
                        file,
                        poll,
                        captions,
                        tags,
                        origin,
                        preview_state,
                        has_full,
                        detail_fetched,
                        next,
                        prev,
                        favorite_count,
                        attachment_count,
                        extra,
                    })
                } else {
                    Err("JSON is not an object".to_string())
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Creator {
    pub id: String,
    pub name: String,
    pub service: String,
    pub public_id: Option<Value>,
    pub relation_id: Option<Value>,
    pub indexed: Option<i64>,
    pub updated: Option<i64>,
    pub favorited: Option<u64>,
    pub kemono_favorited: Option<u64>,
    pub ever_imported: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorProfile {
    pub id: String,
    pub name: String,
    pub service: String,
    pub public_id: Option<Value>,
    pub relation_id: Option<Value>,
    pub indexed: Option<Value>,
    pub updated: Option<Value>,
    pub kemono_favorited: Option<u64>,
    pub ever_imported: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Announcement {
    pub service: String,
    pub user_id: String,
    pub hash: String,
    pub content: String,
    pub added: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fancard {
    pub id: i64,
    pub user_id: String,
    pub file_id: i64,
    pub hash: String,
    pub mtime: String,
    pub ctime: String,
    pub mime: String,
    pub ext: String,
    pub added: String,
    pub size: u64,
    pub ihash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Favorite {
    pub faved_seq: Option<i64>,
    #[serde(deserialize_with = "deserialize_flexible_id")]
    pub id: String,
    pub service: Option<String>,
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub indexed: Option<String>,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub last_imported: Option<String>,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub updated: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

fn deserialize_flexible_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct FlexibleStringVisitor;

    impl<'de> serde::de::Visitor<'de> for FlexibleStringVisitor {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string, number, boolean, map, or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(FlexibleStringVisitor)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(v.to_string()))
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if v.is_empty() {
                Ok(None)
            } else {
                Ok(Some(v.to_string()))
            }
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if v.is_empty() {
                Ok(None)
            } else {
                Ok(Some(v))
            }
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(v.to_string()))
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(v.to_string()))
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(v.to_string()))
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: serde::de::MapAccess<'de>,
        {
            let mut id_val: Option<String> = None;
            let mut url_val: Option<String> = None;
            while let Some((key, value)) = access.next_entry::<String, serde_json::Value>()? {
                if key == "id" || key == "post_id" {
                    if let Some(s) = value.as_str() {
                        id_val = Some(s.to_string());
                    } else if let Some(n) = value.as_i64() {
                        id_val = Some(n.to_string());
                    } else if let Some(n) = value.as_u64() {
                        id_val = Some(n.to_string());
                    }
                } else if key == "url" || key == "link" || key == "path" {
                    if let Some(s) = value.as_str() {
                        url_val = Some(s.to_string());
                    }
                }
            }
            Ok(id_val.or(url_val))
        }

        fn visit_seq<S>(self, mut access: S) -> Result<Self::Value, S::Error>
        where
            S: serde::de::SeqAccess<'de>,
        {
            let mut first_val: Option<String> = None;
            while let Some(val) = access.next_element::<serde_json::Value>()? {
                if first_val.is_none() {
                    if let Some(s) = val.as_str() {
                        first_val = Some(s.to_string());
                    } else if let Some(n) = val.as_i64() {
                        first_val = Some(n.to_string());
                    } else if let Some(n) = val.as_u64() {
                        first_val = Some(n.to_string());
                    }
                }
            }
            Ok(first_val)
        }
    }

    deserializer.deserialize_option(FlexibleStringVisitor)
}

fn deserialize_flexible_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct FlexibleIdVisitor;

    impl<'de> serde::de::Visitor<'de> for FlexibleIdVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string, integer, or map id")
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(String::new())
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(String::new())
        }

        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v.to_string())
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v.to_string())
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v)
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v.to_string())
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v.to_string())
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v.to_string())
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: serde::de::MapAccess<'de>,
        {
            let mut id_val: Option<String> = None;
            while let Some((key, value)) = access.next_entry::<String, serde_json::Value>()? {
                if key == "id" || key == "user" || key == "user_id" || key == "creator_id" {
                    if let Some(s) = value.as_str() {
                        id_val = Some(s.to_string());
                    } else if let Some(n) = value.as_i64() {
                        id_val = Some(n.to_string());
                    } else if let Some(n) = value.as_u64() {
                        id_val = Some(n.to_string());
                    }
                }
            }
            Ok(id_val.unwrap_or_default())
        }

        fn visit_seq<S>(self, mut access: S) -> Result<Self::Value, S::Error>
        where
            S: serde::de::SeqAccess<'de>,
        {
            let mut first_val: Option<String> = None;
            while let Some(val) = access.next_element::<serde_json::Value>()? {
                if first_val.is_none() {
                    if let Some(s) = val.as_str() {
                        first_val = Some(s.to_string());
                    } else if let Some(n) = val.as_i64() {
                        first_val = Some(n.to_string());
                    }
                }
            }
            Ok(first_val.unwrap_or_default())
        }
    }

    deserializer.deserialize_any(FlexibleIdVisitor)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSearchPost {
    pub file_id: Option<i64>,
    pub id: String,
    pub user: Option<String>,
    pub service: Option<String>,
    pub title: Option<String>,
    pub server: Option<String>,
    pub channel: Option<String>,
    pub substring: Option<String>,
    pub published: Option<String>,
    pub file: Option<Attachment>,
    pub attachments: Option<Vec<Attachment>>,
    pub embeds: Option<Vec<Value>>,
    pub mentions: Option<Vec<Value>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSearchResult {
    pub id: i64,
    pub hash: String,
    pub mtime: String,
    pub ctime: String,
    pub mime: String,
    pub ext: String,
    pub added: String,
    pub size: u64,
    pub ihash: Option<String>,
    #[serde(default)]
    pub posts: Vec<FileSearchPost>,
    #[serde(default)]
    pub discord_posts: Vec<FileSearchPost>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostRevision {
    pub revision_id: i64,
    #[serde(flatten)]
    pub post: Post,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentRevision {
    pub id: i64,
    pub content: String,
    pub added: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub parent_id: Option<String>,
    pub commenter: String,
    pub commenter_name: Option<String>,
    pub content: String,
    pub published: String,
    #[serde(default)]
    pub revisions: Vec<CommentRevision>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiActionResult {
    pub status: u16,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSession {
    pub authenticated: bool,
    pub username: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creator_list_uses_unix_timestamps() {
        let creator: Creator = serde_json::from_str(
            r#"{
                "id":"30500811","name":"BOKABA","service":"patreon",
                "indexed":1785420000,"updated":1786262400,
                "favorited":317,"ever_imported":true
            }"#,
        )
        .unwrap();
        assert_eq!(creator.updated, Some(1_786_262_400));
        assert_eq!(creator.favorited, Some(317));
    }

    #[test]
    fn compact_recent_post_is_supported() {
        let post: Post = serde_json::from_str(
            r#"{
                "id":"12408586","user":"6377826","service":"fanbox",
                "title":"post","substring":"preview","published":"2026-08-11T19:26:15",
                "file":{"name":"cover.jpeg","path":"/91/13/file.jpeg"},
                "attachments":[],"preview_state":"scraped","has_full":true,"origin":"import"
            }"#,
        )
        .unwrap();
        assert_eq!(post.content, None);
        assert_eq!(post.preview_state.as_deref(), Some("scraped"));
        assert_eq!(post.has_full, Some(true));
    }

    #[test]
    fn revision_flattens_post_fields() {
        let revision: PostRevision = serde_json::from_str(
            r#"{
                "revision_id":42,"id":"1","user":"2","service":"fanbox",
                "title":"revision","content":"body","attachments":[]
            }"#,
        )
        .unwrap();
        assert_eq!(revision.revision_id, 42);
        assert_eq!(revision.post.id, "1");
    }

    #[test]
    fn real_pawchive_revisions_deserializes() {
        let json = r#"[
            {
                "revision_id": 19576,
                "id": "149912585",
                "user": "5564244",
                "service": "patreon",
                "title": "Links to All My Animations",
                "content": "<p>Content</p>",
                "embed": {},
                "shared_file": false,
                "added": "2026-06-10T05:00:00",
                "published": "2026-02-04T11:29:36",
                "edited": "2026-06-18T09:42:05",
                "file": {},
                "attachments": [],
                "poll": null,
                "captions": null,
                "tags": "{Pack,animation,gif,mp4,nsfw}"
            }
        ]"#;
        let revs: Vec<PostRevision> = serde_json::from_str(json).unwrap();
        assert_eq!(revs.len(), 1);
        assert_eq!(revs[0].revision_id, 19576);
        assert_eq!(revs[0].post.id, "149912585");
    }

    #[test]
    fn duplicate_keys_in_json_are_safely_handled() {
        let json_with_duplicates = r#"{
            "id": "123",
            "user": "456",
            "service": "patreon",
            "title": "Title",
            "edited": "2026-08-28T22:00:00",
            "edited": "2026-08-28T22:00:00"
        }"#;
        let post = Post::from_json_str(json_with_duplicates).unwrap();
        assert_eq!(post.id, "123");
        assert_eq!(post.user, "456");
        assert_eq!(post.edited.as_deref(), Some("2026-08-28T22:00:00"));
    }

    #[test]
    fn flexible_fields_accept_maps_and_objects() {
        let json_with_maps = r#"{
            "id": 98765,
            "user": { "id": "creator_123" },
            "service": "onlyfans",
            "title": "Post with nested objects",
            "prev": { "id": "98764", "title": "Previous post" },
            "next": { "id": 98766 },
            "origin": { "url": "https://onlyfans.com/98765" },
            "preview_state": { "status": "done" },
            "published": 1756543200,
            "added": "2026-08-30T10:00:00"
        }"#;
        let post: Post = serde_json::from_str(json_with_maps).unwrap();
        assert_eq!(post.id, "98765");
        assert_eq!(post.user, "creator_123");
        assert_eq!(post.service, "onlyfans");
        assert_eq!(post.prev.as_deref(), Some("98764"));
        assert_eq!(post.next.as_deref(), Some("98766"));
        assert_eq!(post.origin.as_deref(), Some("https://onlyfans.com/98765"));
        assert_eq!(post.published.as_deref(), Some("1756543200"));
        assert_eq!(post.added.as_deref(), Some("2026-08-30T10:00:00"));
    }
}
