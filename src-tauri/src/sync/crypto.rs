use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chacha20poly1305::{
    aead::{Aead, Payload},
    KeyInit, XChaCha20Poly1305, XNonce,
};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

const KEY_BYTES: usize = 32;
const NONCE_BYTES: usize = 24;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdfEnvelope {
    pub algorithm: String,
    pub version: u32,
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
    pub salt: String,
}

impl KdfEnvelope {
    pub fn generate() -> Self {
        Self {
            algorithm: "argon2id".to_string(),
            version: 19,
            memory_kib: 64 * 1024,
            iterations: 3,
            parallelism: 1,
            salt: encode(&random::<16>()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSecrets {
    pub vault_key: String,
    pub auth_token: String,
}

impl VaultSecrets {
    pub fn generate() -> Self {
        Self {
            vault_key: encode(&random::<KEY_BYTES>()),
            auth_token: encode(&random::<KEY_BYTES>()),
        }
    }

    pub fn vault_key_bytes(&self) -> Result<[u8; KEY_BYTES], String> {
        decode_array(&self.vault_key)
    }
}

#[derive(Debug, Clone)]
pub struct Ciphertext {
    pub ciphertext: String,
    pub nonce: String,
}

pub fn wrap_vault(
    password: &str,
    kdf: &KdfEnvelope,
    secrets: &VaultSecrets,
) -> Result<Ciphertext, String> {
    if password.chars().count() < 10 {
        return Err("Master password must contain at least 10 characters".to_string());
    }
    let key = derive_key(password, kdf)?;
    encrypt(
        &key,
        &serde_json::to_vec(secrets).map_err(|e| e.to_string())?,
        b"pawstash:v1:key-bundle",
    )
}

pub fn unwrap_vault(
    password: &str,
    kdf: &KdfEnvelope,
    ciphertext: &str,
    nonce: &str,
) -> Result<VaultSecrets, String> {
    let key = derive_key(password, kdf)?;
    let plaintext = decrypt(&key, ciphertext, nonce, b"pawstash:v1:key-bundle")
        .map_err(|_| "Wrong master password or damaged key bundle".to_string())?;
    serde_json::from_slice(&plaintext).map_err(|_| "Invalid key bundle".to_string())
}

const PAYLOAD_MARKER_RAW: u8 = 0x00;
const PAYLOAD_MARKER_ZLIB: u8 = 0x01;

pub fn compress_payload(plaintext: &[u8]) -> Vec<u8> {
    if plaintext.is_empty() {
        return vec![PAYLOAD_MARKER_RAW];
    }
    let mut encoder = ZlibEncoder::new(Vec::with_capacity(plaintext.len()), Compression::default());
    if encoder.write_all(plaintext).is_ok() {
        if let Ok(compressed) = encoder.finish() {
            if compressed.len() + 1 < plaintext.len() {
                let mut out = Vec::with_capacity(compressed.len() + 1);
                out.push(PAYLOAD_MARKER_ZLIB);
                out.extend_from_slice(&compressed);
                return out;
            }
        }
    }
    let mut out = Vec::with_capacity(plaintext.len() + 1);
    out.push(PAYLOAD_MARKER_RAW);
    out.extend_from_slice(plaintext);
    out
}

pub fn decompress_payload(decrypted: &[u8]) -> Result<Vec<u8>, String> {
    if decrypted.is_empty() {
        return Ok(Vec::new());
    }
    match decrypted[0] {
        PAYLOAD_MARKER_ZLIB => {
            let mut decoder = ZlibDecoder::new(&decrypted[1..]);
            let mut decompressed = Vec::new();
            decoder
                .read_to_end(&mut decompressed)
                .map_err(|e| format!("Failed to decompress zlib sync payload: {e}"))?;
            Ok(decompressed)
        }
        PAYLOAD_MARKER_RAW => Ok(decrypted[1..].to_vec()),
        _ => {
            // Legacy uncompressed record (starts with '{' 0x7B or other un-prefixed byte)
            Ok(decrypted.to_vec())
        }
    }
}

pub fn encrypt_record(
    key: &[u8; KEY_BYTES],
    record_id: &str,
    plaintext: &[u8],
) -> Result<Ciphertext, String> {
    let payload = compress_payload(plaintext);
    encrypt(
        key,
        &payload,
        format!("pawstash:v1:vault_snapshot:{record_id}").as_bytes(),
    )
}

pub fn decrypt_record(
    key: &[u8; KEY_BYTES],
    record_id: &str,
    ciphertext: &str,
    nonce: &str,
) -> Result<Vec<u8>, String> {
    let decrypted = decrypt(
        key,
        ciphertext,
        nonce,
        format!("pawstash:v1:vault_snapshot:{record_id}").as_bytes(),
    )
    .map_err(|_| "Encrypted sync record failed authentication".to_string())?;
    decompress_payload(&decrypted)
}

fn derive_key(password: &str, envelope: &KdfEnvelope) -> Result<[u8; KEY_BYTES], String> {
    if envelope.algorithm != "argon2id" || envelope.version != 19 {
        return Err("Unsupported key derivation parameters".to_string());
    }
    if !(1024..=128 * 1024).contains(&envelope.memory_kib)
        || !(1..=10).contains(&envelope.iterations)
        || !(1..=8).contains(&envelope.parallelism)
    {
        return Err("Key derivation parameters outside safe bounds".to_string());
    }
    let params = Params::new(
        envelope.memory_kib,
        envelope.iterations,
        envelope.parallelism,
        Some(KEY_BYTES),
    )
    .map_err(|e| e.to_string())?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let salt = decode(&envelope.salt)?;
    if salt.len() < 16 {
        return Err("KDF salt is too short".to_string());
    }
    let mut output = [0u8; KEY_BYTES];
    argon
        .hash_password_into(password.as_bytes(), &salt, &mut output)
        .map_err(|e| e.to_string())?;
    Ok(output)
}

fn encrypt(key: &[u8; KEY_BYTES], plaintext: &[u8], aad: &[u8]) -> Result<Ciphertext, String> {
    let nonce = random::<NONCE_BYTES>();
    let cipher = XChaCha20Poly1305::new(key.into());
    let encrypted = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| "Encryption failed".to_string())?;
    Ok(Ciphertext {
        ciphertext: encode(&encrypted),
        nonce: encode(&nonce),
    })
}

fn decrypt(
    key: &[u8; KEY_BYTES],
    ciphertext: &str,
    nonce: &str,
    aad: &[u8],
) -> Result<Vec<u8>, String> {
    let nonce: [u8; NONCE_BYTES] = decode_array(nonce)?;
    let ciphertext = decode(ciphertext)?;
    XChaCha20Poly1305::new(key.into())
        .decrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &ciphertext,
                aad,
            },
        )
        .map_err(|_| "Decryption failed".to_string())
}

fn random<const N: usize>() -> [u8; N] {
    let mut value = [0u8; N];
    OsRng.fill_bytes(&mut value);
    value
}

fn encode(value: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(value)
}
fn decode(value: &str) -> Result<Vec<u8>, String> {
    URL_SAFE_NO_PAD.decode(value).map_err(|e| e.to_string())
}
fn decode_array<const N: usize>(value: &str) -> Result<[u8; N], String> {
    decode(value)?
        .try_into()
        .map_err(|_| format!("Expected {N} bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_and_records_round_trip_and_authenticate_aad() {
        let kdf = KdfEnvelope {
            memory_kib: 1024,
            iterations: 1,
            ..KdfEnvelope::generate()
        };
        let secrets = VaultSecrets::generate();
        let wrapped = wrap_vault("correct horse battery", &kdf, &secrets).unwrap();
        let opened = unwrap_vault(
            "correct horse battery",
            &kdf,
            &wrapped.ciphertext,
            &wrapped.nonce,
        )
        .unwrap();
        assert_eq!(opened.vault_key, secrets.vault_key);
        assert!(unwrap_vault(
            "wrong password here",
            &kdf,
            &wrapped.ciphertext,
            &wrapped.nonce
        )
        .is_err());
        let key = opened.vault_key_bytes().unwrap();
        let record = encrypt_record(&key, "record-1", b"snapshot").unwrap();
        assert_eq!(
            decrypt_record(&key, "record-1", &record.ciphertext, &record.nonce).unwrap(),
            b"snapshot"
        );
        assert!(decrypt_record(&key, "record-2", &record.ciphertext, &record.nonce).is_err());
    }

    #[test]
    fn kdf_rejects_out_of_bounds_parameters() {
        let bad_kdf = KdfEnvelope {
            memory_kib: 1024 * 1024, // 1 GB
            ..KdfEnvelope::generate()
        };
        let secrets = VaultSecrets::generate();
        assert!(wrap_vault("correct horse battery", &bad_kdf, &secrets).is_err());
    }

    #[test]
    fn compressed_payload_roundtrip_and_legacy_compatibility() {
        let key = [42u8; KEY_BYTES];
        let record_id = "test:post:123";

        let sample_json = r#"{"id":"123","service":"kemono","title":"Test Post With Long Text","content":"repeated content text ".repeat(100)}"#.as_bytes();
        let enc = encrypt_record(&key, record_id, sample_json).unwrap();
        let dec = decrypt_record(&key, record_id, &enc.ciphertext, &enc.nonce).unwrap();
        assert_eq!(dec, sample_json);

        let legacy_json = b"{\"legacy\":true,\"name\":\"uncompressed\"}";
        let legacy_enc = encrypt(
            &key,
            legacy_json,
            format!("pawstash:v1:vault_snapshot:{record_id}").as_bytes(),
        )
        .unwrap();
        let legacy_dec =
            decrypt_record(&key, record_id, &legacy_enc.ciphertext, &legacy_enc.nonce).unwrap();
        assert_eq!(legacy_dec, legacy_json);
    }
}
