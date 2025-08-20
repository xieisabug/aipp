use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::prelude::*;

#[derive(Serialize, Deserialize)]
pub struct SharedAssistant {
    pub version: String,
    #[serde(rename = "type")]
    pub data_type: String,
    pub data: AssistantShareData,
}

#[derive(Serialize, Deserialize)]
pub struct AssistantShareData {
    pub name: String,
    pub description: Option<String>,
    pub assistant_type: i64,
    pub prompt: String,
    pub model_configs: Vec<ModelConfigShare>,
}

#[derive(Serialize, Deserialize)]
pub struct ModelConfigShare {
    pub name: String,
    pub value: String,
    pub value_type: String,
}

#[derive(Serialize, Deserialize)]
struct SharedProvider {
    pub version: String,
    #[serde(rename = "type")]
    pub data_type: String,
    pub encrypted_data: String,
    pub nonce: String,
}

#[derive(Serialize, Deserialize)]
pub struct ProviderShareData {
    pub name: String,
    pub api_type: String,
    pub endpoint: Option<String>,
    pub api_key: String,
}

/// 压缩并Base64编码助手数据
pub fn compress_assistant_data(assistant: &SharedAssistant) -> Result<String> {
    // 序列化为JSON
    let json_data = serde_json::to_string(assistant)?;

    // 使用gzip压缩
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(json_data.as_bytes())?;
    let compressed = encoder.finish()?;

    // Base64编码
    Ok(general_purpose::STANDARD.encode(compressed))
}

/// 解压缩助手数据
pub fn decompress_assistant_data(compressed_data: &str) -> Result<SharedAssistant> {
    // Base64解码
    let compressed = general_purpose::STANDARD.decode(compressed_data)?;

    // 解压缩
    let mut decoder = GzDecoder::new(&compressed[..]);
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed)?;

    // 反序列化
    let assistant: SharedAssistant = serde_json::from_str(&decompressed)?;
    Ok(assistant)
}

/// 使用密码加密Provider数据
pub fn encrypt_provider_data(provider: &ProviderShareData, password: &str) -> Result<String> {
    // 使用密码生成密钥
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let key_bytes = hasher.finalize();
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);

    // 创建加密器
    let cipher = Aes256Gcm::new(key);

    // 生成随机nonce
    let nonce_bytes: [u8; 12] = rand::random();
    let nonce = Nonce::from_slice(&nonce_bytes);

    // 序列化并加密数据
    let json_data = serde_json::to_string(provider)?;
    let encrypted = cipher
        .encrypt(nonce, json_data.as_bytes())
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    let shared_provider = SharedProvider {
        version: "1.0".to_string(),
        data_type: "provider".to_string(),
        encrypted_data: general_purpose::STANDARD.encode(encrypted),
        nonce: general_purpose::STANDARD.encode(nonce_bytes),
    };

    // 序列化为JSON并进行base64编码
    let json_string = serde_json::to_string(&shared_provider)?;
    Ok(general_purpose::STANDARD.encode(json_string))
}

/// 使用密码解密Provider数据
pub fn decrypt_provider_data(share_code: &str, password: &str) -> Result<ProviderShareData> {
    // 先进行base64解码
    let json_string = String::from_utf8(general_purpose::STANDARD.decode(share_code)?)?;

    // 反序列化为SharedProvider
    let shared_provider: SharedProvider = serde_json::from_str(&json_string)?;

    // 使用密码生成密钥
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let key_bytes = hasher.finalize();
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);

    // 创建解密器
    let cipher = Aes256Gcm::new(key);

    // 解码nonce和加密数据
    let nonce_bytes = general_purpose::STANDARD.decode(&shared_provider.nonce)?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let encrypted_data = general_purpose::STANDARD.decode(&shared_provider.encrypted_data)?;

    // 解密数据
    let decrypted = cipher
        .decrypt(nonce, encrypted_data.as_ref())
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;

    // 反序列化
    let json_data = String::from_utf8(decrypted)?;
    let provider: ProviderShareData = serde_json::from_str(&json_data)?;
    Ok(provider)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assistant_compression() {
        let assistant = SharedAssistant {
            version: "1.0".to_string(),
            data_type: "assistant".to_string(),
            data: AssistantShareData {
                name: "Test Assistant".to_string(),
                description: Some("Test Description".to_string()),
                assistant_type: 0,
                prompt: "Test prompt".to_string(),
                model_configs: vec![ModelConfigShare {
                    name: "temperature".to_string(),
                    value: "0.7".to_string(),
                    value_type: "float".to_string(),
                }],
            },
        };

        let compressed = compress_assistant_data(&assistant).unwrap();
        let decompressed = decompress_assistant_data(&compressed).unwrap();

        assert_eq!(assistant.data.name, decompressed.data.name);
        assert_eq!(assistant.data.prompt, decompressed.data.prompt);
    }

    #[test]
    fn test_provider_encryption() {
        let provider = ProviderShareData {
            name: "Test Provider".to_string(),
            api_type: "openai_api".to_string(),
            endpoint: Some("https://api.openai.com".to_string()),
            api_key: "test-key-123".to_string(),
        };

        let password = "test_password_123";
        let encrypted_share_code = encrypt_provider_data(&provider, password).unwrap();

        // 验证返回的是base64编码的字符串
        assert!(general_purpose::STANDARD.decode(&encrypted_share_code).is_ok());

        let decrypted = decrypt_provider_data(&encrypted_share_code, password).unwrap();

        assert_eq!(provider.name, decrypted.name);
        assert_eq!(provider.api_key, decrypted.api_key);
        assert_eq!(provider.api_type, decrypted.api_type);
        assert_eq!(provider.endpoint, decrypted.endpoint);
    }
}
