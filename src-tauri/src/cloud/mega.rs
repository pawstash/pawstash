use super::models::{CloudFolderResult, CloudNode};
use aes::cipher::{BlockDecrypt, KeyInit};
use aes::Aes128;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use cbc::cipher::{block_padding::NoPadding, BlockDecryptMut, KeyIvInit};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};

type Aes128CbcDec = cbc::Decryptor<Aes128>;

#[derive(Debug)]
pub enum MegaLink {
    Folder { id: String, key: String },
    File { id: String, key: String },
}

pub fn parse_mega_url(url_str: &str) -> Option<MegaLink> {
    let url = reqwest::Url::parse(url_str).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    if !host.contains("mega.nz") && !host.contains("mega.co.nz") {
        return None;
    }

    let fragment = url.fragment().unwrap_or("");
    let path = url.path().trim_matches('/');
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    // Format 1: https://mega.nz/folder/{folder_id}#{folder_key}
    if segments.first() == Some(&"folder") {
        let id = segments.get(1)?.to_string();
        let key = fragment.trim().to_string();
        if !key.is_empty() {
            return Some(MegaLink::Folder { id, key });
        }
    }

    // Format 2: https://mega.nz/file/{file_id}#{file_key}
    if segments.first() == Some(&"file") {
        let id = segments.get(1)?.to_string();
        let key = fragment.trim().to_string();
        if !key.is_empty() {
            return Some(MegaLink::File { id, key });
        }
    }

    // Format 3: https://mega.nz/#F!{folder_id}!{folder_key}
    if fragment.starts_with("F!") {
        let parts: Vec<&str> = fragment.split('!').collect();
        if parts.len() >= 3 {
            return Some(MegaLink::Folder {
                id: parts[1].to_string(),
                key: parts[2].to_string(),
            });
        }
    }

    // Format 4: https://mega.nz/#!{file_id}!{file_key}
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
    t: i32, // 0 = file, 1 = folder, 2 = root
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

            let results: Vec<Value> = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse MEGA JSON response: {e}"))?;

            let first_val = results.into_iter().next().ok_or("Empty MEGA response")?;
            let folder_res: MegaFolderApiResponse = serde_json::from_value(first_val)
                .map_err(|e| format!("Failed to deserialize MEGA nodes: {e}"))?;

            let raw_nodes = folder_res.f.unwrap_or_default();
            let mut nodes: Vec<CloudNode> = Vec::new();
            let mut total_size: u64 = 0;
            let mut folder_title = format!("MEGA Folder ({id})");

            for n in raw_nodes {
                let is_folder = n.t == 1;
                let size = n.s;
                if let Some(s) = size {
                    if !is_folder {
                        total_size += s;
                    }
                }

                // Decrypt node key
                let mut node_key = folder_key;
                let mut dec_k_raw: Option<Vec<u8>> = None;
                let mut name = format!("Item_{}", n.h);

                // Collect candidate key parts from n.k
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
                                    // 1. Try AES-128-ECB decryption (Standard MEGA folder share node key encryption)
                                    if let Ok(dec_k) = decrypt_aes_ecb(&folder_key, k_bytes.clone())
                                    {
                                        if dec_k.len() >= 16 {
                                            // Format 1: 32-byte key XOR (standard file node key)
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

                                            // Format 2: Direct 16-byte key (standard folder node key)
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

                                    // 2. Try AES-128-CBC fallback
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

                        // 3. Fallback: Try decrypting attributes with folder_key directly
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
                });
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
                if let Ok(at_bytes) = mega_base64_decode(at_str) {
                    if let Ok(dec_attr) = decrypt_aes_cbc_zeros(&file_key, at_bytes) {
                        if let Some(parsed_name) = parse_attributes(&dec_attr) {
                            name = parsed_name;
                        }
                    }
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
    fn test_parse_mega_urls() {
        // Modern folder
        match parse_mega_url("https://mega.nz/folder/abc12345#key67890") {
            Some(MegaLink::Folder { id, key }) => {
                assert_eq!(id, "abc12345");
                assert_eq!(key, "key67890");
            }
            _ => panic!("Expected MegaLink::Folder"),
        }

        // Modern file
        match parse_mega_url("https://mega.nz/file/file123#filekey456") {
            Some(MegaLink::File { id, key }) => {
                assert_eq!(id, "file123");
                assert_eq!(key, "filekey456");
            }
            _ => panic!("Expected MegaLink::File"),
        }

        // Legacy folder
        match parse_mega_url("https://mega.nz/#F!folderId!folderKey") {
            Some(MegaLink::Folder { id, key }) => {
                assert_eq!(id, "folderId");
                assert_eq!(key, "folderKey");
            }
            _ => panic!("Expected legacy MegaLink::Folder"),
        }

        // Legacy file
        match parse_mega_url("https://mega.nz/#!legacyFile!legacyKey") {
            Some(MegaLink::File { id, key }) => {
                assert_eq!(id, "legacyFile");
                assert_eq!(key, "legacyKey");
            }
            _ => panic!("Expected legacy MegaLink::File"),
        }

        // Invalid URL
        assert!(parse_mega_url("https://example.com/file/123").is_none());
    }
}
