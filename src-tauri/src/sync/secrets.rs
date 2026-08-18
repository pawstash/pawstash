use super::crypto::VaultSecrets;

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
const SERVICE: &str = "app.pawstash.client";

pub struct SecretStore;

impl SecretStore {
    pub fn save_named(name: &str, value: &[u8]) -> Result<(), String> {
        Self::set(name, value)
    }

    pub fn load_named(name: &str) -> Result<Option<Vec<u8>>, String> {
        Self::get(name)
    }

    pub fn delete_named(name: &str) -> Result<(), String> {
        Self::delete(name)
    }

    pub fn save_vault(account_id: &str, secrets: &VaultSecrets) -> Result<(), String> {
        Self::set(
            &format!("sync-vault:{account_id}"),
            &serde_json::to_vec(secrets).map_err(|e| e.to_string())?,
        )
    }

    pub fn load_vault(account_id: &str) -> Result<Option<VaultSecrets>, String> {
        Self::get(&format!("sync-vault:{account_id}"))?
            .map(|value| serde_json::from_slice(&value).map_err(|e| e.to_string()))
            .transpose()
    }

    pub fn delete_vault(account_id: &str) -> Result<(), String> {
        Self::delete(&format!("sync-vault:{account_id}"))
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    fn set(name: &str, value: &[u8]) -> Result<(), String> {
        keyring::Entry::new(SERVICE, name)
            .map_err(|e| e.to_string())?
            .set_secret(value)
            .map_err(|e| e.to_string())
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    fn get(name: &str) -> Result<Option<Vec<u8>>, String> {
        let entry = keyring::Entry::new(SERVICE, name).map_err(|e| e.to_string())?;
        match entry.get_secret() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    fn delete(name: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(SERVICE, name).map_err(|e| e.to_string())?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }

    #[cfg(target_os = "android")]
    fn secret_path(name: &str) -> std::path::PathBuf {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        crate::db::storage::data_root().join(".vault").join(hash)
    }

    #[cfg(target_os = "android")]
    fn set(name: &str, value: &[u8]) -> Result<(), String> {
        let encrypted = android_vault::encrypt(value)?;
        let path = Self::secret_path(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, encrypted).map_err(|e| e.to_string())
    }

    #[cfg(target_os = "android")]
    fn get(name: &str) -> Result<Option<Vec<u8>>, String> {
        let path = Self::secret_path(name);
        if !path.exists() {
            return Ok(None);
        }
        let raw = std::fs::read(&path).map_err(|e| e.to_string())?;
        let decrypted = android_vault::decrypt(&raw)?;
        Ok(Some(decrypted))
    }

    #[cfg(target_os = "android")]
    fn delete(name: &str) -> Result<(), String> {
        let path = Self::secret_path(name);
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
        Ok(())
    }

    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "android"
    )))]
    fn set(_name: &str, _value: &[u8]) -> Result<(), String> {
        Err("Secure storage is not implemented on this platform".to_string())
    }

    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "android"
    )))]
    fn get(_name: &str) -> Result<Option<Vec<u8>>, String> {
        Ok(None)
    }

    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "android"
    )))]
    fn delete(_name: &str) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(target_os = "android")]
mod android_vault {
    use chacha20poly1305::{
        aead::{Aead, Payload},
        KeyInit, XChaCha20Poly1305, XNonce,
    };
    use rand::{rngs::OsRng, RngCore};
    use std::path::PathBuf;

    pub const MAGIC_HEADER: &[u8] = b"PWSEC2";
    const KEY_FILE: &str = ".device_key";

    fn vault_dir() -> PathBuf {
        crate::db::storage::data_root().join(".vault")
    }

    fn get_or_create_device_key() -> Result<[u8; 32], String> {
        let dir = vault_dir();
        if !dir.exists() {
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        }
        let key_path = dir.join(KEY_FILE);
        if key_path.exists() {
            let bytes = std::fs::read(&key_path).map_err(|e| e.to_string())?;
            if bytes.len() == 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                return Ok(key);
            }
        }
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        std::fs::write(&key_path, key).map_err(|e| e.to_string())?;
        Ok(key)
    }

    pub fn encrypt(plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let key = get_or_create_device_key()?;
        let mut nonce = [0u8; 24];
        OsRng.fill_bytes(&mut nonce);
        let cipher = XChaCha20Poly1305::new((&key).into());
        let encrypted = cipher
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: plaintext,
                    aad: b"pawstash:android:vault:v2",
                },
            )
            .map_err(|e| format!("Encryption error: {e}"))?;

        let mut envelope = Vec::with_capacity(MAGIC_HEADER.len() + 24 + encrypted.len());
        envelope.extend_from_slice(MAGIC_HEADER);
        envelope.extend_from_slice(&nonce);
        envelope.extend_from_slice(&encrypted);
        Ok(envelope)
    }

    pub fn decrypt(envelope: &[u8]) -> Result<Vec<u8>, String> {
        if envelope.starts_with(MAGIC_HEADER) {
            let key = get_or_create_device_key()?;
            let header_len = MAGIC_HEADER.len();
            if envelope.len() < header_len + 24 {
                return Err("Corrupted vault entry (too short)".to_string());
            }
            let nonce = &envelope[header_len..header_len + 24];
            let ciphertext = &envelope[header_len + 24..];
            let cipher = XChaCha20Poly1305::new((&key).into());
            cipher
                .decrypt(
                    XNonce::from_slice(nonce),
                    Payload {
                        msg: ciphertext,
                        aad: b"pawstash:android:vault:v2",
                    },
                )
                .map_err(|e| format!("Decryption error: {e}"))
        } else {
            // Unencrypted legacy fallback
            Ok(envelope.to_vec())
        }
    }
}
