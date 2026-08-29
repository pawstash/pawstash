use crate::api::models::Post;
#[cfg(test)]
use crate::db::storage::prepare_connection;
use crate::db::storage::{open_database, INBOX_COLLECTION_ID};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct LibraryCollection {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub item_count: u64,
    pub is_system: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct LibraryPostIdentity {
    pub service: String,
    pub creator_id: String,
    pub post_id: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct PostStashMembership {
    pub collection_id: String,
    pub service: String,
    pub creator_id: String,
    pub post_id: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct LibrarySaveResult {
    pub entry_id: String,
    pub created: bool,
    pub membership_added: bool,
}

pub struct LibraryRepository {
    connection: Mutex<Connection>,
}
impl LibraryRepository {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            connection: Mutex::new(open_database()?),
        })
    }
    #[cfg(test)]
    fn in_memory() -> Self {
        let mut c = Connection::open_in_memory().unwrap();
        prepare_connection(&mut c).unwrap();
        Self {
            connection: Mutex::new(c),
        }
    }

    pub fn list_collections(&self) -> Result<Vec<LibraryCollection>, String> {
        let c = self.connection.lock().map_err(|e| e.to_string())?;
        let mut s=c.prepare("SELECT c.id,c.kind,c.name,c.parent_id,c.is_system,COUNT(cp.post_id) FROM collections c LEFT JOIN collection_posts cp ON cp.collection_id=c.id GROUP BY c.id ORDER BY c.is_system DESC,c.position,c.name COLLATE NOCASE").map_err(|e|e.to_string())?;
        let result = s
            .query_map([], |r| {
                Ok(LibraryCollection {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                    name: r.get(2)?,
                    parent_id: r.get(3)?,
                    is_system: r.get::<_, i64>(4)? != 0,
                    item_count: r.get::<_, i64>(5)?.max(0) as u64,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string());
        result
    }
    pub fn create_stash(&self, name: &str) -> Result<LibraryCollection, String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 100 {
            return Err("Stash name must contain between 1 and 100 characters".into());
        }
        let id = Uuid::new_v4().to_string();
        let c = self.connection.lock().map_err(|e| e.to_string())?;
        let pos: i64 = c
            .query_row(
                "SELECT COALESCE(MAX(position),0)+1 FROM collections",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        c.execute(
            "INSERT INTO collections(id,kind,name,position,is_system) VALUES(?1,'stash',?2,?3,0)",
            params![id, name, pos],
        )
        .map_err(|e| e.to_string())?;
        Ok(LibraryCollection {
            id,
            kind: "stash".into(),
            name: name.into(),
            parent_id: None,
            item_count: 0,
            is_system: false,
        })
    }
    pub fn delete_stash(&self, id: &str) -> Result<bool, String> {
        if id == INBOX_COLLECTION_ID {
            return Err("The Inbox collection cannot be deleted".into());
        }
        let c = self.connection.lock().map_err(|e| e.to_string())?;
        c.execute(
            "DELETE FROM collections WHERE id=?1 AND kind='stash' AND is_system=0",
            params![id],
        )
        .map(|n| n > 0)
        .map_err(|e| e.to_string())
    }

    pub fn rename_stash(&self, id: &str, name: &str) -> Result<bool, String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 100 {
            return Err("Stash name must contain between 1 and 100 characters".into());
        }
        let c = self.connection.lock().map_err(|e| e.to_string())?;
        c.execute(
            "UPDATE collections SET name=?2 WHERE id=?1",
            params![id, name],
        )
        .map(|n| n > 0)
        .map_err(|e| e.to_string())
    }

    pub fn reorder_stashes(&self, collection_ids: &[String]) -> Result<bool, String> {
        let mut c = self.connection.lock().map_err(|e| e.to_string())?;
        let tx = c.transaction().map_err(|e| e.to_string())?;
        for (idx, id) in collection_ids.iter().enumerate() {
            tx.execute(
                "UPDATE collections SET position=?2 WHERE id=?1 AND kind='stash' AND is_system=0",
                params![id, (idx + 1) as i64],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(true)
    }

    pub fn clear_stash(&self, id: &str) -> Result<u64, String> {
        let mut c = self.connection.lock().map_err(|e| e.to_string())?;
        let tx = c.transaction().map_err(|e| e.to_string())?;
        let count = tx
            .execute(
                "DELETE FROM collection_posts WHERE collection_id=?1",
                params![id],
            )
            .map_err(|e| e.to_string())? as u64;
        tx.execute(
            "DELETE FROM content_pins WHERE entity_kind='post' AND reason='library' AND (service, creator_id, post_id) NOT IN (SELECT service, creator_id, post_id FROM collection_posts)",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(count)
    }

    pub fn remove_post_from_stash(
        &self,
        collection_id: &str,
        service: &str,
        creator: &str,
        post: &str,
    ) -> Result<bool, String> {
        let mut c = self.connection.lock().map_err(|e| e.to_string())?;
        let tx = c.transaction().map_err(|e| e.to_string())?;
        let changed = tx
            .execute(
                "DELETE FROM collection_posts WHERE collection_id=?1 AND service=?2 AND creator_id=?3 AND post_id=?4",
                params![collection_id, service, creator, post],
            )
            .map_err(|e| e.to_string())?
            > 0;
        let remaining: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM collection_posts WHERE service=?1 AND creator_id=?2 AND post_id=?3",
                params![service, creator, post],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if remaining == 0 {
            let _ = tx.execute(
                "DELETE FROM content_pins WHERE entity_kind='post' AND service=?1 AND creator_id=?2 AND post_id=?3 AND reason='library'",
                params![service, creator, post],
            );
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(changed)
    }

    pub fn list_post_collections(
        &self,
        service: &str,
        creator: &str,
        post: &str,
    ) -> Result<Vec<String>, String> {
        let c = self.connection.lock().map_err(|e| e.to_string())?;
        let mut s = c
            .prepare("SELECT collection_id FROM collection_posts WHERE service=?1 AND creator_id=?2 AND post_id=?3")
            .map_err(|e| e.to_string())?;
        let result = s
            .query_map(params![service, creator, post], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| e.to_string());
        result
    }

    pub fn save_post(
        &self,
        post: &Post,
        collection_id: Option<&str>,
    ) -> Result<LibrarySaveResult, String> {
        let collection = collection_id.unwrap_or(INBOX_COLLECTION_ID);
        let snapshot = serde_json::to_string(post).map_err(|e| e.to_string())?;
        let mut c = self.connection.lock().map_err(|e| e.to_string())?;
        let tx = c.transaction().map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO creators(service,creator_id,name,snapshot_json) VALUES(?1,?2,?2,json_object('id',?2,'name',?2,'service',?1)) ON CONFLICT(service,creator_id) DO NOTHING",params![post.service,post.user]).map_err(|e|e.to_string())?;
        let created=tx.execute("INSERT OR IGNORE INTO posts(service,creator_id,post_id,title,content,published_at,snapshot_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![post.service,post.user,post.id,post.title,post.content,post.published,snapshot]).map_err(|e|e.to_string())?>0;
        if !created {
            tx.execute("UPDATE posts SET title=?4,content=?5,published_at=?6,snapshot_json=?7,cached_at=CURRENT_TIMESTAMP WHERE service=?1 AND creator_id=?2 AND post_id=?3",params![post.service,post.user,post.id,post.title,post.content,post.published,snapshot]).map_err(|e|e.to_string())?;
        }
        let membership=tx.execute("INSERT OR IGNORE INTO collection_posts(collection_id,service,creator_id,post_id,operation_id) VALUES(?1,?2,?3,?4,?5)",params![collection,post.service,post.user,post.id,Uuid::new_v4().to_string()]).map_err(|e|e.to_string())?>0;
        tx.execute("INSERT OR IGNORE INTO content_pins(entity_kind,service,creator_id,post_id,reason) VALUES('post',?1,?2,?3,'library')",params![post.service,post.user,post.id]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(LibrarySaveResult {
            entry_id: format!("{}:{}:{}", post.service, post.user, post.id),
            created,
            membership_added: membership,
        })
    }
    pub fn remove_post(&self, service: &str, creator: &str, post: &str) -> Result<bool, String> {
        let mut c = self.connection.lock().map_err(|e| e.to_string())?;
        let tx = c.transaction().map_err(|e| e.to_string())?;
        let changed = tx
            .execute(
                "DELETE FROM collection_posts WHERE service=?1 AND creator_id=?2 AND post_id=?3",
                params![service, creator, post],
            )
            .map_err(|e| e.to_string())?
            > 0;
        tx.execute("DELETE FROM content_pins WHERE entity_kind='post' AND service=?1 AND creator_id=?2 AND post_id=?3 AND reason='library'",params![service,creator,post]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(changed)
    }
    pub fn list_saved_post_identities(&self) -> Result<Vec<LibraryPostIdentity>, String> {
        let c = self.connection.lock().map_err(|e| e.to_string())?;
        let mut s=c.prepare("SELECT DISTINCT service,creator_id,post_id FROM collection_posts ORDER BY service,creator_id,post_id").map_err(|e|e.to_string())?;
        let result = s
            .query_map([], |r| {
                Ok(LibraryPostIdentity {
                    service: r.get(0)?,
                    creator_id: r.get(1)?,
                    post_id: r.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string());
        result
    }
    pub fn list_post_stash_memberships(&self) -> Result<Vec<PostStashMembership>, String> {
        let c = self.connection.lock().map_err(|e| e.to_string())?;
        let mut s = c
            .prepare("SELECT collection_id, service, creator_id, post_id FROM collection_posts")
            .map_err(|e| e.to_string())?;
        let result = s
            .query_map([], |r| {
                Ok(PostStashMembership {
                    collection_id: r.get(0)?,
                    service: r.get(1)?,
                    creator_id: r.get(2)?,
                    post_id: r.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string());
        result
    }
    pub fn list_posts(
        &self,
        collection_id: Option<&str>,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<Post>, String> {
        let c = self.connection.lock().map_err(|e| e.to_string())?;
        let (sql, args_collection) = if collection_id.is_some() {
            ("SELECT p.snapshot_json,cp.added_at FROM collection_posts cp JOIN posts p USING(service,creator_id,post_id) WHERE cp.collection_id=?1 ORDER BY cp.added_at DESC LIMIT ?2 OFFSET ?3",collection_id)
        } else {
            ("SELECT p.snapshot_json,MIN(cp.added_at) AS library_added_at FROM collection_posts cp JOIN posts p USING(service,creator_id,post_id) GROUP BY cp.service,cp.creator_id,cp.post_id ORDER BY library_added_at DESC LIMIT ?2 OFFSET ?3",None)
        };
        let mut s = c.prepare(sql).map_err(|e| e.to_string())?;
        let mut output = Vec::new();
        if let Some(id) = args_collection {
            let rows = s
                .query_map(params![id, limit, offset], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                let (snapshot, library_added_at) = row.map_err(|e| e.to_string())?;
                let mut post: Post = Post::from_json_str(&snapshot).map_err(|e| e.to_string())?;
                post.extra.insert(
                    "library_added_at".into(),
                    serde_json::Value::String(library_added_at),
                );
                output.push(post);
            }
        } else {
            let rows = s
                .query_map(params!["", limit, offset], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                let (snapshot, library_added_at) = row.map_err(|e| e.to_string())?;
                let mut post: Post = Post::from_json_str(&snapshot).map_err(|e| e.to_string())?;
                post.extra.insert(
                    "library_added_at".into(),
                    serde_json::Value::String(library_added_at),
                );
                output.push(post);
            }
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn post() -> Post {
        serde_json::from_value(serde_json::json!({"id":"p","user":"c","service":"s","title":"t"}))
            .unwrap()
    }
    #[test]
    fn save_is_idempotent() {
        let r = LibraryRepository::in_memory();
        assert!(r.save_post(&post(), None).unwrap().created);
        assert!(!r.save_post(&post(), None).unwrap().created);
        let posts = r.list_posts(None, 0, 10).unwrap();
        assert_eq!(posts.len(), 1);
        assert!(posts[0].extra["library_added_at"].as_str().is_some());
    }

    #[test]
    fn reorder_stashes_persists_order() {
        let r = LibraryRepository::in_memory();
        let s1 = r.create_stash("First").unwrap();
        let s2 = r.create_stash("Second").unwrap();
        let s3 = r.create_stash("Third").unwrap();

        let initial = r.list_collections().unwrap();
        let initial_stashes: Vec<_> = initial.iter().filter(|c| c.kind == "stash").collect();
        assert_eq!(initial_stashes[0].id, s1.id);
        assert_eq!(initial_stashes[1].id, s2.id);
        assert_eq!(initial_stashes[2].id, s3.id);

        r.reorder_stashes(&[s3.id.clone(), s1.id.clone(), s2.id.clone()])
            .unwrap();

        let reordered = r.list_collections().unwrap();
        let reordered_stashes: Vec<_> = reordered.iter().filter(|c| c.kind == "stash").collect();
        assert_eq!(reordered_stashes[0].id, s3.id);
        assert_eq!(reordered_stashes[1].id, s1.id);
        assert_eq!(reordered_stashes[2].id, s2.id);
    }
}
