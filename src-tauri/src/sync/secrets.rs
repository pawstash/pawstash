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
        let encrypted = android_keystore::encrypt(value)?;
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
        let decrypted = android_keystore::decrypt(&raw)?;
        // If legacy unencrypted format was detected, re-encrypt on disk
        if !raw.starts_with(android_keystore::MAGIC_HEADER) {
            let _ = Self::set(name, &decrypted);
        }
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
mod android_keystore {
    use jni::objects::{JObject, JValue};
    use jni::{JNIEnv, JavaVM};

    const KEY_ALIAS: &str = "app.pawstash.keystore.vault_key_v1";
    const ANDROID_KEY_STORE: &str = "AndroidKeyStore";
    const CIPHER_TRANSFORMATION: &str = "AES/GCM/NoPadding";
    pub const MAGIC_HEADER: &[u8] = b"PWSEC1";
    const TAG_LENGTH_BITS: i32 = 128;

    fn get_vm() -> Result<JavaVM, String> {
        let ctx = ndk_context::android_context();
        let vm_ptr = ctx.vm();
        if vm_ptr.is_null() {
            return Err("Android JavaVM context is not initialized".to_string());
        }
        unsafe { JavaVM::from_raw(vm_ptr.cast()).map_err(|e| format!("Invalid JavaVM: {e}")) }
    }

    fn ensure_master_key(env: &mut JNIEnv) -> Result<(), String> {
        let ks_class = env
            .find_class("java/security/KeyStore")
            .map_err(|e| format!("Find KeyStore: {e}"))?;
        let ks_type = env
            .new_string(ANDROID_KEY_STORE)
            .map_err(|e| format!("New string KeyStore type: {e}"))?;
        let key_store = env
            .call_static_method(
                &ks_class,
                "getInstance",
                "(Ljava/lang/String;)Ljava/security/KeyStore;",
                &[JValue::Object(&ks_type)],
            )
            .map_err(|e| format!("KeyStore.getInstance: {e}"))?
            .l()
            .map_err(|e| format!("KeyStore instance cast: {e}"))?;

        env.call_method(
            &key_store,
            "load",
            "(Ljava/security/KeyStore$LoadStoreParameter;)V",
            &[JValue::Object(&JObject::null())],
        )
        .map_err(|e| format!("KeyStore.load: {e}"))?;

        let alias_jstr = env
            .new_string(KEY_ALIAS)
            .map_err(|e| format!("New string alias: {e}"))?;
        let contains_key = env
            .call_method(
                &key_store,
                "containsAlias",
                "(Ljava/lang/String;)Z",
                &[JValue::Object(&alias_jstr)],
            )
            .map_err(|e| format!("KeyStore.containsAlias: {e}"))?
            .z()
            .map_err(|e| format!("containsAlias cast: {e}"))?;

        if !contains_key {
            let kg_class = env
                .find_class("javax/crypto/KeyGenerator")
                .map_err(|e| format!("Find KeyGenerator: {e}"))?;
            let aes_str = env
                .new_string("AES")
                .map_err(|e| format!("New string AES: {e}"))?;
            let key_gen = env
                .call_static_method(
                    &kg_class,
                    "getInstance",
                    "(Ljava/lang/String;Ljava/lang/String;)Ljavax/crypto/KeyGenerator;",
                    &[JValue::Object(&aes_str), JValue::Object(&ks_type)],
                )
                .map_err(|e| format!("KeyGenerator.getInstance: {e}"))?
                .l()
                .map_err(|e| format!("KeyGenerator instance cast: {e}"))?;

            // PURPOSE_ENCRYPT = 1, PURPOSE_DECRYPT = 2 -> 3
            let builder_class = env
                .find_class("android/security/keystore/KeyGenParameterSpec$Builder")
                .map_err(|e| format!("Find KeyGenParameterSpec$Builder: {e}"))?;
            let builder = env
                .new_object(
                    &builder_class,
                    "(Ljava/lang/String;I)V",
                    &[JValue::Object(&alias_jstr), JValue::Int(1 | 2)],
                )
                .map_err(|e| format!("New KeyGenParameterSpec$Builder: {e}"))?;

            let gcm_str = env
                .new_string("GCM")
                .map_err(|e| format!("New string GCM: {e}"))?;
            let string_array = env
                .new_object_array(1, "java/lang/String", &gcm_str)
                .map_err(|e| format!("New object array GCM: {e}"))?;
            env.call_method(
                &builder,
                "setBlockModes",
                "([Ljava/lang/String;)Landroid/security/keystore/KeyGenParameterSpec$Builder;",
                &[JValue::Object(&string_array)],
            )
            .map_err(|e| format!("Builder.setBlockModes: {e}"))?;

            let nopadding_str = env
                .new_string("NoPadding")
                .map_err(|e| format!("New string NoPadding: {e}"))?;
            let padding_array = env
                .new_object_array(1, "java/lang/String", &nopadding_str)
                .map_err(|e| format!("New object array NoPadding: {e}"))?;
            env.call_method(
                &builder,
                "setEncryptionPaddings",
                "([Ljava/lang/String;)Landroid/security/keystore/KeyGenParameterSpec$Builder;",
                &[JValue::Object(&padding_array)],
            )
            .map_err(|e| format!("Builder.setEncryptionPaddings: {e}"))?;

            env.call_method(
                &builder,
                "setKeySize",
                "(I)Landroid/security/keystore/KeyGenParameterSpec$Builder;",
                &[JValue::Int(256)],
            )
            .map_err(|e| format!("Builder.setKeySize: {e}"))?;

            let spec = env
                .call_method(
                    &builder,
                    "build",
                    "()Landroid/security/keystore/KeyGenParameterSpec;",
                    &[],
                )
                .map_err(|e| format!("Builder.build: {e}"))?
                .l()
                .map_err(|e| format!("Spec cast: {e}"))?;

            env.call_method(
                &key_gen,
                "init",
                "(Ljava/security/spec/AlgorithmParameterSpec;)V",
                &[JValue::Object(&spec)],
            )
            .map_err(|e| format!("KeyGenerator.init: {e}"))?;

            env.call_method(&key_gen, "generateKey", "()Ljavax/crypto/SecretKey;", &[])
                .map_err(|e| format!("KeyGenerator.generateKey: {e}"))?;
        }

        Ok(())
    }

