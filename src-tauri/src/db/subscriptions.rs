#[cfg(test)]
use crate::db::storage::prepare_connection;
use crate::db::storage::{open_database, INBOX_COLLECTION_ID};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub service: String,
    pub creator_id: String,
    pub creator_name: String,
    pub destination_collection_id: String,
    pub enabled: bool,
    pub initial_import: String,
    pub auto_download: bool,
    pub download_scope: String,
    pub poll_interval_minutes: u32,
    pub last_checked_at: Option<String>,
    pub next_check_at: String,
    pub failure_count: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionInput {
    pub service: String,
    pub creator_id: String,
    pub creator_name: String,
    pub destination_collection_id: Option<String>,
    pub initial_import: String,
    pub auto_download: bool,
    pub download_scope: String,
    pub poll_interval_minutes: u32,
}

pub struct SubscriptionRepository {
    connection: Mutex<Connection>,
}

impl SubscriptionRepository {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            connection: Mutex::new(open_database()?),
        })
    }

    #[cfg(test)]
    fn in_memory() -> Self {
        let mut connection = Connection::open_in_memory().unwrap();
        prepare_connection(&mut connection).unwrap();
        Self {
            connection: Mutex::new(connection),
        }
    }

    fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Subscription> {
        Ok(Subscription {
            id: row.get(0)?,
            service: row.get(1)?,
            creator_id: row.get(2)?,
            creator_name: row.get(3)?,
            destination_collection_id: row.get(4)?,
            enabled: row.get::<_, i64>(5)? != 0,
            initial_import: row.get(6)?,
            auto_download: row.get::<_, i64>(7)? != 0,
            download_scope: row.get(8)?,
            poll_interval_minutes: row.get::<_, i64>(9)?.max(0) as u32,
            last_checked_at: row.get(10)?,
            next_check_at: row.get(11)?,
            failure_count: row.get::<_, i64>(12)?.max(0) as u32,
            last_error: row.get(13)?,
        })
    }

    const SELECT: &'static str = "SELECT id, service, creator_id, creator_name,
        destination_collection_id, enabled, initial_import, auto_download, download_scope,
        poll_interval_minutes, last_checked_at, next_check_at, failure_count, last_error
        FROM subscriptions";

    pub fn list(&self) -> Result<Vec<Subscription>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let mut statement = connection
            .prepare(&format!(
                "{} ORDER BY creator_name COLLATE NOCASE, id",
                Self::SELECT
            ))
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], Self::map_row)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get(&self, id: &str) -> Result<Option<Subscription>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .query_row(
                &format!("{} WHERE id = ?1", Self::SELECT),
                params![id],
                Self::map_row,
            )
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn find(&self, service: &str, creator_id: &str) -> Result<Option<Subscription>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .query_row(
                &format!("{} WHERE service = ?1 AND creator_id = ?2", Self::SELECT),
                params![service, creator_id],
                Self::map_row,
            )
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn upsert(&self, input: &SubscriptionInput) -> Result<(Subscription, bool), String> {
        if !matches!(input.initial_import.as_str(), "none" | "latest" | "all") {
            return Err("Invalid initial import policy".to_string());
        }
        if !matches!(input.download_scope.as_str(), "primary" | "all") {
            return Err("Invalid download scope".to_string());
        }
        let interval = input.poll_interval_minutes.clamp(5, 10080);
        let collection = input
            .destination_collection_id
            .as_deref()
            .unwrap_or(INBOX_COLLECTION_ID);
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let exists: Option<String> = connection
            .query_row(
                "SELECT id FROM subscriptions WHERE service = ?1 AND creator_id = ?2",
                params![input.service, input.creator_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let created = exists.is_none();
        let id = exists.unwrap_or_else(|| Uuid::new_v4().to_string());
        connection.execute(
            "INSERT INTO subscriptions (id, service, creator_id, creator_name, destination_collection_id,
                enabled, initial_import, auto_download, download_scope, poll_interval_minutes, next_check_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7, ?8, ?9, CURRENT_TIMESTAMP)
             ON CONFLICT(service, creator_id) DO UPDATE SET
                creator_name = excluded.creator_name,
                destination_collection_id = excluded.destination_collection_id,
                enabled = 1, initial_import = excluded.initial_import,
                auto_download = excluded.auto_download, download_scope = excluded.download_scope,
                poll_interval_minutes = excluded.poll_interval_minutes,
                next_check_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP",
            params![id, input.service, input.creator_id, input.creator_name, collection,
                input.initial_import, input.auto_download as i64, input.download_scope, interval],
        ).map_err(|e| e.to_string())?;
        Ok((
            self.get_locked(&connection, &id)?
                .ok_or("Subscription was not saved")?,
            created,
        ))
    }

    fn get_locked(
        &self,
        connection: &Connection,
        id: &str,
    ) -> Result<Option<Subscription>, String> {
        connection
            .query_row(
                &format!("{} WHERE id = ?1", Self::SELECT),
                params![id],
                Self::map_row,
            )
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn due(&self) -> Result<Vec<Subscription>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let mut statement = connection.prepare(&format!("{} WHERE enabled = 1 AND next_check_at <= CURRENT_TIMESTAMP ORDER BY next_check_at LIMIT 20", Self::SELECT)).map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], Self::map_row)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn seen(&self, id: &str, identities: &[String]) -> Result<HashSet<String>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let mut statement = connection.prepare("SELECT 1 FROM subscription_seen_posts WHERE subscription_id = ?1 AND source_identity = ?2").map_err(|e| e.to_string())?;
        let mut result = HashSet::new();
        for identity in identities {
            if statement
                .query_row(params![id, identity], |_| Ok(()))
                .optional()
                .map_err(|e| e.to_string())?
                .is_some()
            {
                result.insert(identity.clone());
            }
        }
        Ok(result)
    }

    pub fn mark_seen(&self, id: &str, identities: &[String]) -> Result<(), String> {
        let mut connection = self.connection.lock().map_err(|e| e.to_string())?;
        let transaction = connection.transaction().map_err(|e| e.to_string())?;
        for identity in identities {
            transaction.execute("INSERT OR IGNORE INTO subscription_seen_posts (subscription_id, source_identity) VALUES (?1, ?2)", params![id, identity]).map_err(|e| e.to_string())?;
        }
        transaction.commit().map_err(|e| e.to_string())
    }

    pub fn mark_success(&self, id: &str) -> Result<Subscription, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .execute(
                "UPDATE subscriptions SET last_checked_at = CURRENT_TIMESTAMP,
            next_check_at = datetime('now', '+' || poll_interval_minutes || ' minutes'),
            failure_count = 0, last_error = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
                params![id],
            )
            .map_err(|e| e.to_string())?;
        self.get_locked(&connection, id)?
            .ok_or_else(|| "Subscription not found".to_string())
    }

    pub fn mark_failure(&self, id: &str, error: &str) -> Result<Subscription, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection.execute("UPDATE subscriptions SET last_checked_at = CURRENT_TIMESTAMP,
            next_check_at = datetime('now', '+' || MIN(360, 5 * (1 << MIN(failure_count, 6))) || ' minutes'),
            failure_count = failure_count + 1, last_error = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?1", params![id, error]).map_err(|e| e.to_string())?;
        self.get_locked(&connection, id)?
            .ok_or_else(|| "Subscription not found".to_string())
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<Subscription, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection.execute("UPDATE subscriptions SET enabled = ?2, next_check_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = ?1", params![id, enabled as i64]).map_err(|e| e.to_string())?;
        self.get_locked(&connection, id)?
            .ok_or_else(|| "Subscription not found".to_string())
    }

    pub fn delete(&self, id: &str) -> Result<bool, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .execute("DELETE FROM subscriptions WHERE id = ?1", params![id])
            .map(|n| n > 0)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> SubscriptionInput {
        SubscriptionInput {
            service: "patreon".into(),
            creator_id: "42".into(),
            creator_name: "Artist".into(),
            destination_collection_id: None,
            initial_import: "none".into(),
            auto_download: false,
            download_scope: "primary".into(),
            poll_interval_minutes: 30,
        }
    }

    #[test]
    fn upsert_is_unique_and_preserves_identity() {
        let repository = SubscriptionRepository::in_memory();
        let (first, created) = repository.upsert(&input()).unwrap();
        let (second, created_again) = repository.upsert(&input()).unwrap();
        assert!(created);
        assert!(!created_again);
        assert_eq!(first.id, second.id);
        assert_eq!(repository.list().unwrap().len(), 1);
    }

    #[test]
    fn seen_posts_are_deduplicated() {
        let repository = SubscriptionRepository::in_memory();
        let (subscription, _) = repository.upsert(&input()).unwrap();
        repository
            .mark_seen(
                &subscription.id,
                &["patreon:42:7".into(), "patreon:42:7".into()],
            )
            .unwrap();
        let seen = repository
            .seen(
                &subscription.id,
                &["patreon:42:7".into(), "patreon:42:8".into()],
            )
            .unwrap();
        assert_eq!(seen.len(), 1);
    }
}
