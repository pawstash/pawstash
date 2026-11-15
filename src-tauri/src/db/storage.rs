use rusqlite::Connection;
use std::path::PathBuf;
use std::time::Duration;

pub const INBOX_COLLECTION_ID: &str = "00000000-0000-0000-0000-000000000001";

const CURRENT_SCHEMA_VERSION: i64 = 13;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS creators (
    service TEXT NOT NULL,
    creator_id TEXT NOT NULL,
    name TEXT NOT NULL,
    snapshot_json TEXT NOT NULL,
    avatar_path TEXT,
    banner_path TEXT,
    cached_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_checked_at TEXT,
    PRIMARY KEY (service, creator_id)
);

CREATE TABLE IF NOT EXISTS posts (
    service TEXT NOT NULL,
    creator_id TEXT NOT NULL,
    post_id TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT,
    published_at TEXT,
    snapshot_json TEXT NOT NULL,
    preview_path TEXT,
    cached_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_checked_at TEXT,
    remote_state TEXT NOT NULL DEFAULT 'active'
        CHECK (remote_state IN ('active', 'unavailable')),
    PRIMARY KEY (service, creator_id, post_id),
    FOREIGN KEY (service, creator_id) REFERENCES creators(service, creator_id)
        ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX IF NOT EXISTS idx_posts_creator_published ON posts(service, creator_id, published_at DESC, post_id);
CREATE INDEX IF NOT EXISTS idx_posts_service_post ON posts(service, post_id, creator_id);
CREATE INDEX IF NOT EXISTS idx_posts_published ON posts(published_at DESC, post_id);
CREATE INDEX IF NOT EXISTS idx_posts_title ON posts(title);

CREATE TABLE IF NOT EXISTS post_revisions (
    service TEXT NOT NULL,
    creator_id TEXT NOT NULL,
    post_id TEXT NOT NULL,
    revision_id INTEGER NOT NULL,
    provider_id TEXT NOT NULL,
    imported_at TEXT,
    edited_at TEXT,
    snapshot_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (service, creator_id, post_id, revision_id, provider_id),
    FOREIGN KEY (service, creator_id, post_id) REFERENCES posts(service, creator_id, post_id)
        ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX IF NOT EXISTS idx_post_revisions_post ON post_revisions(service, creator_id, post_id, revision_id DESC);

CREATE TABLE IF NOT EXISTS content_pins (
    entity_kind TEXT NOT NULL CHECK (entity_kind IN ('post', 'creator')),
    service TEXT NOT NULL,
    creator_id TEXT NOT NULL,
    post_id TEXT NOT NULL DEFAULT '',
    reason TEXT NOT NULL CHECK (reason IN ('download', 'library', 'favorite', 'subscription')),
    account_id TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (entity_kind, service, creator_id, post_id, reason, account_id)
);
CREATE INDEX IF NOT EXISTS idx_content_pins_entity ON content_pins(service, creator_id, post_id);

CREATE TABLE IF NOT EXISTS content_lists (
    list_key TEXT NOT NULL,
    page_offset INTEGER NOT NULL,
    identities_json TEXT NOT NULL,
    cached_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (list_key, page_offset)
);

CREATE TABLE IF NOT EXISTS content_documents (
    document_kind TEXT NOT NULL,
    service TEXT NOT NULL,
    creator_id TEXT NOT NULL DEFAULT '',
    post_id TEXT NOT NULL DEFAULT '',
    snapshot_json TEXT NOT NULL,
    cached_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (document_kind, service, creator_id, post_id)
);

CREATE TABLE IF NOT EXISTS collections (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('inbox', 'stash', 'folder')),
    parent_id TEXT REFERENCES collections(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    is_system INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    color TEXT
);
CREATE INDEX IF NOT EXISTS idx_collections_parent_position ON collections(parent_id, position, id);

CREATE TABLE IF NOT EXISTS collection_posts (
    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    service TEXT NOT NULL,
    creator_id TEXT NOT NULL,
    post_id TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    operation_id TEXT NOT NULL,
    added_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (collection_id, service, creator_id, post_id),
    FOREIGN KEY (service, creator_id, post_id) REFERENCES posts(service, creator_id, post_id)
        ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_collection_posts_order ON collection_posts(collection_id, position, added_at, post_id);
CREATE INDEX IF NOT EXISTS idx_collection_posts_post ON collection_posts(service, creator_id, post_id);

CREATE TABLE IF NOT EXISTS download_jobs (
    id TEXT PRIMARY KEY,
    logical_key TEXT NOT NULL UNIQUE,
    service TEXT NOT NULL,
    creator_id TEXT NOT NULL,
    post_id TEXT NOT NULL,
    media_id TEXT NOT NULL,
    url TEXT NOT NULL,
    filename TEXT NOT NULL,
    output_dir TEXT NOT NULL,
    temp_path TEXT NOT NULL,
    final_path TEXT NOT NULL,
    engine TEXT NOT NULL CHECK (engine IN ('native', 'aria2c')),
    status TEXT NOT NULL CHECK (status IN (
        'queued', 'resolving', 'downloading', 'paused', 'verifying',
        'completed', 'failed', 'cancelled', 'missing'
    )),
    downloaded_bytes INTEGER NOT NULL DEFAULT 0,
    total_bytes INTEGER NOT NULL DEFAULT 0,
    speed_bps INTEGER NOT NULL DEFAULT 0,
    sha256 TEXT,
    error_code TEXT,
    error_message TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TEXT,
    FOREIGN KEY (service, creator_id, post_id) REFERENCES posts(service, creator_id, post_id)
        ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS idx_download_jobs_queue ON download_jobs(status, created_at, id);
CREATE INDEX IF NOT EXISTS idx_download_jobs_post ON download_jobs(service, creator_id, post_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS media_blobs (
    sha256 TEXT PRIMARY KEY,
    size INTEGER NOT NULL,
    relative_path TEXT NOT NULL UNIQUE,
    verified_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS download_blob_refs (
    job_id TEXT PRIMARY KEY REFERENCES download_jobs(id) ON DELETE CASCADE,
    blob_sha256 TEXT NOT NULL REFERENCES media_blobs(sha256) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS thumb_blobs (
    content_key TEXT PRIMARY KEY,
    size INTEGER NOT NULL,
    relative_path TEXT NOT NULL UNIQUE,
    last_access_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS thumb_refs (
    service TEXT NOT NULL,
    creator_id TEXT NOT NULL,
    post_id TEXT NOT NULL,
    media_id TEXT NOT NULL,
    content_key TEXT NOT NULL REFERENCES thumb_blobs(content_key) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (service, creator_id, post_id, media_id)
);
CREATE INDEX IF NOT EXISTS idx_thumb_refs_key ON thumb_refs(content_key);
CREATE INDEX IF NOT EXISTS idx_thumb_refs_post ON thumb_refs(service, creator_id, post_id);

CREATE TABLE IF NOT EXISTS subscriptions (
    id TEXT PRIMARY KEY,
    service TEXT NOT NULL,
    creator_id TEXT NOT NULL,
    creator_name TEXT NOT NULL DEFAULT '',
    destination_collection_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001'
        REFERENCES collections(id) ON DELETE SET DEFAULT,
    save_to_library INTEGER NOT NULL DEFAULT 1 CHECK (save_to_library IN (0, 1)),
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    initial_import TEXT NOT NULL DEFAULT 'none' CHECK (initial_import IN ('none', 'latest', 'all')),
    auto_download INTEGER NOT NULL DEFAULT 0 CHECK (auto_download IN (0, 1)),
    download_scope TEXT NOT NULL DEFAULT 'primary' CHECK (download_scope IN ('primary', 'all')),
    poll_interval_minutes INTEGER NOT NULL DEFAULT 30 CHECK (poll_interval_minutes BETWEEN 5 AND 10080),
    last_checked_at TEXT,
    next_check_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    failure_count INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (service, creator_id)
);
CREATE INDEX IF NOT EXISTS idx_subscriptions_due ON subscriptions(enabled, next_check_at, id);
CREATE TABLE IF NOT EXISTS subscription_seen_posts (
    subscription_id TEXT NOT NULL REFERENCES subscriptions(id) ON DELETE CASCADE,
    source_identity TEXT NOT NULL,
    first_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (subscription_id, source_identity)
);

CREATE TABLE IF NOT EXISTS sync_state (
    id INTEGER PRIMARY KEY CHECK (id = 1), server_url TEXT NOT NULL, account_id TEXT NOT NULL,
    device_id TEXT NOT NULL, snapshot_record_id TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 0,
    cursor INTEGER NOT NULL DEFAULT 0, last_plaintext_hash TEXT, kdf_json TEXT NOT NULL,
    encrypted_key_bundle TEXT NOT NULL, bundle_nonce TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)), last_synced_at TEXT,
    last_error TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS sync_conflicts (
    id INTEGER PRIMARY KEY CHECK (id = 1), remote_revision INTEGER NOT NULL,
    remote_cursor INTEGER NOT NULL, record_id TEXT NOT NULL, ciphertext TEXT NOT NULL,
    nonce TEXT NOT NULL, detected_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS sync_records (
    record_id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 0,
    content_hash TEXT,
    dirty INTEGER NOT NULL DEFAULT 0,
    tombstone INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_sync_records_dirty ON sync_records(dirty, kind);
CREATE TABLE IF NOT EXISTS profile_avatar (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    image TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

INSERT OR IGNORE INTO collections (id, kind, name, position, is_system)
VALUES ('00000000-0000-0000-0000-000000000001', 'inbox', 'Main Stash', 0, 1);
"#;

#[cfg(target_os = "android")]
pub fn android_package_name() -> String {
    if let Ok(cmdline) = std::fs::read_to_string("/proc/self/cmdline") {
        let pkg = cmdline.split('\0').next().unwrap_or("").trim();
        if !pkg.is_empty() && pkg.contains("app.pawstash") {
            return pkg.to_string();
        }
    }
    let user_id = android_user_id();
    if std::path::Path::new(&format!(
        "/data/user/{user_id}/app.pawstash.client.dev/files"
    ))
    .exists()
        || std::path::Path::new("/data/data/app.pawstash.client.dev/files").exists()
    {
        return "app.pawstash.client.dev".to_string();
    }
    "app.pawstash.client".to_string()
}

/// App uids are `user_id * 100000 + app_id` (AOSP `android_filesystem_config.h`).
#[cfg(target_os = "android")]
pub fn android_user_id() -> u32 {
    const AID_USER_OFFSET: u32 = 100_000;
    // SAFETY: getuid has no preconditions and always succeeds.
    let uid = unsafe { libc::getuid() };
    uid / AID_USER_OFFSET
}

/// `/data/data` is user 0 only; work profiles and Secure Folder live under `/data/user/<id>`.
#[cfg(target_os = "android")]
pub fn android_app_data_dir() -> &'static std::path::Path {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let pkg = android_package_name();
        // Android 13+ sets TMPDIR to the cache dir, which also covers adopted storage.
        let from_tmpdir = std::env::var_os("TMPDIR")
            .map(PathBuf::from)
            .filter(|cache| cache.file_name() == Some(std::ffi::OsStr::new("cache")))
            .and_then(|cache| cache.parent().map(std::path::Path::to_path_buf))
            .filter(|dir| dir.file_name() == Some(std::ffi::OsStr::new(&pkg)) && dir.is_dir());
        if let Some(dir) = from_tmpdir {
            return dir;
        }
        let per_user = PathBuf::from(format!("/data/user/{}/{pkg}", android_user_id()));
        if per_user.is_dir() {
            return per_user;
        }
        PathBuf::from(format!("/data/data/{pkg}"))
    })
}

#[cfg(target_os = "android")]
pub fn android_cache_dir() -> PathBuf {
    android_app_data_dir().join("cache")
}

#[cfg(target_os = "android")]
pub fn android_external_storage() -> PathBuf {
    PathBuf::from(format!("/storage/emulated/{}", android_user_id()))
}

#[cfg(target_os = "android")]
pub fn android_default_download_dir() -> PathBuf {
    android_external_storage().join("Download").join("Pawstash")
}

pub fn data_root() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        android_app_data_dir().join("files").join("Pawstash")
    }
    #[cfg(not(target_os = "android"))]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Pawstash")
    }
}

pub fn database_path() -> PathBuf {
    data_root().join("pawstash.db")
}
pub fn content_cache_path() -> PathBuf {
    data_root().join("content-cache")
}
pub fn thumbs_cache_path() -> PathBuf {
    content_cache_path().join("thumbnails")
}

/// Must match the compaction rewrite, `json_patch('{}', ...)`: nested objects, not arrays.
pub fn snapshot_json<T: serde::Serialize>(value: &T) -> Result<String, String> {
    let mut json = serde_json::to_value(value).map_err(|e| e.to_string())?;
    strip_nulls(&mut json);
    serde_json::to_string(&json).map_err(|e| e.to_string())
}

fn strip_nulls(value: &mut serde_json::Value) {
    if let serde_json::Value::Object(map) = value {
        map.retain(|_, member| !member.is_null());
        map.values_mut().for_each(strip_nulls);
    }
}

pub fn sanitize_cache_key(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn open_database() -> Result<Connection, String> {
    let path = database_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut connection = Connection::open(path).map_err(|e| e.to_string())?;
    prepare_connection(&mut connection)?;
    Ok(connection)
}

pub fn prepare_connection(connection: &mut Connection) -> Result<(), String> {
    connection
        .busy_timeout(Duration::from_secs(30))
        .map_err(|e| e.to_string())?;
    connection
        // auto_vacuum only applies to new databases or after a VACUUM. The long busy
        // timeout lets writes wait out the background compaction.
        .execute_batch("PRAGMA auto_vacuum=INCREMENTAL; PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=30000; PRAGMA journal_size_limit=67108864;")
        .map_err(|e| e.to_string())?;
    initialize_schema(connection)
}

pub fn initialize_schema(connection: &mut Connection) -> Result<(), String> {
    let mut version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if version > CURRENT_SCHEMA_VERSION {
        return Err(format!(
            "Database schema version {version} is newer than supported version {CURRENT_SCHEMA_VERSION}"
        ));
    }

    while version < CURRENT_SCHEMA_VERSION {
        let next_version = version + 1;
        let transaction = connection.transaction().map_err(|e| e.to_string())?;
        match next_version {
            1 => transaction
                .execute_batch(SCHEMA)
                .map_err(|e| e.to_string())?,
            2 => {
                if !column_exists(&transaction, "collections", "color")? {
                    transaction
                        .execute("ALTER TABLE collections ADD COLUMN color TEXT", [])
                        .map_err(|e| e.to_string())?;
                }
            }
            3 => {
                if !column_exists(&transaction, "creators", "favorited")? {
                    transaction
                        .execute(
                            "ALTER TABLE creators ADD COLUMN favorited INTEGER NOT NULL DEFAULT 0",
                            [],
                        )
                        .map_err(|e| e.to_string())?;
                }
                if !column_exists(&transaction, "creators", "updated_at")? {
                    transaction
                        .execute(
                            "ALTER TABLE creators ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
                            [],
                        )
                        .map_err(|e| e.to_string())?;
                }
                if !column_exists(&transaction, "creators", "indexed_at")? {
                    transaction
                        .execute(
                            "ALTER TABLE creators ADD COLUMN indexed_at INTEGER NOT NULL DEFAULT 0",
                            [],
                        )
                        .map_err(|e| e.to_string())?;
                }
                if !column_exists(&transaction, "creators", "is_ai")? {
                    transaction
                        .execute(
                            "ALTER TABLE creators ADD COLUMN is_ai INTEGER NOT NULL DEFAULT 0",
                            [],
                        )
                        .map_err(|e| e.to_string())?;
                }
                transaction
                    .execute_batch(
                        "CREATE INDEX IF NOT EXISTS idx_creators_service ON creators(service);
                         CREATE INDEX IF NOT EXISTS idx_creators_favorited ON creators(favorited DESC, name COLLATE NOCASE);
                         CREATE INDEX IF NOT EXISTS idx_creators_updated ON creators(updated_at DESC);
                         CREATE INDEX IF NOT EXISTS idx_creators_indexed ON creators(indexed_at DESC);
                         CREATE INDEX IF NOT EXISTS idx_creators_name ON creators(name COLLATE NOCASE);
                         CREATE INDEX IF NOT EXISTS idx_creators_is_ai ON creators(is_ai);
                         CREATE INDEX IF NOT EXISTS idx_creators_service_favorited ON creators(service, favorited DESC);
                         UPDATE creators SET
                             favorited = coalesce(json_extract(snapshot_json, '$.favorited'), 0),
                             updated_at = coalesce(json_extract(snapshot_json, '$.updated'), 0),
                             indexed_at = coalesce(json_extract(snapshot_json, '$.indexed'), 0)
                         WHERE favorited = 0 AND updated_at = 0 AND indexed_at = 0;
                         UPDATE creators SET is_ai = 1 WHERE lower(name) LIKE '%[ai]%' OR lower(name) LIKE '%(ai)%';"
                    )
                    .map_err(|e| e.to_string())?;
            }
            4 => {
                transaction
                    .execute(
                        "UPDATE posts SET preview_path = NULL WHERE preview_path LIKE '%previews%'",
                        [],
                    )
                    .map_err(|e| e.to_string())?;
                let legacy = content_cache_path().join("previews");
                if legacy.exists() {
                    let _ = std::fs::remove_dir_all(&legacy);
                }
            }
            5 => {
                transaction
                    .execute_batch(
                        "CREATE TABLE IF NOT EXISTS thumb_blobs (
                            content_key TEXT PRIMARY KEY,
                            size INTEGER NOT NULL,
                            relative_path TEXT NOT NULL UNIQUE,
                            last_access_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                        );
                        CREATE TABLE IF NOT EXISTS thumb_refs (
                            service TEXT NOT NULL,
                            creator_id TEXT NOT NULL,
                            post_id TEXT NOT NULL,
                            media_id TEXT NOT NULL,
                            content_key TEXT NOT NULL REFERENCES thumb_blobs(content_key) ON DELETE CASCADE,
                            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                            PRIMARY KEY (service, creator_id, post_id, media_id)
                        );
                        CREATE INDEX IF NOT EXISTS idx_thumb_refs_key ON thumb_refs(content_key);
                        CREATE INDEX IF NOT EXISTS idx_thumb_refs_post ON thumb_refs(service, creator_id, post_id);",
                    )
                    .map_err(|e| e.to_string())?;
            }
            6 => {
                if !column_exists(&transaction, "subscriptions", "save_to_library")? {
                    transaction
                        .execute(
                            "ALTER TABLE subscriptions ADD COLUMN save_to_library INTEGER NOT NULL DEFAULT 1",
                            [],
                        )
                        .map_err(|e| e.to_string())?;
                }
            }
            7 => {
                transaction
                    .execute(
                        "UPDATE creators SET snapshot_json = CASE
                             WHEN json_type(snapshot_json, '$.provider_ids[0]') = 'text'
                                  AND json_extract(snapshot_json, '$.provider_ids[0]') NOT IN ('', 'auto')
                             THEN json_set(snapshot_json, '$.provider_id', json_extract(snapshot_json, '$.provider_ids[0]'))
                             ELSE json_remove(snapshot_json, '$.provider_id')
                         END
                         WHERE CASE WHEN json_valid(snapshot_json)
                                    THEN json_extract(snapshot_json, '$.provider_id') = 'auto'
                                    ELSE 0 END",
                        [],
                    )
                    .map_err(|e| e.to_string())?;
            }
            8 => {
                transaction
                    .execute_batch(
                        "CREATE TABLE IF NOT EXISTS archive_passwords (
                            archive_id TEXT PRIMARY KEY,
                            service TEXT NOT NULL DEFAULT '',
                            creator_id TEXT NOT NULL DEFAULT '',
                            password TEXT NOT NULL,
                            source TEXT,
                            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                        );
                        CREATE INDEX IF NOT EXISTS idx_archive_passwords_creator ON archive_passwords(service, creator_id);",
                    )
                    .map_err(|e| e.to_string())?;
            }
            9 => {
                transaction
                    .execute_batch(
                        "CREATE TABLE IF NOT EXISTS post_inner_formats (
                            service TEXT NOT NULL,
                            creator_id TEXT NOT NULL,
                            post_id TEXT NOT NULL,
                            sources TEXT NOT NULL,
                            formats TEXT NOT NULL DEFAULT '[]',
                            complete INTEGER NOT NULL DEFAULT 1,
                            checked_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                            PRIMARY KEY (service, creator_id, post_id)
                        );",
                    )
                    .map_err(|e| e.to_string())?;
            }
            10 => {
                transaction
                    .execute(
                        "UPDATE posts SET snapshot_json = json_set(snapshot_json, '$.detail_fetched', json('false'))
                         WHERE CASE WHEN json_valid(snapshot_json)
                                    THEN json_extract(snapshot_json, '$.detail_fetched') = 1
                                         AND (json_extract(snapshot_json, '$.has_full') = 0
                                              OR json_extract(snapshot_json, '$.preview_state') = 'pending')
                                    ELSE 0 END",
                        [],
                    )
                    .map_err(|e| e.to_string())?;
            }
            11 => {
                transaction
                    .execute(
                        "UPDATE post_inner_formats SET complete = 0
                         WHERE sources LIKE '%drive.google.com%' AND sources LIKE '%folders%'",
                        [],
                    )
                    .map_err(|e| e.to_string())?;
            }
            12 => {
                transaction
                    .execute(
                        "UPDATE posts SET snapshot_json = json_remove(snapshot_json, '$.has_full', '$.preview_state')
                         WHERE CASE WHEN json_valid(snapshot_json)
                                    THEN json_extract(snapshot_json, '$.has_full') = 0
                                         AND coalesce(json_extract(snapshot_json, '$.detail_fetched'), 0) = 0
                                    ELSE 0 END",
                        [],
                    )
                    .map_err(|e| e.to_string())?;
            }
            13 => {
                transaction
                    .execute(
                        "CREATE TABLE IF NOT EXISTS profile_avatar (
                            id INTEGER PRIMARY KEY CHECK (id = 1),
                            image TEXT NOT NULL,
                            updated_at TEXT NOT NULL
                        )",
                        [],
                    )
                    .map_err(|e| e.to_string())?;
            }
            _ => return Err(format!("Missing database migration {next_version}")),
        }
        transaction
            .pragma_update(None, "user_version", next_version)
            .map_err(|e| e.to_string())?;
        transaction.commit().map_err(|e| e.to_string())?;
        version = next_version;
    }

    Ok(())
}

fn column_exists(connection: &Connection, table: &str, column: &str) -> Result<bool, String> {
    let sql = format!("PRAGMA table_info({table})");
    connection
        .prepare(&sql)
        .and_then(|mut stmt| {
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let name: String = row.get(1)?;
                if name == column {
                    return Ok(true);
                }
            }
            Ok(false)
        })
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrate_from(version: i64, seed: &str) -> Connection {
        let mut connection = Connection::open_in_memory().unwrap();
        prepare_connection(&mut connection).unwrap();
        connection
            .execute_batch(&format!("{seed} PRAGMA user_version = {version};"))
            .unwrap();
        initialize_schema(&mut connection).unwrap();
        connection
    }

    fn query<T: rusqlite::types::FromSql>(connection: &Connection, sql: &str, id: &str) -> T {
        connection.query_row(sql, [id], |r| r.get(0)).unwrap()
    }

    fn user_version(connection: &Connection) -> i64 {
        connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn schema_is_initialized_once() {
        let mut connection = Connection::open_in_memory().unwrap();
        prepare_connection(&mut connection).unwrap();
        initialize_schema(&mut connection).unwrap();
        assert_eq!(user_version(&connection), CURRENT_SCHEMA_VERSION);
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM collections", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
        for (table, column) in [
            ("collections", "color"),
            ("creators", "favorited"),
            ("creators", "updated_at"),
            ("creators", "indexed_at"),
            ("creators", "is_ai"),
            ("thumb_blobs", "content_key"),
            ("thumb_refs", "content_key"),
            ("subscriptions", "save_to_library"),
            ("archive_passwords", "source"),
            ("post_inner_formats", "sources"),
        ] {
            assert!(
                column_exists(&connection, table, column).unwrap(),
                "{table}.{column}"
            );
        }
    }

    #[test]
    fn migration_7_repairs_auto_provider_ids() {
        let connection = migrate_from(
            6,
            r#"INSERT INTO creators(service, creator_id, name, snapshot_json) VALUES
                 ('fansly', 'with_ids', 'a', '{"id":"with_ids","provider_id":"auto","provider_ids":["onlyhaven","coomer"]}'),
                 ('fansly', 'no_ids', 'b', '{"id":"no_ids","provider_id":"auto"}'),
                 ('patreon', 'concrete', 'c', '{"id":"concrete","provider_id":"pawchive"}'),
                 ('patreon', 'broken', 'd', 'not json');"#,
        );
        let provider_of = |id| -> Option<String> {
            query(
                &connection,
                "SELECT json_extract(snapshot_json, '$.provider_id') FROM creators WHERE creator_id = ?1",
                id,
            )
        };
        assert_eq!(provider_of("with_ids").as_deref(), Some("onlyhaven"));
        assert_eq!(provider_of("no_ids"), None);
        assert_eq!(provider_of("concrete").as_deref(), Some("pawchive"));
        let broken: String = query(
            &connection,
            "SELECT snapshot_json FROM creators WHERE creator_id = ?1",
            "broken",
        );
        assert_eq!(broken, "not json");
        assert_eq!(user_version(&connection), CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn migration_10_reopens_details_cached_before_import() {
        let connection = migrate_from(
            9,
            r#"INSERT INTO creators(service, creator_id, name, snapshot_json) VALUES ('patreon', 'c', 'c', '{}');
               INSERT INTO posts(service, creator_id, post_id, title, snapshot_json) VALUES
                 ('patreon', 'c', 'teaser', 't', '{"id":"teaser","detail_fetched":true,"has_full":false,"preview_state":"pending"}'),
                 ('patreon', 'c', 'full', 't', '{"id":"full","detail_fetched":true,"has_full":true,"preview_state":"scraped"}'),
                 ('patreon', 'c', 'broken', 't', 'not json');"#,
        );
        let fetched = |id| -> Option<bool> {
            query(
                &connection,
                "SELECT json_extract(snapshot_json, '$.detail_fetched') FROM posts WHERE post_id = ?1",
                id,
            )
        };
        assert_eq!(fetched("teaser"), Some(false));
        assert_eq!(fetched("full"), Some(true));
        let broken: String = query(
            &connection,
            "SELECT snapshot_json FROM posts WHERE post_id = ?1",
            "broken",
        );
        assert_eq!(broken, "not json");
    }

    #[test]
    fn migration_11_reindexes_posts_with_drive_folders() {
        let connection = migrate_from(
            10,
            r#"INSERT INTO post_inner_formats(service, creator_id, post_id, sources) VALUES
                 ('patreon', 'c', 'drive', 'https://drive.google.com/drive/folders/abc'),
                 ('patreon', 'c', 'file', 'https://drive.google.com/file/d/abc/view'),
                 ('patreon', 'c', 'mega', 'https://mega.nz/folder/abc');"#,
        );
        for (id, complete) in [("drive", 0), ("file", 1), ("mega", 1)] {
            let actual: i64 = query(
                &connection,
                "SELECT complete FROM post_inner_formats WHERE post_id = ?1",
                id,
            );
            assert_eq!(actual, complete, "{id}");
        }
    }

    #[test]
    fn migration_12_forgets_import_state_that_sync_invented() {
        let connection = migrate_from(
            11,
            r#"INSERT INTO creators(service, creator_id, name, snapshot_json) VALUES ('patreon', 'c', 'c', '{}');
               INSERT INTO posts(service, creator_id, post_id, title, snapshot_json) VALUES
                 ('patreon', 'c', 'slim', 't', '{"id":"slim","has_full":false,"preview_state":"scraped","detail_fetched":false}'),
                 ('patreon', 'c', 'detail', 't', '{"id":"detail","has_full":false,"preview_state":"pending","detail_fetched":true}'),
                 ('patreon', 'c', 'full', 't', '{"id":"full","has_full":true,"detail_fetched":false}');"#,
        );
        for (id, has_full) in [
            ("slim", None),
            ("detail", Some(false)),
            ("full", Some(true)),
        ] {
            let actual: Option<bool> = query(
                &connection,
                "SELECT json_extract(snapshot_json, '$.has_full') FROM posts WHERE post_id = ?1",
                id,
            );
            assert_eq!(actual, has_full, "{id}");
        }
    }
}
