use crate::config::settings::{AppSettings, ProxyMode};
use reqwest::{Client, StatusCode, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize)]
pub struct CreateAccountRequest<'a> {
    pub account_id: &'a str,
    pub auth_token: &'a str,
    pub kdf: &'a serde_json::Value,
    pub encrypted_key_bundle: &'a str,
    pub nonce: &'a str,
}
#[derive(Debug, Deserialize)]
pub struct BundleResponse {
    pub account_id: String,
    pub kdf: serde_json::Value,
    pub encrypted_key_bundle: String,
    pub nonce: String,
}
#[derive(Serialize)]
pub struct ReplaceBundleRequest<'a> {
    pub kdf: &'a serde_json::Value,
    pub encrypted_key_bundle: &'a str,
    pub nonce: &'a str,
}
#[derive(Debug, Deserialize)]
pub struct SessionResponse {
    pub token: String,
    pub expires_at: String,
    pub protocol_version: u32,
}
#[derive(Serialize)]
struct SessionRequest<'a> {
    account_id: &'a str,
    auth_token: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_id: Option<&'a str>,
}
#[derive(Serialize)]
struct DeviceRequest<'a> {
    id: &'a str,
    name: &'a str,
    platform: &'a str,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyncDevice {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub created_at: String,
    pub revoked_at: Option<String>,
}
#[derive(Debug, Deserialize)]
struct DevicesResponse {
    devices: Vec<SyncDevice>,
}
#[derive(Debug, Deserialize)]
pub struct PullResponse {
    pub cursor: i64,
    pub changes: Vec<RemoteChange>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct RemoteChange {
    pub position: i64,
    pub record_id: String,
    pub revision: i64,
    pub kind: String,
    pub device_id: String,
    pub ciphertext: String,
    pub nonce: String,
    pub tombstone: bool,
    pub created_at: String,
}
#[derive(Serialize)]
struct PushRequest<'a> {
    protocol_version: u32,
    records: Vec<PushRecord<'a>>,
}
#[derive(Serialize)]
struct PushRecord<'a> {
    record_id: &'a str,
    kind: &'a str,
    expected_revision: i64,
    device_id: &'a str,
    ciphertext: &'a str,
    nonce: &'a str,
    tombstone: bool,
}
#[derive(Clone, Debug)]
pub struct PushRecordInput<'a> {
    pub record_id: &'a str,
    pub kind: &'a str,
    pub expected_revision: i64,
    pub device_id: &'a str,
    pub ciphertext: &'a str,
    pub nonce: &'a str,
    pub tombstone: bool,
}
#[derive(Debug, Deserialize)]
pub struct PushResponse {
    pub cursor: i64,
    pub accepted: Vec<AcceptedRecord>,
}
#[derive(Debug, Deserialize)]
pub struct AcceptedRecord {
    pub record_id: String,
    pub revision: i64,
    pub position: i64,
}

