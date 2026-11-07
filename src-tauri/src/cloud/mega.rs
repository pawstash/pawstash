use super::models::{CloudFolderResult, CloudNode};
use aes::cipher::{BlockDecrypt, KeyInit};
use aes::Aes128;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use cbc::cipher::{block_padding::NoPadding, BlockDecryptMut, KeyIvInit};
use reqwest::Client;

pub(crate) fn api_url(folder_id: Option<&str>) -> String {
    folder_id
        .map(|id| format!("https://g.api.mega.co.nz/cs?n={id}"))
        .unwrap_or_else(|| "https://g.api.mega.co.nz/cs".to_string())
}
use serde::Deserialize;
use serde_json::{json, Value};

type Aes128CbcDec = cbc::Decryptor<Aes128>;

pub const WRONG_KEY: &str = "wrong_key";

#[derive(Debug)]
pub enum MegaLink {
    Folder { id: String, key: String },
    File { id: String, key: String },
}

pub fn supports_url(url: &str) -> bool {
    super::url_host_matches(url, &["mega.nz", "mega.co.nz"])
}

pub fn parse_mega_url(url_str: &str) -> Option<MegaLink> {
    if !supports_url(url_str) {
        return None;
    }
    let url = reqwest::Url::parse(url_str).ok()?;

    let fragment = url.fragment().unwrap_or("");
    let path = url.path().trim_matches('/');
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if segments.first() == Some(&"folder") {
        let id = segments.get(1)?.to_string();
        let key = fragment.trim().to_string();
        if !key.is_empty() {
            return Some(MegaLink::Folder { id, key });
        }
    }

    if segments.first() == Some(&"file") {
        let id = segments.get(1)?.to_string();
        let key = fragment.trim().to_string();
        if !key.is_empty() {
            return Some(MegaLink::File { id, key });
        }
    }

    if fragment.starts_with("F!") {
        let parts: Vec<&str> = fragment.split('!').collect();
        if parts.len() >= 3 {
            return Some(MegaLink::Folder {
                id: parts[1].to_string(),
                key: parts[2].to_string(),
            });
        }
    }

    if fragment.starts_with('!') {
        let parts: Vec<&str> = fragment.split('!').collect();
        if parts.len() >= 3 {
            return Some(MegaLink::File {
                id: parts[1].to_string(),
                key: parts[2].to_string(),
            });
        }
    }

    None
}

pub fn is_protected_link(url: &str) -> bool {
    supports_url(url) && url.contains("#P!")
}

pub fn is_keyless_link(url: &str) -> bool {
    supports_url(url)
        && !is_protected_link(url)
        && parse_mega_url(url).is_none()
        && (url.contains("/folder/") || url.contains("/file/"))
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProtectedLinkError {
    Malformed,
    Unsupported,
    WrongPassword,
}

const PROTECTED_SALT_LEN: usize = 32;
const PROTECTED_MAC_LEN: usize = 32;

/// Mega's password-protected `#P!` links, per `exportPassword` in meganz/webclient `js/ui/export.js`.
pub fn decrypt_protected_link(url: &str, password: &str) -> Result<String, ProtectedLinkError> {
    use hmac::{Hmac, Mac};
    use sha2::{Sha256, Sha512};

    let payload = url
        .split_once("#P!")
        .map(|(_, rest)| rest.trim_end_matches('='))
        .ok_or(ProtectedLinkError::Malformed)?;
    let data = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| ProtectedLinkError::Malformed)?;
    if data.len() < 2 {
        return Err(ProtectedLinkError::Malformed);
    }
    let algorithm = data[0];
    let rounds = match algorithm {
        0 => 1_000,
        1 | 2 => 100_000,
        _ => return Err(ProtectedLinkError::Unsupported),
    };
    let (kind, key_len) = match data[1] {
        0 => ("folder", 16),
        1 => ("file", 32),
        _ => return Err(ProtectedLinkError::Unsupported),
    };
    let key_start = 2 + 6 + PROTECTED_SALT_LEN;
    let mac_start = key_start + key_len;
    if data.len() != mac_start + PROTECTED_MAC_LEN {
        return Err(ProtectedLinkError::Malformed);
    }

    let mut derived = [0u8; 64];
    pbkdf2::pbkdf2_hmac::<Sha512>(
        password.trim().as_bytes(),
        &data[8..key_start],
        rounds,
        &mut derived,
    );
    let (authenticated, tag) = data.split_at(mac_start);
    let mac_key = &derived[32..];
    // Algorithm 1 shipped with the HMAC key and message swapped; Mega still accepts those links.
    let (hmac_key, message) = if algorithm == 1 {
        (authenticated, mac_key)
    } else {
        (mac_key, authenticated)
    };
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(hmac_key)
        .map_err(|_| ProtectedLinkError::Malformed)?;
    mac.update(message);
    mac.verify_slice(tag)
        .map_err(|_| ProtectedLinkError::WrongPassword)?;

    let key: Vec<u8> = data[key_start..mac_start]
        .iter()
        .zip(&derived[..key_len])
        .map(|(a, b)| a ^ b)
        .collect();
    Ok(format!(
        "https://mega.nz/{kind}/{}#{}",
        URL_SAFE_NO_PAD.encode(&data[2..8]),
        URL_SAFE_NO_PAD.encode(key)
    ))
}