    fn get_secret_key<'a>(env: &mut JNIEnv<'a>) -> Result<JObject<'a>, String> {
        ensure_master_key(env)?;

        let ks_class = env
            .find_class("java/security/KeyStore")
            .map_err(|e| format!("Find KeyStore: {e}"))?;
        let ks_type = env
            .new_string(ANDROID_KEY_STORE)
            .map_err(|e| format!("New string KeyStore type: {e}"))?;
        let key_store = env
            .call_static_method(
                &ks_class,
                "getInstance",
                "(Ljava/lang/String;)Ljava/security/KeyStore;",
                &[JValue::Object(&ks_type)],
            )
            .map_err(|e| format!("KeyStore.getInstance: {e}"))?
            .l()
            .map_err(|e| format!("KeyStore instance cast: {e}"))?;

        env.call_method(
            &key_store,
            "load",
            "(Ljava/security/KeyStore$LoadStoreParameter;)V",
            &[JValue::Object(&JObject::null())],
        )
        .map_err(|e| format!("KeyStore.load: {e}"))?;

        let alias_jstr = env
            .new_string(KEY_ALIAS)
            .map_err(|e| format!("New string alias: {e}"))?;

        let key = env
            .call_method(
                &key_store,
                "getKey",
                "(Ljava/lang/String;[C)Ljava/security/Key;",
                &[
                    JValue::Object(&alias_jstr),
                    JValue::Object(&JObject::null()),
                ],
            )
            .map_err(|e| format!("KeyStore.getKey: {e}"))?
            .l()
            .map_err(|e| format!("Key cast: {e}"))?;

        Ok(key)
    }

    pub fn encrypt(plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let vm = get_vm()?;
        let mut env = vm
            .attach_current_thread()
            .map_err(|e| format!("Attach JNI thread: {e}"))?;

        let key = get_secret_key(&mut env)?;

        let cipher_class = env
            .find_class("javax/crypto/Cipher")
            .map_err(|e| format!("Find Cipher: {e}"))?;
        let trans_str = env
            .new_string(CIPHER_TRANSFORMATION)
            .map_err(|e| format!("New string Cipher transformation: {e}"))?;
        let cipher = env
            .call_static_method(
                &cipher_class,
                "getInstance",
                "(Ljava/lang/String;)Ljavax/crypto/Cipher;",
                &[JValue::Object(&trans_str)],
            )
            .map_err(|e| format!("Cipher.getInstance: {e}"))?
            .l()
            .map_err(|e| format!("Cipher instance cast: {e}"))?;

        env.call_method(
            &cipher,
            "init",
            "(ILjava/security/Key;)V",
            &[JValue::Int(1), JValue::Object(&key)],
        )
        .map_err(|e| format!("Cipher.init (ENCRYPT): {e}"))?;

        let iv_obj = env
            .call_method(&cipher, "getIV", "()[B", &[])
            .map_err(|e| format!("Cipher.getIV: {e}"))?
            .l()
            .map_err(|e| format!("IV cast: {e}"))?;
        let iv_array: &jni::objects::JByteArray = (&iv_obj).into();
        let iv_bytes = env
            .convert_byte_array(iv_array)
            .map_err(|e| format!("convert_byte_array IV: {e}"))?;

        let pt_array = env
            .byte_array_from_slice(plaintext)
            .map_err(|e| format!("byte_array_from_slice: {e}"))?;
        let ct_obj = env
            .call_method(&cipher, "doFinal", "([B)[B", &[JValue::Object(&pt_array)])
            .map_err(|e| format!("Cipher.doFinal: {e}"))?
            .l()
            .map_err(|e| format!("CT cast: {e}"))?;
        let ct_array: &jni::objects::JByteArray = (&ct_obj).into();
        let ct_bytes = env
            .convert_byte_array(ct_array)
            .map_err(|e| format!("convert_byte_array CT: {e}"))?;

        let mut envelope =
            Vec::with_capacity(MAGIC_HEADER.len() + 1 + iv_bytes.len() + ct_bytes.len());
        envelope.extend_from_slice(MAGIC_HEADER);
        envelope.push(iv_bytes.len() as u8);
        envelope.extend_from_slice(&iv_bytes);
        envelope.extend_from_slice(&ct_bytes);

        Ok(envelope)
    }

    pub fn decrypt(envelope: &[u8]) -> Result<Vec<u8>, String> {
        if !envelope.starts_with(MAGIC_HEADER) {
            return Ok(envelope.to_vec());
        }

        let header_len = MAGIC_HEADER.len();
        if envelope.len() <= header_len + 1 {
            return Err("Envelope is too short".to_string());
        }

        let iv_len = envelope[header_len] as usize;
        let iv_start = header_len + 1;
        let iv_end = iv_start + iv_len;
        if envelope.len() < iv_end {
            return Err("Invalid IV length in envelope".to_string());
        }

        let iv_bytes = &envelope[iv_start..iv_end];
        let ct_bytes = &envelope[iv_end..];

        let vm = get_vm()?;
        let mut env = vm
            .attach_current_thread()
            .map_err(|e| format!("Attach JNI thread: {e}"))?;

        let key = get_secret_key(&mut env)?;

        let gcm_spec_class = env
            .find_class("javax/crypto/spec/GCMParameterSpec")
            .map_err(|e| format!("Find GCMParameterSpec: {e}"))?;
        let iv_array = env
            .byte_array_from_slice(iv_bytes)
            .map_err(|e| format!("byte_array_from_slice IV: {e}"))?;
        let gcm_spec = env
            .new_object(
                &gcm_spec_class,
                "(I[B)V",
                &[JValue::Int(TAG_LENGTH_BITS), JValue::Object(&iv_array)],
            )
            .map_err(|e| format!("New GCMParameterSpec: {e}"))?;

        let cipher_class = env
            .find_class("javax/crypto/Cipher")
            .map_err(|e| format!("Find Cipher: {e}"))?;
        let trans_str = env
            .new_string(CIPHER_TRANSFORMATION)
            .map_err(|e| format!("New string Cipher transformation: {e}"))?;
        let cipher = env
            .call_static_method(
                &cipher_class,
                "getInstance",
                "(Ljava/lang/String;)Ljavax/crypto/Cipher;",
                &[JValue::Object(&trans_str)],
            )
            .map_err(|e| format!("Cipher.getInstance: {e}"))?
            .l()
            .map_err(|e| format!("Cipher instance cast: {e}"))?;

        env.call_method(
            &cipher,
            "init",
            "(ILjava/security/Key;Ljava/security/spec/AlgorithmParameterSpec;)V",
            &[
                JValue::Int(2),
                JValue::Object(&key),
                JValue::Object(&gcm_spec),
            ],
        )
        .map_err(|e| format!("Cipher.init (DECRYPT): {e}"))?;

        let ct_array = env
            .byte_array_from_slice(ct_bytes)
            .map_err(|e| format!("byte_array_from_slice CT: {e}"))?;
        let pt_obj = env
            .call_method(&cipher, "doFinal", "([B)[B", &[JValue::Object(&ct_array)])
            .map_err(|e| format!("Cipher.doFinal: {e}"))?
            .l()
            .map_err(|e| format!("Plaintext cast: {e}"))?;
        let pt_array: &jni::objects::JByteArray = (&pt_obj).into();
        let pt_bytes = env
            .convert_byte_array(pt_array)
            .map_err(|e| format!("convert_byte_array PT: {e}"))?;

        Ok(pt_bytes)
    }
}