pub struct SyncHttpClient {
    client: Client,
    base: String,
}
impl SyncHttpClient {
    pub fn new(server_url: &str, settings: &AppSettings) -> Result<Self, String> {
        let url = Url::parse(server_url.trim()).map_err(|e| e.to_string())?;
        let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
        if url.scheme() != "https" && !local {
            return Err("Sync server requires HTTPS".to_string());
        }
        let mut builder = Client::builder().timeout(Duration::from_secs(45));
        match settings.proxy_mode {
            ProxyMode::None => builder = builder.no_proxy(),
            ProxyMode::System => {}
            ProxyMode::Custom => {
                if !settings.proxy_url.trim().is_empty() {
                    let mut proxy = reqwest::Proxy::all(settings.proxy_url.trim())
                        .map_err(|e| e.to_string())?;
                    if !settings.proxy_username.is_empty() {
                        proxy = proxy.basic_auth(&settings.proxy_username, &settings.proxy_password)
                    }
                    builder = builder.proxy(proxy)
                }
            }
        }
        let base = format!("{}/v1", server_url.trim().trim_end_matches('/'));
        Ok(Self {
            client: builder.build().map_err(|e| e.to_string())?,
            base,
        })
    }
    pub async fn create_account(&self, value: &CreateAccountRequest<'_>) -> Result<(), String> {
        self.send_empty(
            self.client
                .post(format!("{}/accounts", self.base))
                .json(value),
            StatusCode::CREATED,
        )
        .await
    }
    pub async fn bundle(&self, account: &str) -> Result<BundleResponse, String> {
        self.send_json(
            self.client.get(format!(
                "{}/accounts/{}/bundle",
                self.base,
                urlencoding::encode(account)
            )),
            StatusCode::OK,
        )
        .await
    }
    pub async fn replace_bundle(
        &self,
        token: &str,
        account: &str,
        value: &ReplaceBundleRequest<'_>,
    ) -> Result<(), String> {
        self.send_empty(
            self.client
                .put(format!(
                    "{}/accounts/{}/bundle",
                    self.base,
                    urlencoding::encode(account)
                ))
                .bearer_auth(token)
                .json(value),
            StatusCode::NO_CONTENT,
        )
        .await
    }
    pub async fn session(
        &self,
        account: &str,
        auth_token: &str,
        device_id: Option<&str>,
    ) -> Result<SessionResponse, String> {
        self.send_json(
            self.client
                .post(format!("{}/sessions", self.base))
                .json(&SessionRequest {
                    account_id: account,
                    auth_token,
                    device_id,
                }),
            StatusCode::CREATED,
        )
        .await
    }
    pub async fn register_device(&self, token: &str, id: &str, name: &str) -> Result<(), String> {
        self.send_empty(
            self.client
                .post(format!("{}/devices", self.base))
                .bearer_auth(token)
                .json(&DeviceRequest {
                    id,
                    name,
                    platform: std::env::consts::OS,
                }),
            StatusCode::CREATED,
        )
        .await
    }
    pub async fn list_devices(&self, token: &str) -> Result<Vec<SyncDevice>, String> {
        self.send_json::<DevicesResponse>(
            self.client
                .get(format!("{}/devices", self.base))
                .bearer_auth(token),
            StatusCode::OK,
        )
        .await
        .map(|response| response.devices)
    }
    pub async fn revoke_device(&self, token: &str, device_id: &str) -> Result<(), String> {
        self.send_empty(
            self.client
                .delete(format!(
                    "{}/devices/{}",
                    self.base,
                    urlencoding::encode(device_id)
                ))
                .bearer_auth(token),
            StatusCode::NO_CONTENT,
        )
        .await
    }
    pub async fn pull(&self, token: &str, after: i64) -> Result<PullResponse, String> {
        self.send_json(
            self.client
                .get(format!("{}/changes", self.base))
                .bearer_auth(token)
                .query(&[("after", after), ("limit", 500)]),
            StatusCode::OK,
        )
        .await
    }
    pub async fn push(
        &self,
        token: &str,
        idempotency: &str,
        records: &[PushRecordInput<'_>],
    ) -> Result<PushResponse, String> {
        let body = PushRequest {
            protocol_version: 1,
            records: records
                .iter()
                .map(|record| PushRecord {
                    record_id: record.record_id,
                    kind: record.kind,
                    expected_revision: record.expected_revision,
                    device_id: record.device_id,
                    ciphertext: record.ciphertext,
                    nonce: record.nonce,
                    tombstone: record.tombstone,
                })
                .collect(),
        };
        self.send_json(
            self.client
                .post(format!("{}/records:push", self.base))
                .bearer_auth(token)
                .header("Idempotency-Key", idempotency)
                .json(&body),
            StatusCode::OK,
        )
        .await
    }
    async fn send_empty(
        &self,
        builder: reqwest::RequestBuilder,
        expected: StatusCode,
    ) -> Result<(), String> {
        let response = builder.send().await.map_err(|e| e.to_string())?;
        if response.status() != expected {
            return Err(Self::error(response).await);
        }
        Ok(())
    }
    async fn send_json<T: DeserializeOwned>(
        &self,
        builder: reqwest::RequestBuilder,
        expected: StatusCode,
    ) -> Result<T, String> {
        let response = builder.send().await.map_err(|e| e.to_string())?;
        if response.status() != expected {
            return Err(Self::error(response).await);
        }
        response.json().await.map_err(|e| e.to_string())
    }
    async fn error(response: reqwest::Response) -> String {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        format!("Sync server HTTP {status}: {}", body.trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::crypto::{
        decrypt_record, encrypt_record, unwrap_vault, wrap_vault, KdfEnvelope, VaultSecrets,
    };
    use uuid::Uuid;

    #[tokio::test]
    #[ignore = "requires a local pawstash sync-server in PAWSTASH_TEST_SYNC_URL"]
    async fn two_devices_exchange_only_encrypted_records() {
        let server = std::env::var("PAWSTASH_TEST_SYNC_URL").unwrap();
        let client = SyncHttpClient::new(&server, &AppSettings::default()).unwrap();
        let account = format!("account_{}", Uuid::new_v4().simple());
        let password = "integration master password";
        let kdf = KdfEnvelope {
            memory_kib: 1024,
            iterations: 1,
            ..KdfEnvelope::generate()
        };
        let secrets = VaultSecrets::generate();
        let wrapped = wrap_vault(password, &kdf, &secrets).unwrap();
        let kdf_value = serde_json::to_value(&kdf).unwrap();
        client
            .create_account(&CreateAccountRequest {
                account_id: &account,
                auth_token: &secrets.auth_token,
                kdf: &kdf_value,
                encrypted_key_bundle: &wrapped.ciphertext,
                nonce: &wrapped.nonce,
            })
            .await
            .unwrap();
        let bootstrap = client
            .session(&account, &secrets.auth_token, None)
            .await
            .unwrap();
        let first_device = format!("device_{}", Uuid::new_v4().simple());
        client
            .register_device(&bootstrap.token, &first_device, "Device A")
            .await
            .unwrap();
        let first_session = client
            .session(&account, &secrets.auth_token, Some(&first_device))
            .await
            .unwrap();
        let record_id = format!("record_{}", Uuid::new_v4().simple());
        let plaintext = br#"{"schema_version":1,"collections":[],"entries":[],"memberships":[],"subscriptions":[]}"#;
        let encrypted =
            encrypt_record(&secrets.vault_key_bytes().unwrap(), &record_id, plaintext).unwrap();
        client
            .push(
                &first_session.token,
                &format!("request_{}", Uuid::new_v4().simple()),
                &[PushRecordInput {
                    record_id: &record_id,
                    kind: "collection",
                    expected_revision: 0,
                    device_id: &first_device,
                    ciphertext: &encrypted.ciphertext,
                    nonce: &encrypted.nonce,
                    tombstone: false,
                }],
            )
            .await
            .unwrap();

        let remote_bundle = client.bundle(&account).await.unwrap();
        let second_secrets = unwrap_vault(
            password,
            &serde_json::from_value(remote_bundle.kdf).unwrap(),
            &remote_bundle.encrypted_key_bundle,
            &remote_bundle.nonce,
        )
        .unwrap();
        let second_bootstrap = client
            .session(&account, &second_secrets.auth_token, None)
            .await
            .unwrap();
        let second_device = format!("device_{}", Uuid::new_v4().simple());
        client
            .register_device(&second_bootstrap.token, &second_device, "Device B")
            .await
            .unwrap();
        let second_session = client
            .session(&account, &second_secrets.auth_token, Some(&second_device))
            .await
            .unwrap();
        let pulled = client.pull(&second_session.token, 0).await.unwrap();
        assert_eq!(pulled.changes.len(), 1);
        let change = &pulled.changes[0];
        let opened = decrypt_record(
            &second_secrets.vault_key_bytes().unwrap(),
            &change.record_id,
            &change.ciphertext,
            &change.nonce,
        )
        .unwrap();
        assert_eq!(opened, plaintext);

        let devices = client.list_devices(&second_session.token).await.unwrap();
        assert_eq!(
            devices
                .iter()
                .filter(|device| device.revoked_at.is_none())
                .count(),
            2
        );

        let new_password = "replacement integration password";
        let new_kdf = KdfEnvelope {
            memory_kib: 1024,
            iterations: 1,
            ..KdfEnvelope::generate()
        };
        let replacement = wrap_vault(new_password, &new_kdf, &second_secrets).unwrap();
        let new_kdf_value = serde_json::to_value(&new_kdf).unwrap();
        client
            .replace_bundle(
                &second_session.token,
                &account,
                &ReplaceBundleRequest {
                    kdf: &new_kdf_value,
                    encrypted_key_bundle: &replacement.ciphertext,
                    nonce: &replacement.nonce,
                },
            )
            .await
            .unwrap();
        let replaced = client.bundle(&account).await.unwrap();
        let replaced_kdf: KdfEnvelope = serde_json::from_value(replaced.kdf).unwrap();
        assert!(unwrap_vault(
            password,
            &replaced_kdf,
            &replaced.encrypted_key_bundle,
            &replaced.nonce
        )
        .is_err());
        unwrap_vault(
            new_password,
            &replaced_kdf,
            &replaced.encrypted_key_bundle,
            &replaced.nonce,
        )
        .unwrap();

        client
            .revoke_device(&second_session.token, &first_device)
            .await
            .unwrap();
        assert!(client
            .session(&account, &secrets.auth_token, Some(&first_device))
            .await
            .is_err());
    }
}
