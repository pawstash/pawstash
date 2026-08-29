use crate::api::models::{Creator, CreatorProfile, Favorite, Post, PostRevision};
use crate::db::storage::{content_cache_path, open_database};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use rusqlite::{params, Connection, OptionalExtension};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub total_bytes: u64,
    pub metadata_bytes: u64,
    pub protected_bytes: u64,
    pub reclaimable_bytes: u64,
    pub preview_bytes: u64,
    pub avatar_bytes: u64,
    pub banner_bytes: u64,
    pub other_bytes: u64,
    pub file_count: u64,
}

struct CacheFile {
    path: PathBuf,
    size: u64,
    modified_at: u64,
    protected: bool,
}

pub struct ContentRepository {
    connection: Mutex<Connection>,
    cache_limit_bytes: AtomicU64,
}

impl ContentRepository {
    pub fn new(cache_limit_mb: u64) -> Result<Self, String> {
        Ok(Self {
            connection: Mutex::new(open_database()?),
            cache_limit_bytes: AtomicU64::new(cache_limit_mb.saturating_mul(1024 * 1024)),
        })
    }

    pub fn cache_stats(&self) -> Result<CacheStats, String> {
        let protected = self.protected_cache_paths()?;
        let files = scan_cache_files(&content_cache_path(), &protected)?;
        let mut stats = cache_stats_from_files(&files);
        stats.metadata_bytes = self.cached_metadata_bytes()?;
        Ok(stats)
    }

    pub fn set_cache_limit_mb(&self, max_mb: u64) -> Result<CacheStats, String> {
        let limit = max_mb.clamp(64, 2048).saturating_mul(1024 * 1024);
        self.cache_limit_bytes.store(limit, Ordering::Release);
        self.prune_cache(limit)
    }

    pub fn clear_cached_images(&self) -> Result<CacheStats, String> {
        let root = content_cache_path();
        let files = scan_cache_files(&root, &HashSet::new())?;
        for file in files {
            if !file.path.starts_with(&root) {
                return Err("Refusing to remove a cache file outside the cache root".to_string());
            }
            match std::fs::remove_file(&file.path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
        }
        let connection = self.connection.lock().map_err(|error| error.to_string())?;
        connection
            .execute("UPDATE posts SET preview_path=NULL", [])
            .map_err(|error| error.to_string())?;
        connection
            .execute("UPDATE creators SET avatar_path=NULL, banner_path=NULL", [])
            .map_err(|error| error.to_string())?;
        drop(connection);
        remove_empty_cache_dirs(&root)?;
        self.cache_stats()
    }

    pub fn clear_all_cache(&self) -> Result<CacheStats, String> {
        self.clear_cached_images()?;
        let mut connection = self.connection.lock().map_err(|error| error.to_string())?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        transaction
            .execute("DELETE FROM content_lists", [])
            .map_err(|error| error.to_string())?;
        transaction
            .execute("DELETE FROM content_documents", [])
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "DELETE FROM posts AS post
                 WHERE NOT EXISTS (
                   SELECT 1 FROM content_pins pin
                   WHERE pin.entity_kind='post' AND pin.service=post.service
                     AND pin.creator_id=post.creator_id AND pin.post_id=post.post_id
                 )
                 AND NOT EXISTS (
                   SELECT 1 FROM collection_posts item
                   WHERE item.service=post.service AND item.creator_id=post.creator_id
                     AND item.post_id=post.post_id
                 )
                 AND NOT EXISTS (
                   SELECT 1 FROM download_jobs job
                   WHERE job.service=post.service AND job.creator_id=post.creator_id
                     AND job.post_id=post.post_id
                 )",
                [],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "DELETE FROM creators AS creator
                 WHERE NOT EXISTS (
                   SELECT 1 FROM posts post
                   WHERE post.service=creator.service AND post.creator_id=creator.creator_id
                 )
                 AND NOT EXISTS (
                   SELECT 1 FROM content_pins pin
                   WHERE pin.service=creator.service AND pin.creator_id=creator.creator_id
                 )
                 AND NOT EXISTS (
                   SELECT 1 FROM subscriptions sub
                   WHERE sub.service=creator.service AND sub.creator_id=creator.creator_id
                 )",
                [],
            )
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
        let _ = connection.execute_batch("PRAGMA optimize; VACUUM;");
        drop(connection);
        self.cache_stats()
    }

