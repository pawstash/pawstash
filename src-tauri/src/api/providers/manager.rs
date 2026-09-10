use super::coomer::CoomerProvider;
use super::onlyhaven::OnlyHavenProvider;
use super::pawchive::PawchiveProvider;
use super::queue::{ProviderQueueConfig, ProviderRequestQueue};
use super::traits::{
    FavoritesSyncResult, PopularCapabilities, PopularPeriodOption, ProviderAuthSchema,
    ProviderCapabilities, ProviderConfig, ProviderHealth, SortOption, SourceProvider,
};
use crate::api::models::*;
use crate::api::reconciliation::{reconcile_post_snapshots, ReconciledPost};
use crate::db::storage::content_cache_path;
use futures_util::future::join_all;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

fn provider_errors(operation: &str, errors: Vec<String>) -> String {
    if errors.is_empty() {
        format!("No provider is configured for {operation}")
    } else {
        format!(
            "All providers failed to fetch {operation}: {}",
            errors.join("; ")
        )
    }
}

fn sanitize_cache_key(value: &str) -> String {
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

fn create_provider(config: ProviderConfig) -> Result<Arc<dyn SourceProvider>, String> {
    if OnlyHavenProvider::matches_config(&config) {
        Ok(Arc::new(OnlyHavenProvider::new(config)?))
    } else if CoomerProvider::matches_config(&config) {
        Ok(Arc::new(CoomerProvider::new(config)?))
    } else {
        Ok(Arc::new(PawchiveProvider::new(config)?))
    }
}

pub struct ProviderManager {
    providers: Arc<RwLock<Vec<Arc<dyn SourceProvider>>>>,
    default_queue: Arc<ProviderRequestQueue>,
}

impl ProviderManager {
    pub fn new(configs: Vec<ProviderConfig>) -> Self {
        let mut list: Vec<Arc<dyn SourceProvider>> = Vec::new();
        for config in configs {
            if let Ok(provider) = create_provider(config) {
                list.push(provider);
            }
        }

        if list.is_empty() {
            let default_configs = Self::default_configs();
            for config in default_configs {
                if let Ok(provider) = create_provider(config) {
                    list.push(provider);
                }
            }
        }

        Self {
            providers: Arc::new(RwLock::new(list)),
            default_queue: Arc::new(ProviderRequestQueue::new(
                "default",
                ProviderQueueConfig::default(),
            )),
        }
    }

    pub fn default_configs() -> Vec<ProviderConfig> {
        vec![
            PawchiveProvider::default_config(),
            CoomerProvider::default_config(),
            OnlyHavenProvider::default_config(),
        ]
    }

    pub fn validate_configs(configs: &[ProviderConfig]) -> Result<(), String> {
        for config in configs {
            create_provider(config.clone())?;
        }
        Ok(())
    }

    pub async fn get_provider_configs(&self) -> Vec<ProviderConfig> {
        let providers = self.providers.read().await;
        providers.iter().map(|p| p.config()).collect()
    }

    pub async fn update_providers(&self, configs: Vec<ProviderConfig>) -> Result<(), String> {
        let mut new_providers: Vec<Arc<dyn SourceProvider>> = Vec::new();
        for config in configs {
            let provider = create_provider(config)?;
            new_providers.push(provider);
        }
        *self.providers.write().await = new_providers;
        Ok(())
    }

    pub async fn get_providers_for_service(&self, service: &str) -> Vec<Arc<dyn SourceProvider>> {
        let providers = self.providers.read().await;
        let mut matching: Vec<Arc<dyn SourceProvider>> = providers
            .iter()
            .filter(|p| {
                let conf = p.config();
                conf.enabled && (conf.services.is_empty() || p.supports_service(service))
            })
            .cloned()
            .collect();

        matching.sort_by_key(|p| p.config().priority);
        matching
    }

    pub async fn get_all_enabled_providers(&self) -> Vec<Arc<dyn SourceProvider>> {
        let providers = self.providers.read().await;
        let mut enabled: Vec<Arc<dyn SourceProvider>> = providers
            .iter()
            .filter(|p| p.config().enabled)
            .cloned()
            .collect();
        enabled.sort_by_key(|p| p.config().priority);
        enabled
    }

    pub async fn get_provider_by_id(&self, id: &str) -> Option<Arc<dyn SourceProvider>> {
        let providers = self.providers.read().await;
        providers.iter().find(|p| p.config().id == id).cloned()
    }

    async fn get_provider_for_resolution(
        &self,
        service: &str,
        provider_id: Option<&str>,
    ) -> Option<Arc<dyn SourceProvider>> {
        if let Some(provider) = match provider_id {
            Some(id) => self.get_provider_by_id(id).await,
            None => None,
        } {
            return Some(provider);
        }
        if let Some(provider) = self
            .get_providers_for_service(service)
            .await
            .into_iter()
            .next()
        {
            return Some(provider);
        }
        if let Some(provider) = self.get_all_enabled_providers().await.into_iter().next() {
            return Some(provider);
        }
        if let Some(provider) = self.providers.read().await.first().cloned() {
            return Some(provider);
        }

        Self::default_configs()
            .into_iter()
            .find(|config| {
                config
                    .services
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(service))
            })
            .or_else(|| Self::default_configs().into_iter().next())
            .and_then(|config| create_provider(config).ok())
    }

    pub async fn get_queue_for_url(&self, url: &str) -> Arc<ProviderRequestQueue> {
        if let Ok(parsed) = reqwest::Url::parse(url) {
            if let Some(target_host) = parsed.host_str() {
                let target_host_clean = target_host.trim_start_matches("www.").to_ascii_lowercase();
                let providers = self.providers.read().await;
                for p in providers.iter() {
                    let conf = p.config();
                    let mut candidate_urls = vec![conf.api_url.clone()];
                    if let Some(fu) = &conf.file_url {
                        candidate_urls.push(fu.clone());
                    }
                    if let Some(iu) = &conf.image_url {
                        candidate_urls.push(iu.clone());
                    }
                    candidate_urls.extend(conf.fallback_urls.clone());

                    for cand in candidate_urls {
                        if let Ok(cand_parsed) = reqwest::Url::parse(&cand) {
                            if let Some(cand_host) = cand_parsed.host_str() {
                                let cand_host_clean = cand_host
                                    .trim_start_matches("www.")
                                    .trim_start_matches("api.")
                                    .to_ascii_lowercase();

                                if target_host_clean == cand_host_clean
                                    || target_host_clean.ends_with(&format!(".{cand_host_clean}"))
                                    || cand_host_clean.ends_with(&format!(".{target_host_clean}"))
                                {
                                    if let Some(q) = p.request_queue() {
                                        return q;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        self.default_queue.clone()
    }

    pub async fn test_provider_health(&self, id: &str) -> Result<ProviderHealth, String> {
        let provider = self
            .get_provider_by_id(id)
            .await
            .ok_or_else(|| format!("Provider '{id}' not found"))?;
        provider.test_connection().await
    }

    pub async fn fetch_creators(&self) -> Result<Vec<Creator>, String> {
        let enabled = self.get_all_enabled_providers().await;
        if enabled.is_empty() {
            return Err("No enabled providers configured".to_string());
        }

        let tasks: Vec<_> = enabled
            .iter()
            .map(|p| {
                let p = p.clone();
                async move { (p.id().to_string(), p.fetch_creators().await) }
            })
            .collect();

        let results = join_all(tasks).await;
        let mut creators_map: BTreeMap<String, Creator> = BTreeMap::new();
        let mut any_success = false;
        let mut last_error = String::new();

        for (prov_id, res) in results {
            match res {
                Ok(creators) => {
                    any_success = true;
                    for mut c in creators {
                        let key = format!("{}:{}", c.service.to_lowercase(), c.id.to_lowercase());
                        c.extra
                            .entry("provider_id".to_string())
                            .or_insert_with(|| serde_json::Value::String(prov_id.clone()));

                        if let Some(existing) = creators_map.get_mut(&key) {
                            let max_fav = match (existing.favorited, c.favorited) {
                                (Some(a), Some(b)) => Some(a.max(b)),
                                (Some(a), None) => Some(a),
                                (None, Some(b)) => Some(b),
                                (None, None) => None,
                            };
                            let mut provider_ids: Vec<String> = match existing
                                .extra
                                .get("provider_ids")
                            {
                                Some(serde_json::Value::Array(arr)) => arr
                                    .iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect(),
                                _ => {
                                    let mut pids = Vec::new();
                                    if let Some(pid) =
                                        existing.extra.get("provider_id").and_then(|v| v.as_str())
                                    {
                                        pids.push(pid.to_string());
                                    }
                                    pids
                                }
                            };
                            if !provider_ids.contains(&prov_id) {
                                provider_ids.push(prov_id.clone());
                            }

                            if c.updated.unwrap_or(0) > existing.updated.unwrap_or(0) {
                                *existing = c;
                            }
                            existing.favorited = max_fav;
                            existing.extra.insert(
                                "provider_ids".to_string(),
                                serde_json::Value::Array(
                                    provider_ids
                                        .into_iter()
                                        .map(serde_json::Value::String)
                                        .collect(),
                                ),
                            );
                        } else {
                            c.extra.insert(
                                "provider_ids".to_string(),
                                serde_json::Value::Array(vec![serde_json::Value::String(
                                    prov_id.clone(),
                                )]),
                            );
                            creators_map.insert(key, c);
                        }
                    }
                }
                Err(e) => {
                    last_error = e;
                }
            }
        }

        if !any_success {
            return Err(format!("Failed to fetch creators: {last_error}"));
        }

        Ok(creators_map.into_values().collect())
    }

    pub async fn fetch_creator_profile(
        &self,
        service: &str,
        creator_id: &str,
        provider_id: Option<&str>,
    ) -> Result<Creator, String> {
        if let Some(pid) = provider_id {
            if pid != "auto" {
                if let Some(p) = self.get_provider_by_id(pid).await {
                    return p.fetch_creator_profile(service, creator_id).await;
                } else {
                    return Err(format!("Provider '{pid}' not found"));
                }
            }
        }

        let candidates = self.get_providers_for_service(service).await;
        if candidates.is_empty() {
            return Err(format!("No provider configured for service '{service}'"));
        }

        let mut profiles: Vec<Creator> = Vec::new();
        let mut last_error = String::new();
        for provider in &candidates {
            match provider.fetch_creator_profile(service, creator_id).await {
                Ok(profile) => profiles.push(profile),
                Err(e) => last_error = e,
            }
        }

        if profiles.is_empty() {
            return Err(last_error);
        }

        let mut base = profiles.remove(0);
        let mut candidate_avatars: Vec<String> = Vec::new();
        if let Some(ref av) = base.avatar_url {
            if !av.trim().is_empty() {
                candidate_avatars.push(av.clone());
            }
        }
        for other in profiles {
            if let Some(ref av) = other.avatar_url {
                if !av.trim().is_empty() && !candidate_avatars.contains(av) {
                    candidate_avatars.push(av.clone());
                }
            }
            if base.avatar_url.as_deref().unwrap_or("").trim().is_empty() {
                if let Some(av) = other.avatar_url.filter(|s| !s.trim().is_empty()) {
                    base.avatar_url = Some(av);
                }
            }
            if base.banner_url.as_deref().unwrap_or("").trim().is_empty() {
                if let Some(bn) = other.banner_url.filter(|s| !s.trim().is_empty()) {
                    base.banner_url = Some(bn);
                }
            }
            if (base.name.trim().is_empty() || base.name == creator_id)
                && !other.name.trim().is_empty()
                && other.name != creator_id
            {
                base.name = other.name;
            }
            if let Some(other_idx) = other.indexed {
                base.indexed = Some(base.indexed.map_or(other_idx, |b| b.min(other_idx)));
            }
            if let Some(other_upd) = other.updated {
                base.updated = Some(base.updated.map_or(other_upd, |b| b.max(other_upd)));
            }
            for (k, v) in other.extra {
                base.extra.entry(k).or_insert(v);
            }
        }

        if !candidate_avatars.is_empty() {
            base.extra.insert(
                "candidate_avatar_urls".to_string(),
                serde_json::Value::Array(
                    candidate_avatars
                        .into_iter()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }

        Ok(base)
    }

    pub async fn fetch_creator_links(
        &self,
        service: &str,
        creator_id: &str,
        provider_id: Option<&str>,
    ) -> Result<Vec<CreatorProfile>, String> {
        if let Some(pid) = provider_id {
            if pid != "auto" {
                if let Some(p) = self.get_provider_by_id(pid).await {
                    return p.fetch_creator_links(service, creator_id).await;
                } else {
                    return Err(format!("Provider '{pid}' not found"));
                }
            }
        }

        let candidates = self.get_providers_for_service(service).await;
        let mut had_success = false;
        let mut errors = Vec::new();
        for provider in candidates {
            match provider.fetch_creator_links(service, creator_id).await {
                Ok(links) => {
                    had_success = true;
                    if !links.is_empty() {
                        return Ok(links);
                    }
                }
                Err(error) => errors.push(error),
            }
        }
        if had_success {
            Ok(Vec::new())
        } else {
            Err(provider_errors("creator links", errors))
        }
    }

    pub async fn fetch_similar_creators(
        &self,
        service: &str,
        creator_id: &str,
        provider_id: Option<&str>,
    ) -> Result<Vec<CreatorProfile>, String> {
        if let Some(pid) = provider_id {
            if pid != "auto" {
                if let Some(p) = self.get_provider_by_id(pid).await {
                    return p.fetch_similar_creators(service, creator_id).await;
                } else {
                    return Err(format!("Provider '{pid}' not found"));
                }
            }
        }

        let candidates = self.get_providers_for_service(service).await;
        let mut had_success = false;
        let mut errors = Vec::new();
        for provider in candidates {
            match provider.fetch_similar_creators(service, creator_id).await {
                Ok(similar) => {
                    had_success = true;
                    if !similar.is_empty() {
                        return Ok(similar);
                    }
                }
                Err(error) => errors.push(error),
            }
        }
        if had_success {
            Ok(Vec::new())
        } else {
            Err(provider_errors("similar creators", errors))
        }
    }

    pub async fn fetch_creator_tags(
        &self,
        service: &str,
        creator_id: &str,
        provider_id: Option<&str>,
    ) -> Result<Vec<String>, String> {
        if let Some(pid) = provider_id {
            if pid != "auto" {
                if let Some(p) = self.get_provider_by_id(pid).await {
                    return p.fetch_creator_tags(service, creator_id).await;
                } else {
                    return Err(format!("Provider '{pid}' not found"));
                }
            }
        }

        let candidates = self.get_providers_for_service(service).await;
        let mut had_success = false;
        let mut errors = Vec::new();
        for provider in candidates {
            match provider.fetch_creator_tags(service, creator_id).await {
                Ok(tags) => {
                    had_success = true;
                    if !tags.is_empty() {
                        return Ok(tags);
                    }
                }
                Err(error) => errors.push(error),
            }
        }
        if had_success {
            Ok(Vec::new())
        } else {
            Err(provider_errors("creator tags", errors))
        }
    }

    pub async fn fetch_announcements(
        &self,
        service: &str,
        creator_id: &str,
        provider_id: Option<&str>,
    ) -> Result<Vec<Announcement>, String> {
        if let Some(pid) = provider_id {
            if pid != "auto" {
                if let Some(p) = self.get_provider_by_id(pid).await {
                    return p.fetch_announcements(service, creator_id).await;
                } else {
                    return Err(format!("Provider '{pid}' not found"));
                }
            }
        }

        let candidates = self.get_providers_for_service(service).await;
        let mut had_success = false;
        let mut errors = Vec::new();
        for provider in candidates {
            match provider.fetch_announcements(service, creator_id).await {
                Ok(items) => {
                    had_success = true;
                    if !items.is_empty() {
                        return Ok(items);
                    }
                }
                Err(error) => errors.push(error),
            }
        }
        if had_success {
            Ok(Vec::new())
        } else {
            Err(provider_errors("announcements", errors))
        }
    }

    pub async fn fetch_posts(
        &self,
        service: &str,
        creator_id: &str,
        offset: u32,
        query: Option<&str>,
        provider_id: Option<&str>,
    ) -> Result<Vec<Post>, String> {
        if let Some(pid) = provider_id {
            if pid != "auto" {
                if let Some(p) = self.get_provider_by_id(pid).await {
                    return p.fetch_posts(service, creator_id, offset, query).await;
                } else {
                    return Err(format!("Provider '{pid}' not found"));
                }
            }
        }

        let candidates = self.get_providers_for_service(service).await;
        if candidates.is_empty() {
            return Err(format!("No provider configured for service '{service}'"));
        }

        let mut had_empty_success = false;
        let mut last_error = String::new();
        for provider in candidates {
            match provider
                .fetch_posts(service, creator_id, offset, query)
                .await
            {
                Ok(posts) => {
                    if !posts.is_empty() {
                        return Ok(posts);
                    }
                    had_empty_success = true;
                }
                Err(e) => last_error = e,
            }
        }

        if had_empty_success {
            Ok(Vec::new())
        } else {
            Err(last_error)
        }
    }

    pub async fn fetch_post(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
        provider_id: Option<&str>,
    ) -> Result<Option<ReconciledPost>, String> {
        if let Some(pid) = provider_id {
            if pid != "auto" {
                if let Some(p) = self.get_provider_by_id(pid).await {
                    let post_opt = p
                        .fetch_post(service, creator_id, post_id)
                        .await
                        .map_err(|e| format!("Provider '{pid}' error: {e}"))?;
                    if let Some(mut post) = post_opt {
                        post.extra
                            .entry("provider_id".to_string())
                            .or_insert_with(|| serde_json::Value::String(pid.to_string()));
                        return Ok(Some(ReconciledPost {
                            post: post.clone(),
                            revisions: vec![],
                            available_providers: vec![pid.to_string()],
                            attachment_sources: std::collections::HashMap::new(),
                        }));
                    } else {
                        return Ok(None);
                    }
                } else {
                    return Err(format!("Provider '{pid}' not found"));
                }
            }
        }

        let candidates = self.get_providers_for_service(service).await;
        if candidates.is_empty() {
            return Err(format!("No provider configured for service '{service}'"));
        }

        let tasks: Vec<_> = candidates
            .iter()
            .map(|p| {
                let p = p.clone();
                let conf = p.config();
                async move {
                    let post_opt = p
                        .fetch_post(service, creator_id, post_id)
                        .await
                        .ok()
                        .flatten();
                    post_opt.map(|mut post| {
                        post.extra
                            .entry("provider_id".to_string())
                            .or_insert_with(|| serde_json::Value::String(conf.id.clone()));
                        (conf.id.clone(), post)
                    })
                }
            })
            .collect();

        let results = join_all(tasks).await;
        let snapshots: Vec<(String, Post)> = results.into_iter().flatten().collect();

        if snapshots.is_empty() {
            return Ok(None);
        }

        let mut reconciled = reconcile_post_snapshots(snapshots);
        if let Some(ref mut r) = reconciled {
            r.post.extra.insert(
                "available_providers".to_string(),
                serde_json::Value::Array(
                    r.available_providers
                        .iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }
        Ok(reconciled)
    }

    pub async fn fetch_post_revisions(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Vec<PostRevision>, String> {
        let candidates = self.get_providers_for_service(service).await;
        let mut all_revs = Vec::new();
        let mut seen_keys = std::collections::HashSet::new();
        for provider in candidates {
            let pid = provider.config().id.clone();
            if let Ok(revs) = provider
                .fetch_post_revisions(service, creator_id, post_id)
                .await
            {
                for mut r in revs {
                    r.post
                        .extra
                        .entry("provider_id".to_string())
                        .or_insert_with(|| serde_json::Value::String(pid.clone()));
                    let key = (pid.clone(), r.revision_id);
                    if seen_keys.insert(key) {
                        all_revs.push(r);
                    }
                }
            }
        }
        Ok(all_revs)
    }

    pub async fn fetch_recent_posts(
        &self,
        query: Option<&str>,
        offset: u32,
    ) -> Result<Vec<Post>, String> {
        let enabled = self.get_all_enabled_providers().await;
        if enabled.is_empty() {
            return Err("No enabled providers configured".to_string());
        }

        let tasks: Vec<_> = enabled
            .iter()
            .map(|p| {
                let p = p.clone();
                async move { p.fetch_recent_posts(query, offset).await }
            })
            .collect();

        let results = join_all(tasks).await;
        let mut all_posts: Vec<Post> = Vec::new();
        let mut any_success = false;
        let mut last_error = String::new();

        for res in results {
            match res {
                Ok(posts) => {
                    any_success = true;
                    all_posts.extend(posts);
                }
                Err(e) => {
                    last_error = e;
                }
            }
        }

        if !any_success {
            return Err(format!("Failed to fetch recent posts: {last_error}"));
        }

        let mut seen = HashSet::new();
        all_posts.retain(|p| {
            let key = format!("{}:{}", p.service.to_lowercase(), p.id);
            seen.insert(key)
        });

        all_posts.sort_by(|a, b| {
            let ts_a = a
                .published
                .as_deref()
                .or(a.added.as_deref())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            let ts_b = b
                .published
                .as_deref()
                .or(b.added.as_deref())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            ts_b.cmp(&ts_a)
        });

        Ok(all_posts)
    }

    pub async fn fetch_popular_posts(
        &self,
        period: &str,
        date: Option<&str>,
        offset: u32,
    ) -> Result<Vec<Post>, String> {
        let enabled = self.get_all_enabled_providers().await;
        if enabled.is_empty() {
            return Err("No enabled providers configured".to_string());
        }

        let tasks: Vec<_> = enabled
            .iter()
            .map(|p| {
                let p = p.clone();
                async move { p.fetch_popular_posts(period, date, offset).await }
            })
            .collect();

        let results = join_all(tasks).await;
        let mut all_posts: Vec<Post> = Vec::new();
        let mut any_success = false;
        let mut last_error = String::new();

        for res in results {
            match res {
                Ok(posts) => {
                    any_success = true;
                    all_posts.extend(posts);
                }
                Err(e) => {
                    last_error = e;
                }
            }
        }

        if !any_success {
            return Err(format!("Failed to fetch popular posts: {last_error}"));
        }

        let mut seen = HashSet::new();
        all_posts.retain(|p| {
            let key = format!("{}:{}", p.service.to_lowercase(), p.id);
            seen.insert(key)
        });

        all_posts.sort_by(|a, b| {
            let fav_a = a.favorite_count.unwrap_or(0);
            let fav_b = b.favorite_count.unwrap_or(0);
            fav_b.cmp(&fav_a)
        });

        Ok(all_posts)
    }

    pub async fn fetch_post_comments(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Vec<Comment>, String> {
        let candidates = self.get_providers_for_service(service).await;
        for provider in candidates {
            if let Ok(comments) = provider
                .fetch_post_comments(service, creator_id, post_id)
                .await
            {
                if !comments.is_empty() {
                    return Ok(comments);
                }
            }
        }
        Ok(Vec::new())
    }

    pub async fn fetch_account_favorites(
        &self,
        provider_id: Option<&str>,
        favorite_type: Option<&str>,
    ) -> Result<Vec<Favorite>, String> {
        if let Some(id) = provider_id {
            if let Some(p) = self.get_provider_by_id(id).await {
                return p.fetch_account_favorites(favorite_type).await;
            }
        }
        let enabled = self.get_all_enabled_providers().await;
        let mut all_favorites = Vec::new();
        let mut seen_keys = HashSet::new();
        let mut any_success = false;

        for provider in enabled {
            if let Ok(favs) = provider.fetch_account_favorites(favorite_type).await {
                any_success = true;
                for fav in favs {
                    let srv = fav.service.as_deref().unwrap_or("").to_lowercase();
                    let id = fav.id.to_lowercase();
                    if seen_keys.insert((srv, id)) {
                        all_favorites.push(fav);
                    }
                }
            }
        }

        if any_success {
            Ok(all_favorites)
        } else {
            Err("No provider available for favorites".to_string())
        }
    }

    pub async fn set_creator_favorite(
        &self,
        service: &str,
        creator_id: &str,
        favorite: bool,
    ) -> Result<ApiActionResult, String> {
        let candidates = self.get_providers_for_service(service).await;
        let mut any_success = false;
        for provider in candidates {
            if let Ok(res) = provider
                .set_creator_favorite(service, creator_id, favorite)
                .await
            {
                if res.success {
                    any_success = true;
                }
            }
        }
        if any_success {
            Ok(ApiActionResult {
                status: 200,
                success: true,
            })
        } else {
            Err("Failed to favorite creator".to_string())
        }
    }

    pub async fn set_post_favorite(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
        favorite: bool,
    ) -> Result<ApiActionResult, String> {
        let candidates = self.get_providers_for_service(service).await;
        let mut any_success = false;
        for provider in candidates {
            if let Ok(res) = provider
                .set_post_favorite(service, creator_id, post_id, favorite)
                .await
            {
                if res.success {
                    any_success = true;
                }
            }
        }
        if any_success {
            Ok(ApiActionResult {
                status: 200,
                success: true,
            })
        } else {
            Err("Failed to favorite post".to_string())
        }
    }

    pub async fn get_provider_auth_schema(
        &self,
        provider_id: &str,
    ) -> Result<ProviderAuthSchema, String> {
        if let Some(p) = self.get_provider_by_id(provider_id).await {
            Ok(p.auth_schema())
        } else {
            Err(format!("Provider '{provider_id}' not found"))
        }
    }

    pub async fn get_provider_capabilities(&self) -> HashMap<String, ProviderCapabilities> {
        let providers = self.providers.read().await;
        let mut map = HashMap::new();
        for p in providers.iter() {
            map.insert(p.id().to_string(), p.capabilities());
        }
        map
    }

    pub async fn get_active_capabilities(&self) -> ProviderCapabilities {
        let enabled = self.get_all_enabled_providers().await;
        if enabled.is_empty() {
            return ProviderCapabilities {
                provider_id: "none".to_string(),
                popular: PopularCapabilities {
                    supported: false,
                    periods: Vec::new(),
                    default_period: None,
                    supports_date: false,
                },
                creator_sorts: Vec::new(),
                post_sorts: Vec::new(),
                supports_query_search: false,
                supports_date_filter: false,
                supports_hash_search: false,
                supports_announcements: false,
                supports_fancards: false,
                supports_similar_creators: false,
                supports_creator_tags: false,
            };
        }

        if enabled.len() == 1 {
            return enabled[0].capabilities();
        }

        let caps: Vec<ProviderCapabilities> = enabled.iter().map(|p| p.capabilities()).collect();

        let popular_supported = caps.iter().any(|c| c.popular.supported);
        let mut common_periods: Vec<PopularPeriodOption> = Vec::new();
        let mut default_period = None;
        let mut supports_date = false;

        if popular_supported {
            let pop_caps: Vec<&PopularCapabilities> = caps
                .iter()
                .filter(|c| c.popular.supported)
                .map(|c| &c.popular)
                .collect();

            if let Some(first) = pop_caps.first() {
                for period in &first.periods {
                    if pop_caps
                        .iter()
                        .all(|c| c.periods.iter().any(|p| p.id == period.id))
                    {
                        common_periods.push(period.clone());
                    }
                }
                default_period = common_periods.first().map(|p| p.id.clone());
            }

            supports_date = pop_caps.iter().all(|c| c.supports_date);
        }

        let mut common_creator_sorts: Vec<SortOption> = Vec::new();
        if let Some(first) = caps.first() {
            for sort in &first.creator_sorts {
                if caps
                    .iter()
                    .all(|c| c.creator_sorts.iter().any(|s| s.id == sort.id))
                {
                    let is_server_side = caps.iter().all(|c| {
                        c.creator_sorts
                            .iter()
                            .find(|s| s.id == sort.id)
                            .is_some_and(|s| s.is_server_side)
                    });
                    common_creator_sorts.push(SortOption {
                        id: sort.id.clone(),
                        label_key: sort.label_key.clone(),
                        is_server_side,
                    });
                }
            }
        }

        let mut common_post_sorts: Vec<SortOption> = Vec::new();
        if let Some(first) = caps.first() {
            for sort in &first.post_sorts {
                if caps
                    .iter()
                    .all(|c| c.post_sorts.iter().any(|s| s.id == sort.id))
                {
                    let is_server_side = caps.iter().all(|c| {
                        c.post_sorts
                            .iter()
                            .find(|s| s.id == sort.id)
                            .is_some_and(|s| s.is_server_side)
                    });
                    common_post_sorts.push(SortOption {
                        id: sort.id.clone(),
                        label_key: sort.label_key.clone(),
                        is_server_side,
                    });
                }
            }
        }

        ProviderCapabilities {
            provider_id: "aggregated".to_string(),
            popular: PopularCapabilities {
                supported: popular_supported,
                periods: common_periods,
                default_period,
                supports_date,
            },
            creator_sorts: common_creator_sorts,
            post_sorts: common_post_sorts,
            supports_query_search: caps.iter().all(|c| c.supports_query_search),
            supports_date_filter: caps.iter().all(|c| c.supports_date_filter),
            supports_hash_search: caps.iter().any(|c| c.supports_hash_search),
            supports_announcements: caps.iter().any(|c| c.supports_announcements),
            supports_fancards: caps.iter().any(|c| c.supports_fancards),
            supports_similar_creators: caps.iter().any(|c| c.supports_similar_creators),
            supports_creator_tags: caps.iter().any(|c| c.supports_creator_tags),
        }
    }

    pub async fn save_provider_session(
        &self,
        provider_id: &str,
        cookie: &str,
        username: Option<&str>,
    ) -> Result<(), String> {
        let mut list = self.providers.write().await;
        if let Some(p) = list.iter_mut().find(|p| p.config().id == provider_id) {
            let mut conf = p.config();
            conf.session_cookie = cookie.trim().to_string();
            if let Some(u) = username {
                conf.username = u.trim().to_string();
            }
            p.update_config(conf).await?;
            Ok(())
        } else {
            Err(format!("Provider '{provider_id}' not found"))
        }
    }

    pub async fn login_provider_with_credentials(
        &self,
        provider_id: &str,
        credentials: &HashMap<String, String>,
    ) -> Result<String, String> {
        let mut list = self.providers.write().await;
        if let Some(p) = list.iter_mut().find(|p| p.config().id == provider_id) {
            let (cookie, username): (String, String) = if let (Some(u), Some(pass)) =
                (credentials.get("username"), credentials.get("password"))
            {
                let session_res = p.login(u, pass).await?;
                let derived_user = if let Ok(sess) = p.get_account_session().await {
                    sess.username.unwrap_or_else(|| u.to_string())
                } else {
                    u.to_string()
                };
                (session_res, derived_user)
            } else if let Some(cookie) = credentials.get("session_cookie") {
                let mut conf = p.config();
                conf.session_cookie = cookie.trim().to_string();
                if let Some(u) = credentials.get("username") {
                    conf.username = u.trim().to_string();
                }
                p.update_config(conf).await?;
                let derived_user = if let Ok(sess) = p.get_account_session().await {
                    sess.username.unwrap_or_default()
                } else {
                    credentials.get("username").cloned().unwrap_or_default()
                };
                (cookie.trim().to_string(), derived_user)
            } else {
                return Err("Missing login credentials".to_string());
            };

            let mut conf = p.config();
            conf.session_cookie = cookie;
            if !username.is_empty() {
                conf.username = username;
            }
            p.update_config(conf).await?;
            Ok(p.config().username)
        } else {
            Err(format!("Provider '{provider_id}' not found"))
        }
    }

    pub async fn logout_provider_session(
        &self,
        provider_id: &str,
        remove_session_favorites: bool,
        content_repo: &crate::db::content::ContentRepository,
    ) -> Result<(), String> {
        let username_to_clear = {
            let mut list = self.providers.write().await;
            if let Some(p) = list.iter_mut().find(|p| p.config().id == provider_id) {
                let mut conf = p.config();
                let user = conf.username.clone();
                conf.session_cookie.clear();
                conf.username.clear();
                p.update_config(conf).await?;
                user
            } else {
                return Err(format!("Provider '{provider_id}' not found"));
            }
        };
        if remove_session_favorites && !username_to_clear.trim().is_empty() {
            let _ = content_repo.remove_account_favorites(&username_to_clear);
        }
        Ok(())
    }

    pub async fn sync_provider_favorites(
        &self,
        provider_id: &str,
        direction: &str,
        content_repo: &crate::db::content::ContentRepository,
    ) -> Result<FavoritesSyncResult, String> {
        let provider = self
            .get_provider_by_id(provider_id)
            .await
            .ok_or_else(|| format!("Provider '{provider_id}' not found"))?;

        let mut pulled_count = 0;
        let mut pushed_count = 0;
        let mut errors = Vec::new();

        let conf = provider.config();
        let account_id = if !conf.username.is_empty() {
            conf.username.clone()
        } else {
            conf.id.clone()
        };

        if direction == "pull" || direction == "both" {
            match provider.fetch_account_favorites(None).await {
                Ok(remote_favs) => {
                    for fav in remote_favs {
                        let srv = fav.service.as_deref().unwrap_or("").to_lowercase();
                        if srv.is_empty() || fav.id.is_empty() {
                            continue;
                        }
                        let kind = fav
                            .extra
                            .get("kind")
                            .and_then(|k| k.as_str())
                            .unwrap_or("creator");
                        let is_post = kind == "post";
                        let creator_id = if is_post {
                            fav.extra
                                .get("user")
                                .and_then(|u| u.as_str())
                                .unwrap_or("")
                                .to_string()
                        } else {
                            fav.id.clone()
                        };
                        let post_id = if is_post {
                            fav.id.clone()
                        } else {
                            String::new()
                        };
                        let entity_kind = if is_post { "post" } else { "creator" };

                        let opt_post = if is_post {
                            Some(post_id.as_str())
                        } else {
                            None
                        };
                        if let Err(e) = content_repo.set_pin(
                            entity_kind,
                            &srv,
                            &creator_id,
                            opt_post,
                            "favorite",
                            &account_id,
                            true,
                        ) {
                            errors.push(format!(
                                "Failed to save local favorite {srv}:{creator_id}:{post_id}: {e}"
                            ));
                        } else {
                            pulled_count += 1;
                        }
                    }
                }
                Err(e) => {
                    errors.push(format!("Pull favorites failed: {e}"));
                }
            }
        }

        if direction == "push" || direction == "both" {
            let local_creators = content_repo
                .list_favorites("artist", &account_id)
                .unwrap_or_default();
            for creator in local_creators {
                if let Some(srv) = creator.service.as_deref() {
                    if provider.supports_service(srv) {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        match provider.set_creator_favorite(srv, &creator.id, true).await {
                            Ok(res) if res.success => {
                                pushed_count += 1;
                            }
                            Ok(_) => {
                                errors.push(format!(
                                    "Push creator favorite rejected by server for {srv}:{}",
                                    creator.id
                                ));
                            }
                            Err(e) => {
                                errors
                                    .push(format!("Push creator {srv}:{} failed: {e}", creator.id));
                            }
                        }
                    }
                }
            }

            let local_posts = content_repo
                .list_favorites("post", &account_id)
                .unwrap_or_default();
            for post in local_posts {
                if let Some(srv) = post.service.as_deref() {
                    if provider.supports_service(srv) {
                        let user_id = post
                            .extra
                            .get("user")
                            .and_then(|u| u.as_str())
                            .unwrap_or("");
                        if !user_id.is_empty() {
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            match provider
                                .set_post_favorite(srv, user_id, &post.id, true)
                                .await
                            {
                                Ok(res) if res.success => {
                                    pushed_count += 1;
                                }
                                Ok(_) => {
                                    errors.push(format!("Push post favorite rejected by server for {srv}:{user_id}:{}", post.id));
                                }
                                Err(e) => {
                                    errors.push(format!(
                                        "Push post {srv}:{user_id}:{} failed: {e}",
                                        post.id
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(FavoritesSyncResult {
            provider_id: provider_id.to_string(),
            pulled_count,
            pushed_count,
            errors,
        })
    }

    pub async fn resolve_media_url(
        &self,
        service: &str,
        file_path: &str,
        server: Option<&str>,
        provider_id: Option<&str>,
    ) -> String {
        if file_path.starts_with("http://") || file_path.starts_with("https://") {
            return file_path.to_string();
        }
        self.get_provider_for_resolution(service, provider_id)
            .await
            .map(|provider| provider.resolve_media_url(file_path, server))
            .unwrap_or_default()
    }

    pub async fn resolve_thumbnail_url(
        &self,
        service: &str,
        thumb_path: &str,
        provider_id: Option<&str>,
    ) -> String {
        if thumb_path.starts_with("http://") || thumb_path.starts_with("https://") {
            return thumb_path.to_string();
        }
        self.get_provider_for_resolution(service, provider_id)
            .await
            .map(|provider| provider.resolve_thumbnail_url(thumb_path))
            .unwrap_or_default()
    }

    pub async fn resolve_post_url(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
        provider_id: Option<&str>,
    ) -> String {
        if let Some(id) = provider_id {
            if let Some(p) = self.get_provider_by_id(id).await {
                return p.resolve_post_url(service, creator_id, post_id);
            }
        }
        let candidates = self.get_providers_for_service(service).await;
        if let Some(first) = candidates.first() {
            first.resolve_post_url(service, creator_id, post_id)
        } else {
            let enabled = self.get_all_enabled_providers().await;
            if let Some(first) = enabled.first() {
                first.resolve_post_url(service, creator_id, post_id)
            } else if let Some(first) = self.providers.read().await.first() {
                first.resolve_post_url(service, creator_id, post_id)
            } else {
                let confs = Self::default_configs();
                let config = confs
                    .into_iter()
                    .find(|c| c.services.iter().any(|s| s.eq_ignore_ascii_case(service)))
                    .or_else(|| Self::default_configs().into_iter().next());
                if let Some(cfg) = config {
                    if let Ok(prov) = create_provider(cfg) {
                        return prov.resolve_post_url(service, creator_id, post_id);
                    }
                }
                String::new()
            }
        }
    }

    pub async fn resolve_creator_url(
        &self,
        service: &str,
        creator_id: &str,
        provider_id: Option<&str>,
    ) -> String {
        if let Some(id) = provider_id {
            if let Some(p) = self.get_provider_by_id(id).await {
                return p.resolve_creator_url(service, creator_id);
            }
        }
        let candidates = self.get_providers_for_service(service).await;
        if let Some(first) = candidates.first() {
            first.resolve_creator_url(service, creator_id)
        } else {
            let enabled = self.get_all_enabled_providers().await;
            if let Some(first) = enabled.first() {
                first.resolve_creator_url(service, creator_id)
            } else if let Some(first) = self.providers.read().await.first() {
                first.resolve_creator_url(service, creator_id)
            } else {
                let confs = Self::default_configs();
                let config = confs
                    .into_iter()
                    .find(|c| c.services.iter().any(|s| s.eq_ignore_ascii_case(service)))
                    .or_else(|| Self::default_configs().into_iter().next());
                if let Some(cfg) = config {
                    if let Ok(prov) = create_provider(cfg) {
                        return prov.resolve_creator_url(service, creator_id);
                    }
                }
                String::new()
            }
        }
    }

    pub async fn resolve_avatar_url(
        &self,
        service: &str,
        creator_id: &str,
        provider_id: Option<&str>,
    ) -> String {
        if let Some(id) = provider_id {
            if let Some(p) = self.get_provider_by_id(id).await {
                return p.resolve_avatar_url(service, creator_id);
            }
        }
        let candidates = self.get_providers_for_service(service).await;
        if let Some(first) = candidates.first() {
            first.resolve_avatar_url(service, creator_id)
        } else {
            let enabled = self.get_all_enabled_providers().await;
            if let Some(first) = enabled.first() {
                first.resolve_avatar_url(service, creator_id)
            } else if let Some(first) = self.providers.read().await.first() {
                first.resolve_avatar_url(service, creator_id)
            } else {
                String::new()
            }
        }
    }

    pub async fn resolve_banner_url(
        &self,
        service: &str,
        creator_id: &str,
        provider_id: Option<&str>,
    ) -> String {
        if let Some(id) = provider_id {
            if let Some(p) = self.get_provider_by_id(id).await {
                return p.resolve_banner_url(service, creator_id);
            }
        }
        let candidates = self.get_providers_for_service(service).await;
        if let Some(first) = candidates.first() {
            first.resolve_banner_url(service, creator_id)
        } else {
            let enabled = self.get_all_enabled_providers().await;
            if let Some(first) = enabled.first() {
                first.resolve_banner_url(service, creator_id)
            } else if let Some(first) = self.providers.read().await.first() {
                first.resolve_banner_url(service, creator_id)
            } else {
                String::new()
            }
        }
    }

    pub async fn enrich_creator(&self, creator: &mut Creator) {
        let prov_id = creator
            .extra
            .get("provider_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if creator.avatar_url.as_deref().unwrap_or("").is_empty() {
            let url = self
                .resolve_avatar_url(&creator.service, &creator.id, prov_id.as_deref())
                .await;
            if !url.is_empty() {
                creator.avatar_url = Some(url);
            }
        }

        if creator.banner_url.as_deref().unwrap_or("").is_empty() {
            let url = self
                .resolve_banner_url(&creator.service, &creator.id, prov_id.as_deref())
                .await;
            if !url.is_empty() {
                creator.banner_url = Some(url);
            }
        }

        if creator.page_url.as_deref().unwrap_or("").is_empty() {
            let url = self
                .resolve_creator_url(&creator.service, &creator.id, prov_id.as_deref())
                .await;
            if !url.is_empty() {
                creator.page_url = Some(url);
            }
        }

        if creator.avatar_path.is_none() {
            let dir = content_cache_path().join("avatars");
            let s = sanitize_cache_key(&creator.service);
            let id = sanitize_cache_key(&creator.id);
            for ext in &["webp", "png", "jpg", "jpeg", "gif"] {
                let p = dir.join(format!("{s}_{id}_avatar.{ext}"));
                if p.is_file() {
                    creator.avatar_path = Some(p.to_string_lossy().into_owned());
                    break;
                }
            }
        }

        if creator.banner_path.is_none() {
            let dir = content_cache_path().join("banners");
            let s = sanitize_cache_key(&creator.service);
            let id = sanitize_cache_key(&creator.id);
            for ext in &["webp", "png", "jpg", "jpeg", "gif"] {
                let p = dir.join(format!("{s}_{id}_banner.{ext}"));
                if p.is_file() {
                    creator.banner_path = Some(p.to_string_lossy().into_owned());
                    break;
                }
            }
        }
    }

    pub async fn enrich_creator_profile(&self, profile: &mut CreatorProfile) {
        let prov_id = profile
            .extra
            .get("provider_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if profile.avatar_url.as_deref().unwrap_or("").is_empty() {
            let url = self
                .resolve_avatar_url(&profile.service, &profile.id, prov_id.as_deref())
                .await;
            if !url.is_empty() {
                profile.avatar_url = Some(url);
            }
        }

        if profile.banner_url.as_deref().unwrap_or("").is_empty() {
            let url = self
                .resolve_banner_url(&profile.service, &profile.id, prov_id.as_deref())
                .await;
            if !url.is_empty() {
                profile.banner_url = Some(url);
            }
        }

        if profile.page_url.as_deref().unwrap_or("").is_empty() {
            let url = self
                .resolve_creator_url(&profile.service, &profile.id, prov_id.as_deref())
                .await;
            if !url.is_empty() {
                profile.page_url = Some(url);
            }
        }

        if profile.avatar_path.is_none() {
            let dir = content_cache_path().join("avatars");
            let s = sanitize_cache_key(&profile.service);
            let id = sanitize_cache_key(&profile.id);
            if let Some(ref pid) = prov_id {
                if pid != "auto" {
                    let p_san = sanitize_cache_key(pid);
                    for ext in &["webp", "png", "jpg", "jpeg", "gif"] {
                        let p = dir.join(format!("{s}_{id}_{p_san}_avatar.{ext}"));
                        if p.is_file() {
                            profile.avatar_path = Some(p.to_string_lossy().into_owned());
                            break;
                        }
                    }
                }
            }
            if profile.avatar_path.is_none()
                && (prov_id.is_none() || prov_id.as_deref() == Some("auto"))
            {
                for ext in &["webp", "png", "jpg", "jpeg", "gif"] {
                    let p = dir.join(format!("{s}_{id}_avatar.{ext}"));
                    if p.is_file() {
                        profile.avatar_path = Some(p.to_string_lossy().into_owned());
                        break;
                    }
                }
            }
        }

        if profile.banner_path.is_none() {
            let dir = content_cache_path().join("banners");
            let s = sanitize_cache_key(&profile.service);
            let id = sanitize_cache_key(&profile.id);
            if let Some(ref pid) = prov_id {
                if pid != "auto" {
                    let p_san = sanitize_cache_key(pid);
                    for ext in &["webp", "png", "jpg", "jpeg", "gif"] {
                        let p = dir.join(format!("{s}_{id}_{p_san}_banner.{ext}"));
                        if p.is_file() {
                            profile.banner_path = Some(p.to_string_lossy().into_owned());
                            break;
                        }
                    }
                }
            }
            if profile.banner_path.is_none()
                && (prov_id.is_none() || prov_id.as_deref() == Some("auto"))
            {
                for ext in &["webp", "png", "jpg", "jpeg", "gif"] {
                    let p = dir.join(format!("{s}_{id}_banner.{ext}"));
                    if p.is_file() {
                        profile.banner_path = Some(p.to_string_lossy().into_owned());
                        break;
                    }
                }
            }
        }
    }

    pub async fn enrich_post(&self, post: &mut Post) {
        let mut prov_id = post
            .extra
            .get("provider_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if prov_id.is_none() {
            prov_id = self
                .get_providers_for_service(&post.service)
                .await
                .first()
                .map(|provider| provider.id().to_string());
            if let Some(provider_id) = prov_id.as_ref() {
                post.extra.insert(
                    "provider_id".to_string(),
                    serde_json::Value::String(provider_id.clone()),
                );
            }
        }

        if post.page_url.as_deref().unwrap_or("").is_empty() {
            let url = self
                .resolve_post_url(&post.service, &post.user, &post.id, prov_id.as_deref())
                .await;
            if !url.is_empty() {
                post.page_url = Some(url);
            }
        }

        if let Some(ref mut file) = post.file {
            if file.path.is_some() {
                let file_provider_id = file
                    .extra
                    .get("provider_id")
                    .and_then(|value| value.as_str())
                    .or(prov_id.as_deref())
                    .map(str::to_string);
                if let Some(provider) = self
                    .get_provider_for_resolution(&post.service, file_provider_id.as_deref())
                    .await
                {
                    provider.enrich_attachment(file);
                }
            }
        }

        if let Some(ref mut attachments) = post.attachments {
            for att in attachments.iter_mut() {
                if att.path.is_some() {
                    let attachment_provider_id = att
                        .extra
                        .get("provider_id")
                        .and_then(|value| value.as_str())
                        .or(prov_id.as_deref())
                        .map(str::to_string);
                    if let Some(provider) = self
                        .get_provider_for_resolution(
                            &post.service,
                            attachment_provider_id.as_deref(),
                        )
                        .await
                    {
                        provider.enrich_attachment(att);
                    }
                }
            }
        }

        let mut cloud_source = post.content.as_deref().unwrap_or_default().to_string();
        if let Some(substring) = post.substring.as_deref() {
            cloud_source.push(' ');
            cloud_source.push_str(substring);
        }
        if let Some(embed) = post.embed.as_ref() {
            cloud_source.push(' ');
            cloud_source.push_str(&embed.to_string());
        }
        if !cloud_source.trim().is_empty() {
            post.cloud_urls = crate::cloud::extract_supported_urls(&cloud_source);
        }

        let derived_media_url = post
            .file
            .as_ref()
            .and_then(|attachment| attachment.url.clone())
            .or_else(|| {
                post.attachments.as_ref().and_then(|attachments| {
                    attachments
                        .iter()
                        .find_map(|attachment| attachment.url.clone())
                })
            });
        if derived_media_url.is_some() {
            post.media_url = derived_media_url;
        }

        let derived_thumbnail_url = post
            .file
            .as_ref()
            .and_then(|attachment| attachment.thumbnail_url.clone())
            .or_else(|| {
                post.attachments.as_ref().and_then(|attachments| {
                    attachments
                        .iter()
                        .find_map(|attachment| attachment.thumbnail_url.clone())
                })
            });
        if derived_thumbnail_url.is_some() {
            post.thumbnail_url = derived_thumbnail_url;
        }

        if post.preview_path.is_none() {
            let dir = content_cache_path().join("previews");
            let s = sanitize_cache_key(&post.service);
            let u = sanitize_cache_key(&post.user);
            let id = sanitize_cache_key(&post.id);
            for ext in &["jpg", "jpeg", "webp", "png", "gif", "mp4", "webm"] {
                let p = dir.join(format!("{s}_{u}_{id}.{ext}"));
                if p.is_file() {
                    post.preview_path = Some(p.to_string_lossy().into_owned());
                    break;
                }
            }
        }
    }

    pub async fn fetch_creator_artwork_data_url(
        &self,
        service: &str,
        creator_id: &str,
        artwork_type: &str,
    ) -> Result<String, String> {
        let candidates = self.get_providers_for_service(service).await;
        for provider in candidates {
            if let Ok(url) = provider
                .fetch_creator_artwork_data_url(service, creator_id, artwork_type)
                .await
            {
                return Ok(url);
            }
        }
        Err("Failed to fetch creator artwork data URL".to_string())
    }

    pub async fn search_hash(&self, file_hash: &str) -> Result<FileSearchResult, String> {
        let enabled = self.get_all_enabled_providers().await;
        for provider in enabled {
            if let Ok(res) = provider.search_hash(file_hash).await {
                return Ok(res);
            }
        }
        Err("Hash search failed".to_string())
    }

    pub async fn enrich_fancards(
        &self,
        service: &str,
        cards: &mut [Fancard],
        provider_id: Option<&str>,
    ) {
        let provider = if let Some(pid) = provider_id {
            if pid != "auto" {
                self.get_provider_by_id(pid).await
            } else {
                self.get_providers_for_service(service)
                    .await
                    .into_iter()
                    .next()
            }
        } else {
            self.get_providers_for_service(service)
                .await
                .into_iter()
                .next()
        };
        if let Some(p) = provider {
            for c in cards.iter_mut() {
                if c.media_url.is_none() {
                    let m = p.resolve_fancard_media_url(service, &c.hash, &c.ext);
                    if !m.is_empty() {
                        c.media_url = Some(m);
                    }
                }
                if c.thumbnail_url.is_none() {
                    let t = p.resolve_fancard_thumbnail_url(service, &c.hash, &c.ext);
                    if !t.is_empty() {
                        c.thumbnail_url = Some(t);
                    }
                }
            }
        }
    }

    pub async fn fetch_fancards(
        &self,
        service: &str,
        creator_id: &str,
        provider_id: Option<&str>,
    ) -> Result<Vec<Fancard>, String> {
        let enrich_cards =
            |cards: &mut [Fancard], provider: &std::sync::Arc<dyn SourceProvider>| {
                for c in cards.iter_mut() {
                    if c.media_url.is_none() {
                        let m = provider.resolve_fancard_media_url(service, &c.hash, &c.ext);
                        if !m.is_empty() {
                            c.media_url = Some(m);
                        }
                    }
                    if c.thumbnail_url.is_none() {
                        let t = provider.resolve_fancard_thumbnail_url(service, &c.hash, &c.ext);
                        if !t.is_empty() {
                            c.thumbnail_url = Some(t);
                        }
                    }
                }
            };

        if let Some(pid) = provider_id {
            if pid != "auto" {
                if let Some(p) = self.get_provider_by_id(pid).await {
                    let mut cards = p.fetch_fancards(service, creator_id).await?;
                    enrich_cards(&mut cards, &p);
                    return Ok(cards);
                } else {
                    return Err(format!("Provider '{pid}' not found"));
                }
            }
        }

        let candidates = self.get_providers_for_service(service).await;
        for provider in candidates {
            if let Ok(mut cards) = provider.fetch_fancards(service, creator_id).await {
                if !cards.is_empty() {
                    enrich_cards(&mut cards, &provider);
                    return Ok(cards);
                }
            }
        }
        Ok(Vec::new())
    }

    pub async fn flag_post(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<ApiActionResult, String> {
        let candidates = self.get_providers_for_service(service).await;
        for provider in candidates {
            if let Ok(res) = provider.flag_post(service, creator_id, post_id).await {
                return Ok(res);
            }
        }
        Err("Failed to flag post".to_string())
    }

    pub async fn is_post_flagged(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<bool, String> {
        let candidates = self.get_providers_for_service(service).await;
        for p in candidates {
            if let Ok(flagged) = p.is_post_flagged(service, creator_id, post_id).await {
                return Ok(flagged);
            }
        }
        Ok(false)
    }

    pub async fn login(
        &self,
        provider_id: &str,
        username: &str,
        password: &str,
    ) -> Result<String, String> {
        let p = self
            .get_provider_by_id(provider_id)
            .await
            .ok_or_else(|| format!("Provider '{provider_id}' not found"))?;
        p.login(username, password).await
    }

    pub async fn logout(&self, provider_id: &str) -> Result<(), String> {
        if let Some(p) = self.get_provider_by_id(provider_id).await {
            p.logout().await
        } else {
            Ok(())
        }
    }

    pub async fn get_account_session(&self, provider_id: &str) -> Result<AccountSession, String> {
        let p = self
            .get_provider_by_id(provider_id)
            .await
            .ok_or_else(|| format!("Provider '{provider_id}' not found"))?;
        p.get_account_session().await
    }

    pub async fn app_version(&self) -> Result<String, String> {
        let enabled = self.get_all_enabled_providers().await;
        for p in enabled {
            if let Ok(v) = p.app_version().await {
                return Ok(v);
            }
        }
        Ok(env!("CARGO_PKG_VERSION").to_string())
    }

    pub async fn resolve_post_identity(
        &self,
        service: &str,
        post_id: &str,
    ) -> Result<Option<(String, String, String)>, String> {
        let candidates = self.get_providers_for_service(service).await;
        for p in candidates {
            if let Ok(Some(res)) = p.resolve_post_identity(service, post_id).await {
                return Ok(Some(res));
            }
        }
        Ok(None)
    }

    pub async fn expand_short_link(&self, raw_url: &str) -> Result<Option<String>, String> {
        let enabled = self.get_all_enabled_providers().await;
        for p in enabled {
            if let Ok(Some(res)) = p.expand_short_link(raw_url).await {
                return Ok(Some(res));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled(mut config: ProviderConfig, priority: u32) -> ProviderConfig {
        config.enabled = true;
        config.priority = priority;
        config
    }

    #[tokio::test]
    async fn test_provider_service_isolation_no_unwanted_fallback() {
        let coomer = enabled(CoomerProvider::default_config(), 1);
        let coomer_id = coomer.id.clone();
        let mut pawchive = PawchiveProvider::default_config();
        pawchive.enabled = false;
        pawchive.priority = 2;
        let configs = vec![coomer, pawchive];

        let manager = ProviderManager::new(configs);
        let patreon_providers = manager.get_providers_for_service("patreon").await;
        assert!(
            patreon_providers.is_empty(),
            "Must NOT fall back to Coomer when Patreon is requested!"
        );

        let fansly_providers = manager.get_providers_for_service("fansly").await;
        assert_eq!(fansly_providers.len(), 1);
        assert_eq!(fansly_providers[0].id(), coomer_id);
    }

    #[test]
    fn test_create_provider_distinct_types() {
        let coomer_cfg = enabled(CoomerProvider::default_config(), 1);
        let coomer_id = coomer_cfg.id.clone();
        let coomer_name = coomer_cfg.name.clone();
        let coomer_prov = create_provider(coomer_cfg).unwrap();
        assert_eq!(coomer_prov.id(), coomer_id);
        assert_eq!(coomer_prov.name(), coomer_name);
        assert!(!coomer_prov.auth_schema().supports_remote_favorites);

        let oh_cfg = enabled(OnlyHavenProvider::default_config(), 2);
        let oh_id = oh_cfg.id.clone();
        let oh_name = oh_cfg.name.clone();
        let oh_prov = create_provider(oh_cfg).unwrap();
        assert_eq!(oh_prov.id(), oh_id);
        assert_eq!(oh_prov.name(), oh_name);

        let paw_cfg = enabled(PawchiveProvider::default_config(), 3);
        let paw_id = paw_cfg.id.clone();
        let paw_name = paw_cfg.name.clone();
        let paw_prov = create_provider(paw_cfg).unwrap();
        assert_eq!(paw_prov.id(), paw_id);
        assert_eq!(paw_prov.name(), paw_name);
        assert!(paw_prov.auth_schema().supports_remote_favorites);
    }

    #[tokio::test]
    async fn test_manager_multi_provider_priority_and_filtering() {
        let coomer = enabled(CoomerProvider::default_config(), 1);
        let coomer_id = coomer.id.clone();
        let onlyhaven = enabled(OnlyHavenProvider::default_config(), 2);
        let onlyhaven_id = onlyhaven.id.clone();
        let pawchive = enabled(PawchiveProvider::default_config(), 3);
        let pawchive_id = pawchive.id.clone();
        let configs = vec![coomer, onlyhaven, pawchive];
        let manager = ProviderManager::new(configs);

        let of_providers = manager.get_providers_for_service("onlyfans").await;
        assert_eq!(of_providers.len(), 2);
        assert_eq!(of_providers[0].id(), coomer_id);
        assert_eq!(of_providers[1].id(), onlyhaven_id);

        let patreon_providers = manager.get_providers_for_service("patreon").await;
        assert_eq!(patreon_providers.len(), 2);
        assert_eq!(patreon_providers[0].id(), onlyhaven_id);
        assert_eq!(patreon_providers[1].id(), pawchive_id);

        let unknown = manager
            .get_providers_for_service("unsupported-test-service")
            .await;
        assert!(unknown.is_empty());
    }

    #[tokio::test]
    async fn live_manager_multi_provider_contracts() {
        let coomer = enabled(CoomerProvider::default_config(), 1);
        let coomer_id = coomer.id.clone();
        let coomer_file_url = coomer.file_url.clone().unwrap();
        let onlyhaven = enabled(OnlyHavenProvider::default_config(), 2);
        let onlyhaven_id = onlyhaven.id.clone();
        let onlyhaven_file_url = onlyhaven.file_url.clone().unwrap();
        let pawchive = enabled(PawchiveProvider::default_config(), 3);
        let pawchive_id = pawchive.id.clone();
        let pawchive_file_url = pawchive.file_url.clone().unwrap();
        let configs = vec![coomer, onlyhaven, pawchive];
        let manager = ProviderManager::new(configs);

        let coomer_health = manager.test_provider_health(&coomer_id).await.unwrap();
        assert!(
            coomer_health.is_healthy,
            "Coomer health failed: {:?}",
            coomer_health.error
        );

        let onlyhaven_health = manager.test_provider_health(&onlyhaven_id).await.unwrap();
        assert!(
            onlyhaven_health.is_healthy,
            "OnlyHaven health failed: {:?}",
            onlyhaven_health.error
        );

        let pawchive_health = manager.test_provider_health(&pawchive_id).await.unwrap();
        assert!(
            pawchive_health.is_healthy,
            "Pawchive health failed: {:?}",
            pawchive_health.error
        );

        let of_profile = manager
            .fetch_creator_profile("onlyfans", "prettykitttt", None)
            .await
            .unwrap();
        assert_eq!(of_profile.id, "prettykitttt");

        let patreon_profile = manager
            .fetch_creator_profile("patreon", "3340149", None)
            .await
            .unwrap();
        assert_eq!(patreon_profile.id, "3340149");

        let recent = manager.fetch_recent_posts(None, 0).await.unwrap();
        assert!(!recent.is_empty(), "Manager recent feed returned 0 posts");

        let recent_paged = manager.fetch_recent_posts(None, 50).await.unwrap();
        assert!(
            !recent_paged.is_empty(),
            "Manager recent paged returned 0 posts"
        );

        let recent_queried = manager.fetch_recent_posts(Some("cat"), 0).await.unwrap();
        assert!(
            !recent_queried.is_empty(),
            "Manager recent query returned 0 posts"
        );

        let popular = manager.fetch_popular_posts("day", None, 0).await.unwrap();
        assert!(!popular.is_empty(), "Manager popular feed returned 0 posts");
        for window in popular.windows(2) {
            let fav_a = window[0].favorite_count.unwrap_or(0);
            let fav_b = window[1].favorite_count.unwrap_or(0);
            assert!(
                fav_a >= fav_b,
                "Popular feed not sorted by favorites descending: {fav_a} < {fav_b}"
            );
        }

        let of_post = manager
            .fetch_post("onlyfans", "prettykitttt", "366178580", None)
            .await
            .unwrap();
        assert!(
            of_post.is_some(),
            "Manager failed to fetch Coomer post 366178580"
        );
        assert_eq!(of_post.unwrap().post.id, "366178580");

        let patreon_post = manager
            .fetch_post("patreon", "3340149", "142680139", None)
            .await
            .unwrap();
        assert!(
            patreon_post.is_some(),
            "Manager failed to fetch Pawchive post 142680139"
        );
        assert_eq!(patreon_post.unwrap().post.id, "142680139");

        let non_existent = manager
            .fetch_post("patreon", "3340149", "999999999999999", None)
            .await
            .unwrap();
        assert!(
            non_existent.is_none(),
            "Manager non-existent post returned Some"
        );

        let comments = manager
            .fetch_post_comments("fanbox", "6570768", "1836570")
            .await
            .unwrap();
        assert!(!comments.is_empty(), "Manager comments returned 0 items");

        let revisions = manager
            .fetch_post_revisions("patreon", "3340149", "142680139")
            .await
            .unwrap();
        let _ = revisions;

        let coomer_media = manager
            .resolve_media_url("onlyfans", "/data/aa/bb/c.mp4", None, Some(&coomer_id))
            .await;
        assert!(coomer_media.starts_with(&coomer_file_url));

        let pawchive_media = manager
            .resolve_media_url("patreon", "/data/aa/bb/c.mp4", None, Some(&pawchive_id))
            .await;
        assert!(pawchive_media.starts_with(&pawchive_file_url));

        let onlyhaven_media = manager
            .resolve_media_url(
                "fansly",
                "7426b2f88640e8807ec0f23a00e9702eb99ff2fd51913d6b27be12887e295fe2",
                None,
                Some(&onlyhaven_id),
            )
            .await;
        assert!(onlyhaven_media.starts_with(&onlyhaven_file_url));
    }

    #[tokio::test]
    async fn live_capabilities_and_coomer_popular_periods() {
        let coomer = enabled(CoomerProvider::default_config(), 1);
        let coomer_id = coomer.id.clone();
        let pawchive = enabled(PawchiveProvider::default_config(), 2);
        let pawchive_id = pawchive.id.clone();
        let onlyhaven = enabled(OnlyHavenProvider::default_config(), 3);
        let onlyhaven_id = onlyhaven.id.clone();
        let configs = vec![coomer, pawchive, onlyhaven];
        let manager = ProviderManager::new(configs);

        let all_caps = manager.get_provider_capabilities().await;
        let coomer_cap = all_caps.get(&coomer_id).unwrap();
        assert!(coomer_cap.popular.supported);
        assert_eq!(coomer_cap.popular.periods.len(), 4);
        assert!(coomer_cap.popular.periods.iter().any(|p| p.id == "recent"));
        assert!(coomer_cap.popular.supports_date);

        let pawchive_cap = all_caps.get(&pawchive_id).unwrap();
        assert!(pawchive_cap.popular.supported);
        assert_eq!(pawchive_cap.popular.periods.len(), 5);
        assert!(pawchive_cap.popular.periods.iter().any(|p| p.id == "all"));
        assert!(pawchive_cap.popular.supports_date);

        let onlyhaven_cap = all_caps.get(&onlyhaven_id).unwrap();
        assert!(onlyhaven_cap.popular.supported);
        assert_eq!(onlyhaven_cap.popular.periods.len(), 3);
        assert!(onlyhaven_cap.popular.supports_date);

        let active_cap = manager.get_active_capabilities().await;
        assert!(active_cap.popular.supported);
        assert_eq!(active_cap.popular.periods.len(), 3);
        let period_ids: Vec<&str> = active_cap
            .popular
            .periods
            .iter()
            .map(|p| p.id.as_str())
            .collect();
        assert_eq!(period_ids, vec!["day", "week", "month"]);
        assert!(active_cap.popular.supports_date);

        let coomer_provider = manager.get_provider_by_id(&coomer_id).await.unwrap();

        let popular_week = coomer_provider
            .fetch_popular_posts("week", None, 0)
            .await
            .unwrap();
        assert!(
            !popular_week.is_empty(),
            "Coomer popular week returned 0 posts"
        );

        let popular_recent = coomer_provider
            .fetch_popular_posts("recent", None, 0)
            .await
            .unwrap();
        assert!(
            !popular_recent.is_empty(),
            "Coomer popular recent returned 0 posts"
        );

        let popular_date = coomer_provider
            .fetch_popular_posts("day", Some("2025-01-01"), 0)
            .await
            .unwrap();
        assert!(
            !popular_date.is_empty(),
            "Coomer popular with date returned 0 posts"
        );

        let onlyhaven_provider = manager.get_provider_by_id(&onlyhaven_id).await.unwrap();
        let oh_popular_week = onlyhaven_provider
            .fetch_popular_posts("week", None, 0)
            .await
            .unwrap();
        assert!(
            !oh_popular_week.is_empty(),
            "OnlyHaven popular week returned 0 posts"
        );
    }

    #[tokio::test]
    async fn test_get_queue_for_url_dynamic_routing() {
        let mut pawchive = enabled(PawchiveProvider::default_config(), 1);
        pawchive.api_url = "https://source-one.test".into();
        pawchive.fallback_urls = vec!["https://source-one-mirror.test".into()];
        pawchive.file_url = Some("https://source-one-files.test".into());
        pawchive.image_url = Some("https://source-one-images.test".into());
        pawchive.is_custom = true;
        let pawchive_id = pawchive.id.clone();
        let coomer = enabled(CoomerProvider::default_config(), 2);
        let coomer_id = coomer.id.clone();
        let coomer_file_url = coomer.file_url.clone().unwrap();
        let configs = vec![pawchive, coomer];
        let manager = ProviderManager::new(configs);

        let q1 = manager
            .get_queue_for_url("https://source-one-files.test/data/aa/bb/image.jpg")
            .await;
        let pawchive_prov = manager.get_provider_by_id(&pawchive_id).await.unwrap();
        assert_eq!(
            q1.provider_id(),
            pawchive_prov.request_queue().unwrap().provider_id()
        );

        let q2 = manager
            .get_queue_for_url("https://source-one-mirror.test/api/v1/posts")
            .await;
        assert_eq!(
            q2.provider_id(),
            pawchive_prov.request_queue().unwrap().provider_id()
        );

        let q3 = manager
            .get_queue_for_url(&format!("{coomer_file_url}/data/11/22/video.mp4"))
            .await;
        let coomer_prov = manager.get_provider_by_id(&coomer_id).await.unwrap();
        assert_eq!(
            q3.provider_id(),
            coomer_prov.request_queue().unwrap().provider_id()
        );

        let q4 = manager
            .get_queue_for_url("https://unknown-third-party.com/file.zip")
            .await;
        assert_eq!(q4.provider_id(), "default");
    }
}
