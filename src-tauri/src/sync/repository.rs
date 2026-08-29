use crate::db::storage::open_database;
#[cfg(test)]
use crate::db::storage::prepare_connection;
use rusqlite::{params, Connection, OptionalExtension};
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubscriptionRecord {
    pub id: String,
    pub service: String,
    pub creator_id: String,
    pub creator_name: String,
    pub destination_collection_id: String,
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

pub struct SyncRepository {
    connection: Mutex<Connection>,
}

impl SyncRepository {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            connection: Mutex::new(open_database()?),
        })
    }
    #[cfg(test)]
    pub fn in_memory() -> Self {
        let mut connection = Connection::open_in_memory().unwrap();
        prepare_connection(&mut connection).unwrap();
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

    pub fn detect_and_get_dirty_records(&self) -> Result<Vec<DirtySyncRecord>, String> {
        let mut connection = self.connection.lock().map_err(|e| e.to_string())?;
        let tx = connection.transaction().map_err(|e| e.to_string())?;
        let mut dirty_records = Vec::new();

        // 1. Collections (non-system)
        let collections = collect(
            &tx,
            "SELECT id,kind,parent_id,name,position,created_at,updated_at FROM collections WHERE is_system=0",
            |r| {
                Ok(CollectionRecord {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                    parent_id: r.get(2)?,
                    name: r.get(3)?,
                    position: r.get(4)?,
                    created_at: r.get(5)?,
                    updated_at: r.get(6)?,
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

        // 2. Posts (referenced in collection_posts)
        let posts = collect(
            &tx,
            "SELECT DISTINCT p.service,p.creator_id,p.post_id,p.title,p.published_at,p.snapshot_json,p.cached_at
             FROM posts p
             JOIN collection_posts cp USING(service,creator_id,post_id)",
            |r| {
                Ok(PostRecord {
                    service: r.get(0)?,
                    creator_id: r.get(1)?,
                    post_id: r.get(2)?,
                    title: r.get(3)?,
                    published_at: r.get(4)?,
                    snapshot_json: r.get(5)?,
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

        // 3. Memberships (collection_posts)
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

        // 4. Subscriptions
        let subscriptions = collect(
            &tx,
            "SELECT id,service,creator_id,creator_name,destination_collection_id,enabled,initial_import,auto_download,download_scope,poll_interval_minutes,created_at,updated_at FROM subscriptions",
            |r| {
                Ok(SubscriptionRecord {
                    id: r.get(0)?,
                    service: r.get(1)?,
                    creator_id: r.get(2)?,
                    creator_name: r.get(3)?,
                    destination_collection_id: r.get(4)?,
                    enabled: r.get::<_, i64>(5)? != 0,
                    initial_import: r.get(6)?,
                    auto_download: r.get::<_, i64>(7)? != 0,
                    download_scope: r.get(8)?,
                    poll_interval_minutes: r.get(9)?,
                    created_at: r.get(10)?,
                    updated_at: r.get(11)?,
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

        // 5. Pawchive Session (sec:pawchive_session) if enabled in settings
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

        // 6. Favorite Posts
        let fav_posts = collect(
            &tx,
            "SELECT pin.service, pin.creator_id, pin.post_id, p.snapshot_json, pin.created_at
             FROM content_pins pin
             JOIN posts p USING(service, creator_id, post_id)
             WHERE pin.entity_kind = 'post' AND pin.reason = 'favorite'",
            |r| {
                Ok(FavoritePostRecord {
                    service: r.get(0)?,
                    creator_id: r.get(1)?,
                    post_id: r.get(2)?,
                    snapshot_json: r.get(3)?,
                    created_at: r.get(4)?,
                })
            },
        )?;
        let mut live_fav_post_ids = std::collections::HashSet::new();
        for fp in fav_posts {
            let record_id = favorite_post_record_id(&fp.service, &fp.creator_id, &fp.post_id);
            live_fav_post_ids.insert(record_id.clone());
            let bytes = serde_json::to_vec(&fp).map_err(|e| e.to_string())?;
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

        // 7. Favorite Creators
        let fav_creators = collect(
            &tx,
            "SELECT pin.service, pin.creator_id, c.name, c.snapshot_json, pin.created_at
             FROM content_pins pin
             JOIN creators c USING(service, creator_id)
             WHERE pin.entity_kind = 'creator' AND pin.reason = 'favorite'",
            |r| {
                Ok(FavoriteCreatorRecord {
                    service: r.get(0)?,
                    creator_id: r.get(1)?,
                    name: r.get(2)?,
                    snapshot_json: r.get(3)?,
                    created_at: r.get(4)?,
                })
            },
        )?;
        let mut live_fav_creator_ids = std::collections::HashSet::new();
        for fc in fav_creators {
            let record_id = favorite_creator_record_id(&fc.service, &fc.creator_id);
            live_fav_creator_ids.insert(record_id.clone());
            let bytes = serde_json::to_vec(&fc).map_err(|e| e.to_string())?;
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
        let tx = connection.transaction().map_err(|e| e.to_string())?;

        if tombstone {
            match kind {
                "collection" => {
                    let id = record_id.strip_prefix("col:").unwrap_or(record_id);
                    tx.execute(
                        "DELETE FROM collections WHERE id=?1 AND is_system=0",
                        params![id],
                    )
                    .map_err(|e| e.to_string())?;
                }
                "membership" => {
                    let raw = record_id.strip_prefix("mem:").unwrap_or(record_id);
                    let parts: Vec<&str> = raw.splitn(4, ':').collect();
                    if parts.len() == 4 {
                        tx.execute(
                            "DELETE FROM collection_posts WHERE collection_id=?1 AND service=?2 AND creator_id=?3 AND post_id=?4",
                            params![parts[0], parts[1], parts[2], parts[3]],
                        )
                        .map_err(|e| e.to_string())?;
                    }
                }
                "subscription" => {
                    let id = record_id.strip_prefix("sub:").unwrap_or(record_id);
                    tx.execute("DELETE FROM subscriptions WHERE id=?1", params![id])
                        .map_err(|e| e.to_string())?;
                }
                "fav_post" => {
                    let raw = record_id.strip_prefix("fav:post:").unwrap_or(record_id);
                    let parts: Vec<&str> = raw.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        tx.execute(
                            "DELETE FROM content_pins WHERE entity_kind='post' AND reason='favorite' AND service=?1 AND creator_id=?2 AND post_id=?3",
                            params![parts[0], parts[1], parts[2]],
                        )
                        .map_err(|e| e.to_string())?;
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
                        .map_err(|e| e.to_string())?;
                    }
                }
                "post" => {
                    // Posts can remain cached locally unless unpinned, but we delete from sync_records
                }
                _ => {}
            }
            tx.execute(
                "INSERT INTO sync_records(record_id,kind,revision,content_hash,dirty,tombstone,updated_at)
                 VALUES(?1,?2,?3,NULL,0,1,CURRENT_TIMESTAMP)
                 ON CONFLICT(record_id) DO UPDATE SET revision=max(revision,?3), content_hash=NULL, dirty=0, tombstone=1, updated_at=CURRENT_TIMESTAMP",
                params![record_id, kind, revision],
            )
            .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
            return Ok(());
        }

        let bytes =
            plaintext.ok_or_else(|| "Missing plaintext payload for live record".to_string())?;
        let hash = format!("{:x}", Sha256::digest(bytes));

        match kind {
            "collection" => {
                let rec: CollectionRecord =
                    serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
                tx.execute(
                    "INSERT INTO collections(id,kind,parent_id,name,position,is_system,created_at,updated_at)
                     VALUES(?1,?2,?3,?4,?5,0,?6,?7)
                     ON CONFLICT(id) DO UPDATE SET
                       kind=excluded.kind, parent_id=excluded.parent_id, name=excluded.name,
                       position=excluded.position, updated_at=excluded.updated_at
                       WHERE collections.updated_at <= excluded.updated_at",
                    params![rec.id, rec.kind, rec.parent_id, rec.name, rec.position, rec.created_at, rec.updated_at],
                )
                .map_err(|e| e.to_string())?;
            }
            "post" => {
                let rec: PostRecord = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
                let post: crate::api::models::Post =
                    crate::api::models::Post::from_json_str(&rec.snapshot_json)
                        .map_err(|e| e.to_string())?;
                tx.execute(
                    "INSERT INTO creators(service,creator_id,name,snapshot_json)
                     VALUES(?1,?2,?2,json_object('id',?2,'name',?2,'service',?1))
                     ON CONFLICT(service,creator_id) DO NOTHING",
                    params![rec.service, rec.creator_id],
                )
                .map_err(|e| e.to_string())?;
                tx.execute(
                    "INSERT INTO posts(service,creator_id,post_id,title,content,published_at,snapshot_json,cached_at)
                     VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
                     ON CONFLICT(service,creator_id,post_id) DO UPDATE SET
                       title=excluded.title, content=excluded.content,
                       published_at=excluded.published_at, snapshot_json=excluded.snapshot_json",
                    params![
                        rec.service,
                        rec.creator_id,
                        rec.post_id,
                        rec.title,
                        post.content,
                        rec.published_at,
                        rec.snapshot_json,
                        rec.cached_at
                    ],
                )
                .map_err(|e| e.to_string())?;
            }
            "membership" => {
                let rec: MembershipRecord =
                    serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
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
                .map_err(|e| e.to_string())?;
            }
            "subscription" => {
                let rec: SubscriptionRecord =
                    serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
                tx.execute(
                    "INSERT INTO subscriptions(id,service,creator_id,creator_name,destination_collection_id,enabled,initial_import,auto_download,download_scope,poll_interval_minutes,next_check_at,created_at,updated_at)
                     VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,CURRENT_TIMESTAMP,?11,?12)
                     ON CONFLICT(service,creator_id) DO UPDATE SET
                       destination_collection_id=excluded.destination_collection_id,
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
                        rec.enabled as i64,
                        rec.initial_import,
                        rec.auto_download as i64,
                        rec.download_scope,
                        rec.poll_interval_minutes,
                        rec.created_at,
                        rec.updated_at
                    ],
                )
                .map_err(|e| e.to_string())?;
            }
            "session" => {
                if let Ok(config_mgr) = crate::config::settings::ConfigManager::new() {
                    if let Ok(settings) = config_mgr.load() {
                        if settings.sync_pawchive_session {
                            if let Ok(rec) = serde_json::from_slice::<PawchiveSessionRecord>(bytes)
                            {
                                if !rec.session_cookie.is_empty() {
                                    let _ = crate::sync::secrets::SecretStore::save_named(
                                        "pawchive-session",
                                        rec.session_cookie.as_bytes(),
                                    );
                                    if !rec.username.is_empty()
                                        && settings.pawchive_username != rec.username
                                    {
                                        let mut updated = settings;
                                        updated.pawchive_username = rec.username;
                                        let _ = config_mgr.save(&updated);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            "fav_post" => {
                let rec: FavoritePostRecord =
                    serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
                if let Ok(post) = crate::api::models::Post::from_json_str(&rec.snapshot_json) {
                    tx.execute(
                        "INSERT INTO creators(service,creator_id,name,snapshot_json)
                         VALUES(?1,?2,?2,json_object('id',?2,'name',?2,'service',?1))
                         ON CONFLICT(service,creator_id) DO NOTHING",
                        params![rec.service, rec.creator_id],
                    )
                    .map_err(|e| e.to_string())?;
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
                            rec.snapshot_json
                        ],
                    )
                    .map_err(|e| e.to_string())?;
                    tx.execute(
                        "INSERT OR IGNORE INTO content_pins(entity_kind,service,creator_id,post_id,reason,account_id,created_at)
                         VALUES('post',?1,?2,?3,'favorite','',?4)",
                        params![rec.service, rec.creator_id, rec.post_id, rec.created_at],
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
            "fav_creator" => {
                let rec: FavoriteCreatorRecord =
                    serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
                tx.execute(
                    "INSERT INTO creators(service,creator_id,name,snapshot_json,cached_at)
                     VALUES(?1,?2,?3,?4,CURRENT_TIMESTAMP)
                     ON CONFLICT(service,creator_id) DO UPDATE SET
                       name=coalesce(excluded.name, creators.name),
                       snapshot_json=excluded.snapshot_json",
                    params![rec.service, rec.creator_id, rec.name, rec.snapshot_json],
                )
                .map_err(|e| e.to_string())?;
                tx.execute(
                    "INSERT OR IGNORE INTO content_pins(entity_kind,service,creator_id,post_id,reason,account_id,created_at)
                     VALUES('creator',?1,?2,'','favorite','',?3)",
                    params![rec.service, rec.creator_id, rec.created_at],
                )
                .map_err(|e| e.to_string())?;
            }
            _ => {}
        }

        tx.execute(
            "INSERT INTO sync_records(record_id,kind,revision,content_hash,dirty,tombstone,updated_at)
             VALUES(?1,?2,?3,?4,0,0,CURRENT_TIMESTAMP)
             ON CONFLICT(record_id) DO UPDATE SET revision=?3, content_hash=?4, dirty=0, tombstone=0, updated_at=CURRENT_TIMESTAMP",
            params![record_id, kind, revision, hash],
        )
        .map_err(|e| e.to_string())?;

        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }
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

    #[test]
    fn detect_dirty_collections_and_tombstones() {
        let repo = SyncRepository::in_memory();
        {
            let conn = repo.connection.lock().unwrap();
            conn.execute(
                "INSERT INTO collections(id,kind,name,position,is_system) VALUES('col_01','stash','Art Stash',0,0)",
                [],
            )
            .unwrap();
        }

        let dirty = repo.detect_and_get_dirty_records().unwrap();
        assert_eq!(dirty.len(), 1);
        assert_eq!(dirty[0].record_id, "col:col_01");
        assert_eq!(dirty[0].kind, "collection");
        assert!(!dirty[0].tombstone);
        assert!(dirty[0].payload.is_some());

        // Mark synced
        repo.mark_records_synced(&[crate::sync::client::AcceptedRecord {
            record_id: "col:col_01".into(),
            revision: 1,
            position: 1,
        }])
        .unwrap();

        // Second check with no changes
        let dirty_after = repo.detect_and_get_dirty_records().unwrap();
        assert!(dirty_after.is_empty());

        // Delete collection locally -> detect tombstone
        {
            let conn = repo.connection.lock().unwrap();
            conn.execute("DELETE FROM collections WHERE id='col_01'", [])
                .unwrap();
        }
        let dirty_deleted = repo.detect_and_get_dirty_records().unwrap();
        assert_eq!(dirty_deleted.len(), 1);
        assert_eq!(dirty_deleted[0].record_id, "col:col_01");
        assert!(dirty_deleted[0].tombstone);
        assert_eq!(dirty_deleted[0].expected_revision, 1);
    }

    #[test]
    fn apply_remote_changes_and_tombstones() {
        let repo = SyncRepository::in_memory();
        let col = CollectionRecord {
            id: "remote_col_01".into(),
            kind: "stash".into(),
            parent_id: None,
            name: "Remote Stash".into(),
            position: 0,
            created_at: "2026-08-15T00:00:00Z".into(),
            updated_at: "2026-08-15T00:00:00Z".into(),
        };
        let bytes = serde_json::to_vec(&col).unwrap();

        repo.apply_remote_change("col:remote_col_01", "collection", 1, Some(&bytes), false)
            .unwrap();

        {
            let conn = repo.connection.lock().unwrap();
            let name: String = conn
                .query_row(
                    "SELECT name FROM collections WHERE id='remote_col_01'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(name, "Remote Stash");
        }

        // Apply tombstone
        repo.apply_remote_change("col:remote_col_01", "collection", 2, None, true)
            .unwrap();

        {
            let conn = repo.connection.lock().unwrap();
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM collections WHERE id='remote_col_01'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 0);
        }
    }

    #[test]
    fn full_entity_sync_and_merge_roundtrip() {
        let repo_a = SyncRepository::in_memory();
        let repo_b = SyncRepository::in_memory();

        // 1. Device A creates a collection, post, membership, subscription, and favorites
        {
            let conn = repo_a.connection.lock().unwrap();
            conn.execute(
                "INSERT INTO collections(id,kind,name,position,is_system) VALUES('stash_01','stash','My Stash',0,0)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO creators(service,creator_id,name,snapshot_json) VALUES('fanbox','c1','Artist 1','{\"id\":\"c1\",\"name\":\"Artist 1\",\"service\":\"fanbox\"}')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO posts(service,creator_id,post_id,title,content,published_at,snapshot_json)
                 VALUES('fanbox','c1','p1','Artwork 1','Hello world','2026-08-15T00:00:00Z','{\"id\":\"p1\",\"title\":\"Artwork 1\",\"user\":\"c1\",\"service\":\"fanbox\",\"content\":\"Hello world\"}')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO collection_posts(collection_id,service,creator_id,post_id,position,operation_id,added_at)
                 VALUES('stash_01','fanbox','c1','p1',0,'op_01','2026-08-15T00:00:00Z')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO subscriptions(id,service,creator_id,creator_name,destination_collection_id,enabled,initial_import,auto_download,download_scope,poll_interval_minutes)
                 VALUES('sub_01','fanbox','c1','Artist 1','stash_01',1,'none',0,'primary',30)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO content_pins(entity_kind,service,creator_id,post_id,reason,account_id)
                 VALUES('post','fanbox','c1','p1','favorite','')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO content_pins(entity_kind,service,creator_id,post_id,reason,account_id)
                 VALUES('creator','fanbox','c1','','favorite','')",
                [],
            )
            .unwrap();
        }

        // Device A detects dirty records
        let dirty_a = repo_a.detect_and_get_dirty_records().unwrap();
        let kinds: Vec<(&str, &str, bool)> = dirty_a
            .iter()
            .map(|d| (d.kind.as_str(), d.record_id.as_str(), d.payload.is_some()))
            .collect();
        eprintln!("DEBUG dirty_a: {:?}", kinds);
        assert_eq!(dirty_a.len(), 6); // collection, post, membership, subscription, fav_post, fav_creator

        // Simulate server accept
        let mut accepted = Vec::new();
        for (i, d) in dirty_a.iter().enumerate() {
            accepted.push(crate::sync::client::AcceptedRecord {
                record_id: d.record_id.clone(),
                revision: 1,
                position: (i + 1) as i64,
            });
        }
        repo_a.mark_records_synced(&accepted).unwrap();

        // 2. Device B pulls and applies Device A's changes
        for d in &dirty_a {
            repo_b
                .apply_remote_change(&d.record_id, &d.kind, 1, d.payload.as_deref(), d.tombstone)
                .unwrap();
        }

        // Verify Device B now has all entities
        {
            let conn = repo_b.connection.lock().unwrap();
            let col_name: String = conn
                .query_row(
                    "SELECT name FROM collections WHERE id='stash_01'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(col_name, "My Stash");

            let post_title: String = conn
                .query_row(
                    "SELECT title FROM posts WHERE service='fanbox' AND creator_id='c1' AND post_id='p1'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(post_title, "Artwork 1");

            let mem_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM collection_posts WHERE collection_id='stash_01'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(mem_count, 1);

            let sub_name: String = conn
                .query_row(
                    "SELECT creator_name FROM subscriptions WHERE id='sub_01'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(sub_name, "Artist 1");

            let fav_post_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM content_pins WHERE entity_kind='post' AND reason='favorite' AND service='fanbox' AND creator_id='c1' AND post_id='p1'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(fav_post_count, 1);

            let fav_creator_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM content_pins WHERE entity_kind='creator' AND reason='favorite' AND service='fanbox' AND creator_id='c1'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(fav_creator_count, 1);
        }

        // Device B has no local dirty records
        let dirty_b = repo_b.detect_and_get_dirty_records().unwrap();
        assert!(dirty_b.is_empty());
    }
}