fn mega_base64_decode(raw: &str) -> Result<Vec<u8>, String> {
    let mut clean = raw.trim().replace('-', "+").replace('_', "/");
    while !clean.len().is_multiple_of(4) {
        clean.push('=');
    }
    base64::engine::general_purpose::STANDARD
        .decode(&clean)
        .or_else(|_| URL_SAFE_NO_PAD.decode(raw))
        .map_err(|e| format!("Base64 decode failed: {e}"))
}

fn decrypt_aes_ecb(key: &[u8; 16], mut data: Vec<u8>) -> Result<Vec<u8>, String> {
    if !data.len().is_multiple_of(16) {
        return Err("Data length must be multiple of 16".into());
    }
    let cipher = Aes128::new(key.into());
    for chunk in data.chunks_mut(16) {
        let block = aes::cipher::generic_array::GenericArray::from_mut_slice(chunk);
        cipher.decrypt_block(block);
    }
    Ok(data)
}

fn decrypt_aes_cbc_zeros(key: &[u8; 16], mut data: Vec<u8>) -> Result<Vec<u8>, String> {
    if !data.len().is_multiple_of(16) {
        return Err("Data length must be multiple of 16".into());
    }
    let iv = [0u8; 16];
    let cipher = Aes128CbcDec::new(key.into(), &iv.into());
    cipher
        .decrypt_padded_mut::<NoPadding>(&mut data)
        .map_err(|e| format!("AES decryption error: {e:?}"))?;
    Ok(data)
}

fn parse_attributes(decrypted: &[u8]) -> Option<String> {
    if !decrypted.starts_with(b"MEGA") {
        return None;
    }
    let json_bytes = &decrypted[4..];
    let last_brace = json_bytes.iter().rposition(|&b| b == b'}')?;
    let json_str = std::str::from_utf8(&json_bytes[..=last_brace]).ok()?;
    let parsed: Value = serde_json::from_str(json_str).ok()?;
    parsed.get("n")?.as_str().map(str::to_string)
}

