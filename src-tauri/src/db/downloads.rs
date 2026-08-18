use crate::db::storage::open_database;
#[cfg(test)]
use crate::db::storage::prepare_connection;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadJob {
    pub id: String,
    pub service: String,
    pub creator_id: String,
    pub post_id: String,
    pub media_id: String,
    pub url: String,
    pub filename: String,
    pub output_dir: String,
    pub temp_path: String,
    pub final_path: String,
    pub engine: String,
    pub status: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
    pub sha256: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub retry_count: u32,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub post_title: String,
    pub creator_name: String,
    pub post_preview_path: Option<String>,
    pub post_preview_url: Option<String>,
    pub creator_avatar_path: Option<String>,
}

pub struct NewDownloadJob<'a> {
    pub id: &'a str,
    pub logical_key: &'a str,
    pub service: &'a str,
    pub creator_id: &'a str,
    pub post_id: &'a str,
    pub media_id: &'a str,
    pub url: &'a str,
    pub filename: &'a str,
    pub output_dir: &'a str,
    pub temp_path: &'a str,
    pub final_path: &'a str,
    pub engine: &'a str,
}

pub struct DownloadRepository {
    connection: Mutex<Connection>,
}

impl DownloadRepository {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            connection: Mutex::new(open_database()?),
        })
    }

    #[cfg(test)]
    fn in_memory() -> Self {
        let mut connection = Connection::open_in_memory().unwrap();
        prepare_connection(&mut connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO creators (service,creator_id,name,snapshot_json)
                 VALUES ('patreon','creator','Creator','{}');
                 INSERT INTO posts (service,creator_id,post_id,title,snapshot_json)
                 VALUES ('patreon','creator','post','Post','{}');",
            )
            .unwrap();
        Self {
            connection: Mutex::new(connection),
        }
    }

    pub fn create_or_get(&self, input: NewDownloadJob<'_>) -> Result<DownloadJob, String> {
        let connection = self.connection.lock().map_err(|error| error.to_string())?;
        connection
            .execute(
                "INSERT OR IGNORE INTO download_jobs (
                    id, logical_key, service, creator_id, post_id, media_id,
                    url, filename, output_dir, temp_path, final_path, engine, status
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,'queued')",
                params![
                    input.id,
                    input.logical_key,
                    input.service,
                    input.creator_id,
                    input.post_id,
                    input.media_id,
                    input.url,
                    input.filename,
                    input.output_dir,
                    input.temp_path,
                    input.final_path,
                    input.engine
                ],
            )
            .map_err(|error| error.to_string())?;
        Self::get_by_logical_key(&connection, input.logical_key)?
            .ok_or_else(|| "Download job could not be created".to_string())
    }

    pub fn list(&self) -> Result<Vec<DownloadJob>, String> {
        let connection = self.connection.lock().map_err(|error| error.to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT id, service, creator_id, post_id, media_id, url, filename, output_dir, temp_path, final_path, engine,
                        status, downloaded_bytes, total_bytes, speed_bps, sha256,
                        error_code, error_message, retry_count, created_at, updated_at,
                        completed_at, p.title, c.name, p.preview_path, c.avatar_path
                 FROM download_jobs d JOIN posts p USING(service,creator_id,post_id)
                 JOIN creators c USING(service,creator_id)
                 ORDER BY
                    CASE status
                      WHEN 'downloading' THEN 0 WHEN 'resolving' THEN 1
                      WHEN 'verifying' THEN 2 WHEN 'queued' THEN 3
                      WHEN 'paused' THEN 4 WHEN 'failed' THEN 5 ELSE 6 END,
                    updated_at DESC, id DESC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], Self::map_job)
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn get(&self, id: &str) -> Result<Option<DownloadJob>, String> {
        let connection = self.connection.lock().map_err(|error| error.to_string())?;
        connection
            .query_row(
                "SELECT id, service, creator_id, post_id, media_id, url, filename, output_dir, temp_path, final_path, engine,
                        status, downloaded_bytes, total_bytes, speed_bps, sha256,
                        error_code, error_message, retry_count, created_at, updated_at,
                        completed_at
                        , p.title, c.name, p.preview_path, c.avatar_path
                 FROM download_jobs d JOIN posts p USING(service,creator_id,post_id)
                 JOIN creators c USING(service,creator_id) WHERE d.id = ?1",
                params![id],
                Self::map_job,
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    pub fn recover_interrupted(&self) -> Result<Vec<String>, String> {
        let connection = self.connection.lock().map_err(|error| error.to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT id FROM download_jobs
                 WHERE status IN ('queued', 'resolving', 'downloading', 'verifying')",
            )
            .map_err(|error| error.to_string())?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "UPDATE download_jobs SET status = 'queued', speed_bps = 0,
                    error_code = NULL, error_message = NULL, updated_at = CURRENT_TIMESTAMP
                 WHERE status IN ('resolving', 'downloading', 'verifying')",
                [],
            )
            .map_err(|error| error.to_string())?;
        Ok(ids)
    }

    pub fn update_status(&self, id: &str, status: &str) -> Result<DownloadJob, String> {
        let connection = self.connection.lock().map_err(|error| error.to_string())?;
        connection
            .execute(
                "UPDATE download_jobs SET status = ?2, speed_bps = 0,
                    error_code = NULL, error_message = NULL, updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1",
                params![id, status],
            )
            .map_err(|error| error.to_string())?;
        Self::get_by_id(&connection, id)?.ok_or_else(|| "Download job not found".to_string())
    }

    pub fn update_progress(
        &self,
        id: &str,
        downloaded: u64,
        total: u64,
        speed: u64,
    ) -> Result<DownloadJob, String> {
        let connection = self.connection.lock().map_err(|error| error.to_string())?;
        connection
            .execute(
                "UPDATE download_jobs SET status = 'downloading', downloaded_bytes = ?2,
                    total_bytes = CASE WHEN ?3 > 0 THEN ?3 ELSE total_bytes END,
                    speed_bps = ?4, updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1",
                params![id, downloaded, total, speed],
            )
            .map_err(|error| error.to_string())?;
        Self::get_by_id(&connection, id)?.ok_or_else(|| "Download job not found".to_string())
    }

    pub fn update_total_size(&self, id: &str, total: u64) -> Result<DownloadJob, String> {
        let connection = self.connection.lock().map_err(|error| error.to_string())?;
        connection
            .execute(
                "UPDATE download_jobs SET total_bytes=?2, updated_at=CURRENT_TIMESTAMP
                 WHERE id=?1 AND total_bytes=0",
                params![id, total],
            )
            .map_err(|error| error.to_string())?;
        Self::get_by_id(&connection, id)?.ok_or_else(|| "Download job not found".to_string())
    }

    pub fn mark_failed(&self, id: &str, code: &str, message: &str) -> Result<DownloadJob, String> {
        let connection = self.connection.lock().map_err(|error| error.to_string())?;
        connection
            .execute(
                "UPDATE download_jobs SET status = 'failed', speed_bps = 0,
                    error_code = ?2, error_message = ?3, updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1",
                params![id, code, message],
            )
            .map_err(|error| error.to_string())?;
        Self::get_by_id(&connection, id)?.ok_or_else(|| "Download job not found".to_string())
    }

    pub fn mark_completed(
        &self,
        id: &str,
        sha256: &str,
        size: u64,
        relative_blob_path: &str,
    ) -> Result<DownloadJob, String> {
        let mut connection = self.connection.lock().map_err(|error| error.to_string())?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "INSERT INTO media_blobs (sha256, size, relative_path)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(sha256) DO UPDATE SET last_seen_at = CURRENT_TIMESTAMP",
                params![sha256, size, relative_blob_path],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "INSERT INTO download_blob_refs (job_id, blob_sha256)
                 VALUES (?1, ?2)
                 ON CONFLICT(job_id) DO UPDATE SET blob_sha256 = excluded.blob_sha256",
                params![id, sha256],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "UPDATE download_jobs SET status = 'completed', downloaded_bytes = ?2,
                    total_bytes = ?2, speed_bps = 0, sha256 = ?3,
                    error_code = NULL, error_message = NULL,
                    updated_at = CURRENT_TIMESTAMP, completed_at = CURRENT_TIMESTAMP
                 WHERE id = ?1",
                params![id, size, sha256],
            )
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
        Self::get_by_id(&connection, id)?.ok_or_else(|| "Download job not found".to_string())
    }

    pub fn retry(&self, id: &str) -> Result<DownloadJob, String> {
        let connection = self.connection.lock().map_err(|error| error.to_string())?;
        connection
            .execute(
                "UPDATE download_jobs SET status = 'queued', retry_count = retry_count + 1,
                    speed_bps = 0, error_code = NULL, error_message = NULL,
                    updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1 AND status IN ('failed', 'paused', 'missing', 'cancelled')",
                params![id],
            )
            .map_err(|error| error.to_string())?;
        Self::get_by_id(&connection, id)?.ok_or_else(|| "Download job not found".to_string())
    }

    pub fn remove(&self, id: &str) -> Result<bool, String> {
        let mut connection = self.connection.lock().map_err(|error| error.to_string())?;
        let identity: Option<(String, String, String)> = connection
            .query_row(
                "SELECT service,creator_id,post_id FROM download_jobs WHERE id=?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let transaction = connection.transaction().map_err(|e| e.to_string())?;
        let changed = transaction
            .execute("DELETE FROM download_jobs WHERE id = ?1", params![id])
            .map_err(|error| error.to_string())?
            > 0;
        if changed {
            if let Some((service, creator, post)) = identity {
                transaction.execute(
            "DELETE FROM content_pins WHERE entity_kind='post' AND service=?1 AND creator_id=?2 AND post_id=?3 AND reason='download'
             AND NOT EXISTS(SELECT 1 FROM download_jobs WHERE service=?1 AND creator_id=?2 AND post_id=?3)",
            params![service,creator,post]
        ).map_err(|e|e.to_string())?;
            }
        }
        transaction.commit().map_err(|e| e.to_string())?;
        Ok(changed)
    }

    pub fn take_orphan_blob(&self, sha256: &str) -> Result<Option<String>, String> {
        let mut connection = self.connection.lock().map_err(|error| error.to_string())?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        let relative_path: Option<String> = transaction
            .query_row(
                "SELECT relative_path FROM media_blobs b WHERE b.sha256=?1
                 AND NOT EXISTS(SELECT 1 FROM download_blob_refs r WHERE r.blob_sha256=b.sha256)",
                params![sha256],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if relative_path.is_some() {
            transaction
                .execute("DELETE FROM media_blobs WHERE sha256=?1", params![sha256])
                .map_err(|error| error.to_string())?;
        }
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(relative_path)
    }

    pub fn queue_progress_stats(&self) -> Result<(i32, i32, u64, u64, u64), String> {
        let connection = self.connection.lock().map_err(|error| error.to_string())?;
        connection
            .query_row(
                "SELECT
                    COALESCE(SUM(CASE WHEN status IN ('downloading', 'resolving', 'verifying') THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN status IN ('queued', 'downloading', 'resolving', 'verifying') THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN status IN ('queued', 'downloading', 'resolving', 'verifying') THEN downloaded_bytes ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN status IN ('queued', 'downloading', 'resolving', 'verifying') THEN total_bytes ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN status IN ('downloading', 'resolving', 'verifying') THEN speed_bps ELSE 0 END), 0)
                 FROM download_jobs",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)? as i32,
                        row.get::<_, i64>(1)? as i32,
                        row.get::<_, i64>(2)? as u64,
                        row.get::<_, i64>(3)? as u64,
                        row.get::<_, i64>(4)? as u64,
                    ))
                },
            )
            .map_err(|error| error.to_string())
    }

    fn get_by_logical_key(
        connection: &Connection,
        logical_key: &str,
    ) -> Result<Option<DownloadJob>, String> {
        connection
            .query_row(
                "SELECT id, service, creator_id, post_id, media_id, url, filename, output_dir, temp_path, final_path, engine,
                        status, downloaded_bytes, total_bytes, speed_bps, sha256,
                        error_code, error_message, retry_count, created_at, updated_at,
                        completed_at
                        , p.title, c.name, p.preview_path, c.avatar_path
                 FROM download_jobs d JOIN posts p USING(service,creator_id,post_id)
                 JOIN creators c USING(service,creator_id) WHERE d.logical_key = ?1",
                params![logical_key],
                Self::map_job,
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    fn get_by_id(connection: &Connection, id: &str) -> Result<Option<DownloadJob>, String> {
        connection
            .query_row(
                "SELECT id, service, creator_id, post_id, media_id, url, filename, output_dir, temp_path, final_path, engine,
                        status, downloaded_bytes, total_bytes, speed_bps, sha256,
                        error_code, error_message, retry_count, created_at, updated_at,
                        completed_at
                        , p.title, c.name, p.preview_path, c.avatar_path
                 FROM download_jobs d JOIN posts p USING(service,creator_id,post_id)
                 JOIN creators c USING(service,creator_id) WHERE d.id = ?1",
                params![id],
                Self::map_job,
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    fn map_job(row: &rusqlite::Row<'_>) -> rusqlite::Result<DownloadJob> {
        Ok(DownloadJob {
            id: row.get(0)?,
            service: row.get(1)?,
            creator_id: row.get(2)?,
            post_id: row.get(3)?,
            media_id: row.get(4)?,
            url: row.get(5)?,
            filename: row.get(6)?,
            output_dir: row.get(7)?,
            temp_path: row.get(8)?,
            final_path: row.get(9)?,
            engine: row.get(10)?,
            status: row.get(11)?,
            downloaded_bytes: row.get::<_, i64>(12)?.max(0) as u64,
            total_bytes: row.get::<_, i64>(13)?.max(0) as u64,
            speed_bps: row.get::<_, i64>(14)?.max(0) as u64,
            sha256: row.get(15)?,
            error_code: row.get(16)?,
            error_message: row.get(17)?,
            retry_count: row.get::<_, i64>(18)?.max(0) as u32,
            created_at: row.get(19)?,
            updated_at: row.get(20)?,
            completed_at: row.get(21)?,
            post_title: row.get(22)?,
            creator_name: row.get(23)?,
            post_preview_path: row.get(24)?,
            post_preview_url: None,
            creator_avatar_path: row.get(25)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input<'a>() -> NewDownloadJob<'a> {
        NewDownloadJob {
            id: "job-1",
            logical_key: "https://example.test/file\nC:/Downloads/file.bin",
            service: "patreon",
            creator_id: "creator",
            post_id: "post",
            media_id: "media",
            url: "https://example.test/file",
            filename: "file.bin",
            output_dir: "C:/Downloads",
            temp_path: "C:/Downloads/.temp/job-1.part",
            final_path: "C:/Downloads/file.bin",
            engine: "native",
        }
    }

    #[test]
    fn duplicate_logical_download_returns_existing_job() {
        let repository = DownloadRepository::in_memory();
        let first = repository.create_or_get(input()).unwrap();
        let duplicate = repository
            .create_or_get(NewDownloadJob {
                id: "job-2",
                ..input()
            })
            .unwrap();
        assert_eq!(first.id, duplicate.id);
        assert_eq!(repository.list().unwrap().len(), 1);
    }

    #[test]
    fn interrupted_jobs_return_to_queue() {
        let repository = DownloadRepository::in_memory();
        repository.create_or_get(input()).unwrap();
        repository.update_status("job-1", "downloading").unwrap();
        assert_eq!(repository.recover_interrupted().unwrap(), vec!["job-1"]);
        assert_eq!(repository.get("job-1").unwrap().unwrap().status, "queued");
    }
}
