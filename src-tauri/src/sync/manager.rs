use super::client::{
    CreateAccountRequest, PushRecordInput, ReplaceBundleRequest, SyncDevice, SyncHttpClient,
    PULL_PAGE_LIMIT,
};
use super::crypto::{
    decrypt_record, encrypt_record, unwrap_vault, wrap_vault, KdfEnvelope, VaultSecrets, KEY_BYTES,
};
use super::repository::{
    PawchiveSessionRecord, RemoteApply, SyncRepository, SyncState, MAX_AVATAR_DATA_URL_LEN,
};
use super::secrets::SecretStore;
use crate::config::settings::ConfigManager;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::Emitter;
use uuid::Uuid;

const SESSION_REUSE: Duration = Duration::from_secs(25 * 24 * 60 * 60);

struct CachedSession {
    account_id: String,
    device_id: String,
    token: String,
    created: Instant,
}

fn is_unauthorized(error: &str) -> bool {
    error.contains("HTTP 401")
}

fn valid_avatar_data_url(image: &str) -> bool {
    image.len() <= MAX_AVATAR_DATA_URL_LEN
        && (image.starts_with("data:image/jpeg;base64,")
            || image.starts_with("data:image/png;base64,")
            || image.starts_with("data:image/webp;base64,"))
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncStatus {
    pub configured: bool,
    pub enabled: bool,
    pub unlocked: bool,
    pub syncing: bool,
    pub account_id: Option<String>,
    pub server_url: Option<String>,
    pub device_id: Option<String>,
    pub revision: i64,
    pub cursor: i64,
    pub conflict: bool,
    pub last_synced_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProgress {
    pub phase: String,
    pub current: Option<usize>,
    pub total: Option<usize>,
    pub progress: Option<f32>,
    pub message: Option<String>,
}

fn emit_sync_progress(
    app: &tauri::AppHandle,
    phase: &str,
    current: Option<usize>,
    total: Option<usize>,
    progress: Option<f32>,
    message: Option<&str>,
) {
    let _ = app.emit(
        "sync-progress",
        &SyncProgress {
            phase: phase.to_string(),
            current,
            total,
            progress,
            message: message.map(|s| s.to_string()),
        },
    );
}

#[derive(Serialize, Deserialize)]
struct RecoveryKit {
    format: String,
    server_url: String,
    account_id: String,
    secrets: VaultSecrets,
}
pub struct SyncManager {
    repository: Arc<SyncRepository>,
    config: Arc<ConfigManager>,
    syncing: Mutex<bool>,
    session: Mutex<Option<CachedSession>>,
    on_change_notify: Arc<tokio::sync::Notify>,
}
impl SyncManager {
    pub fn is_syncing(&self) -> bool {
        self.syncing.lock().map(|syncing| *syncing).unwrap_or(true)
    }

    pub fn new(repository: Arc<SyncRepository>, config: Arc<ConfigManager>) -> Self {
        Self {
            repository,
            config,
            syncing: Mutex::new(false),
            session: Mutex::new(None),
            on_change_notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    async fn session_token(
        &self,
        client: &SyncHttpClient,
        account_id: &str,
        auth_token: &str,
        device_id: &str,
    ) -> Result<String, String> {
        if let Ok(cache) = self.session.lock() {
            if let Some(cached) = cache.as_ref() {
                if cached.account_id == account_id
                    && cached.device_id == device_id
                    && cached.created.elapsed() < SESSION_REUSE
                {
                    return Ok(cached.token.clone());
                }
            }
        }
        let session = client
            .session(account_id, auth_token, Some(device_id))
            .await?;
        if let Ok(mut cache) = self.session.lock() {
            *cache = Some(CachedSession {
                account_id: account_id.to_string(),
                device_id: device_id.to_string(),
                token: session.token.clone(),
                created: Instant::now(),
            });
        }
        Ok(session.token)
    }

    fn forget_session(&self) {
        if let Ok(mut cache) = self.session.lock() {
            *cache = None;
        }
    }

    pub fn avatar(&self) -> Result<Option<String>, String> {
        self.repository.avatar()
    }

    pub fn set_avatar(&self, image: &str) -> Result<Option<String>, String> {
        if !valid_avatar_data_url(image) {
            return Err("Avatar must be a JPEG, PNG, or WebP image under 64 KB".to_string());
        }
        self.repository.save_avatar(image)?;
        Ok(Some(image.to_string()))
    }

    pub fn clear_avatar(&self) -> Result<Option<String>, String> {
        self.repository.clear_avatar()?;
        Ok(None)
    }
    pub fn start(self: &Arc<Self>, app: tauri::AppHandle) {
        let manager = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                let mut interval_secs = 60;
                if let Ok(settings) = manager.config.load() {
                    let push_interval = if !settings.sync_on_change {
                        settings.sync_push_interval_seconds.max(5) as u64
                    } else {
                        u64::MAX
                    };
                    let pull_interval = settings.sync_pull_interval_seconds.max(5) as u64;
                    interval_secs = pull_interval.min(push_interval).max(5);

                    if settings.sync_enabled && settings.sync_auto {
                        if let Ok(Some(st)) = manager.repository.state() {
                            if SecretStore::load_vault(&st.account_id)
                                .ok()
                                .flatten()
                                .is_some()
                                && manager.repository.conflict().ok().flatten().is_none()
                            {
                                tracing::debug!("Triggering background auto-sync");
                                if let Err(e) = manager.sync(app.clone()).await {
                                    tracing::warn!(error = %e, "Background auto-sync failed");
                                }
                            }
                        }
                    }
                }

                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_secs(interval_secs)) => {},
                    _ = manager.on_change_notify.notified() => {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        tracing::debug!("Triggering sync-on-change");
                        if let Err(e) = manager.sync(app.clone()).await {
                            tracing::warn!(error = %e, "Sync-on-change failed");
                        }
                    }
                }
            }
        });
    }
    pub fn status(&self) -> Result<SyncStatus, String> {
        let state = self.repository.state()?;
        let syncing = *self.syncing.lock().map_err(|e| e.to_string())?;
        let settings = self.config.load().unwrap_or_default();
        match state {
            Some(v) => {
                let unlocked = match SecretStore::load_vault(&v.account_id) {
                    Ok(vault) => vault.is_some(),
                    Err(e) => {
                        tracing::warn!(account_id = %v.account_id, error = %e, "Failed to load vault secrets during status check");
                        false
                    }
                };
                Ok(SyncStatus {
                    configured: true,
                    enabled: settings.sync_enabled,
                    unlocked,
                    syncing,
                    account_id: Some(v.account_id),
                    server_url: Some(v.server_url),
                    device_id: Some(v.device_id),
                    revision: v.revision,
                    cursor: v.cursor,
                    conflict: self.repository.conflict().unwrap_or(None).is_some(),
                    last_synced_at: v.last_synced_at,
                    last_error: v.last_error,
                })
            }
            None => Ok(SyncStatus {
                configured: false,
                enabled: false,
                unlocked: false,
                syncing,
                account_id: None,
                server_url: None,
                device_id: None,
                revision: 0,
                cursor: 0,
                conflict: false,
                last_synced_at: None,
                last_error: None,
            }),
        }
    }
    pub fn set_enabled(&self, enabled: bool) -> Result<SyncStatus, String> {
        if let Ok(mut settings) = self.config.load() {
            if settings.sync_enabled != enabled {
                settings.sync_enabled = enabled;
                let _ = self.config.save(&settings);
            }
        }
        let state = self.repository.state()?;
        if let Some(mut st) = state {
            st.enabled = enabled;
            self.repository.save_state(&st)?;
        }
        self.status()
    }
    pub fn trigger_sync_on_change(self: &Arc<Self>, _app: tauri::AppHandle) {
        if let Ok(settings) = self.config.load() {
            if settings.sync_enabled && settings.sync_on_change {
                if let Ok(Some(st)) = self.repository.state() {
                    if SecretStore::load_vault(&st.account_id)
                        .ok()
                        .flatten()
                        .is_some()
                        && self.repository.conflict().ok().flatten().is_none()
                    {
                        self.on_change_notify.notify_one();
                    }
                }
            }
        }
    }
    pub async fn create_account(
        self: &Arc<Self>,
        server_url: String,
        account_id: String,
        password: String,
        device_name: String,
        app: tauri::AppHandle,
    ) -> Result<SyncStatus, String> {
        tracing::info!(account_id = %account_id, server_url = %server_url, "Creating new sync account...");
        if let Some(existing) = self.repository.state()? {
            tracing::warn!(
                existing_account = %existing.account_id,
                "Overwriting prior sync state with newly created account"
            );
            let _ = SecretStore::delete_vault(&existing.account_id);
            self.repository.clear()?;
        }
        if password.chars().count() < 12 {
            return Err("Master password must contain at least 12 characters".to_string());
        }
        let settings = self.config.load()?;
        let client = SyncHttpClient::new(&server_url, &settings)?;
        let kdf = KdfEnvelope::generate();
        let secrets = VaultSecrets::generate();
        let wrapped = wrap_vault(&password, &kdf, &secrets)?;
        let kdf_value = serde_json::to_value(&kdf).map_err(|e| e.to_string())?;
        client
            .create_account(&CreateAccountRequest {
                account_id: &account_id,
                auth_token: &secrets.auth_token,
                kdf: &kdf_value,
                encrypted_key_bundle: &wrapped.ciphertext,
                nonce: &wrapped.nonce,
            })
            .await?;
        let session = client
            .session(&account_id, &secrets.auth_token, None)
            .await?;
        let device_id = Uuid::new_v4().to_string();
        client
            .register_device(&session.token, &device_id, &device_name)
            .await?;
        SecretStore::save_vault(&account_id, &secrets)?;
        self.repository.save_state(&SyncState {
            server_url,
            account_id,
            device_id,
            snapshot_record_id: Uuid::new_v4().to_string(),
            revision: 0,
            cursor: 0,
            last_plaintext_hash: None,
            kdf_json: serde_json::to_string(&kdf).map_err(|e| e.to_string())?,
            encrypted_key_bundle: wrapped.ciphertext,
            bundle_nonce: wrapped.nonce,
            enabled: true,
            last_synced_at: None,
            last_error: None,
        })?;
        tracing::info!("Account created successfully. Initiating initial sync.");
        self.sync(app).await
    }
    pub async fn connect(
        self: &Arc<Self>,
        server_url: String,
        account_id: String,
        password: String,
        device_name: String,
        app: tauri::AppHandle,
    ) -> Result<SyncStatus, String> {
        tracing::info!(account_id = %account_id, server_url = %server_url, "Connecting sync account...");
        if let Some(existing) = self.repository.state()? {
            tracing::warn!(
                existing_account = %existing.account_id,
                new_account = %account_id,
                "Overwriting prior sync state with newly connected account"
            );
            let _ = SecretStore::delete_vault(&existing.account_id);
            self.repository.clear()?;
        }
        let client = SyncHttpClient::new(&server_url, &self.config.load()?)?;
        let bundle = client.bundle(&account_id).await?;
        if let Some(ref bundle_acc) = bundle.account_id {
            if bundle_acc != &account_id {
                return Err("Sync server returned the wrong account envelope".to_string());
            }
        }
        let kdf: KdfEnvelope = serde_json::from_value(bundle.kdf).map_err(|e| e.to_string())?;
        let secrets = unwrap_vault(&password, &kdf, &bundle.encrypted_key_bundle, &bundle.nonce)?;
        let session = client
            .session(&account_id, &secrets.auth_token, None)
            .await?;
        let device_id = Uuid::new_v4().to_string();
        client
            .register_device(&session.token, &device_id, &device_name)
            .await?;
        SecretStore::save_vault(&account_id, &secrets)?;
        self.repository.save_state(&SyncState {
            server_url,
            account_id,
            device_id,
            snapshot_record_id: Uuid::new_v4().to_string(),
            revision: 0,
            cursor: 0,
            last_plaintext_hash: None,
            kdf_json: serde_json::to_string(&kdf).map_err(|e| e.to_string())?,
            encrypted_key_bundle: bundle.encrypted_key_bundle,
            bundle_nonce: bundle.nonce,
            enabled: true,
            last_synced_at: None,
            last_error: None,
        })?;
        tracing::info!(
            "Sync account credentials verified and state saved. Initiating initial sync."
        );
        self.sync(app).await
    }
    pub fn unlock(&self, password: &str) -> Result<SyncStatus, String> {
        let state = self
            .repository
            .state()?
            .ok_or_else(|| "Sync is not configured".to_string())?;
        let kdf: KdfEnvelope = serde_json::from_str(&state.kdf_json).map_err(|e| e.to_string())?;
        let secrets = unwrap_vault(
            password,
            &kdf,
            &state.encrypted_key_bundle,
            &state.bundle_nonce,
        )?;
        SecretStore::save_vault(&state.account_id, &secrets)?;
        self.status()
    }
    pub fn lock(&self) -> Result<SyncStatus, String> {
        if let Some(state) = self.repository.state()? {
            SecretStore::delete_vault(&state.account_id)?
        }
        self.status()
    }
    pub fn disconnect(&self) -> Result<SyncStatus, String> {
        if let Some(state) = self.repository.state()? {
            SecretStore::delete_vault(&state.account_id)?
        }
        self.repository.clear()?;
        self.status()
    }
    pub fn recovery_kit(&self) -> Result<String, String> {
        let state = self
            .repository
            .state()?
            .ok_or_else(|| "Sync is not configured".to_string())?;
        let secrets = SecretStore::load_vault(&state.account_id)?
            .ok_or_else(|| "Sync vault is locked".to_string())?;
        serde_json::to_string_pretty(&RecoveryKit {
            format: "pawstash-recovery-v1".to_string(),
            server_url: state.server_url,
            account_id: state.account_id,
            secrets,
        })
        .map_err(|e| e.to_string())
    }
    pub async fn recover(
        self: &Arc<Self>,
        recovery_kit: &str,
        new_password: &str,
        device_name: &str,
        app: tauri::AppHandle,
    ) -> Result<SyncStatus, String> {
        tracing::info!("Recovering sync account from recovery kit...");
        if let Some(existing) = self.repository.state()? {
            tracing::warn!(
                existing_account = %existing.account_id,
                "Overwriting prior sync state during recovery"
            );
            let _ = SecretStore::delete_vault(&existing.account_id);
            self.repository.clear()?;
        }
        if new_password.chars().count() < 12 {
            return Err("New master password must contain at least 12 characters".to_string());
        }
        let kit: RecoveryKit = serde_json::from_str(recovery_kit).map_err(|e| e.to_string())?;
        if kit.format != "pawstash-recovery-v1"
            || kit.account_id.trim().is_empty()
            || kit.device_name_invalid(device_name)
        {
            return Err("Invalid Pawstash recovery kit".to_string());
        }
        kit.secrets.vault_key_bytes()?;
        let client = SyncHttpClient::new(&kit.server_url, &self.config.load()?)?;
        let bootstrap = client
            .session(&kit.account_id, &kit.secrets.auth_token, None)
            .await?;
        let device_id = Uuid::new_v4().to_string();
        client
            .register_device(&bootstrap.token, &device_id, device_name)
            .await?;
        let session = client
            .session(&kit.account_id, &kit.secrets.auth_token, Some(&device_id))
            .await?;
        let kdf = KdfEnvelope::generate();
        let wrapped = wrap_vault(new_password, &kdf, &kit.secrets)?;
        let kdf_value = serde_json::to_value(&kdf).map_err(|e| e.to_string())?;
        client
            .replace_bundle(
                &session.token,
                &kit.account_id,
                &ReplaceBundleRequest {
                    kdf: &kdf_value,
                    encrypted_key_bundle: &wrapped.ciphertext,
                    nonce: &wrapped.nonce,
                },
            )
            .await?;
        SecretStore::save_vault(&kit.account_id, &kit.secrets)?;
        self.repository.save_state(&SyncState {
            server_url: kit.server_url,
            account_id: kit.account_id,
            device_id,
            snapshot_record_id: Uuid::new_v4().to_string(),
            revision: 0,
            cursor: 0,
            last_plaintext_hash: None,
            kdf_json: serde_json::to_string(&kdf).map_err(|e| e.to_string())?,
            encrypted_key_bundle: wrapped.ciphertext,
            bundle_nonce: wrapped.nonce,
            enabled: true,
            last_synced_at: None,
            last_error: None,
        })?;
        self.sync(app).await
    }
    pub async fn change_password(
        &self,
        current_password: &str,
        new_password: &str,
    ) -> Result<SyncStatus, String> {
        if new_password.chars().count() < 12 {
            return Err("New master password must contain at least 12 characters".to_string());
        }
        let mut state = self
            .repository
            .state()?
            .ok_or_else(|| "Sync is not configured".to_string())?;
        let previous_state = state.clone();
        let client = SyncHttpClient::new(&state.server_url, &self.config.load()?)?;
        let remote = client.bundle(&state.account_id).await?;
        let current_kdf: KdfEnvelope =
            serde_json::from_value(remote.kdf).map_err(|e| e.to_string())?;
        let secrets = unwrap_vault(
            current_password,
            &current_kdf,
            &remote.encrypted_key_bundle,
            &remote.nonce,
        )?;
        let token = self
            .session_token(
                &client,
                &state.account_id,
                &secrets.auth_token,
                &state.device_id,
            )
            .await?;
        let new_kdf = KdfEnvelope::generate();
        let wrapped = wrap_vault(new_password, &new_kdf, &secrets)?;
        let kdf_value = serde_json::to_value(&new_kdf).map_err(|e| e.to_string())?;
        state.kdf_json = serde_json::to_string(&new_kdf).map_err(|e| e.to_string())?;
        state.encrypted_key_bundle = wrapped.ciphertext.clone();
        state.bundle_nonce = wrapped.nonce.clone();
        self.repository.save_state(&state)?;
        if let Err(error) = client
            .replace_bundle(
                &token,
                &state.account_id,
                &ReplaceBundleRequest {
                    kdf: &kdf_value,
                    encrypted_key_bundle: &wrapped.ciphertext,
                    nonce: &wrapped.nonce,
                },
            )
            .await
        {
            let _ = self.repository.save_state(&previous_state);
            return Err(error);
        }
        SecretStore::save_vault(&state.account_id, &secrets)?;
        self.status()
    }
    pub async fn devices(&self) -> Result<Vec<SyncDevice>, String> {
        let state = self
            .repository
            .state()?
            .ok_or_else(|| "Sync is not configured".to_string())?;
        let secrets = SecretStore::load_vault(&state.account_id)?
            .ok_or_else(|| "Sync vault is locked".to_string())?;
        let client = SyncHttpClient::new(&state.server_url, &self.config.load()?)?;
        let token = self
            .session_token(
                &client,
                &state.account_id,
                &secrets.auth_token,
                &state.device_id,
            )
            .await?;
        match client.list_devices(&token).await {
            Err(error) if is_unauthorized(&error) => {
                self.forget_session();
                let token = self
                    .session_token(
                        &client,
                        &state.account_id,
                        &secrets.auth_token,
                        &state.device_id,
                    )
                    .await?;
                client.list_devices(&token).await
            }
            result => result,
        }
    }
    pub async fn revoke_device(&self, device_id: &str) -> Result<Vec<SyncDevice>, String> {
        let state = self
            .repository
            .state()?
            .ok_or_else(|| "Sync is not configured".to_string())?;
        if device_id == state.device_id {
            return Err("The current device cannot revoke itself".to_string());
        }
        let secrets = SecretStore::load_vault(&state.account_id)?
            .ok_or_else(|| "Sync vault is locked".to_string())?;
        let client = SyncHttpClient::new(&state.server_url, &self.config.load()?)?;
        let token = self
            .session_token(
                &client,
                &state.account_id,
                &secrets.auth_token,
                &state.device_id,
            )
            .await?;
        if let Err(error) = client.revoke_device(&token, device_id).await {
            if !is_unauthorized(&error) {
                return Err(error);
            }
            self.forget_session();
            let token = self
                .session_token(
                    &client,
                    &state.account_id,
                    &secrets.auth_token,
                    &state.device_id,
                )
                .await?;
            client.revoke_device(&token, device_id).await?;
        }
        client.list_devices(&token).await
    }
    pub async fn sync(self: &Arc<Self>, app: tauri::AppHandle) -> Result<SyncStatus, String> {
        struct SyncingGuard<'a>(&'a Mutex<bool>, tauri::AppHandle);
        impl<'a> Drop for SyncingGuard<'a> {
            fn drop(&mut self) {
                if let Ok(mut lock) = self.0.lock() {
                    *lock = false;
                }
                emit_sync_progress(&self.1, "idle", None, None, None, None);
            }
        }

        let result = {
            let _guard = {
                let mut syncing = self.syncing.lock().map_err(|e| e.to_string())?;
                if *syncing {
                    return Err("Sync is already running".to_string());
                }
                *syncing = true;
                emit_sync_progress(&app, "connecting", None, None, None, Some("Connecting..."));
                SyncingGuard(&self.syncing, app.clone())
            };

            self.sync_inner(&app).await
        };

        if let Err(error) = &result {
            tracing::error!(error = %error, "Sync failed");
            let _ = self.repository.set_error(Some(error));
        } else {
            tracing::info!("Sync completed successfully");
        }
        let status = self.status()?;
        let _ = app.emit("sync-status-updated", &status);
        result.map(|_| status)
    }
    /// A full page that doesn't advance the cursor ends the pull; asking again would
    /// return the same page forever.
    async fn pull_all(
        &self,
        app: &tauri::AppHandle,
        client: &SyncHttpClient,
        token: &str,
        key: &[u8; KEY_BYTES],
        cursor: &mut i64,
    ) -> Result<(), String> {
        loop {
            if self.repository.state()?.is_none() {
                return Err("Sync was disconnected".to_string());
            }
            emit_sync_progress(
                app,
                "pulling",
                None,
                None,
                None,
                Some("Fetching changes..."),
            );
            tracing::debug!(cursor = *cursor, "Sync: pulling changes...");
            let pull = client.pull(token, *cursor).await?;
            let change_count = pull.changes.len();
            tracing::info!(
                count = change_count,
                cursor = pull.cursor,
                "Sync: received changes from server"
            );

            let mut batch = Vec::with_capacity(change_count);
            for (idx, change) in pull.changes.iter().enumerate() {
                if idx % 50 == 0 || idx + 1 == change_count {
                    emit_sync_progress(
                        app,
                        "pulling",
                        Some(idx + 1),
                        Some(change_count),
                        Some((idx + 1) as f32 / change_count as f32),
                        Some("Applying changes..."),
                    );
                }
                let plaintext = if change.tombstone {
                    None
                } else {
                    Some(decrypt_record(
                        key,
                        &change.record_id,
                        &change.ciphertext,
                        &change.nonce,
                    )?)
                };
                batch.push(RemoteApply {
                    record_id: &change.record_id,
                    kind: &change.kind,
                    revision: change.revision,
                    plaintext,
                    tombstone: change.tombstone,
                });
            }
            if !batch.is_empty() {
                if let Err(e) = self.repository.apply_remote_changes(&batch) {
                    tracing::error!(error = %e, count = batch.len(), "Sync: failed to apply remote changes");
                    return Err(e);
                }
                for change in batch.iter().filter(|c| c.kind == "session" && !c.tombstone) {
                    let Some(bytes) = change.plaintext.as_deref() else {
                        continue;
                    };
                    if let Err(e) = self.apply_session_record(bytes) {
                        tracing::error!(error = %e, record_id = %change.record_id, "Sync: failed to apply synced Pawchive session");
                    }
                }
                if batch.iter().any(|c| c.kind == "profile") {
                    let _ = app.emit("account-avatar-updated", ());
                }
            }

            let page_max = pull.changes.iter().map(|c| c.position).max().unwrap_or(0);
            let mut next = pull.cursor.max(page_max);
            // Servers with a pruned change log answer a snapshot with cursor 0. Positions are
            // global, so 1 safely marks the snapshot as applied.
            if next == 0 && *cursor == 0 && change_count > 0 {
                next = 1;
            }
            let advanced = next > *cursor;
            if advanced {
                *cursor = next;
                self.repository.update_cursor(next)?;
            }
            if change_count < PULL_PAGE_LIMIT {
                return Ok(());
            }
            if !advanced {
                tracing::warn!(
                    count = change_count,
                    cursor = pull.cursor,
                    "Sync: server returned a full page without advancing the cursor; stopping pull"
                );
                return Ok(());
            }
        }
    }

    fn apply_session_record(&self, bytes: &[u8]) -> Result<(), String> {
        let mut settings = self.config.load()?;
        if !settings.sync_pawchive_session {
            return Ok(());
        }
        let record: PawchiveSessionRecord = serde_json::from_slice(bytes)
            .map_err(|e| format!("Failed to parse session record: {e}"))?;
        if record.session_cookie.is_empty() {
            return Ok(());
        }
        let username_changed =
            !record.username.is_empty() && settings.pawchive_username != record.username;
        if settings.session_cookie == record.session_cookie && !username_changed {
            return Ok(());
        }
        settings.session_cookie = record.session_cookie;
        if username_changed {
            settings.pawchive_username = record.username;
        }
        self.config.save(&settings)
    }

    async fn sync_inner(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let state = self
            .repository
            .state()?
            .ok_or_else(|| "Sync is not configured".to_string())?;
        let secrets = SecretStore::load_vault(&state.account_id)?
            .ok_or_else(|| "Sync vault is locked".to_string())?;
        let key = secrets.vault_key_bytes()?;
        let client = SyncHttpClient::new(&state.server_url, &self.config.load()?)?;

        tracing::info!(account_id = %state.account_id, server = %state.server_url, "Sync: starting session check...");
        emit_sync_progress(
            app,
            "connecting",
            None,
            None,
            None,
            Some("Checking session..."),
        );
        let mut token = self
            .session_token(
                &client,
                &state.account_id,
                &secrets.auth_token,
                &state.device_id,
            )
            .await?;
        tracing::debug!("Sync: session authenticated successfully");

        let mut current_cursor = state.cursor;
        if let Err(error) = self
            .pull_all(app, &client, &token, &key, &mut current_cursor)
            .await
        {
            if !is_unauthorized(&error) {
                return Err(error);
            }
            self.forget_session();
            token = self
                .session_token(
                    &client,
                    &state.account_id,
                    &secrets.auth_token,
                    &state.device_id,
                )
                .await?;
            self.pull_all(app, &client, &token, &key, &mut current_cursor)
                .await?;
        }

        let mut snapshot_refreshed = false;
        for _ in 0..5 {
            let dirty_records = self.repository.detect_and_get_dirty_records()?;
            if dirty_records.is_empty() {
                tracing::debug!("Sync: no dirty local records to push");
                break;
            }
            let total_dirty = dirty_records.len();
            tracing::info!(count = total_dirty, "Sync: pushing dirty records to server");
            let mut pushed_count = 0;

            let mut had_conflict = false;
            for chunk in dirty_records.chunks(100) {
                emit_sync_progress(
                    app,
                    "pushing",
                    Some(pushed_count),
                    Some(total_dirty),
                    Some(if total_dirty > 0 {
                        pushed_count as f32 / total_dirty as f32
                    } else {
                        0.0
                    }),
                    Some("Uploading records..."),
                );

                let mut encrypted_records = Vec::with_capacity(chunk.len());
                for rec in chunk {
                    let (ciphertext, nonce) = if let Some(payload) = &rec.payload {
                        let enc = encrypt_record(&key, &rec.record_id, payload)?;
                        (enc.ciphertext, enc.nonce)
                    } else {
                        let enc = encrypt_record(&key, &rec.record_id, b"{}")?;
                        (enc.ciphertext, enc.nonce)
                    };
                    encrypted_records.push((rec, ciphertext, nonce));
                }

                let inputs: Vec<PushRecordInput<'_>> = encrypted_records
                    .iter()
                    .map(|(rec, cipher, nonce)| PushRecordInput {
                        record_id: &rec.record_id,
                        kind: &rec.kind,
                        expected_revision: rec.expected_revision,
                        device_id: &state.device_id,
                        ciphertext: cipher,
                        nonce,
                        tombstone: rec.tombstone,
                    })
                    .collect();

                let idempotency = Uuid::new_v4().to_string();
                let mut push_result = client.push(&token, &idempotency, &inputs).await;
                if let Err(error) = &push_result {
                    if is_unauthorized(error) {
                        tracing::warn!("Sync: session rejected, re-authenticating");
                        self.forget_session();
                        token = self
                            .session_token(
                                &client,
                                &state.account_id,
                                &secrets.auth_token,
                                &state.device_id,
                            )
                            .await?;
                        push_result = client.push(&token, &idempotency, &inputs).await;
                    }
                }

                match push_result {
                    Ok(push_response) => {
                        // The cursor isn't advanced from the push: changes other
                        // devices pushed since our pull sit below our position.
                        self.repository
                            .mark_records_synced(&push_response.accepted)?;
                        pushed_count += chunk.len();
                        tracing::debug!(
                            pushed = pushed_count,
                            total = total_dirty,
                            "Sync: push batch accepted"
                        );
                        emit_sync_progress(
                            app,
                            "pushing",
                            Some(pushed_count),
                            Some(total_dirty),
                            Some((pushed_count as f32 / total_dirty as f32).min(1.0)),
                            Some("Uploading records..."),
                        );
                    }
                    Err(e) if e.contains("409") || e.contains("conflict") => {
                        tracing::warn!(
                            "Sync: push conflict detected, pulling latest changes to resolve"
                        );
                        emit_sync_progress(
                            app,
                            "pulling",
                            None,
                            None,
                            None,
                            Some("Resolving conflict..."),
                        );
                        if !snapshot_refreshed {
                            snapshot_refreshed = true;
                            current_cursor = 0;
                        }
                        self.pull_all(app, &client, &token, &key, &mut current_cursor)
                            .await?;
                        had_conflict = true;
                        break;
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Sync: push request failed");
                        return Err(e);
                    }
                }
            }

            if !had_conflict {
                break;
            }
        }

        emit_sync_progress(app, "idle", None, None, None, None);
        Ok(())
    }
    pub async fn resolve_remote(
        self: &Arc<Self>,
        app: tauri::AppHandle,
    ) -> Result<SyncStatus, String> {
        self.repository.clear_conflict()?;
        self.sync(app).await
    }
    pub async fn resolve_local(
        self: &Arc<Self>,
        app: tauri::AppHandle,
    ) -> Result<SyncStatus, String> {
        self.repository.clear_conflict()?;
        self.sync(app).await
    }
}

impl RecoveryKit {
    fn device_name_invalid(&self, device_name: &str) -> bool {
        device_name.trim().is_empty() || device_name.chars().count() > 100
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avatar_payloads_must_be_small_image_data_urls() {
        assert!(valid_avatar_data_url("data:image/jpeg;base64,AAAA"));
        assert!(valid_avatar_data_url("data:image/png;base64,AAAA"));
        assert!(!valid_avatar_data_url("data:text/html;base64,AAAA"));
        assert!(!valid_avatar_data_url("https://example.com/avatar.jpg"));
        let huge = format!(
            "data:image/jpeg;base64,{}",
            "A".repeat(MAX_AVATAR_DATA_URL_LEN)
        );
        assert!(!valid_avatar_data_url(&huge));
    }

    #[test]
    fn unauthorized_errors_are_detected() {
        assert!(is_unauthorized(
            "Sync server HTTP 401 Unauthorized: {\"error\":\"invalid or expired session\"}"
        ));
        assert!(!is_unauthorized("Sync server HTTP 409 Conflict: {}"));
    }
}
