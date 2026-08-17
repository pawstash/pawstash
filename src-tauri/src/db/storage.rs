use rusqlite::Connection;
use std::path::PathBuf;
use std::time::Duration;

pub const INBOX_COLLECTION_ID: &str = "00000000-0000-0000-0000-000000000001";

// Pawstash single version-one schema.
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
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
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

CREATE TABLE IF NOT EXISTS subscriptions (
    id TEXT PRIMARY KEY,
    service TEXT NOT NULL,
    creator_id TEXT NOT NULL,
    creator_name TEXT NOT NULL DEFAULT '',
    destination_collection_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001'
        REFERENCES collections(id) ON DELETE SET DEFAULT,
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

INSERT OR IGNORE INTO collections (id, kind, name, position, is_system)
VALUES ('00000000-0000-0000-0000-000000000001', 'inbox', 'Inbox', 0, 1);
"#;

pub fn data_root() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        PathBuf::from("/data/data/app.pawstash.client/files/Pawstash")
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
        .busy_timeout(Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    connection
        .execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=5000;")
        .map_err(|e| e.to_string())?;
    initialize_schema(connection)
}

pub fn initialize_schema(connection: &mut Connection) -> Result<(), String> {
    connection.execute_batch(SCHEMA).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn schema_is_initialized_once() {
        let mut connection = Connection::open_in_memory().unwrap();
        prepare_connection(&mut connection).unwrap();
        initialize_schema(&mut connection).unwrap();
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM collections", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