#[derive(Debug, Deserialize)]
struct MegaFolderNode {
    h: String,
    #[serde(default)]
    p: Option<String>,
    #[serde(default)]
    t: i32,
    #[serde(default)]
    a: Option<String>,
    #[serde(default)]
    k: Option<String>,
    #[serde(default)]
    s: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct MegaFolderApiResponse {
    #[serde(default)]
    f: Option<Vec<MegaFolderNode>>,
}

pub async fn resolve_mega(client: &Client, url_str: &str) -> Result<CloudFolderResult, String> {
    let link = parse_mega_url(url_str).ok_or("Invalid or unsupported MEGA URL format")?;

    match link {
        MegaLink::Folder { id, key } => {
            let master_key_bytes = mega_base64_decode(&key)?;
            if master_key_bytes.len() < 16 {
                return Err("Invalid MEGA folder key length".into());
            }
            let mut folder_key = [0u8; 16];
            folder_key.copy_from_slice(&master_key_bytes[..16]);

            let api_url = format!("https://g.api.mega.co.nz/cs?n={id}");
            let payload = json!([
                { "a": "f", "c": 1, "r": 1, "ca": 1 }
            ]);

            let resp = client
                .post(&api_url)
                .json(&payload)
                .send()
                .await
                .map_err(|e| format!("MEGA API request failed: {e}"))?;

            if !resp.status().is_success() {
                return Err(format!("MEGA API returned HTTP {}", resp.status()));
            }

            let body: Value = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse MEGA JSON response: {e}"))?;
            let first_val = match body {
                Value::Array(mut items) => items.drain(..).next().ok_or("Empty MEGA response")?,
                Value::Number(number) => Value::Number(number),
                _ => return Err("MEGA API returned an unexpected response".into()),
            };
            if matches!(first_val.as_i64(), Some(-9) | Some(-11) | Some(-16)) {
                return Err("MEGA link is unavailable (deleted, expired or private)".into());
            }
            let folder_res: MegaFolderApiResponse = serde_json::from_value(first_val)
                .map_err(|e| format!("Failed to deserialize MEGA nodes: {e}"))?;

            let raw_nodes = folder_res.f.unwrap_or_default();
            let raw_nodes_empty = raw_nodes.is_empty();
            let mut nodes: Vec<CloudNode> = Vec::new();
            let mut total_size: u64 = 0;
            let mut folder_title = format!("MEGA Folder ({id})");
            let mut decrypted_any = false;

            for n in raw_nodes {
                let is_folder = n.t == 1;
                let size = n.s;
                if let Some(s) = size {
                    if !is_folder {
                        total_size += s;
                    }
                }

                let mut node_key = folder_key;
                let mut dec_k_raw: Option<Vec<u8>> = None;
                let mut name = format!("Item_{}", n.h);

                let mut candidate_keys: Vec<String> = Vec::new();
                if let Some(ref k_str) = n.k {
                    for token in k_str.split(['/', ',']) {
                        let parts: Vec<&str> = token.split(':').collect();
                        if let Some(k_part) = parts.last() {
                            let trimmed = k_part.trim();
                            if !trimmed.is_empty() {
                                candidate_keys.push(trimmed.to_string());
                            }
                        }
                        for part in parts {
                            let trimmed = part.trim();
                            if trimmed.len() >= 20 && !candidate_keys.contains(&trimmed.to_string())
                            {
                                candidate_keys.push(trimmed.to_string());
                            }
                        }
                    }
                    if candidate_keys.is_empty() && !k_str.trim().is_empty() {
                        candidate_keys.push(k_str.trim().to_string());
                    }
                }

                let mut decrypted_name: Option<String> = None;
                for cand in &candidate_keys {
                    if let Ok(k_bytes) = mega_base64_decode(cand) {
                        if k_bytes.len() == 32 {
                            let mut k_arr = [0u8; 32];
                            k_arr.copy_from_slice(&k_bytes);
                            let dec_k = decrypt_aes_cbc_zeros(&folder_key, k_arr.to_vec());
                            if let Ok(dec_k) = dec_k {
                                if dec_k.len() == 32 {
                                    let mut k128 = [0u8; 16];
                                    for i in 0..16 {
                                        k128[i] = dec_k[i] ^ dec_k[i + 16];
                                    }
                                    node_key = k128;
                                    dec_k_raw = Some(dec_k);
                                    break;
                                }
                            }
                        } else if k_bytes.len() == 16 {
                            let mut k_arr = [0u8; 16];
                            k_arr.copy_from_slice(&k_bytes);
                            let dec_k = decrypt_aes_ecb(&folder_key, k_arr.to_vec());
                            if let Ok(dec_k) = dec_k {
                                if dec_k.len() == 16 {
                                    let mut k128 = [0u8; 16];
                                    k128.copy_from_slice(&dec_k);
                                    node_key = k128;
                                    dec_k_raw = Some(dec_k);
                                    break;
                                }
                            }
                        }
                    }
                }

                if let Some(ref a_str) = n.a {
                    if let Ok(a_bytes) = mega_base64_decode(a_str) {
                        for k_part in &candidate_keys {
                            if let Ok(k_bytes) = mega_base64_decode(k_part) {
                                if k_bytes.len() >= 16 {
                                    if let Ok(dec_k) = decrypt_aes_ecb(&folder_key, k_bytes.clone())
                                    {
                                        if dec_k.len() >= 16 {
                                            let mut test_key = [0u8; 16];
                                            if dec_k.len() >= 32 {
                                                for i in 0..16 {
                                                    test_key[i] = dec_k[i] ^ dec_k[i + 16];
                                                }
                                            } else {
                                                test_key.copy_from_slice(&dec_k[..16]);
                                            }

                                            if let Ok(dec_attr) =
                                                decrypt_aes_cbc_zeros(&test_key, a_bytes.clone())
                                            {
                                                if let Some(p_name) = parse_attributes(&dec_attr) {
                                                    node_key = test_key;
                                                    dec_k_raw = Some(dec_k.clone());
                                                    decrypted_name = Some(p_name);
                                                    break;
                                                }
                                            }

                                            let mut test_key_direct = [0u8; 16];
                                            test_key_direct.copy_from_slice(&dec_k[..16]);
                                            if let Ok(dec_attr) = decrypt_aes_cbc_zeros(
                                                &test_key_direct,
                                                a_bytes.clone(),
                                            ) {
                                                if let Some(p_name) = parse_attributes(&dec_attr) {
                                                    node_key = test_key_direct;
                                                    dec_k_raw = Some(dec_k.clone());
                                                    decrypted_name = Some(p_name);
                                                    break;
                                                }
                                            }
                                        }
                                    }

                                    if let Ok(dec_k) = decrypt_aes_cbc_zeros(&folder_key, k_bytes) {
                                        if dec_k.len() >= 16 {
                                            let mut test_key = [0u8; 16];
                                            if dec_k.len() >= 32 {
                                                for i in 0..16 {
                                                    test_key[i] = dec_k[i] ^ dec_k[i + 16];
                                                }
                                            } else {
                                                test_key.copy_from_slice(&dec_k[..16]);
                                            }

                                            if let Ok(dec_attr) =
                                                decrypt_aes_cbc_zeros(&test_key, a_bytes.clone())
                                            {
                                                if let Some(p_name) = parse_attributes(&dec_attr) {
                                                    node_key = test_key;
                                                    dec_k_raw = Some(dec_k.clone());
                                                    decrypted_name = Some(p_name);
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if decrypted_name.is_none() {
                            if let Ok(dec_attr) =
                                decrypt_aes_cbc_zeros(&folder_key, a_bytes.clone())
                            {
                                if let Some(p_name) = parse_attributes(&dec_attr) {
                                    decrypted_name = Some(p_name);
                                }
                            }
                        }
                    }
                }

                if let Some(p_name) = decrypted_name {
                    name = p_name;
                    decrypted_any = true;
                }

                if n.p.is_none() && is_folder {
                    folder_title = name.clone();
                }

                let mime = if !is_folder {
                    mime_guess::from_path(&name).first_raw().map(str::to_string)
                } else {
                    None
                };

                let stream_url = if !is_folder {
                    let key_param = if let Some(ref k_raw) = dec_k_raw {
                        URL_SAFE_NO_PAD.encode(k_raw)
                    } else {
                        URL_SAFE_NO_PAD.encode(node_key)
                    };
                    Some(format!(
                        "/cloud_stream/mega?folder_id={id}&node_id={}&key={key_param}&name={}",
                        n.h,
                        urlencoding::encode(&name)
                    ))
                } else {
                    None
                };

                nodes.push(CloudNode {
                    id: n.h.clone(),
                    parent_id: n.p.clone(),
                    name,
                    size,
                    is_folder,
                    mime_type: mime,
                    download_url: stream_url.clone(),
                    stream_url,
                    thumbnail_url: None,
                    children: None,
                    is_encrypted: false,
                });
            }

            if !raw_nodes_empty && !decrypted_any {
                return Err(WRONG_KEY.into());
            }

            Ok(CloudFolderResult {
                provider: "mega".into(),
                url: url_str.to_string(),
                title: folder_title,
                total_files: nodes.iter().filter(|n| !n.is_folder).count() as u64,
                total_size,
                is_single_file: false,
                nodes,
            })
        }
        MegaLink::File { id, key } => {
            let key_bytes = mega_base64_decode(&key)?;
            let mut file_key = [0u8; 16];
            if key_bytes.len() >= 16 {
                if key_bytes.len() >= 32 {
                    for i in 0..16 {
                        file_key[i] = key_bytes[i] ^ key_bytes[i + 16];
                    }
                } else {
                    file_key.copy_from_slice(&key_bytes[..16]);
                }
            }

            let api_url = "https://g.api.mega.co.nz/cs";
            let payload = json!([
                { "a": "g", "p": id, "g": 1 }
            ]);

            let resp = client
                .post(api_url)
                .json(&payload)
                .send()
                .await
                .map_err(|e| format!("MEGA API request failed: {e}"))?;

            let results: Vec<Value> = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse MEGA file response: {e}"))?;

            let first_val = results
                .into_iter()
                .next()
                .ok_or("Empty MEGA file response")?;
            let size = first_val.get("s").and_then(|v| v.as_u64()).unwrap_or(0);
            let mut name = format!("mega_file_{id}");

            if let Some(at_str) = first_val.get("at").and_then(|v| v.as_str()) {
                let parsed_name = mega_base64_decode(at_str)
                    .ok()
                    .and_then(|at_bytes| decrypt_aes_cbc_zeros(&file_key, at_bytes).ok())
                    .and_then(|dec_attr| parse_attributes(&dec_attr));
                match parsed_name {
                    Some(parsed_name) => name = parsed_name,
                    None => return Err(WRONG_KEY.into()),
                }
            }

            let stream_url = Some(format!(
                "/cloud_stream/mega?file_id={id}&key={key}&name={}",
                urlencoding::encode(&name)
            ));
            let mime = mime_guess::from_path(&name).first_raw().map(str::to_string);

            let node = CloudNode {
                id: id.clone(),
                parent_id: None,
                name: name.clone(),
                size: Some(size),
                is_folder: false,
                mime_type: mime,
                download_url: stream_url.clone(),
                stream_url,
                thumbnail_url: None,
                children: None,
                is_encrypted: false,
            };

            Ok(CloudFolderResult {
                provider: "mega".into(),
                url: url_str.to_string(),
                title: name,
                total_files: 1,
                total_size: size,
                is_single_file: true,
                nodes: vec![node],
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_current_and_legacy_links() {
        let parts = |url: &str| match parse_mega_url(url) {
            Some(MegaLink::Folder { id, key }) => Some(("folder", id, key)),
            Some(MegaLink::File { id, key }) => Some(("file", id, key)),
            None => None,
        };
        for (url, kind, id, key) in [
            (
                "https://mega.nz/folder/abc12345#key67890",
                "folder",
                "abc12345",
                "key67890",
            ),
            (
                "https://mega.nz/file/file123#filekey456",
                "file",
                "file123",
                "filekey456",
            ),
            (
                "https://mega.nz/#F!folderId!folderKey",
                "folder",
                "folderId",
                "folderKey",
            ),
            (
                "https://mega.nz/#!legacyFile!legacyKey",
                "file",
                "legacyFile",
                "legacyKey",
            ),
        ] {
            assert_eq!(parts(url), Some((kind, id.into(), key.into())), "{url}");
        }
        assert!(parts("https://example.com/file/123").is_none());

        assert!(is_keyless_link("https://mega.nz/folder/Jj1liCiQ"));
        assert!(is_keyless_link("https://mega.nz/file/jPwAmYqZ"));
        assert!(!is_keyless_link(
            "https://mega.nz/folder/Jj1liCiQ#TeHAplSadyE1w--Mt861ig"
        ));
        assert!(!is_keyless_link("https://mega.nz/#P!AgHEnCCR"));
    }

    fn protect_link(
        algorithm: u8,
        kind: u8,
        handle: &[u8; 6],
        key: &[u8],
        password: &str,
    ) -> String {
        use hmac::{Hmac, Mac};
        use sha2::{Sha256, Sha512};
        let salt = [7u8; PROTECTED_SALT_LEN];
        let rounds = if algorithm == 0 { 1_000 } else { 100_000 };
        let mut derived = [0u8; 64];
        pbkdf2::pbkdf2_hmac::<Sha512>(password.as_bytes(), &salt, rounds, &mut derived);
        let mut data = vec![algorithm, kind];
        data.extend_from_slice(handle);
        data.extend_from_slice(&salt);
        data.extend(key.iter().zip(&derived).map(|(a, b)| a ^ b));
        let (hmac_key, message) = if algorithm == 1 {
            (data.clone(), derived[32..].to_vec())
        } else {
            (derived[32..].to_vec(), data.clone())
        };
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&hmac_key).unwrap();
        mac.update(&message);
        data.extend_from_slice(&mac.finalize().into_bytes());
        format!("https://mega.nz/#P!{}", URL_SAFE_NO_PAD.encode(data))
    }

    #[test]
    fn protected_links_round_trip_for_every_algorithm_and_kind() {
        let handle = [1u8, 2, 3, 4, 5, 6];
        let folder_key = [0x11u8; 16];
        let file_key = [0x22u8; 32];
        for algorithm in [0u8, 1, 2] {
            let folder = protect_link(algorithm, 0, &handle, &folder_key, "hunter2");
            assert_eq!(
                decrypt_protected_link(&folder, " hunter2 ").unwrap(),
                format!(
                    "https://mega.nz/folder/{}#{}",
                    URL_SAFE_NO_PAD.encode(handle),
                    URL_SAFE_NO_PAD.encode(folder_key)
                )
            );
            assert_eq!(
                decrypt_protected_link(&folder, "hunter3"),
                Err(ProtectedLinkError::WrongPassword)
            );
            let file = protect_link(algorithm, 1, &handle, &file_key, "пароль");
            assert_eq!(
                decrypt_protected_link(&file, "пароль").unwrap(),
                format!(
                    "https://mega.nz/file/{}#{}",
                    URL_SAFE_NO_PAD.encode(handle),
                    URL_SAFE_NO_PAD.encode(file_key)
                )
            );
        }
    }

    #[test]
    fn real_protected_link_rejects_a_wrong_password() {
        let url = "https://mega.nz/#P!AgHEnCCRIkhv6Ny-CPml0t4TbDvzrDnUqm2VtANrA7APZ4DV4vsIOFmevFD6lh_KAkcfoUCknhh77OG4G47yEXE0DDeqXrldDLBo1fXTEbxa1kNJt6imo2wXPWL-FS8LxTfgsoV8cVw";
        assert!(is_protected_link(url));
        assert_eq!(
            decrypt_protected_link(url, "not-the-password"),
            Err(ProtectedLinkError::WrongPassword)
        );
    }

    #[test]
    fn malformed_and_unsupported_protected_links() {
        assert_eq!(
            decrypt_protected_link("https://mega.nz/#P!AgHEnCCR", "x"),
            Err(ProtectedLinkError::Malformed)
        );
        for header in [[2u8, 2, 0, 0], [9u8, 0, 0, 0]] {
            let link = format!("https://mega.nz/#P!{}", URL_SAFE_NO_PAD.encode(header));
            assert_eq!(
                decrypt_protected_link(&link, "x"),
                Err(ProtectedLinkError::Unsupported)
            );
        }
    }
}