    pub fn wipe_all_data(&self) -> Result<CacheStats, String> {
        let root = content_cache_path();
        if root.exists() {
            let _ = std::fs::remove_dir_all(&root);
            let _ = std::fs::create_dir_all(&root);
        }
        let mut connection = self.connection.lock().map_err(|error| error.to_string())?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        transaction
            .execute("DELETE FROM collection_posts", [])
            .map_err(|e| e.to_string())?;
        transaction
            .execute("DELETE FROM collections WHERE is_system = 0", [])
            .map_err(|e| e.to_string())?;
        transaction
            .execute("DELETE FROM subscription_seen_posts", [])
            .map_err(|e| e.to_string())?;
        transaction
            .execute("DELETE FROM subscriptions", [])
            .map_err(|e| e.to_string())?;
        transaction
            .execute("DELETE FROM content_pins", [])
            .map_err(|e| e.to_string())?;
        transaction
            .execute("DELETE FROM content_lists", [])
            .map_err(|e| e.to_string())?;
        transaction
            .execute("DELETE FROM content_documents", [])
            .map_err(|e| e.to_string())?;
        transaction
            .execute("DELETE FROM download_blob_refs", [])
            .map_err(|e| e.to_string())?;
        transaction
            .execute("DELETE FROM download_jobs", [])
            .map_err(|e| e.to_string())?;
        transaction
            .execute("DELETE FROM media_blobs", [])
            .map_err(|e| e.to_string())?;
        transaction
            .execute("DELETE FROM posts", [])
            .map_err(|e| e.to_string())?;
        transaction
            .execute("DELETE FROM creators", [])
            .map_err(|e| e.to_string())?;
        transaction
            .execute("DELETE FROM sync_conflicts", [])
            .map_err(|e| e.to_string())?;
        transaction
            .execute("DELETE FROM sync_records", [])
            .map_err(|e| e.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
        let _ = connection.execute_batch("PRAGMA optimize; VACUUM;");
        drop(connection);
        self.cache_stats()
    }

    fn cached_metadata_bytes(&self) -> Result<u64, String> {
        let connection = self.connection.lock().map_err(|error| error.to_string())?;
        connection
            .query_row(
                "SELECT
                   COALESCE((SELECT SUM(length(CAST(identities_json AS BLOB))) FROM content_lists), 0) +
                   COALESCE((SELECT SUM(length(CAST(snapshot_json AS BLOB))) FROM content_documents), 0) +
                   COALESCE((
                     SELECT SUM(
                       length(CAST(snapshot_json AS BLOB)) +
                       length(CAST(title AS BLOB)) +
                       length(CAST(COALESCE(content,'') AS BLOB))
                     )
                     FROM posts post
                     WHERE NOT EXISTS (
                       SELECT 1 FROM content_pins pin
                       WHERE pin.entity_kind='post' AND pin.service=post.service
                         AND pin.creator_id=post.creator_id AND pin.post_id=post.post_id
                     )
                     AND NOT EXISTS (
                       SELECT 1 FROM collection_posts item
                       WHERE item.service=post.service AND item.creator_id=post.creator_id
                         AND item.post_id=post.post_id
                     )
                     AND NOT EXISTS (
                       SELECT 1 FROM download_jobs job
                       WHERE job.service=post.service AND job.creator_id=post.creator_id
                         AND job.post_id=post.post_id
                     )
                   ), 0) +
                   COALESCE((
                     SELECT SUM(length(CAST(snapshot_json AS BLOB)) + length(CAST(name AS BLOB)))
                     FROM creators creator
                     WHERE NOT EXISTS (
                       SELECT 1 FROM posts post
                       WHERE post.service=creator.service AND post.creator_id=creator.creator_id
                     )
                     AND NOT EXISTS (
                       SELECT 1 FROM content_pins pin
                       WHERE pin.service=creator.service AND pin.creator_id=creator.creator_id
                     )
                     AND NOT EXISTS (
                       SELECT 1 FROM subscriptions sub
                       WHERE sub.service=creator.service AND sub.creator_id=creator.creator_id
                     )
                   ), 0)",
                [],
                |row| row.get::<_, u64>(0),
            )
            .map_err(|error| error.to_string())
    }

    fn enforce_cache_limit(&self) {
        let limit = self.cache_limit_bytes.load(Ordering::Acquire);
        let _ = self.prune_cache(limit);
    }

    fn prune_cache(&self, target_bytes: u64) -> Result<CacheStats, String> {
        let root = content_cache_path();
        let protected = self.protected_cache_paths()?;
        let mut files = scan_cache_files(&root, &protected)?;
        let mut total = files.iter().map(|file| file.size).sum::<u64>();
        files.sort_by_key(|file| file.modified_at);

        let mut removed = Vec::new();
        for file in files.iter().filter(|file| !file.protected) {
            if total <= target_bytes {
                break;
            }
            if !file.path.starts_with(&root) {
                return Err("Refusing to remove a cache file outside the cache root".to_string());
            }
            match std::fs::remove_file(&file.path) {
                Ok(()) => {
                    total = total.saturating_sub(file.size);
                    removed.push(file.path.clone());
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    total = total.saturating_sub(file.size);
                    removed.push(file.path.clone());
                }
                Err(error) => return Err(error.to_string()),
            }
        }

        if !removed.is_empty() {
            let connection = self.connection.lock().map_err(|error| error.to_string())?;
            for path in removed {
                let value = path.to_string_lossy();
                connection
                    .execute(
                        "UPDATE posts SET preview_path=NULL WHERE preview_path=?1",
                        [&value],
                    )
                    .map_err(|error| error.to_string())?;
                connection
                    .execute(
                        "UPDATE creators SET avatar_path=NULL WHERE avatar_path=?1",
                        [&value],
                    )
                    .map_err(|error| error.to_string())?;
                connection
                    .execute(
                        "UPDATE creators SET banner_path=NULL WHERE banner_path=?1",
                        [&value],
                    )
                    .map_err(|error| error.to_string())?;
            }
            drop(connection);
            remove_empty_cache_dirs(&root)?;
        }

        self.cache_stats()
    }

    fn protected_cache_paths(&self) -> Result<HashSet<PathBuf>, String> {
        let connection = self.connection.lock().map_err(|error| error.to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT p.preview_path FROM posts p
                 WHERE p.preview_path IS NOT NULL AND (
                   EXISTS (SELECT 1 FROM content_pins pin WHERE pin.service=p.service AND pin.creator_id=p.creator_id AND pin.entity_kind='post' AND pin.post_id=p.post_id)
                   OR EXISTS (SELECT 1 FROM collection_posts cp WHERE cp.service=p.service AND cp.creator_id=p.creator_id AND cp.post_id=p.post_id)
                   OR EXISTS (SELECT 1 FROM download_jobs job WHERE job.service=p.service AND job.creator_id=p.creator_id AND job.post_id=p.post_id)
                 )
                 UNION ALL
                 SELECT c.avatar_path FROM creators c
                 WHERE c.avatar_path IS NOT NULL AND (
                   EXISTS (SELECT 1 FROM content_pins pin WHERE pin.service=c.service AND pin.creator_id=c.creator_id)
                   OR EXISTS (SELECT 1 FROM collection_posts cp WHERE cp.service=c.service AND cp.creator_id=c.creator_id)
                   OR EXISTS (SELECT 1 FROM download_jobs job WHERE job.service=c.service AND job.creator_id=c.creator_id)
                   OR EXISTS (SELECT 1 FROM subscriptions sub WHERE sub.service=c.service AND sub.creator_id=c.creator_id)
                 )
                 UNION ALL
                 SELECT c.banner_path FROM creators c
                 WHERE c.banner_path IS NOT NULL AND (
                   EXISTS (SELECT 1 FROM content_pins pin WHERE pin.service=c.service AND pin.creator_id=c.creator_id)
                   OR EXISTS (SELECT 1 FROM collection_posts cp WHERE cp.service=c.service AND cp.creator_id=c.creator_id)
                   OR EXISTS (SELECT 1 FROM download_jobs job WHERE job.service=c.service AND job.creator_id=c.creator_id)
                   OR EXISTS (SELECT 1 FROM subscriptions sub WHERE sub.service=c.service AND sub.creator_id=c.creator_id)
                 )",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
            .map(|paths| paths.into_iter().map(PathBuf::from).collect())
    }

    pub fn save_posts(&self, posts: &[Post]) -> Result<(), String> {
        let mut connection = self.connection.lock().map_err(|e| e.to_string())?;
        let tx = connection.transaction().map_err(|e| e.to_string())?;
        for post in posts {
            let mut clean_post = post.clone();
            clean_post.clean_extra();
            tx.execute(
                "INSERT INTO creators(service, creator_id, name, snapshot_json)
                 VALUES(?1, ?2, ?2, json_object('id', ?2, 'name', ?2, 'service', ?1))
                 ON CONFLICT(service, creator_id) DO NOTHING",
                params![clean_post.service, clean_post.user],
            )
            .map_err(|e| e.to_string())?;
            let snapshot = serde_json::to_string(&clean_post).map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO posts(service, creator_id, post_id, title, content, published_at, snapshot_json, last_checked_at)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,CURRENT_TIMESTAMP)
                 ON CONFLICT(service, creator_id, post_id) DO UPDATE SET
                    title=excluded.title, content=excluded.content, published_at=excluded.published_at,
                    snapshot_json=excluded.snapshot_json, remote_state='active',
                    cached_at=CURRENT_TIMESTAMP, last_checked_at=CURRENT_TIMESTAMP",
                params![clean_post.service, clean_post.user, clean_post.id, clean_post.title, clean_post.content, clean_post.published, snapshot],
            ).map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn get_post(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Option<Post>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let row: Option<(String,Option<String>)> = connection
            .query_row(
                "SELECT snapshot_json,preview_path FROM posts WHERE service=?1 AND creator_id=?2 AND post_id=?3",
                params![service, creator_id, post_id],
                |r| Ok((r.get(0)?,r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        row.map(|(value, preview)| {
            let mut post: Post = Post::from_json_str(&value).map_err(|e| e.to_string())?;
            if let Some(path) = preview {
                post.extra
                    .insert("local_preview_path".into(), serde_json::Value::String(path));
            }
            Ok(post)
        })
        .transpose()
    }

    pub fn find_post_identity(
        &self,
        service: &str,
        post_id: &str,
        preferred_creator_id: Option<&str>,
    ) -> Result<Option<(String, String, String)>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .query_row(
                "SELECT service,creator_id,post_id FROM posts
                 WHERE service=?1 AND post_id=?2
                 ORDER BY CASE WHEN creator_id=COALESCE(?3,'') THEN 0 ELSE 1 END
                 LIMIT 1",
                params![service, post_id, preferred_creator_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn find_creator_by_alias(
        &self,
        service: &str,
        alias: &str,
    ) -> Result<Option<String>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .query_row(
                "SELECT creator_id FROM creators
                 WHERE service=?1 AND (
                   creator_id=?2 OR
                   name=?2 COLLATE NOCASE OR
                   CAST(json_extract(snapshot_json,'$.name') AS TEXT)=?2 COLLATE NOCASE OR
                   CAST(json_extract(snapshot_json,'$.public_id') AS TEXT)=?2 OR
                   CAST(json_extract(snapshot_json,'$.relation_id') AS TEXT)=?2
                 ) LIMIT 1",
                params![service, alias],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn list_creator_posts(
        &self,
        service: &str,
        creator_id: &str,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<Post>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT snapshot_json FROM posts WHERE service=?1 AND creator_id=?2
             ORDER BY published_at DESC, post_id DESC LIMIT ?3 OFFSET ?4",
            )
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map(params![service, creator_id, limit, offset], |r| {
                r.get::<_, String>(0)
            })
            .map_err(|e| e.to_string())?;
        rows.map(|row| {
            row.map_err(|e| e.to_string())
                .and_then(|json| Post::from_json_str(&json))
        })
        .collect()
    }

    pub fn search_posts(&self, query: &str) -> Result<Vec<Post>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let pattern = format!("%{query}%");
        let mut statement = connection.prepare(
            "SELECT snapshot_json FROM posts WHERE title LIKE ?1 OR content LIKE ?1 ORDER BY published_at DESC LIMIT 100"
        ).map_err(|e| e.to_string())?;
        let rows = statement
            .query_map(params![pattern], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.map(|row| {
            row.map_err(|e| e.to_string())
                .and_then(|json| Post::from_json_str(&json))
        })
        .collect()
    }

    pub fn list_recent_posts(&self, offset: u32, limit: u32) -> Result<Vec<Post>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let mut statement = connection.prepare("SELECT snapshot_json FROM posts ORDER BY published_at DESC, post_id DESC LIMIT ?1 OFFSET ?2").map_err(|e| e.to_string())?;
        let rows = statement
            .query_map(params![limit, offset], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.map(|row| {
            row.map_err(|e| e.to_string())
                .and_then(|json| Post::from_json_str(&json))
        })
        .collect()
    }

    pub fn save_post_list(
        &self,
        list_key: &str,
        offset: u32,
        posts: &[Post],
    ) -> Result<(), String> {
        self.save_posts(posts)?;
        let identities: Vec<(&str, &str, &str)> = posts
            .iter()
            .map(|post| (post.service.as_str(), post.user.as_str(), post.id.as_str()))
            .collect();
        let json = serde_json::to_string(&identities).map_err(|e| e.to_string())?;
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .execute(
                "INSERT INTO content_lists(list_key,page_offset,identities_json) VALUES(?1,?2,?3)
                 ON CONFLICT(list_key,page_offset) DO UPDATE SET
                   identities_json=excluded.identities_json,cached_at=CURRENT_TIMESTAMP",
                params![list_key, offset, json],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_post_list(&self, list_key: &str, offset: u32) -> Result<Vec<Post>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let json: Option<String> = connection
            .query_row(
                "SELECT identities_json FROM content_lists WHERE list_key=?1 AND page_offset=?2",
                params![list_key, offset],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let Some(json) = json else {
            return Ok(Vec::new());
        };
        let identities: Vec<(String, String, String)> =
            serde_json::from_str(&json).map_err(|e| e.to_string())?;
        let mut posts = Vec::with_capacity(identities.len());
        for (service, creator_id, post_id) in identities {
            let row: Option<(String, Option<String>)> = connection
                .query_row(
                    "SELECT snapshot_json,preview_path FROM posts WHERE service=?1 AND creator_id=?2 AND post_id=?3",
                    params![service, creator_id, post_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            if let Some((snapshot, preview)) = row {
                let mut post: Post = Post::from_json_str(&snapshot).map_err(|e| e.to_string())?;
                if let Some(path) = preview {
                    post.extra
                        .insert("local_preview_path".into(), serde_json::Value::String(path));
                }
                posts.push(post);
            }
        }
        Ok(posts)
    }

    pub fn save_document<T: serde::Serialize>(
        &self,
        document_kind: &str,
        service: &str,
        creator_id: &str,
        post_id: &str,
        value: &T,
    ) -> Result<(), String> {
        let snapshot = serde_json::to_string(value).map_err(|e| e.to_string())?;
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .execute(
                "INSERT INTO content_documents(document_kind,service,creator_id,post_id,snapshot_json)
                 VALUES(?1,?2,?3,?4,?5)
                 ON CONFLICT(document_kind,service,creator_id,post_id) DO UPDATE SET
                   snapshot_json=excluded.snapshot_json,cached_at=CURRENT_TIMESTAMP",
                params![document_kind, service, creator_id, post_id, snapshot],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_document<T: DeserializeOwned>(
        &self,
        document_kind: &str,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Option<T>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let snapshot: Option<String> = connection
            .query_row(
                "SELECT snapshot_json FROM content_documents
                 WHERE document_kind=?1 AND service=?2 AND creator_id=?3 AND post_id=?4",
                params![document_kind, service, creator_id, post_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        snapshot
            .map(|json| serde_json::from_str(&json).map_err(|e| e.to_string()))
            .transpose()
    }

    pub fn save_post_revisions(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
        provider_id: &str,
        revisions: &[PostRevision],
    ) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        for rev in revisions {
            let snapshot = serde_json::to_string(&rev.post).map_err(|e| e.to_string())?;
            connection
                .execute(
                    "INSERT INTO post_revisions(service,creator_id,post_id,revision_id,provider_id,imported_at,edited_at,snapshot_json)
                     VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
                     ON CONFLICT(service,creator_id,post_id,revision_id,provider_id) DO UPDATE SET
                       snapshot_json=excluded.snapshot_json,
                       imported_at=excluded.imported_at,
                       edited_at=excluded.edited_at",
                    params![
                        service,
                        creator_id,
                        post_id,
                        rev.revision_id,
                        provider_id,
                        rev.post.added,
                        rev.post.edited,
                        snapshot
                    ],
                )
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn load_post_revisions(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Vec<PostRevision>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT revision_id, snapshot_json FROM post_revisions
                 WHERE service=?1 AND creator_id=?2 AND post_id=?3
                 ORDER BY revision_id DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map(params![service, creator_id, post_id], |row| {
                let revision_id: i64 = row.get(0)?;
                let json: String = row.get(1)?;
                let post: Post = Post::from_json_str(&json).unwrap_or_else(|_| Post {
                    id: post_id.to_string(),
                    user: creator_id.to_string(),
                    service: service.to_string(),
                    title: String::new(),
                    content: None,
                    substring: None,
                    published: None,
                    added: None,
                    edited: None,
                    embed: None,
                    shared_file: None,
                    attachments: None,
                    file: None,
                    poll: None,
                    captions: None,
                    tags: None,
                    origin: None,
                    preview_state: None,
                    has_full: None,
                    detail_fetched: None,
                    next: None,
                    prev: None,
                    favorite_count: None,
                    attachment_count: None,
                    extra: Default::default(),
                });
                Ok(PostRevision { revision_id, post })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn save_creator(&self, creator: &CreatorProfile) -> Result<(), String> {
        self.save_creator_json(&creator.service, &creator.id, &creator.name, creator)
    }

    pub fn save_creators(&self, creators: &[Creator]) -> Result<(), String> {
        for creator in creators {
            self.save_creator_json(&creator.service, &creator.id, &creator.name, creator)?;
        }
        Ok(())
    }

    pub fn list_creators(&self) -> Result<Vec<Creator>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let mut statement = connection
            .prepare("SELECT creator_id,name,service FROM creators ORDER BY name COLLATE NOCASE")
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |r| {
                Ok(Creator {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    service: r.get(2)?,
                    public_id: None,
                    relation_id: None,
                    indexed: None,
                    updated: None,
                    favorited: None,
                    kemono_favorited: None,
                    ever_imported: None,
                    extra: Default::default(),
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    fn save_creator_json<T: serde::Serialize>(
        &self,
        service: &str,
        id: &str,
        name: &str,
        creator: &T,
    ) -> Result<(), String> {
        let snapshot = serde_json::to_string(creator).map_err(|e| e.to_string())?;
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection.execute(
            "INSERT INTO creators(service,creator_id,name,snapshot_json,last_checked_at)
             VALUES(?1,?2,?3,?4,CURRENT_TIMESTAMP)
             ON CONFLICT(service,creator_id) DO UPDATE SET name=excluded.name,
               snapshot_json=excluded.snapshot_json,cached_at=CURRENT_TIMESTAMP,last_checked_at=CURRENT_TIMESTAMP",
            params![service,id,name,snapshot]
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_creator(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Option<CreatorProfile>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let json: Option<String> = connection
            .query_row(
                "SELECT snapshot_json FROM creators WHERE service=?1 AND creator_id=?2",
                params![service, creator_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        json.map(|value| serde_json::from_str(&value).map_err(|e| e.to_string()))
            .transpose()
    }

    pub fn pin_post(&self, post: &Post, reason: &str, account_id: &str) -> Result<(), String> {
        self.save_posts(std::slice::from_ref(post))?;
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection.execute(
            "INSERT OR IGNORE INTO content_pins(entity_kind,service,creator_id,post_id,reason,account_id) VALUES('post',?1,?2,?3,?4,?5)",
            params![post.service,post.user,post.id,reason,account_id]
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_pin(
        &self,
        kind: &str,
        service: &str,
        creator_id: &str,
        post_id: Option<&str>,
        reason: &str,
        account_id: &str,
        active: bool,
    ) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let post_id = post_id.unwrap_or("");
        if active {
            connection.execute(
                "INSERT OR IGNORE INTO content_pins(entity_kind,service,creator_id,post_id,reason,account_id) VALUES(?1,?2,?3,?4,?5,?6)",
                params![kind,service,creator_id,post_id,reason,account_id]
            ).map_err(|e| e.to_string())?;
        } else {
            connection.execute(
                "DELETE FROM content_pins WHERE entity_kind=?1 AND service=?2 AND creator_id=?3 AND post_id=?4 AND reason=?5 AND account_id=?6",
                params![kind,service,creator_id,post_id,reason,account_id]
            ).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn list_favorites(&self, kind: &str, account_id: &str) -> Result<Vec<Favorite>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let (sql, entity_kind) = if kind == "artist" {
            (
                "SELECT c.snapshot_json, MAX(pin.created_at) AS faved_at FROM content_pins pin JOIN creators c USING(service,creator_id) WHERE pin.entity_kind=?1 AND pin.reason='favorite' AND (pin.account_id=?2 OR pin.account_id='' OR ?2='') GROUP BY c.service, c.creator_id ORDER BY faved_at DESC",
                "creator",
            )
        } else {
            (
                "SELECT p.snapshot_json, MAX(pin.created_at) AS faved_at FROM content_pins pin JOIN posts p USING(service,creator_id,post_id) WHERE pin.entity_kind=?1 AND pin.reason='favorite' AND (pin.account_id=?2 OR pin.account_id='' OR ?2='') GROUP BY p.service, p.creator_id, p.post_id ORDER BY faved_at DESC",
                "post",
            )
        };
        let mut statement = connection.prepare(sql).map_err(|e| e.to_string())?;
        let rows = statement
            .query_map(params![entity_kind, account_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
            })
            .map_err(|e| e.to_string())?;
        let mut favorites = Vec::new();
        for item in rows.flatten() {
            let (json, created_at_str) = item;
            match serde_json::from_str::<Favorite>(&json) {
                Ok(mut fav) => {
                    if let Some(created_str) = created_at_str {
                        fav.extra.insert(
                            "faved_at".to_string(),
                            serde_json::Value::String(created_str),
                        );
                    }
                    favorites.push(fav);
                }
                Err(e) => {
                    tracing::warn!("Failed to deserialize favorite {}: {}", entity_kind, e);
                }
            }
        }
        Ok(favorites)
    }

    pub fn store_artwork_data_url(
        &self,
        service: &str,
        creator_id: &str,
        kind: &str,
        data_url: &str,
    ) -> Result<PathBuf, String> {
        let (header, encoded) = data_url.split_once(',').ok_or("Invalid artwork data URL")?;
        let extension = if header.contains("png") {
            "png"
        } else if header.contains("webp") {
            "webp"
        } else if header.contains("gif") {
            "gif"
        } else {
            "jpg"
        };
        let bytes = BASE64_STANDARD.decode(encoded).map_err(|e| e.to_string())?;
        let dir = content_cache_path().join(if kind == "banner" {
            "banners"
        } else {
            "avatars"
        });
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let safe = format!(
            "{}_{}_{}.{}",
            sanitize(service),
            sanitize(creator_id),
            kind,
            extension
        );
        let path = dir.join(safe);
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        let column = if kind == "banner" {
            "banner_path"
        } else {
            "avatar_path"
        };
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .execute(
                &format!("UPDATE creators SET {column}=?3 WHERE service=?1 AND creator_id=?2"),
                params![service, creator_id, path.to_string_lossy()],
            )
            .map_err(|e| e.to_string())?;
        drop(connection);
        self.enforce_cache_limit();
        Ok(path)
    }

    pub fn artwork_path(
        &self,
        service: &str,
        creator_id: &str,
        kind: &str,
    ) -> Result<Option<String>, String> {
        let column = if kind == "banner" {
            "banner_path"
        } else {
            "avatar_path"
        };
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .query_row(
                &format!("SELECT {column} FROM creators WHERE service=?1 AND creator_id=?2"),
                params![service, creator_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())
            .map(|v| v.flatten())
    }

    pub fn artwork_data_url(
        &self,
        service: &str,
        creator_id: &str,
        kind: &str,
    ) -> Result<Option<String>, String> {
        let Some(path) = self.artwork_path(service, creator_id, kind)? else {
            return Ok(None);
        };
        let path = PathBuf::from(path);
        if !path.is_file() {
            return Ok(None);
        }
        let mime = mime_guess::from_path(&path).first_or_octet_stream();
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        Ok(Some(format!(
            "data:{mime};base64,{}",
            BASE64_STANDARD.encode(bytes)
        )))
    }

    pub fn store_thumbnail_data_url(&self, key: &str, data_url: &str) -> Result<PathBuf, String> {
        let (header, encoded) = data_url
            .split_once(',')
            .ok_or("Invalid thumbnail data URL")?;
        let extension = if header.contains("png") {
            "png"
        } else if header.contains("webp") {
            "webp"
        } else {
            "jpg"
        };
        let bytes = BASE64_STANDARD.decode(encoded).map_err(|e| e.to_string())?;
        let dir = content_cache_path().join("thumbnails");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let safe = format!("{}.{}", sanitize(key), extension);
        let path = dir.join(safe);
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        self.enforce_cache_limit();
        Ok(path)
    }

    pub fn thumbnail_data_url(&self, key: &str) -> Result<Option<String>, String> {
        let dir = content_cache_path().join("thumbnails");
        for ext in &["webp", "jpg", "png"] {
            let path = dir.join(format!("{}.{}", sanitize(key), ext));
            if path.is_file() {
                let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
                let mime = if *ext == "webp" {
                    "image/webp"
                } else if *ext == "png" {
                    "image/png"
                } else {
                    "image/jpeg"
                };
                let encoded = BASE64_STANDARD.encode(bytes);
                return Ok(Some(format!("data:{mime};base64,{encoded}")));
            }
        }
        Ok(None)
    }

    pub async fn cache_post_preview(&self, post: &Post, url: &str) -> Result<PathBuf, String> {
        let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(format!("Preview HTTP {}", response.status()));
        }
        if response
            .content_length()
            .is_some_and(|n| n > 16 * 1024 * 1024)
        {
            return Err("Post preview is too large".into());
        }
        let mime = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_string();
        let bytes = response.bytes().await.map_err(|e| e.to_string())?;
        if bytes.len() > 16 * 1024 * 1024 {
            return Err("Post preview is too large".into());
        }
        let ext = if mime.contains("png") {
            "png"
        } else if mime.contains("webp") {
            "webp"
        } else if mime.contains("gif") {
            "gif"
        } else if mime.contains("mp4") {
            "mp4"
        } else if mime.contains("webm") {
            "webm"
        } else {
            "jpg"
        };
        let dir = content_cache_path().join("previews");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!(
            "{}_{}_{}.{}",
            sanitize(&post.service),
            sanitize(&post.user),
            sanitize(&post.id),
            ext
        ));
        std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
        let c = self.connection.lock().map_err(|e| e.to_string())?;
        c.execute(
            "UPDATE posts SET preview_path=?4 WHERE service=?1 AND creator_id=?2 AND post_id=?3",
            params![post.service, post.user, post.id, path.to_string_lossy()],
        )
        .map_err(|e| e.to_string())?;
        drop(c);
        self.enforce_cache_limit();
        Ok(path)
    }
}

fn scan_cache_files(root: &Path, protected: &HashSet<PathBuf>) -> Result<Vec<CacheFile>, String> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            let metadata = entry.metadata().map_err(|error| error.to_string())?;
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() {
                let modified_at = metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| duration.as_secs())
                    .unwrap_or_default();
                files.push(CacheFile {
                    protected: protected.contains(&path),
                    path,
                    size: metadata.len(),
                    modified_at,
                });
            }
        }
    }
    Ok(files)
}

fn cache_stats_from_files(files: &[CacheFile]) -> CacheStats {
    let protected_bytes = files
        .iter()
        .filter(|file| file.protected)
        .map(|file| file.size)
        .sum::<u64>();
    let total_bytes = files.iter().map(|file| file.size).sum::<u64>();
    let bytes_for = |directory: &str| {
        files
            .iter()
            .filter(|file| {
                file.path
                    .parent()
                    .and_then(Path::file_name)
                    .and_then(|value| value.to_str())
                    == Some(directory)
            })
            .map(|file| file.size)
            .sum::<u64>()
    };
    let preview_bytes = bytes_for("previews");
    let avatar_bytes = bytes_for("avatars");
    let banner_bytes = bytes_for("banners");
    CacheStats {
        total_bytes,
        metadata_bytes: 0,
        protected_bytes,
        reclaimable_bytes: total_bytes.saturating_sub(protected_bytes),
        preview_bytes,
        avatar_bytes,
        banner_bytes,
        other_bytes: total_bytes
            .saturating_sub(preview_bytes)
            .saturating_sub(avatar_bytes)
            .saturating_sub(banner_bytes),
        file_count: files.len() as u64,
    }
}

fn remove_empty_cache_dirs(root: &Path) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }
    let directories = std::fs::read_dir(root)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    for directory in directories {
        remove_empty_cache_dirs(&directory)?;
        if std::fs::read_dir(&directory)
            .map_err(|error| error.to_string())?
            .next()
            .is_none()
        {
            std::fs::remove_dir(&directory).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_stats_separate_protected_files() {
        let root = std::env::temp_dir().join(format!(
            "pawstash-content-cache-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let nested = root.join("previews");
        std::fs::create_dir_all(&nested).unwrap();
        let kept = nested.join("kept.jpg");
        let reclaimable = nested.join("old.jpg");
        std::fs::write(&kept, [0_u8; 7]).unwrap();
        std::fs::write(&reclaimable, [0_u8; 11]).unwrap();

        let files = scan_cache_files(&root, &HashSet::from([kept])).unwrap();
        let stats = cache_stats_from_files(&files);
        assert_eq!(stats.total_bytes, 18);
        assert_eq!(stats.metadata_bytes, 0);
        assert_eq!(stats.protected_bytes, 7);
        assert_eq!(stats.reclaimable_bytes, 11);
        assert_eq!(stats.preview_bytes, 18);
        assert_eq!(stats.avatar_bytes, 0);
        assert_eq!(stats.banner_bytes, 0);
        assert_eq!(stats.other_bytes, 0);
        assert_eq!(stats.file_count, 2);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_list_favorites_attaches_faved_at() {
        let repo = ContentRepository::new(512).unwrap();
        let post: Post = serde_json::from_str(
            r#"{
                "id": "100",
                "user": "creator_a",
                "service": "patreon",
                "title": "Test Post",
                "content": "Hello",
                "published": "2026-08-28 10:00:00",
                "added": "2026-08-28 10:00:00"
            }"#,
        )
        .unwrap();

        repo.pin_post(&post, "favorite", "my_account").unwrap();
        let favorites = repo.list_favorites("post", "my_account").unwrap();
        assert!(favorites.iter().any(|f| f.id == "100"
            && f.service.as_deref() == Some("patreon")
            && f.extra.contains_key("faved_at")));
    }
}
