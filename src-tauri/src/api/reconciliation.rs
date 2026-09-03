use crate::api::models::{Attachment, Post, PostRevision};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciledPost {
    pub post: Post,
    pub revisions: Vec<PostRevision>,
    pub available_providers: Vec<String>,
    pub attachment_sources: HashMap<String, Vec<String>>,
}

fn parse_timestamp(date_str: Option<&str>) -> i64 {
    let Some(s) = date_str else { return 0 };
    if let Ok(ts) = chrono_parse_timestamp(s) {
        return ts;
    }
    0
}

fn chrono_parse_timestamp(s: &str) -> Result<i64, ()> {
    let clean = s.trim().trim_matches('"');
    if clean.is_empty() {
        return Err(());
    }

    if let Ok(num) = clean.parse::<i64>() {
        return Ok(num);
    }

    if let Ok(num) = clean.parse::<f64>() {
        return Ok(num as i64);
    }

    let mut parts = clean.replace('T', " ");
    if let Some(idx) = parts.find('+') {
        parts.truncate(idx);
    }
    if let Some(idx) = parts.find('Z') {
        parts.truncate(idx);
    }

    let p: Vec<&str> = parts.split(&['-', ':', ' ', '.'][..]).collect();
    if p.len() >= 3 {
        let year = p[0].parse::<i64>().unwrap_or(0);
        let month = p[1].parse::<i64>().unwrap_or(0);
        let day = p[2].parse::<i64>().unwrap_or(0);
        let hour = p.get(3).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
        let min = p.get(4).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
        let sec = p.get(5).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);

        let rough_seconds =
            year * 31_536_000 + month * 2_592_000 + day * 86_400 + hour * 3600 + min * 60 + sec;
        return Ok(rough_seconds);
    }

    Err(())
}

fn score_snapshot(post: &Post) -> (i64, i64, usize, bool) {
    let edited_ts = parse_timestamp(post.edited.as_deref());
    let added_ts = parse_timestamp(post.added.as_deref());
    let att_count = post.attachments.as_ref().map(|a| a.len()).unwrap_or(0)
        + (if post.file.is_some() { 1 } else { 0 });
    let has_full = post.has_full.unwrap_or(false);
    (edited_ts, added_ts, att_count, has_full)
}

pub fn reconcile_post_snapshots(snapshots: Vec<(String, Post)>) -> Option<ReconciledPost> {
    if snapshots.is_empty() {
        return None;
    }

    let mut available_providers = Vec::new();
    for (prov_id, _) in &snapshots {
        if !available_providers.contains(prov_id) {
            available_providers.push(prov_id.clone());
        }
    }

    let mut best_index = 0;
    let mut best_score = score_snapshot(&snapshots[0].1);

    for (i, (_, post)) in snapshots.iter().enumerate().skip(1) {
        let score = score_snapshot(post);
        if score > best_score {
            best_score = score;
            best_index = i;
        }
    }

    let mut canonical_post = snapshots[best_index].1.clone();
    let mut attachment_sources: HashMap<String, Vec<String>> = HashMap::new();

    let mut merged_attachments: Vec<Attachment> = Vec::new();
    let mut seen_keys = HashSet::new();

    for (prov_id, post) in &snapshots {
        let mut all_files = Vec::new();
        if let Some(ref f) = post.file {
            all_files.push(f.clone());
        }
        if let Some(ref atts) = post.attachments {
            all_files.extend(atts.clone());
        }

        for file in all_files {
            let key = file
                .path
                .clone()
                .or_else(|| file.name.clone())
                .unwrap_or_default();
            if key.is_empty() {
                continue;
            }

            attachment_sources
                .entry(key.clone())
                .or_default()
                .push(prov_id.clone());

            if seen_keys.insert(key) {
                merged_attachments.push(file);
            }
        }
    }

    if !merged_attachments.is_empty() {
        if canonical_post.file.is_none() && !merged_attachments.is_empty() {
            canonical_post.file = Some(merged_attachments.remove(0));
        }
        canonical_post.attachments = Some(merged_attachments);
    }

    if canonical_post.prev.is_none() {
        canonical_post.prev = snapshots.iter().find_map(|(_, p)| p.prev.clone());
    }
    if canonical_post.next.is_none() {
        canonical_post.next = snapshots.iter().find_map(|(_, p)| p.next.clone());
    }

    let mut revisions_map: BTreeMap<String, PostRevision> = BTreeMap::new();
    let mut rev_counter = 1;

    for (_, post) in &snapshots {
        let key = format!(
            "{}:{}:{}",
            post.edited.as_deref().unwrap_or(""),
            post.added.as_deref().unwrap_or(""),
            post.title
        );
        if let std::collections::btree_map::Entry::Vacant(entry) = revisions_map.entry(key) {
            entry.insert(PostRevision {
                revision_id: rev_counter,
                post: post.clone(),
            });
            rev_counter += 1;
        }
    }

    let revisions: Vec<PostRevision> = revisions_map.into_values().collect();

    Some(ReconciledPost {
        post: canonical_post,
        revisions,
        available_providers,
        attachment_sources,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconciles_newer_edited_snapshot() {
        let post_old = Post {
            id: "1".into(),
            user: "user".into(),
            service: "patreon".into(),
            title: "Old Title".into(),
            content: Some("Old text".into()),
            substring: None,
            published: Some("2026-08-01T00:00:00".into()),
            added: Some("2026-08-01T12:00:00".into()),
            edited: None,
            embed: None,
            shared_file: None,
            attachments: Some(vec![Attachment {
                name: Some("file1.png".into()),
                path: Some("/path/file1.png".into()),
                server: None,
                size: Some(1024),
                extra: HashMap::new(),
            }]),
            file: None,
            poll: None,
            captions: None,
            tags: None,
            origin: None,
            preview_state: None,
            has_full: Some(true),
            detail_fetched: Some(true),
            next: None,
            prev: None,
            favorite_count: None,
            attachment_count: Some(1),
            extra: HashMap::new(),
        };

        let post_new = Post {
            id: "1".into(),
            user: "user".into(),
            service: "patreon".into(),
            title: "Updated Title".into(),
            content: Some("New text with additional attachments".into()),
            substring: None,
            published: Some("2026-08-01T00:00:00".into()),
            added: Some("2026-08-10T12:00:00".into()),
            edited: Some("2026-08-09T18:00:00".into()),
            embed: None,
            shared_file: None,
            attachments: Some(vec![
                Attachment {
                    name: Some("file1.png".into()),
                    path: Some("/path/file1.png".into()),
                    server: None,
                    size: Some(1024),
                    extra: HashMap::new(),
                },
                Attachment {
                    name: Some("file2.png".into()),
                    path: Some("/path/file2.png".into()),
                    server: None,
                    size: Some(2048),
                    extra: HashMap::new(),
                },
            ]),
            file: None,
            poll: None,
            captions: None,
            tags: None,
            origin: None,
            preview_state: None,
            has_full: Some(true),
            detail_fetched: Some(true),
            next: None,
            prev: None,
            favorite_count: None,
            attachment_count: Some(2),
            extra: HashMap::new(),
        };

        let snapshots = vec![
            ("mirror1".to_string(), post_old),
            ("mirror2".to_string(), post_new),
        ];

        let reconciled = reconcile_post_snapshots(snapshots).unwrap();
        assert_eq!(reconciled.post.title, "Updated Title");
        assert_eq!(reconciled.available_providers.len(), 2);
        assert_eq!(reconciled.revisions.len(), 2);
    }
}
