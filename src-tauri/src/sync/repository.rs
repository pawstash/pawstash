use crate::db::storage::open_database;
#[cfg(test)]
use crate::db::storage::prepare_connection;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub server_url: String,
    pub account_id: String,
    pub device_id: String,
    pub snapshot_record_id: String,
    pub revision: i64,
    pub cursor: i64,
    pub last_plaintext_hash: Option<String>,
    pub kdf_json: String,
    pub encrypted_key_bundle: String,
    pub bundle_nonce: String,
    pub enabled: bool,
    pub last_synced_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConflict {
    pub remote_revision: i64,
    pub remote_cursor: i64,
    pub record_id: String,
    pub ciphertext: String,
    pub nonce: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionRecord {
    pub id: String,
    pub kind: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostRecord {
    pub service: String,
    pub creator_id: String,
    pub post_id: String,
    pub title: String,
    pub published_at: Option<String>,
    pub snapshot_json: String,
    pub cached_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MembershipRecord {
    pub collection_id: String,
    pub service: String,
    pub creator_id: String,
    pub post_id: String,
    pub position: i64,
    pub operation_id: String,
    pub added_at: String,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubscriptionRecord {
    pub id: String,
    pub service: String,
    pub creator_id: String,
    pub creator_name: String,
    pub destination_collection_id: String,
    #[serde(default = "default_true")]
    pub save_to_library: bool,
    pub enabled: bool,
    pub initial_import: String,
    pub auto_download: bool,
    pub download_scope: String,
    pub poll_interval_minutes: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PawchiveSessionRecord {
    pub session_cookie: String,
    pub username: String,
    pub updated_at: String,
}

pub const PAWCHIVE_SESSION_RECORD_ID: &str = "sec:pawchive_session";
pub const PROFILE_AVATAR_RECORD_ID: &str = "sec:avatar";
pub const MAX_AVATAR_DATA_URL_LEN: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileAvatarRecord {
    pub image: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FavoritePostRecord {
    pub service: String,
    pub creator_id: String,
    pub post_id: String,
    pub snapshot_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FavoriteCreatorRecord {
    pub service: String,
    pub creator_id: String,
    pub name: String,
    pub snapshot_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct DirtySyncRecord {
    pub record_id: String,
    pub kind: String,
    pub expected_revision: i64,
    pub payload: Option<Vec<u8>>,
    pub tombstone: bool,
}

fn stable_sync_hash(kind: &str, record_id: &str, bytes: &[u8]) -> String {
    let pinned_at = match kind {
        "fav_post" => serde_json::from_slice::<FavoritePostRecord>(bytes)
            .ok()
            .map(|record| record.created_at),
        "fav_creator" => serde_json::from_slice::<FavoriteCreatorRecord>(bytes)
            .ok()
            .map(|record| record.created_at),
        _ => None,
    };
    match pinned_at {
        Some(pinned_at) => {
            let mut hasher = Sha256::new();
            hasher.update(kind.as_bytes());
            hasher.update([0]);
            hasher.update(record_id.as_bytes());
            hasher.update([0]);
            hasher.update(pinned_at.as_bytes());
            format!("{:x}", hasher.finalize())
        }
        None => format!("{:x}", Sha256::digest(bytes)),
    }
}

pub fn collection_record_id(id: &str) -> String {
    format!("col:{id}")
}

pub fn post_record_id(service: &str, creator_id: &str, post_id: &str) -> String {
    format!("post:{service}:{creator_id}:{post_id}")
}

pub fn membership_record_id(
    collection_id: &str,
    service: &str,
    creator_id: &str,
    post_id: &str,
) -> String {
    format!("mem:{collection_id}:{service}:{creator_id}:{post_id}")
}

pub fn subscription_record_id(id: &str) -> String {
    format!("sub:{id}")
}

pub fn favorite_post_record_id(service: &str, creator_id: &str, post_id: &str) -> String {
    format!("fav:post:{service}:{creator_id}:{post_id}")
}

pub fn favorite_creator_record_id(service: &str, creator_id: &str) -> String {
    format!("fav:creator:{service}:{creator_id}")
}

fn without_import_state(raw_json: &str) -> Option<(crate::api::models::Post, String)> {
    let mut post = crate::api::models::Post::from_json_str(raw_json).ok()?;
    post.has_full = None;
    post.preview_state = None;
    let json = serde_json::to_string(&post).ok()?;
    Some((post, json))
}

pub fn slim_post_snapshot(raw_json: &str) -> String {
    if let Ok(post) = crate::api::models::Post::from_json_str(raw_json) {
        let slim = post.to_slim();
        if let Ok(serialized) = serde_json::to_string(&slim) {
            return serialized;
        }
    }
    raw_json.to_string()
}

fn stable_media_url(url: Option<String>) -> Option<String> {
    let url = url?;
    let base = url.split(['?', '#']).next().unwrap_or("").trim();
    if base.is_empty() {
        None
    } else {
        Some(base.to_string())
    }
}

pub fn favorite_post_snapshot(raw_json: &str) -> String {
    let Ok(post) = crate::api::models::Post::from_json_str(raw_json) else {
        return raw_json.to_string();
    };
    let mut slim = post.to_slim();
    slim.added = None;
    slim.edited = None;
    slim.detail_fetched = None;
    slim.preview_state = None;
    slim.preview_path = None;
    slim.has_full = None;
    slim.favorite_count = None;
    slim.attachment_count = None;
    slim.thumbnail_url = stable_media_url(slim.thumbnail_url);
    slim.media_url = stable_media_url(slim.media_url);
    slim.cloud_urls.clear();
    slim.extra.clear();
    for attachment in slim
        .file
        .iter_mut()
        .chain(slim.attachments.iter_mut().flatten())
    {
        attachment.url = None;
        attachment.thumbnail_url = stable_media_url(attachment.thumbnail_url.take());
        attachment.extra.clear();
    }
    serde_json::to_string(&slim).unwrap_or_else(|_| raw_json.to_string())
}

pub fn favorite_creator_snapshot(raw_json: &str) -> String {
    let Ok(serde_json::Value::Object(object)) = serde_json::from_str(raw_json) else {
        return raw_json.to_string();
    };
    let mut stable = serde_json::Map::new();
    for key in [
        "id",
        "name",
        "service",
        "public_id",
        "relation_id",
        "avatar_url",
        "banner_url",
        "page_url",
    ] {
        if let Some(value) = object.get(key) {
            if !value.is_null() {
                stable.insert(key.to_string(), value.clone());
            }
        }
    }
    serde_json::to_string(&stable).unwrap_or_else(|_| raw_json.to_string())
}

pub struct SyncRepository {
    connection: Mutex<Connection>,
}

impl SyncRepository {
    pub fn new() -> Result<Self, String> {
        let conn = open_database()?;
        conn.execute_batch("PRAGMA foreign_keys = OFF;")
            .map_err(|e| format!("Failed to disable foreign keys for sync connection: {e}"))?;
        Ok(Self {
            connection: Mutex::new(conn),
        })
    }
    #[cfg(test)]
    pub fn in_memory() -> Self {
        let mut connection = Connection::open_in_memory().unwrap();
        prepare_connection(&mut connection).unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        Self {
            connection: Mutex::new(connection),
        }
    }

    pub fn state(&self) -> Result<Option<SyncState>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection.query_row("SELECT server_url,account_id,device_id,snapshot_record_id,revision,cursor,last_plaintext_hash,kdf_json,encrypted_key_bundle,bundle_nonce,enabled,last_synced_at,last_error FROM sync_state WHERE id=1",[],Self::map_state).optional().map_err(|e|e.to_string())
    }
    fn map_state(row: &rusqlite::Row<'_>) -> rusqlite::Result<SyncState> {
        Ok(SyncState {
            server_url: row.get(0)?,
            account_id: row.get(1)?,
            device_id: row.get(2)?,
            snapshot_record_id: row.get(3)?,
            revision: row.get(4)?,
            cursor: row.get(5)?,
            last_plaintext_hash: row.get(6)?,
            kdf_json: row.get(7)?,
            encrypted_key_bundle: row.get(8)?,
            bundle_nonce: row.get(9)?,
            enabled: row.get::<_, i64>(10)? != 0,
            last_synced_at: row.get(11)?,
            last_error: row.get(12)?,
        })
    }
    pub fn save_state(&self, state: &SyncState) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection.execute("INSERT INTO sync_state(id,server_url,account_id,device_id,snapshot_record_id,revision,cursor,last_plaintext_hash,kdf_json,encrypted_key_bundle,bundle_nonce,enabled,last_synced_at,last_error) VALUES(1,?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13) ON CONFLICT(id) DO UPDATE SET server_url=excluded.server_url,account_id=excluded.account_id,device_id=excluded.device_id,snapshot_record_id=excluded.snapshot_record_id,revision=excluded.revision,cursor=excluded.cursor,last_plaintext_hash=excluded.last_plaintext_hash,kdf_json=excluded.kdf_json,encrypted_key_bundle=excluded.encrypted_key_bundle,bundle_nonce=excluded.bundle_nonce,enabled=excluded.enabled,last_synced_at=excluded.last_synced_at,last_error=excluded.last_error,updated_at=CURRENT_TIMESTAMP",params![state.server_url,state.account_id,state.device_id,state.snapshot_record_id,state.revision,state.cursor,state.last_plaintext_hash,state.kdf_json,state.encrypted_key_bundle,state.bundle_nonce,state.enabled as i64,state.last_synced_at,state.last_error]).map_err(|e|e.to_string())?;
        Ok(())
    }
    pub fn update_cursor(&self, cursor: i64) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection.execute("UPDATE sync_state SET cursor=?1,last_synced_at=CURRENT_TIMESTAMP,last_error=NULL,updated_at=CURRENT_TIMESTAMP WHERE id=1",params![cursor]).map_err(|e|e.to_string())?;
        Ok(())
    }
    pub fn set_error(&self, error: Option<&str>) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .execute(
                "UPDATE sync_state SET last_error=?1,updated_at=CURRENT_TIMESTAMP WHERE id=1",
                params![error],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn clear(&self) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .execute_batch(
                "DELETE FROM sync_conflicts; DELETE FROM sync_records; DELETE FROM sync_state;",
            )
            .map_err(|e| e.to_string())
    }

    pub fn conflict(&self) -> Result<Option<SyncConflict>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection.query_row("SELECT remote_revision,remote_cursor,record_id,ciphertext,nonce FROM sync_conflicts WHERE id=1",[],|row|Ok(SyncConflict{remote_revision:row.get(0)?,remote_cursor:row.get(1)?,record_id:row.get(2)?,ciphertext:row.get(3)?,nonce:row.get(4)?})).optional().map_err(|e|e.to_string())
    }
    pub fn save_conflict(&self, value: &SyncConflict) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection.execute("INSERT INTO sync_conflicts(id,remote_revision,remote_cursor,record_id,ciphertext,nonce) VALUES(1,?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET remote_revision=excluded.remote_revision,remote_cursor=excluded.remote_cursor,record_id=excluded.record_id,ciphertext=excluded.ciphertext,nonce=excluded.nonce,detected_at=CURRENT_TIMESTAMP",params![value.remote_revision,value.remote_cursor,value.record_id,value.ciphertext,value.nonce]).map_err(|e|e.to_string())?;
        Ok(())
    }
    pub fn clear_conflict(&self) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .execute("DELETE FROM sync_conflicts", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn avatar(&self) -> Result<Option<String>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .query_row("SELECT image FROM profile_avatar WHERE id=1", [], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn save_avatar(&self, image: &str) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .execute(
                "INSERT INTO profile_avatar(id,image,updated_at)
                 VALUES(1,?1,strftime('%Y-%m-%dT%H:%M:%fZ','now'))
                 ON CONFLICT(id) DO UPDATE SET image=excluded.image, updated_at=excluded.updated_at",
                params![image],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn clear_avatar(&self) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .execute("DELETE FROM profile_avatar WHERE id=1", [])
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn detect_and_get_dirty_records(&self) -> Result<Vec<DirtySyncRecord>, String> {
        let mut connection = self.connection.lock().map_err(|e| e.to_string())?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let mut dirty_records = Vec::new();

        let collections = collect(
            &tx,
            "SELECT id,kind,parent_id,name,position,created_at,updated_at,color FROM collections WHERE is_system=0",
            |r| {
                Ok(CollectionRecord {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                    parent_id: r.get(2)?,
                    name: r.get(3)?,
                    position: r.get(4)?,
                    created_at: r.get(5)?,
                    updated_at: r.get(6)?,
                    color: r.get(7)?,
                })
            },
        )?;
        let mut live_col_ids = std::collections::HashSet::new();
        for col in collections {
            let record_id = collection_record_id(&col.id);
            live_col_ids.insert(record_id.clone());
            let bytes = serde_json::to_vec(&col).map_err(|e| e.to_string())?;
            let hash = format!("{:x}", Sha256::digest(&bytes));

            let current: Option<(i64, Option<String>, bool, bool)> = tx
                .query_row(
                    "SELECT revision, content_hash, dirty, tombstone FROM sync_records WHERE record_id=?1",
                    params![record_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0, r.get::<_, i64>(3)? != 0)),
                )
                .optional()
                .map_err(|e| e.to_string())?;

            match current {
                Some((rev, existing_hash, dirty, tombstone)) => {
                    let changed = existing_hash.as_deref() != Some(&hash) || tombstone;
                    if changed || dirty {
                        tx.execute(
                            "UPDATE sync_records SET content_hash=?1, dirty=1, tombstone=0, updated_at=CURRENT_TIMESTAMP WHERE record_id=?2",
                            params![hash, record_id],
                        )
                        .map_err(|e| e.to_string())?;
                        dirty_records.push(DirtySyncRecord {
                            record_id,
                            kind: "collection".into(),
                            expected_revision: rev,
                            payload: Some(bytes),
                            tombstone: false,
                        });
                    }
                }
                None => {
                    tx.execute(
                        "INSERT INTO sync_records(record_id,kind,revision,content_hash,dirty,tombstone,updated_at) VALUES(?1,'collection',0,?2,1,0,CURRENT_TIMESTAMP)",
                        params![record_id, hash],
                    )
                    .map_err(|e| e.to_string())?;
                    dirty_records.push(DirtySyncRecord {
                        record_id,
                        kind: "collection".into(),
                        expected_revision: 0,
                        payload: Some(bytes),
                        tombstone: false,
                    });
                }
            }
        }
        let existing_cols = collect(
            &tx,
            "SELECT record_id, revision FROM sync_records WHERE kind='collection' AND tombstone=0",
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
        )?;
        for (rec_id, rev) in existing_cols {
            if !live_col_ids.contains(&rec_id) {
                tx.execute(
                    "UPDATE sync_records SET dirty=1, tombstone=1, content_hash=NULL, updated_at=CURRENT_TIMESTAMP WHERE record_id=?1",
                    params![rec_id],
                )
                .map_err(|e| e.to_string())?;
                dirty_records.push(DirtySyncRecord {
                    record_id: rec_id,
                    kind: "collection".into(),
                    expected_revision: rev,
                    payload: None,
                    tombstone: true,
                });
            }
        }

        let posts = collect(
            &tx,
            "SELECT DISTINCT p.service,p.creator_id,p.post_id,p.title,p.published_at,p.snapshot_json,p.cached_at
             FROM posts p
             JOIN collection_posts cp USING(service,creator_id,post_id)",
            |r| {
                let raw_snapshot: String = r.get(5)?;
                Ok(PostRecord {
                    service: r.get(0)?,
                    creator_id: r.get(1)?,
                    post_id: r.get(2)?,
                    title: r.get(3)?,
                    published_at: r.get(4)?,
                    snapshot_json: slim_post_snapshot(&raw_snapshot),
                    cached_at: r.get(6)?,
                })
            },
        )?;
        let mut live_post_ids = std::collections::HashSet::new();
        for post in posts {
            let record_id = post_record_id(&post.service, &post.creator_id, &post.post_id);
            live_post_ids.insert(record_id.clone());
            let bytes = serde_json::to_vec(&post).map_err(|e| e.to_string())?;
            let hash = format!("{:x}", Sha256::digest(&bytes));

            let current: Option<(i64, Option<String>, bool, bool)> = tx
                .query_row(
                    "SELECT revision, content_hash, dirty, tombstone FROM sync_records WHERE record_id=?1",
                    params![record_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0, r.get::<_, i64>(3)? != 0)),
                )
                .optional()
                .map_err(|e| e.to_string())?;

            match current {
                Some((rev, existing_hash, dirty, tombstone)) => {
                    let changed = existing_hash.as_deref() != Some(&hash) || tombstone;
                    if changed || dirty {
                        tx.execute(
                            "UPDATE sync_records SET content_hash=?1, dirty=1, tombstone=0, updated_at=CURRENT_TIMESTAMP WHERE record_id=?2",
                            params![hash, record_id],
                        )
                        .map_err(|e| e.to_string())?;
                        dirty_records.push(DirtySyncRecord {
                            record_id,
                            kind: "post".into(),
                            expected_revision: rev,
                            payload: Some(bytes),
                            tombstone: false,
                        });
                    }
                }
                None => {
                    tx.execute(
                        "INSERT INTO sync_records(record_id,kind,revision,content_hash,dirty,tombstone,updated_at) VALUES(?1,'post',0,?2,1,0,CURRENT_TIMESTAMP)",
                        params![record_id, hash],
                    )
                    .map_err(|e| e.to_string())?;
                    dirty_records.push(DirtySyncRecord {
                        record_id,
                        kind: "post".into(),
                        expected_revision: 0,
                        payload: Some(bytes),
                        tombstone: false,
                    });
                }
            }
        }
        let existing_posts = collect(
            &tx,
            "SELECT record_id, revision FROM sync_records WHERE kind='post' AND tombstone=0",
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
        )?;
        for (rec_id, rev) in existing_posts {
            if !live_post_ids.contains(&rec_id) {
                tx.execute(
                    "UPDATE sync_records SET dirty=1, tombstone=1, content_hash=NULL, updated_at=CURRENT_TIMESTAMP WHERE record_id=?1",
                    params![rec_id],
                )
                .map_err(|e| e.to_string())?;
                dirty_records.push(DirtySyncRecord {
                    record_id: rec_id,
                    kind: "post".into(),
                    expected_revision: rev,
                    payload: None,
                    tombstone: true,
                });
            }
        }

        let memberships = collect(
            &tx,
            "SELECT collection_id,service,creator_id,post_id,position,operation_id,added_at FROM collection_posts",
            |r| {
                Ok(MembershipRecord {
                    collection_id: r.get(0)?,
                    service: r.get(1)?,
                    creator_id: r.get(2)?,
                    post_id: r.get(3)?,
                    position: r.get(4)?,
                    operation_id: r.get(5)?,
                    added_at: r.get(6)?,
                })
            },
        )?;
        let mut live_mem_ids = std::collections::HashSet::new();
        for mem in memberships {
            let record_id = membership_record_id(
                &mem.collection_id,
                &mem.service,
                &mem.creator_id,
                &mem.post_id,
            );
            live_mem_ids.insert(record_id.clone());
            let bytes = serde_json::to_vec(&mem).map_err(|e| e.to_string())?;
            let hash = format!("{:x}", Sha256::digest(&bytes));

            let current: Option<(i64, Option<String>, bool, bool)> = tx
                .query_row(
                    "SELECT revision, content_hash, dirty, tombstone FROM sync_records WHERE record_id=?1",
                    params![record_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0, r.get::<_, i64>(3)? != 0)),
                )
                .optional()
                .map_err(|e| e.to_string())?;

            match current {
                Some((rev, existing_hash, dirty, tombstone)) => {
                    let changed = existing_hash.as_deref() != Some(&hash) || tombstone;
                    if changed || dirty {
                        tx.execute(
                            "UPDATE sync_records SET content_hash=?1, dirty=1, tombstone=0, updated_at=CURRENT_TIMESTAMP WHERE record_id=?2",
                            params![hash, record_id],
                        )
                        .map_err(|e| e.to_string())?;
                        dirty_records.push(DirtySyncRecord {
                            record_id,
                            kind: "membership".into(),
                            expected_revision: rev,
                            payload: Some(bytes),
                            tombstone: false,
                        });
                    }
                }
                None => {
                    tx.execute(
                        "INSERT INTO sync_records(record_id,kind,revision,content_hash,dirty,tombstone,updated_at) VALUES(?1,'membership',0,?2,1,0,CURRENT_TIMESTAMP)",
                        params![record_id, hash],
                    )
                    .map_err(|e| e.to_string())?;
                    dirty_records.push(DirtySyncRecord {
                        record_id,
                        kind: "membership".into(),
                        expected_revision: 0,
                        payload: Some(bytes),
                        tombstone: false,
                    });
                }
            }
        }
        let existing_mems = collect(
            &tx,
            "SELECT record_id, revision FROM sync_records WHERE kind='membership' AND tombstone=0",
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
        )?;
        for (rec_id, rev) in existing_mems {
            if !live_mem_ids.contains(&rec_id) {
                tx.execute(
                    "UPDATE sync_records SET dirty=1, tombstone=1, content_hash=NULL, updated_at=CURRENT_TIMESTAMP WHERE record_id=?1",
                    params![rec_id],
                )
                .map_err(|e| e.to_string())?;
                dirty_records.push(DirtySyncRecord {
                    record_id: rec_id,
                    kind: "membership".into(),
                    expected_revision: rev,
                    payload: None,
                    tombstone: true,
                });
            }
        }

        let subscriptions = collect(
            &tx,
            "SELECT id,service,creator_id,creator_name,destination_collection_id,save_to_library,enabled,initial_import,auto_download,download_scope,poll_interval_minutes,created_at,updated_at FROM subscriptions",
            |r| {
                Ok(SubscriptionRecord {
                    id: r.get(0)?,
                    service: r.get(1)?,
                    creator_id: r.get(2)?,
                    creator_name: r.get(3)?,
                    destination_collection_id: r.get(4)?,
                    save_to_library: r.get::<_, i64>(5)? != 0,
                    enabled: r.get::<_, i64>(6)? != 0,
                    initial_import: r.get(7)?,
                    auto_download: r.get::<_, i64>(8)? != 0,
                    download_scope: r.get(9)?,
                    poll_interval_minutes: r.get(10)?,
                    created_at: r.get(11)?,
                    updated_at: r.get(12)?,
                })
            },
        )?;
        let mut live_sub_ids = std::collections::HashSet::new();
        for sub in subscriptions {
            let record_id = subscription_record_id(&sub.id);
            live_sub_ids.insert(record_id.clone());
            let bytes = serde_json::to_vec(&sub).map_err(|e| e.to_string())?;
            let hash = format!("{:x}", Sha256::digest(&bytes));

            let current: Option<(i64, Option<String>, bool, bool)> = tx
                .query_row(
                    "SELECT revision, content_hash, dirty, tombstone FROM sync_records WHERE record_id=?1",
                    params![record_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0, r.get::<_, i64>(3)? != 0)),
                )
                .optional()
                .map_err(|e| e.to_string())?;

            match current {
                Some((rev, existing_hash, dirty, tombstone)) => {
                    let changed = existing_hash.as_deref() != Some(&hash) || tombstone;
                    if changed || dirty {
                        tx.execute(
                            "UPDATE sync_records SET content_hash=?1, dirty=1, tombstone=0, updated_at=CURRENT_TIMESTAMP WHERE record_id=?2",
                            params![hash, record_id],
                        )
                        .map_err(|e| e.to_string())?;
                        dirty_records.push(DirtySyncRecord {
                            record_id,
                            kind: "subscription".into(),
                            expected_revision: rev,
                            payload: Some(bytes),
                            tombstone: false,
                        });
                    }
                }
                None => {
                    tx.execute(
                        "INSERT INTO sync_records(record_id,kind,revision,content_hash,dirty,tombstone,updated_at) VALUES(?1,'subscription',0,?2,1,0,CURRENT_TIMESTAMP)",
                        params![record_id, hash],
                    )
                    .map_err(|e| e.to_string())?;
                    dirty_records.push(DirtySyncRecord {
                        record_id,
                        kind: "subscription".into(),
                        expected_revision: 0,
                        payload: Some(bytes),
                        tombstone: false,
                    });
                }
            }
        }
        let existing_subs = collect(
            &tx,
            "SELECT record_id, revision FROM sync_records WHERE kind='subscription' AND tombstone=0",
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
        )?;
        for (rec_id, rev) in existing_subs {
            if !live_sub_ids.contains(&rec_id) {
                tx.execute(
                    "UPDATE sync_records SET dirty=1, tombstone=1, content_hash=NULL, updated_at=CURRENT_TIMESTAMP WHERE record_id=?1",
                    params![rec_id],
                )
                .map_err(|e| e.to_string())?;
                dirty_records.push(DirtySyncRecord {
                    record_id: rec_id,
                    kind: "subscription".into(),
                    expected_revision: rev,
                    payload: None,
                    tombstone: true,
                });
            }
        }

        let sync_session_enabled: bool = tx
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'setting.sync_pawchive_session'",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .and_then(|v| v.parse().ok())
            .unwrap_or(false);

        if sync_session_enabled {
            let username = tx
                .query_row(
                    "SELECT value FROM app_settings WHERE key = 'setting.pawchive_username'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .optional()
                .map_err(|e| e.to_string())?
                .unwrap_or_default();

            if let Ok(Some(cookie_bytes)) =
                crate::sync::secrets::SecretStore::load_named("pawchive-session")
            {
                let cookie_str = String::from_utf8_lossy(&cookie_bytes).trim().to_string();
                if !cookie_str.is_empty() {
                    let session_rec = PawchiveSessionRecord {
                        session_cookie: cookie_str,
                        username,
                        updated_at: "CURRENT_TIMESTAMP".into(),
                    };
                    let bytes = serde_json::to_vec(&session_rec).map_err(|e| e.to_string())?;
                    let hash = format!("{:x}", Sha256::digest(&bytes));

                    let current: Option<(i64, Option<String>, bool, bool)> = tx
                        .query_row(
                            "SELECT revision, content_hash, dirty, tombstone FROM sync_records WHERE record_id=?1",
                            params![PAWCHIVE_SESSION_RECORD_ID],
                            |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0, r.get::<_, i64>(3)? != 0)),
                        )
                        .optional()
                        .map_err(|e| e.to_string())?;

                    match current {
                        Some((rev, existing_hash, dirty, tombstone)) => {
                            let changed = existing_hash.as_deref() != Some(&hash) || tombstone;
                            if changed || dirty {
                                tx.execute(
                                    "UPDATE sync_records SET content_hash=?1, dirty=1, tombstone=0, updated_at=CURRENT_TIMESTAMP WHERE record_id=?2",
                                    params![hash, PAWCHIVE_SESSION_RECORD_ID],
                                )
                                .map_err(|e| e.to_string())?;
                                dirty_records.push(DirtySyncRecord {
                                    record_id: PAWCHIVE_SESSION_RECORD_ID.to_string(),
                                    kind: "session".into(),
                                    expected_revision: rev,
                                    payload: Some(bytes),
                                    tombstone: false,
                                });
                            }
                        }
                        None => {
                            tx.execute(
                                "INSERT INTO sync_records(record_id,kind,revision,content_hash,dirty,tombstone,updated_at) VALUES(?1,'session',0,?2,1,0,CURRENT_TIMESTAMP)",
                                params![PAWCHIVE_SESSION_RECORD_ID, hash],
                            )
                            .map_err(|e| e.to_string())?;
                            dirty_records.push(DirtySyncRecord {
                                record_id: PAWCHIVE_SESSION_RECORD_ID.to_string(),
                                kind: "session".into(),
                                expected_revision: 0,
                                payload: Some(bytes),
                                tombstone: false,
                            });
                        }
                    }
                }
            }
        }

        let avatar = tx
            .query_row(
                "SELECT image, updated_at FROM profile_avatar WHERE id=1",
                [],
                |row| {
                    Ok(ProfileAvatarRecord {
                        image: row.get(0)?,
                        updated_at: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(|e| e.to_string())?;
        match avatar {
            Some(record) => {
                let bytes = serde_json::to_vec(&record).map_err(|e| e.to_string())?;
                stage_dirty_record(
                    &tx,
                    &mut dirty_records,
                    PROFILE_AVATAR_RECORD_ID,
                    "profile",
                    bytes,
                )?;
            }
            None => {
                let existing: Option<i64> = tx
                    .query_row(
                        "SELECT revision FROM sync_records WHERE record_id=?1 AND kind='profile' AND tombstone=0",
                        params![PROFILE_AVATAR_RECORD_ID],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|e| e.to_string())?;
                if let Some(revision) = existing {
                    tx.execute(
                        "UPDATE sync_records SET dirty=1, tombstone=1, content_hash=NULL, updated_at=CURRENT_TIMESTAMP WHERE record_id=?1",
                        params![PROFILE_AVATAR_RECORD_ID],
                    )
                    .map_err(|e| e.to_string())?;
                    dirty_records.push(DirtySyncRecord {
                        record_id: PROFILE_AVATAR_RECORD_ID.to_string(),
                        kind: "profile".into(),
                        expected_revision: revision,
                        payload: None,
                        tombstone: true,
                    });
                }
            }
        }

        let fav_posts = collect(
            &tx,
            "SELECT pin.service, pin.creator_id, pin.post_id, p.snapshot_json, MAX(pin.created_at)
             FROM content_pins pin
             JOIN posts p USING(service, creator_id, post_id)
             WHERE pin.entity_kind = 'post' AND pin.reason = 'favorite'
             GROUP BY pin.service, pin.creator_id, pin.post_id",
            |r| {
                let raw_snapshot: String = r.get(3)?;
                Ok(FavoritePostRecord {
                    service: r.get(0)?,
                    creator_id: r.get(1)?,
                    post_id: r.get(2)?,
                    snapshot_json: favorite_post_snapshot(&raw_snapshot),
                    created_at: r.get(4)?,
                })
            },
        )?;
        let mut live_fav_post_ids = std::collections::HashSet::new();
        for fp in fav_posts {
            let record_id = favorite_post_record_id(&fp.service, &fp.creator_id, &fp.post_id);
            live_fav_post_ids.insert(record_id.clone());
            let bytes = serde_json::to_vec(&fp).map_err(|e| e.to_string())?;
            let hash = stable_sync_hash("fav_post", &record_id, &bytes);

            let current: Option<(i64, Option<String>, bool, bool)> = tx
                .query_row(
                    "SELECT revision, content_hash, dirty, tombstone FROM sync_records WHERE record_id=?1",
                    params![record_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0, r.get::<_, i64>(3)? != 0)),
                )
                .optional()
                .map_err(|e| e.to_string())?;

            match current {
                Some((rev, existing_hash, dirty, tombstone)) => {
                    let changed = existing_hash.as_deref() != Some(&hash) || tombstone;
                    if changed || dirty {
                        tx.execute(
                            "UPDATE sync_records SET content_hash=?1, dirty=1, tombstone=0, updated_at=CURRENT_TIMESTAMP WHERE record_id=?2",
                            params![hash, record_id],
                        )
                        .map_err(|e| e.to_string())?;
                        dirty_records.push(DirtySyncRecord {
                            record_id,
                            kind: "fav_post".into(),
                            expected_revision: rev,
                            payload: Some(bytes),
                            tombstone: false,
                        });
                    }
                }
                None => {
                    tx.execute(
                        "INSERT INTO sync_records(record_id,kind,revision,content_hash,dirty,tombstone,updated_at) VALUES(?1,'fav_post',0,?2,1,0,CURRENT_TIMESTAMP)",
                        params![record_id, hash],
                    )
                    .map_err(|e| e.to_string())?;
                    dirty_records.push(DirtySyncRecord {
                        record_id,
                        kind: "fav_post".into(),
                        expected_revision: 0,
                        payload: Some(bytes),
                        tombstone: false,
                    });
                }
            }
        }
        let existing_fav_posts = collect(
            &tx,
            "SELECT record_id, revision FROM sync_records WHERE kind='fav_post' AND tombstone=0",
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
        )?;
        for (rec_id, rev) in existing_fav_posts {
            if !live_fav_post_ids.contains(&rec_id) {
                tx.execute(
                    "UPDATE sync_records SET dirty=1, tombstone=1, content_hash=NULL, updated_at=CURRENT_TIMESTAMP WHERE record_id=?1",
                    params![rec_id],
                )
                .map_err(|e| e.to_string())?;
                dirty_records.push(DirtySyncRecord {
                    record_id: rec_id,
                    kind: "fav_post".into(),
                    expected_revision: rev,
                    payload: None,
                    tombstone: true,
                });
            }
        }

        let fav_creators = collect(
            &tx,
            "SELECT pin.service, pin.creator_id, c.name, c.snapshot_json, MAX(pin.created_at)
             FROM content_pins pin
             JOIN creators c USING(service, creator_id)
             WHERE pin.entity_kind = 'creator' AND pin.reason = 'favorite'
             GROUP BY pin.service, pin.creator_id",
            |r| {
                let raw_snapshot: String = r.get(3)?;
                Ok(FavoriteCreatorRecord {
                    service: r.get(0)?,
                    creator_id: r.get(1)?,
                    name: r.get(2)?,
                    snapshot_json: favorite_creator_snapshot(&raw_snapshot),
                    created_at: r.get(4)?,
                })
            },
        )?;
        let mut live_fav_creator_ids = std::collections::HashSet::new();
        for fc in fav_creators {
            let record_id = favorite_creator_record_id(&fc.service, &fc.creator_id);
            live_fav_creator_ids.insert(record_id.clone());
            let bytes = serde_json::to_vec(&fc).map_err(|e| e.to_string())?;
            let hash = stable_sync_hash("fav_creator", &record_id, &bytes);

            let current: Option<(i64, Option<String>, bool, bool)> = tx
                .query_row(
                    "SELECT revision, content_hash, dirty, tombstone FROM sync_records WHERE record_id=?1",
                    params![record_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0, r.get::<_, i64>(3)? != 0)),
                )
                .optional()
                .map_err(|e| e.to_string())?;

            match current {
                Some((rev, existing_hash, dirty, tombstone)) => {
                    let changed = existing_hash.as_deref() != Some(&hash) || tombstone;
                    if changed || dirty {
                        tx.execute(
                            "UPDATE sync_records SET content_hash=?1, dirty=1, tombstone=0, updated_at=CURRENT_TIMESTAMP WHERE record_id=?2",
                            params![hash, record_id],
                        )
                        .map_err(|e| e.to_string())?;
                        dirty_records.push(DirtySyncRecord {
                            record_id,
                            kind: "fav_creator".into(),
                            expected_revision: rev,
                            payload: Some(bytes),
                            tombstone: false,
                        });
                    }
                }
                None => {
                    tx.execute(
                        "INSERT INTO sync_records(record_id,kind,revision,content_hash,dirty,tombstone,updated_at) VALUES(?1,'fav_creator',0,?2,1,0,CURRENT_TIMESTAMP)",
                        params![record_id, hash],
                    )
                    .map_err(|e| e.to_string())?;
                    dirty_records.push(DirtySyncRecord {
                        record_id,
                        kind: "fav_creator".into(),
                        expected_revision: 0,
                        payload: Some(bytes),
                        tombstone: false,
                    });
                }
            }
        }
        let existing_fav_creators = collect(
            &tx,
            "SELECT record_id, revision FROM sync_records WHERE kind='fav_creator' AND tombstone=0",
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
        )?;
        for (rec_id, rev) in existing_fav_creators {
            if !live_fav_creator_ids.contains(&rec_id) {
                tx.execute(
                    "UPDATE sync_records SET dirty=1, tombstone=1, content_hash=NULL, updated_at=CURRENT_TIMESTAMP WHERE record_id=?1",
                    params![rec_id],
                )
                .map_err(|e| e.to_string())?;
                dirty_records.push(DirtySyncRecord {
                    record_id: rec_id,
                    kind: "fav_creator".into(),
                    expected_revision: rev,
                    payload: None,
                    tombstone: true,
                });
            }
        }

        tx.commit().map_err(|e| e.to_string())?;
        Ok(dirty_records)
    }

    pub fn mark_records_synced(
        &self,
        accepted: &[crate::sync::client::AcceptedRecord],
    ) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        for rec in accepted {
            connection
                .execute(
                    "UPDATE sync_records SET revision=?1, dirty=0, updated_at=CURRENT_TIMESTAMP WHERE record_id=?2",
                    params![rec.revision, rec.record_id],
                )
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn apply_remote_change(
        &self,
        record_id: &str,
        kind: &str,
        revision: i64,
        plaintext: Option<&[u8]>,
        tombstone: bool,
    ) -> Result<(), String> {
        let mut connection = self.connection.lock().map_err(|e| e.to_string())?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        Self::apply_change(&tx, record_id, kind, revision, plaintext, tombstone)?;
        tx.commit()
            .map_err(|e| format!("Failed to commit sync change ({record_id}): {e}"))
    }

    pub fn apply_remote_changes(&self, changes: &[RemoteApply<'_>]) -> Result<(), String> {
        let mut connection = self.connection.lock().map_err(|e| e.to_string())?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        for change in changes {
            Self::apply_change(
                &tx,
                change.record_id,
                change.kind,
                change.revision,
                change.plaintext.as_deref(),
                change.tombstone,
            )?;
        }
        tx.commit()
            .map_err(|e| format!("Failed to commit {} sync changes: {e}", changes.len()))
    }

    fn apply_change(
        tx: &rusqlite::Transaction<'_>,
        record_id: &str,
        kind: &str,
        revision: i64,
        plaintext: Option<&[u8]>,
        tombstone: bool,
    ) -> Result<(), String> {
        if tombstone {
            match kind {
                "collection" => {
                    let id = record_id.strip_prefix("col:").unwrap_or(record_id);
                    tx.execute(
                        "DELETE FROM collection_posts WHERE collection_id=?1",
                        params![id],
                    )
                    .map_err(|e| {
                        format!(
                            "Failed to clean up collection posts for tombstone ({record_id}): {e}"
                        )
                    })?;
                    tx.execute(
                        "UPDATE collections SET parent_id=NULL WHERE parent_id=?1",
                        params![id],
                    )
                    .map_err(|e| {
                        format!(
                            "Failed to detach child collections for tombstone ({record_id}): {e}"
                        )
                    })?;
                    tx.execute(
                        "DELETE FROM collections WHERE id=?1 AND is_system=0",
                        params![id],
                    )
                    .map_err(|e| {
                        format!("Failed to delete collection for tombstone ({record_id}): {e}")
                    })?;
                }
                "membership" => {
                    let raw = record_id.strip_prefix("mem:").unwrap_or(record_id);
                    let parts: Vec<&str> = raw.splitn(4, ':').collect();
                    if parts.len() == 4 {
                        tx.execute(
                            "DELETE FROM collection_posts WHERE collection_id=?1 AND service=?2 AND creator_id=?3 AND post_id=?4",
                            params![parts[0], parts[1], parts[2], parts[3]],
                        )
                        .map_err(|e| format!("Failed to delete membership for tombstone ({record_id}): {e}"))?;
                    } else {
                        return Err(format!("Malformed membership record ID: {record_id}"));
                    }
                }
                "subscription" => {
                    let id = record_id.strip_prefix("sub:").unwrap_or(record_id);
                    tx.execute("DELETE FROM subscriptions WHERE id=?1", params![id])
                        .map_err(|e| {
                            format!(
                                "Failed to delete subscription for tombstone ({record_id}): {e}"
                            )
                        })?;
                }
                "fav_post" => {
                    let raw = record_id.strip_prefix("fav:post:").unwrap_or(record_id);
                    let parts: Vec<&str> = raw.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        tx.execute(
                            "DELETE FROM content_pins WHERE entity_kind='post' AND reason='favorite' AND service=?1 AND creator_id=?2 AND post_id=?3",
                            params![parts[0], parts[1], parts[2]],
                        )
                        .map_err(|e| format!("Failed to delete favorite post for tombstone ({record_id}): {e}"))?;
                    } else {
                        return Err(format!("Malformed favorite post record ID: {record_id}"));
                    }
                }
                "fav_creator" => {
                    let raw = record_id.strip_prefix("fav:creator:").unwrap_or(record_id);
                    let parts: Vec<&str> = raw.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        tx.execute(
                            "DELETE FROM content_pins WHERE entity_kind='creator' AND reason='favorite' AND service=?1 AND creator_id=?2",
                            params![parts[0], parts[1]],
                        )
                        .map_err(|e| format!("Failed to delete favorite creator for tombstone ({record_id}): {e}"))?;
                    } else {
                        return Err(format!("Malformed favorite creator record ID: {record_id}"));
                    }
                }
                "post" | "session" => {}
                "profile" => {
                    tx.execute("DELETE FROM profile_avatar WHERE id=1", [])
                        .map_err(|e| {
                            format!(
                                "Failed to clear profile avatar for tombstone ({record_id}): {e}"
                            )
                        })?;
                }
                unknown => {
                    tracing::warn!(kind = %unknown, record_id = %record_id, "Skipping tombstone for unsupported sync record kind");
                    return Ok(());
                }
            }
            tx.execute(
                "INSERT INTO sync_records(record_id,kind,revision,content_hash,dirty,tombstone,updated_at)
                 VALUES(?1,?2,?3,NULL,0,1,CURRENT_TIMESTAMP)
                 ON CONFLICT(record_id) DO UPDATE SET revision=max(revision,?3), content_hash=NULL, dirty=0, tombstone=1, updated_at=CURRENT_TIMESTAMP",
                params![record_id, kind, revision],
            )
            .map_err(|e| format!("Failed to update tombstone sync_records ({record_id}): {e}"))?;
            return Ok(());
        }

        let bytes =
            plaintext.ok_or_else(|| "Missing plaintext payload for live record".to_string())?;
        let hash = stable_sync_hash(kind, record_id, bytes);

        match kind {
            "collection" => {
                let rec: CollectionRecord = serde_json::from_slice(bytes).map_err(|e| {
                    format!("Failed to deserialize collection record ({record_id}): {e}")
                })?;
                if let Some(ref pid) = rec.parent_id {
                    tx.execute(
                        "INSERT INTO collections(id,kind,parent_id,name,position,is_system,created_at,updated_at)
                         VALUES(?1,'folder',NULL,'Folder',0,0,'1970-01-01T00:00:00Z','1970-01-01T00:00:00Z')
                         ON CONFLICT(id) DO NOTHING",
                        params![pid],
                    )
                    .map_err(|e| format!("Failed to ensure placeholder parent collection {pid} ({record_id}): {e}"))?;
                }
                tx.execute(
                    "INSERT INTO collections(id,kind,parent_id,name,position,is_system,created_at,updated_at,color)
                     VALUES(?1,?2,?3,?4,?5,0,?6,?7,?8)
                     ON CONFLICT(id) DO UPDATE SET
                       kind=excluded.kind, parent_id=excluded.parent_id, name=excluded.name,
                       position=excluded.position, updated_at=excluded.updated_at, color=excluded.color
                       WHERE collections.updated_at <= excluded.updated_at OR collections.updated_at = '1970-01-01T00:00:00Z'",
                    params![rec.id, rec.kind, rec.parent_id, rec.name, rec.position, rec.created_at, rec.updated_at, rec.color],
                )
                .map_err(|e| format!("Failed to insert/update collection ({record_id}): {e}"))?;
            }
            "post" => {
                let rec: PostRecord = serde_json::from_slice(bytes)
                    .map_err(|e| format!("Failed to deserialize post record ({record_id}): {e}"))?;
                let (post, incoming) = without_import_state(&rec.snapshot_json)
                    .ok_or_else(|| format!("Failed to parse post snapshot ({record_id})"))?;
                tx.execute(
                    "INSERT INTO creators(service,creator_id,name,snapshot_json)
                     VALUES(?1,?2,?2,json_object('id',?2,'name',?2,'service',?1))
                     ON CONFLICT(service,creator_id) DO NOTHING",
                    params![rec.service, rec.creator_id],
                )
                .map_err(|e| format!("Failed to ensure creator for post ({record_id}): {e}"))?;
                let snapshot = crate::db::content::post_snapshot_to_store(
                    tx,
                    (&rec.service, &rec.creator_id, &rec.post_id),
                    &post,
                    incoming,
                )?;
                tx.execute(
                    "INSERT INTO posts(service,creator_id,post_id,title,content,published_at,snapshot_json,cached_at)
                     VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
                     ON CONFLICT(service,creator_id,post_id) DO UPDATE SET
                       title=excluded.title,
                       content=coalesce(posts.content, excluded.content),
                       published_at=excluded.published_at,
                       snapshot_json=excluded.snapshot_json",
                    params![
                        rec.service,
                        rec.creator_id,
                        rec.post_id,
                        rec.title,
                        post.content,
                        rec.published_at,
                        snapshot,
                        rec.cached_at
                    ],
                )
                .map_err(|e| format!("Failed to insert/update post ({record_id}): {e}"))?;
            }
            "membership" => {
                let rec: MembershipRecord = serde_json::from_slice(bytes).map_err(|e| {
                    format!("Failed to deserialize membership record ({record_id}): {e}")
                })?;
                tx.execute(
                    "INSERT INTO collections(id,kind,parent_id,name,position,is_system,created_at,updated_at)
                     VALUES(?1,'folder',NULL,'Folder',0,0,'1970-01-01T00:00:00Z','1970-01-01T00:00:00Z')
                     ON CONFLICT(id) DO NOTHING",
                    params![rec.collection_id],
                )
                .map_err(|e| format!("Failed to ensure collection for membership ({record_id}): {e}"))?;
                tx.execute(
                    "INSERT INTO creators(service,creator_id,name,snapshot_json)
                     VALUES(?1,?2,?2,json_object('id',?2,'name',?2,'service',?1))
                     ON CONFLICT(service,creator_id) DO NOTHING",
                    params![rec.service, rec.creator_id],
                )
                .map_err(|e| {
                    format!("Failed to ensure creator for membership ({record_id}): {e}")
                })?;
                tx.execute(
                    "INSERT INTO posts(service,creator_id,post_id,title,content,published_at,snapshot_json,cached_at)
                     VALUES(?1,?2,?3,'','',NULL,'{}','1970-01-01T00:00:00Z')
                     ON CONFLICT(service,creator_id,post_id) DO NOTHING",
                    params![rec.service, rec.creator_id, rec.post_id],
                )
                .map_err(|e| format!("Failed to ensure post for membership ({record_id}): {e}"))?;
                tx.execute(
                    "INSERT INTO collection_posts(collection_id,service,creator_id,post_id,position,operation_id,added_at)
                     VALUES(?1,?2,?3,?4,?5,?6,?7)
                     ON CONFLICT(collection_id,service,creator_id,post_id) DO UPDATE SET
                       position=excluded.position, operation_id=excluded.operation_id, added_at=excluded.added_at",
                    params![
                        rec.collection_id,
                        rec.service,
                        rec.creator_id,
                        rec.post_id,
                        rec.position,
                        rec.operation_id,
                        rec.added_at
                    ],
                )
                .map_err(|e| format!("Failed to insert/update collection_posts ({record_id}): {e}"))?;
            }
            "subscription" => {
                let rec: SubscriptionRecord = serde_json::from_slice(bytes).map_err(|e| {
                    format!("Failed to deserialize subscription record ({record_id}): {e}")
                })?;
                tx.execute(
                    "INSERT INTO collections(id,kind,parent_id,name,position,is_system,created_at,updated_at)
                     VALUES(?1,'folder',NULL,'Folder',0,0,'1970-01-01T00:00:00Z','1970-01-01T00:00:00Z')
                     ON CONFLICT(id) DO NOTHING",
                    params![rec.destination_collection_id],
                )
                .map_err(|e| format!("Failed to ensure collection for subscription ({record_id}): {e}"))?;
                tx.execute(
                    "INSERT INTO subscriptions(id,service,creator_id,creator_name,destination_collection_id,save_to_library,enabled,initial_import,auto_download,download_scope,poll_interval_minutes,next_check_at,created_at,updated_at)
                     VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,CURRENT_TIMESTAMP,?12,?13)
                     ON CONFLICT(service,creator_id) DO UPDATE SET
                       destination_collection_id=excluded.destination_collection_id,
                       save_to_library=excluded.save_to_library,
                       enabled=excluded.enabled, initial_import=excluded.initial_import,
                       auto_download=excluded.auto_download, download_scope=excluded.download_scope,
                       poll_interval_minutes=excluded.poll_interval_minutes, updated_at=excluded.updated_at
                       WHERE subscriptions.updated_at <= excluded.updated_at",
                    params![
                        rec.id,
                        rec.service,
                        rec.creator_id,
                        rec.creator_name,
                        rec.destination_collection_id,
                        rec.save_to_library as i64,
                        rec.enabled as i64,
                        rec.initial_import,
                        rec.auto_download as i64,
                        rec.download_scope,
                        rec.poll_interval_minutes,
                        rec.created_at,
                        rec.updated_at
                    ],
                )
                .map_err(|e| format!("Failed to insert/update subscription ({record_id}): {e}"))?;
            }
            // Applied by SyncManager after commit; opening settings here deadlocks on this transaction.
            "session" => {}
            "profile" => {
                let rec: ProfileAvatarRecord = serde_json::from_slice(bytes).map_err(|e| {
                    format!("Failed to deserialize profile avatar ({record_id}): {e}")
                })?;
                if !rec.image.starts_with("data:image/")
                    || rec.image.len() > MAX_AVATAR_DATA_URL_LEN
                {
                    return Err(format!("Invalid profile avatar payload ({record_id})"));
                }
                tx.execute(
                    "INSERT INTO profile_avatar(id,image,updated_at) VALUES(1,?1,?2)
                     ON CONFLICT(id) DO UPDATE SET image=excluded.image, updated_at=excluded.updated_at",
                    params![rec.image, rec.updated_at],
                )
                .map_err(|e| format!("Failed to store profile avatar ({record_id}): {e}"))?;
            }
            "fav_post" => {
                let rec: FavoritePostRecord = serde_json::from_slice(bytes).map_err(|e| {
                    format!("Failed to deserialize fav_post record ({record_id}): {e}")
                })?;
                if let Some((post, incoming)) = without_import_state(&rec.snapshot_json) {
                    tx.execute(
                        "INSERT INTO creators(service,creator_id,name,snapshot_json)
                         VALUES(?1,?2,?2,json_object('id',?2,'name',?2,'service',?1))
                         ON CONFLICT(service,creator_id) DO NOTHING",
                        params![rec.service, rec.creator_id],
                    )
                    .map_err(|e| {
                        format!("Failed to ensure creator for fav_post ({record_id}): {e}")
                    })?;
                    let snapshot = crate::db::content::post_snapshot_to_store(
                        tx,
                        (&rec.service, &rec.creator_id, &rec.post_id),
                        &post,
                        incoming,
                    )?;
                    tx.execute(
                        "INSERT INTO posts(service,creator_id,post_id,title,content,published_at,snapshot_json,cached_at)
                         VALUES(?1,?2,?3,?4,?5,?6,?7,CURRENT_TIMESTAMP)
                         ON CONFLICT(service,creator_id,post_id) DO UPDATE SET
                           title=coalesce(excluded.title, posts.title),
                           content=coalesce(excluded.content, posts.content),
                           published_at=coalesce(excluded.published_at, posts.published_at),
                           snapshot_json=excluded.snapshot_json",
                        params![
                            rec.service,
                            rec.creator_id,
                            rec.post_id,
                            post.title,
                            post.content,
                            post.published,
                            snapshot
                        ],
                    )
                    .map_err(|e| format!("Failed to insert/update post for fav_post ({record_id}): {e}"))?;
                    tx.execute(
                        "INSERT OR IGNORE INTO content_pins(entity_kind,service,creator_id,post_id,reason,account_id,created_at)
                         VALUES('post',?1,?2,?3,'favorite','',?4)",
                        params![rec.service, rec.creator_id, rec.post_id, rec.created_at],
                    )
                    .map_err(|e| format!("Failed to insert content_pins for fav_post ({record_id}): {e}"))?;
                }
            }
            "fav_creator" => {
                let rec: FavoriteCreatorRecord = serde_json::from_slice(bytes).map_err(|e| {
                    format!("Failed to deserialize fav_creator record ({record_id}): {e}")
                })?;
                tx.execute(
                    "INSERT INTO creators(service,creator_id,name,snapshot_json,cached_at)
                     VALUES(?1,?2,?3,?4,CURRENT_TIMESTAMP)
                     ON CONFLICT(service,creator_id) DO UPDATE SET
                       name=coalesce(excluded.name, creators.name),
                       snapshot_json=CASE WHEN json_valid(creators.snapshot_json)
                         THEN json_patch(creators.snapshot_json, excluded.snapshot_json)
                         ELSE excluded.snapshot_json END",
                    params![
                        rec.service,
                        rec.creator_id,
                        rec.name,
                        serde_json::from_str::<serde_json::Value>(&rec.snapshot_json)
                            .ok()
                            .and_then(|json| crate::db::storage::snapshot_json(&json).ok())
                            .unwrap_or(rec.snapshot_json)
                    ],
                )
                .map_err(|e| {
                    format!("Failed to insert/update creator for fav_creator ({record_id}): {e}")
                })?;
                tx.execute(
                    "INSERT OR IGNORE INTO content_pins(entity_kind,service,creator_id,post_id,reason,account_id,created_at)
                     VALUES('creator',?1,?2,'','favorite','',?3)",
                    params![rec.service, rec.creator_id, rec.created_at],
                )
                .map_err(|e| format!("Failed to insert content_pins for fav_creator ({record_id}): {e}"))?;
            }
            unknown => {
                tracing::warn!(
                    kind = %unknown,
                    record_id = %record_id,
                    "Skipping unsupported sync record kind to maintain forward compatibility"
                );
                return Ok(());
            }
        }

        tx.execute(
            "INSERT INTO sync_records(record_id,kind,revision,content_hash,dirty,tombstone,updated_at)
             VALUES(?1,?2,?3,?4,0,0,CURRENT_TIMESTAMP)
             ON CONFLICT(record_id) DO UPDATE SET revision=?3, content_hash=?4, dirty=0, tombstone=0, updated_at=CURRENT_TIMESTAMP",
            params![record_id, kind, revision, hash],
        )
        .map_err(|e| format!("Failed to update sync_records ({record_id}): {e}"))?;
        Ok(())
    }
}

pub struct RemoteApply<'a> {
    pub record_id: &'a str,
    pub kind: &'a str,
    pub revision: i64,
    pub plaintext: Option<Vec<u8>>,
    pub tombstone: bool,
}

fn stage_dirty_record(
    tx: &rusqlite::Transaction<'_>,
    dirty: &mut Vec<DirtySyncRecord>,
    record_id: &str,
    kind: &str,
    bytes: Vec<u8>,
) -> Result<(), String> {
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let current: Option<(i64, Option<String>, bool, bool)> = tx
        .query_row(
            "SELECT revision, content_hash, dirty, tombstone FROM sync_records WHERE record_id=?1",
            params![record_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get::<_, i64>(2)? != 0,
                    r.get::<_, i64>(3)? != 0,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;
    match current {
        Some((revision, stored_hash, dirty_flag, tombstone)) => {
            if stored_hash.as_deref() != Some(&hash) || dirty_flag || tombstone {
                tx.execute(
                    "UPDATE sync_records SET content_hash=?1, dirty=1, tombstone=0, updated_at=CURRENT_TIMESTAMP WHERE record_id=?2",
                    params![hash, record_id],
                )
                .map_err(|e| e.to_string())?;
                dirty.push(DirtySyncRecord {
                    record_id: record_id.to_string(),
                    kind: kind.to_string(),
                    expected_revision: revision,
                    payload: Some(bytes),
                    tombstone: false,
                });
            }
        }
        None => {
            tx.execute(
                "INSERT INTO sync_records(record_id,kind,revision,content_hash,dirty,tombstone,updated_at) VALUES(?1,?2,0,?3,1,0,CURRENT_TIMESTAMP)",
                params![record_id, kind, hash],
            )
            .map_err(|e| e.to_string())?;
            dirty.push(DirtySyncRecord {
                record_id: record_id.to_string(),
                kind: kind.to_string(),
                expected_revision: 0,
                payload: Some(bytes),
                tombstone: false,
            });
        }
    }
    Ok(())
}

fn collect<T, F>(connection: &Connection, sql: &str, mapper: F) -> Result<Vec<T>, String>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    let mut statement = connection.prepare(sql).map_err(|e| e.to_string())?;
    let rows = statement.query_map([], mapper).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::client::AcceptedRecord;

    const AT: &str = "2026-08-15T00:00:00Z";

    fn exec(repo: &SyncRepository, sql: &str) {
        repo.connection.lock().unwrap().execute_batch(sql).unwrap();
    }

    fn query<T: rusqlite::types::FromSql>(repo: &SyncRepository, sql: &str) -> T {
        repo.connection
            .lock()
            .unwrap()
            .query_row(sql, [], |r| r.get(0))
            .unwrap()
    }

    fn mark_synced(repo: &SyncRepository, dirty: &[DirtySyncRecord]) {
        let accepted: Vec<_> = dirty
            .iter()
            .enumerate()
            .map(|(i, d)| AcceptedRecord {
                record_id: d.record_id.clone(),
                revision: 1,
                position: i as i64 + 1,
            })
            .collect();
        repo.mark_records_synced(&accepted).unwrap();
    }

    fn apply<T: Serialize>(
        repo: &SyncRepository,
        record_id: &str,
        kind: &str,
        rev: i64,
        record: &T,
    ) {
        let bytes = serde_json::to_vec(record).unwrap();
        repo.apply_remote_change(record_id, kind, rev, Some(&bytes), false)
            .unwrap();
    }

    #[test]
    fn synced_favorites_never_mark_a_loaded_post_unimported() {
        let repo = SyncRepository::in_memory();
        exec(
            &repo,
            r#"INSERT INTO creators(service,creator_id,name,snapshot_json) VALUES('patreon','c','c','{}');
               INSERT INTO posts(service,creator_id,post_id,title,snapshot_json) VALUES('patreon','c','1','t',
                 '{"id":"1","user":"c","service":"patreon","title":"t","has_full":true,"preview_state":"scraped","detail_fetched":true}');"#,
        );
        let teaser = r#"{"id":"1","user":"c","service":"patreon","title":"t","has_full":false,"preview_state":"pending","detail_fetched":false}"#;
        apply(
            &repo,
            "fav:post:patreon:c:1",
            "fav_post",
            1,
            &FavoritePostRecord {
                service: "patreon".into(),
                creator_id: "c".into(),
                post_id: "1".into(),
                snapshot_json: teaser.into(),
                created_at: AT.into(),
            },
        );
        let has_full: Option<bool> = query(
            &repo,
            "SELECT json_extract(snapshot_json,'$.has_full') FROM posts WHERE post_id='1'",
        );
        assert_eq!(has_full, Some(true));

        let slim: serde_json::Value = serde_json::from_str(&slim_post_snapshot(teaser)).unwrap();
        assert!(slim["has_full"].is_null() && slim["preview_state"].is_null());
    }

    fn collection(id: &str, parent_id: Option<&str>, name: &str) -> CollectionRecord {
        CollectionRecord {
            id: id.into(),
            kind: if parent_id.is_some() {
                "folder"
            } else {
                "stash"
            }
            .into(),
            parent_id: parent_id.map(Into::into),
            name: name.into(),
            position: 0,
            created_at: AT.into(),
            updated_at: AT.into(),
            color: None,
        }
    }

    #[test]
    fn local_edits_stay_dirty_until_synced_and_deletes_become_tombstones() {
        let repo = SyncRepository::in_memory();
        exec(&repo, "INSERT INTO collections(id,kind,name,position,is_system) VALUES('col_01','stash','Art Stash',0,0)");

        let dirty = repo.detect_and_get_dirty_records().unwrap();
        assert_eq!(dirty.len(), 1);
        assert_eq!(dirty[0].record_id, "col:col_01");
        assert_eq!(dirty[0].kind, "collection");
        assert!(!dirty[0].tombstone && dirty[0].payload.is_some());

        mark_synced(&repo, &dirty);
        assert!(repo.detect_and_get_dirty_records().unwrap().is_empty());

        exec(&repo, "DELETE FROM collections WHERE id='col_01'");
        let deleted = repo.detect_and_get_dirty_records().unwrap();
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].record_id, "col:col_01");
        assert!(deleted[0].tombstone);
        assert_eq!(deleted[0].expected_revision, 1);
    }

    #[test]
    fn every_entity_kind_replicates_to_another_device() {
        let repo_a = SyncRepository::in_memory();
        let repo_b = SyncRepository::in_memory();
        exec(
            &repo_a,
            r#"
            INSERT INTO collections(id,kind,name,position,is_system) VALUES('stash_01','stash','My Stash',0,0);
            INSERT INTO creators(service,creator_id,name,snapshot_json)
                VALUES('fanbox','c1','Artist 1','{"id":"c1","name":"Artist 1","service":"fanbox"}');
            INSERT INTO posts(service,creator_id,post_id,title,content,published_at,snapshot_json)
                VALUES('fanbox','c1','p1','Artwork 1','Hello world','2026-08-15T00:00:00Z','{"id":"p1","title":"Artwork 1","user":"c1","service":"fanbox","content":"Hello world"}');
            INSERT INTO collection_posts(collection_id,service,creator_id,post_id,position,operation_id,added_at)
                VALUES('stash_01','fanbox','c1','p1',0,'op_01','2026-08-15T00:00:00Z');
            INSERT INTO subscriptions(id,service,creator_id,creator_name,destination_collection_id,enabled,initial_import,auto_download,download_scope,poll_interval_minutes)
                VALUES('sub_01','fanbox','c1','Artist 1','stash_01',1,'none',0,'primary',30);
            INSERT INTO content_pins(entity_kind,service,creator_id,post_id,reason,account_id)
                VALUES('post','fanbox','c1','p1','favorite',''), ('creator','fanbox','c1','','favorite','');
            "#,
        );

        let dirty = repo_a.detect_and_get_dirty_records().unwrap();
        assert_eq!(dirty.len(), 6);
        mark_synced(&repo_a, &dirty);
        for d in &dirty {
            repo_b
                .apply_remote_change(&d.record_id, &d.kind, 1, d.payload.as_deref(), d.tombstone)
                .unwrap();
        }

        assert_eq!(
            query::<String>(&repo_b, "SELECT name FROM collections WHERE id='stash_01'"),
            "My Stash"
        );
        assert_eq!(
            query::<String>(&repo_b, "SELECT title FROM posts WHERE post_id='p1'"),
            "Artwork 1"
        );
        assert_eq!(
            query::<String>(
                &repo_b,
                "SELECT creator_name FROM subscriptions WHERE id='sub_01'"
            ),
            "Artist 1"
        );
        for sql in [
            "SELECT COUNT(*) FROM collection_posts WHERE collection_id='stash_01'",
            "SELECT COUNT(*) FROM content_pins WHERE entity_kind='post' AND reason='favorite' AND post_id='p1'",
            "SELECT COUNT(*) FROM content_pins WHERE entity_kind='creator' AND reason='favorite' AND creator_id='c1'",
        ] {
            assert_eq!(query::<i64>(&repo_b, sql), 1, "{sql}");
        }
        assert!(repo_b.detect_and_get_dirty_records().unwrap().is_empty());

        repo_b
            .apply_remote_change("col:stash_01", "collection", 2, None, true)
            .unwrap();
        assert_eq!(
            query::<i64>(
                &repo_b,
                "SELECT COUNT(*) FROM collections WHERE id='stash_01'"
            ),
            0
        );
    }

    #[test]
    fn remote_children_may_arrive_before_their_parents() {
        let repo = SyncRepository::in_memory();
        let membership = MembershipRecord {
            collection_id: "col_future".into(),
            service: "kemono".into(),
            creator_id: "creator_future".into(),
            post_id: "post_future".into(),
            position: 0,
            operation_id: "op_01".into(),
            added_at: AT.into(),
        };
        let subscription = SubscriptionRecord {
            id: "sub_future".into(),
            service: "kemono".into(),
            creator_id: "creator_future".into(),
            creator_name: "Future Creator".into(),
            destination_collection_id: "col_sub_dest".into(),
            save_to_library: true,
            enabled: true,
            initial_import: "none".into(),
            auto_download: false,
            download_scope: "primary".into(),
            poll_interval_minutes: 30,
            created_at: AT.into(),
            updated_at: AT.into(),
        };
        let post = PostRecord {
            service: "kemono".into(),
            creator_id: "creator_future".into(),
            post_id: "post_future".into(),
            title: "Future Post".into(),
            published_at: None,
            snapshot_json: r#"{"id":"post_future","title":"Future Post","user":"creator_future","service":"kemono"}"#.into(),
            cached_at: AT.into(),
        };

        apply(
            &repo,
            "mem:col_future:kemono:creator_future:post_future",
            "membership",
            1,
            &membership,
        );
        apply(
            &repo,
            "col:child_col",
            "collection",
            2,
            &collection("child_col", Some("parent_col"), "Child Folder"),
        );
        apply(&repo, "sub:sub_future", "subscription", 3, &subscription);
        apply(
            &repo,
            "post:kemono:creator_future:post_future",
            "post",
            4,
            &post,
        );
        apply(
            &repo,
            "col:col_future",
            "collection",
            5,
            &collection("col_future", None, "Future Stash"),
        );

        assert_eq!(
            query::<String>(&repo, "SELECT name FROM collections WHERE id='col_future'"),
            "Future Stash"
        );
        assert_eq!(
            query::<String>(&repo, "SELECT title FROM posts WHERE post_id='post_future'"),
            "Future Post"
        );
        assert_eq!(
            query::<i64>(&repo, "SELECT COUNT(*) FROM collection_posts WHERE collection_id='col_future' AND post_id='post_future'"),
            1
        );
    }

    #[test]
    fn slim_post_snapshot_drops_bodies_but_keeps_the_card_preview() {
        let slim = slim_post_snapshot(
            r#"{
                "id": "123", "user": "creator_a", "service": "fanbox", "title": "A Great Post",
                "content": "<p>Massive HTML body</p>", "published": "2026-09-08T12:00:00Z",
                "file": {"name": "thumb.jpg", "path": "/files/thumb.jpg"},
                "attachments": [{"name": "image1.png", "path": "/files/image1.png"}],
                "poll": {"title": "huge poll object"}, "embed": {"html": "iframe content"},
                "extra": {"huge_dump": true}
            }"#,
        );
        for dropped in [
            "Massive HTML body",
            "huge poll object",
            "iframe content",
            "huge_dump",
        ] {
            assert!(!slim.contains(dropped), "{dropped}");
        }

        let parsed: crate::api::models::Post = serde_json::from_str(&slim).unwrap();
        assert_eq!(
            (parsed.id.as_str(), parsed.title.as_str()),
            ("123", "A Great Post")
        );
        assert_eq!(parsed.content, None);
        assert_eq!(parsed.detail_fetched, Some(false));
        assert_eq!(parsed.file.unwrap().name.as_deref(), Some("thumb.jpg"));
    }

    #[test]
    fn favorite_snapshots_ignore_volatile_provider_fields() {
        let first = r#"{
            "id":"p1","user":"c1","service":"fanbox","title":"Art",
            "published":"2026-08-01T00:00:00Z","favorite_count":10,"attachment_count":2,
            "preview_path":"/cache/a.jpg","thumbnail_url":"https://cdn.example/x.jpg?sig=1",
            "file":{"name":"a.jpg","path":"/a.jpg","url":"https://cdn.example/a?sig=1",
                    "thumbnail_url":"https://cdn.example/a_t.jpg?sig=1"}
        }"#;
        let second = r#"{
            "id":"p1","user":"c1","service":"fanbox","title":"Art",
            "published":"2026-08-01T00:00:00Z","favorite_count":99,"attachment_count":3,
            "preview_path":"/cache/b.jpg","thumbnail_url":"https://cdn.example/x.jpg?sig=2",
            "file":{"name":"a.jpg","path":"/a.jpg","url":"https://cdn.example/a?sig=2",
                    "thumbnail_url":"https://cdn.example/a_t.jpg?sig=2"}
        }"#;
        let snapshot = favorite_post_snapshot(first);
        assert_eq!(snapshot, favorite_post_snapshot(second));
        assert!(!snapshot.contains("/cache/"));
        assert!(!snapshot.contains("sig="));
        let parsed: crate::api::models::Post = serde_json::from_str(&snapshot).unwrap();
        assert_eq!(parsed.title, "Art");
        assert_eq!(parsed.file.unwrap().path.as_deref(), Some("/a.jpg"));

        let creator_first = r#"{
            "id":"c1","name":"Artist","service":"fanbox",
            "avatar_url":"https://cdn.example/a.png","avatar_path":"/cache/a.png",
            "updated":"2026-08-01","indexed":5,"extra":{"followers":10}
        }"#;
        let creator_second = r#"{
            "id":"c1","name":"Artist","service":"fanbox",
            "avatar_url":"https://cdn.example/a.png","avatar_path":"/cache/b.png",
            "updated":"2026-09-01","indexed":9,"extra":{"followers":12}
        }"#;
        let creator = favorite_creator_snapshot(creator_first);
        assert_eq!(creator, favorite_creator_snapshot(creator_second));
        assert!(!creator.contains("avatar_path"));
        assert!(!creator.contains("followers"));
        assert!(creator.contains("avatar_url"));
    }

    #[test]
    fn favorite_hashes_ignore_snapshot_drift() {
        let post = FavoritePostRecord {
            service: "fanbox".into(),
            creator_id: "c".into(),
            post_id: "p".into(),
            snapshot_json: r#"{"title":"A","favorite_count":1}"#.into(),
            created_at: AT.into(),
        };
        let mut drifted = post.clone();
        drifted.snapshot_json = r#"{"title":"A","favorite_count":99,"attachments":[1,2]}"#.into();
        let post_id = favorite_post_record_id("fanbox", "c", "p");
        assert_eq!(
            stable_sync_hash("fav_post", &post_id, &serde_json::to_vec(&post).unwrap()),
            stable_sync_hash("fav_post", &post_id, &serde_json::to_vec(&drifted).unwrap())
        );
        let mut repinned = post.clone();
        repinned.created_at = "2026-08-16T00:00:00Z".into();
        assert_ne!(
            stable_sync_hash("fav_post", &post_id, &serde_json::to_vec(&post).unwrap()),
            stable_sync_hash(
                "fav_post",
                &post_id,
                &serde_json::to_vec(&repinned).unwrap()
            )
        );

        let creator = FavoriteCreatorRecord {
            service: "fanbox".into(),
            creator_id: "c".into(),
            name: "N".into(),
            snapshot_json: r#"{"indexed":1}"#.into(),
            created_at: AT.into(),
        };
        let mut creator_drift = creator.clone();
        creator_drift.snapshot_json = r#"{"indexed":2,"updated":"2026-09-01"}"#.into();
        let creator_id = favorite_creator_record_id("fanbox", "c");
        assert_eq!(
            stable_sync_hash(
                "fav_creator",
                &creator_id,
                &serde_json::to_vec(&creator).unwrap()
            ),
            stable_sync_hash(
                "fav_creator",
                &creator_id,
                &serde_json::to_vec(&creator_drift).unwrap()
            )
        );
    }

    #[test]
    fn duplicate_favorite_pins_stage_one_record() {
        let repo = SyncRepository::in_memory();
        exec(
            &repo,
            r#"
            INSERT INTO creators(service,creator_id,name,snapshot_json) VALUES('fanbox','c','c','{}');
            INSERT INTO posts(service,creator_id,post_id,title,snapshot_json)
                VALUES('fanbox','c','p','Title','{"id":"p","user":"c","service":"fanbox","title":"Title"}');
            INSERT INTO content_pins(entity_kind,service,creator_id,post_id,reason,account_id,created_at)
                VALUES('post','fanbox','c','p','favorite','','2026-08-15T00:00:00Z'),
                      ('post','fanbox','c','p','favorite','nichind','2026-08-16T00:00:00Z');
            "#,
        );
        let dirty = repo.detect_and_get_dirty_records().unwrap();
        let favorites: Vec<_> = dirty
            .iter()
            .filter(|record| record.kind == "fav_post")
            .collect();
        assert_eq!(favorites.len(), 1);
        let payload: FavoritePostRecord =
            serde_json::from_slice(favorites[0].payload.as_deref().unwrap()).unwrap();
        assert_eq!(payload.created_at, "2026-08-16T00:00:00Z");

        mark_synced(&repo, &dirty);
        assert!(repo.detect_and_get_dirty_records().unwrap().is_empty());
    }

    #[test]
    fn favorite_snapshot_drift_does_not_repush() {
        let repo = SyncRepository::in_memory();
        exec(
            &repo,
            r#"
            INSERT INTO creators(service,creator_id,name,snapshot_json) VALUES('fanbox','c','c','{}');
            INSERT INTO posts(service,creator_id,post_id,title,snapshot_json)
                VALUES('fanbox','c','p','Title','{"id":"p","user":"c","service":"fanbox","title":"Title"}');
            INSERT INTO content_pins(entity_kind,service,creator_id,post_id,reason,account_id)
                VALUES('post','fanbox','c','p','favorite','');
            "#,
        );
        let dirty = repo.detect_and_get_dirty_records().unwrap();
        assert_eq!(dirty.len(), 1);
        assert_eq!(dirty[0].kind, "fav_post");
        mark_synced(&repo, &dirty);

        exec(
            &repo,
            "UPDATE posts SET snapshot_json = json_set(snapshot_json, '$.favorite_count', 42, '$.preview_path', '/cache/new.jpg') WHERE post_id='p'",
        );
        assert!(repo.detect_and_get_dirty_records().unwrap().is_empty());

        exec(
            &repo,
            "UPDATE content_pins SET created_at='2026-08-16T00:00:00Z' WHERE post_id='p'",
        );
        assert_eq!(repo.detect_and_get_dirty_records().unwrap().len(), 1);
    }

    #[test]
    fn avatar_syncs_once_and_clears_with_a_tombstone() {
        let repo = SyncRepository::in_memory();
        let image = "data:image/jpeg;base64,AAAA";
        repo.save_avatar(image).unwrap();

        let dirty = repo.detect_and_get_dirty_records().unwrap();
        assert_eq!(dirty.len(), 1);
        assert_eq!(dirty[0].record_id, PROFILE_AVATAR_RECORD_ID);
        assert_eq!(dirty[0].kind, "profile");
        assert!(!dirty[0].tombstone);
        let payload: ProfileAvatarRecord =
            serde_json::from_slice(dirty[0].payload.as_deref().unwrap()).unwrap();
        assert_eq!(payload.image, image);

        mark_synced(&repo, &dirty);
        assert!(repo.detect_and_get_dirty_records().unwrap().is_empty());

        repo.clear_avatar().unwrap();
        let cleared = repo.detect_and_get_dirty_records().unwrap();
        assert_eq!(cleared.len(), 1);
        assert!(cleared[0].tombstone);
        assert_eq!(cleared[0].expected_revision, 1);
    }

    #[test]
    fn remote_avatar_applies_without_echoing_back() {
        let repo = SyncRepository::in_memory();
        let record = ProfileAvatarRecord {
            image: "data:image/png;base64,BBBB".into(),
            updated_at: AT.into(),
        };
        apply(&repo, PROFILE_AVATAR_RECORD_ID, "profile", 1, &record);
        assert_eq!(
            repo.avatar().unwrap().as_deref(),
            Some(record.image.as_str())
        );
        assert!(repo.detect_and_get_dirty_records().unwrap().is_empty());

        repo.apply_remote_change(PROFILE_AVATAR_RECORD_ID, "profile", 2, None, true)
            .unwrap();
        assert!(repo.avatar().unwrap().is_none());
        assert!(repo.detect_and_get_dirty_records().unwrap().is_empty());
    }
}
