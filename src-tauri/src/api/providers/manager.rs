use super::coomer::CoomerProvider;
use super::onlyhaven::OnlyHavenProvider;
use super::pawchive::PawchiveProvider;
use super::traits::{
    default_coomer_services, default_onlyhaven_services, default_pawchive_services,
    FavoritesSyncResult, ProviderAuthSchema, ProviderConfig, ProviderHealth, SourceProvider,
};
use crate::api::models::*;
use crate::api::reconciliation::{reconcile_post_snapshots, ReconciledPost};
use futures_util::future::join_all;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

fn create_provider(config: ProviderConfig) -> Result<Arc<dyn SourceProvider>, String> {
    let id_lower = config.id.to_lowercase();
    let url_lower = config.api_url.to_lowercase();

    if id_lower == "onlyhaven" || url_lower.contains("cum.st") {
        Ok(Arc::new(OnlyHavenProvider::new(config)?))
    } else if id_lower == "coomer" || url_lower.contains("coomer") {
        Ok(Arc::new(CoomerProvider::new(config)?))
    } else {
        Ok(Arc::new(PawchiveProvider::new(config)?))
    }
}

pub struct ProviderManager {
    providers: Arc<RwLock<Vec<Arc<dyn SourceProvider>>>>,
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
        }
    }

    pub fn default_configs() -> Vec<ProviderConfig> {
        vec![
            ProviderConfig {
                id: "pawchive".into(),
                name: "Pawchive".into(),
                enabled: true,
                api_url: "https://pawchive.pw".into(),
                fallback_urls: vec![],
                file_url: Some("https://file.pawchive.pw".into()),
                image_url: Some("https://img.pawchive.pw".into()),
                file_prefix: Some("file".into()),
                image_prefix: Some("img".into()),
                session_cookie: String::new(),
                username: String::new(),
                services: default_pawchive_services(),
                is_custom: false,
                priority: 1,
            },
            ProviderConfig {
                id: "coomer".into(),
                name: "Coomer".into(),
                enabled: false,
                api_url: "https://coomer.st".into(),
                fallback_urls: vec![],
                file_url: Some("https://c1.coomer.st".into()),
                image_url: Some("https://img.coomer.st".into()),
                file_prefix: Some("c1".into()),
                image_prefix: Some("img".into()),
                session_cookie: String::new(),
                username: String::new(),
                services: default_coomer_services(),
                is_custom: false,
                priority: 2,
            },
            ProviderConfig {
                id: "onlyhaven".into(),
                name: "OnlyHaven".into(),
                enabled: false,
                api_url: "https://cum.st".into(),
                fallback_urls: vec![],
                file_url: Some("https://e1.cum.st".into()),
                image_url: Some("https://img.cum.st".into()),
                file_prefix: Some("e1".into()),
                image_prefix: Some("img".into()),
                session_cookie: String::new(),
                username: String::new(),
                services: default_onlyhaven_services(),
                is_custom: false,
                priority: 3,
            },
        ]
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
                async move { p.fetch_creators().await }
            })
            .collect();

        let results = join_all(tasks).await;
        let mut creators_map: BTreeMap<String, Creator> = BTreeMap::new();
        let mut any_success = false;
        let mut last_error = String::new();

        for res in results {
            match res {
                Ok(creators) => {
                    any_success = true;
                    for c in creators {
                        let key = format!("{}:{}", c.service.to_lowercase(), c.id.to_lowercase());
                        if let Some(existing) = creators_map.get_mut(&key) {
                            if c.updated.unwrap_or(0) > existing.updated.unwrap_or(0) {
                                *existing = c;
                            }
                        } else {
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
    ) -> Result<Creator, String> {
        let candidates = self.get_providers_for_service(service).await;
        if candidates.is_empty() {
            return Err(format!("No provider configured for service '{service}'"));
        }

        let mut last_error = String::new();
        for provider in candidates {
            match provider.fetch_creator_profile(service, creator_id).await {
                Ok(profile) => return Ok(profile),
                Err(e) => last_error = e,
            }
        }

        Err(last_error)
    }

    pub async fn fetch_creator_links(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<CreatorProfile>, String> {
        let candidates = self.get_providers_for_service(service).await;
        for provider in candidates {
            if let Ok(links) = provider.fetch_creator_links(service, creator_id).await {
                if !links.is_empty() {
                    return Ok(links);
                }
            }
        }
        Ok(Vec::new())
    }

    pub async fn fetch_similar_creators(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<CreatorProfile>, String> {
        let candidates = self.get_providers_for_service(service).await;
        for provider in candidates {
            if let Ok(similar) = provider.fetch_similar_creators(service, creator_id).await {
                if !similar.is_empty() {
                    return Ok(similar);
                }
            }
        }
        Ok(Vec::new())
    }

    pub async fn fetch_creator_tags(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<String>, String> {
        let candidates = self.get_providers_for_service(service).await;
        for provider in candidates {
            if let Ok(tags) = provider.fetch_creator_tags(service, creator_id).await {
                if !tags.is_empty() {
                    return Ok(tags);
                }
            }
        }
        Ok(Vec::new())
    }

    pub async fn fetch_announcements(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<Announcement>, String> {
        let candidates = self.get_providers_for_service(service).await;
        for provider in candidates {
            if let Ok(items) = provider.fetch_announcements(service, creator_id).await {
                if !items.is_empty() {
                    return Ok(items);
                }
            }
        }
        Ok(Vec::new())
    }

    pub async fn fetch_posts(
        &self,
        service: &str,
        creator_id: &str,
        offset: u32,
        query: Option<&str>,
    ) -> Result<Vec<Post>, String> {
        let candidates = self.get_providers_for_service(service).await;
        if candidates.is_empty() {
            return Err(format!("No provider configured for service '{service}'"));
        }

        let mut last_error = String::new();
        for provider in candidates {
            match provider
                .fetch_posts(service, creator_id, offset, query)
                .await
            {
                Ok(posts) => return Ok(posts),
                Err(e) => last_error = e,
            }
        }

        Err(last_error)
    }

    pub async fn fetch_post(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Option<ReconciledPost>, String> {
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
                    post_opt.map(|post| (conf.id.clone(), post))
                }
            })
            .collect();

        let results = join_all(tasks).await;
        let snapshots: Vec<(String, Post)> = results.into_iter().flatten().collect();

        if snapshots.is_empty() {
            return Ok(None);
        }

        Ok(reconcile_post_snapshots(snapshots))
    }

    pub async fn fetch_post_revisions(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Vec<PostRevision>, String> {
        let candidates = self.get_providers_for_service(service).await;
        for provider in candidates {
            if let Ok(revs) = provider
                .fetch_post_revisions(service, creator_id, post_id)
                .await
            {
                if !revs.is_empty() {
                    return Ok(revs);
                }
            }
        }
        Ok(Vec::new())
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

        // 1. Pull from remote to local
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

        // 2. Push from local to remote (with 100ms throttle to prevent HTTP 429)
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
        if let Some(id) = provider_id {
            if let Some(p) = self.get_provider_by_id(id).await {
                return p.resolve_media_url(file_path, server);
            }
        }
        let candidates = self.get_providers_for_service(service).await;
        if let Some(first) = candidates.first() {
            first.resolve_media_url(file_path, server)
        } else {
            let enabled = self.get_all_enabled_providers().await;
            if let Some(first) = enabled.first() {
                first.resolve_media_url(file_path, server)
            } else if let Some(first) = self.providers.read().await.first() {
                first.resolve_media_url(file_path, server)
            } else {
                let clean = file_path
                    .trim_start_matches('/')
                    .trim_start_matches("data/")
                    .trim_start_matches('/');
                format!("https://file.pawchive.pw/data/{clean}")
            }
        }
    }

    pub async fn resolve_thumbnail_url(&self, service: &str, thumb_path: &str) -> String {
        let candidates = self.get_providers_for_service(service).await;
        if let Some(first) = candidates.first() {
            first.resolve_thumbnail_url(thumb_path)
        } else {
            let enabled = self.get_all_enabled_providers().await;
            if let Some(first) = enabled.first() {
                first.resolve_thumbnail_url(thumb_path)
            } else if let Some(first) = self.providers.read().await.first() {
                first.resolve_thumbnail_url(thumb_path)
            } else {
                let clean = thumb_path
                    .trim_start_matches('/')
                    .trim_start_matches("data/")
                    .trim_start_matches('/');
                format!("https://img.pawchive.pw/thumbnail/data/{clean}")
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

    pub async fn fetch_fancards(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<Fancard>, String> {
        let candidates = self.get_providers_for_service(service).await;
        for provider in candidates {
            if let Ok(cards) = provider.fetch_fancards(service, creator_id).await {
                if !cards.is_empty() {
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

    #[tokio::test]
    async fn test_provider_service_isolation_no_unwanted_fallback() {
        let configs = vec![
            ProviderConfig {
                id: "coomer".into(),
                name: "Coomer".into(),
                enabled: true,
                api_url: "https://coomer.st".into(),
                fallback_urls: vec![],
                file_url: None,
                image_url: None,
                file_prefix: None,
                image_prefix: None,
                session_cookie: String::new(),
                username: String::new(),
                services: vec!["onlyfans".into(), "fansly".into(), "candfans".into()],
                is_custom: false,
                priority: 1,
            },
            ProviderConfig {
                id: "pawchive".into(),
                name: "Pawchive".into(),
                enabled: false,
                api_url: "https://pawchive.pw".into(),
                fallback_urls: vec![],
                file_url: None,
                image_url: None,
                file_prefix: None,
                image_prefix: None,
                session_cookie: String::new(),
                username: String::new(),
                services: vec!["patreon".into(), "fanbox".into(), "discord".into()],
                is_custom: false,
                priority: 2,
            },
        ];

        let manager = ProviderManager::new(configs);
        let patreon_providers = manager.get_providers_for_service("patreon").await;
        assert!(
            patreon_providers.is_empty(),
            "Must NOT fall back to Coomer when Patreon is requested!"
        );

        let fansly_providers = manager.get_providers_for_service("fansly").await;
        assert_eq!(fansly_providers.len(), 1);
        assert_eq!(fansly_providers[0].id(), "coomer");
    }
}
