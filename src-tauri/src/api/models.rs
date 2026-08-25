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
pub struct PawchivePost {
    pub id: String,
    pub user: String,
    pub service: String,
    #[serde(default)]
    pub title: String,
    pub content: Option<String>,
    pub substring: Option<String>,
    pub published: Option<String>,
    pub added: Option<String>,
    pub edited: Option<String>,
    pub embed: Option<Value>,
    pub shared_file: Option<bool>,
    pub attachments: Option<Vec<Attachment>>,
    pub file: Option<Attachment>,
    pub poll: Option<Value>,
    pub captions: Option<Value>,
    pub tags: Option<Value>,
    pub origin: Option<String>,
    pub preview_state: Option<String>,
    pub has_full: Option<bool>,
    pub detail_fetched: Option<bool>,
    pub next: Option<String>,
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
            formatter.write_str("a string, integer, float, or null")
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

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(v.to_string()))
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(v))
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
            formatter.write_str("a string or integer id")
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
    pub post: PawchivePost,
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
        let post: PawchivePost = serde_json::from_str(
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
}
