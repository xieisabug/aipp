use serde::{Deserialize, Serialize};
use crate::db::llm_db::LLMDatabase;

#[derive(Serialize, Deserialize)]
pub struct LlmProvider {
    pub id: i64,
    pub name: String,
    pub api_type: String,
    pub description: String,
    pub is_official: bool,
    pub is_enabled: bool,
}

#[derive(Serialize, Deserialize)]
pub struct LlmModel {
    pub id: i64,
    pub name: String,
    pub llm_provider_id: i64,
    pub code: String,
    pub description: String,
    pub vision_support: bool,
    pub audio_support: bool,
    pub video_support: bool,
}

#[derive(Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub id: i64,
    pub name: String,
    pub llm_provider_id: i64,
    pub value: String,
    pub append_location: String,
    pub is_addition: bool,
}

#[tauri::command]
pub async fn get_llm_providers() -> Result<Vec<LlmProvider>, String> {
    let db = LLMDatabase::new().map_err(|e| e.to_string())?;
    let providers = db.get_llm_providers().map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for (id, name, api_type, description, is_official, is_enabled) in providers {
        result.push(LlmProvider {
            id,
            name,
            api_type,
            description,
            is_official,
            is_enabled
        });
    }
    Ok(result)
}

#[tauri::command]
pub async fn update_llm_provider(id: i64, name: String, api_type: String, description: String, is_enabled: bool) -> Result<(), String> {
    let db = LLMDatabase::new().map_err(|e| e.to_string())?;
    db.update_llm_provider(id, &*name, &*api_type, &*description, is_enabled).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_llm_provider_config(id: i64) -> Result<Vec<LlmProviderConfig>, String> {
    let db = LLMDatabase::new().map_err(|e| e.to_string())?;
    let configs = db.get_llm_provider_config(id).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for (id, name, llm_provider_id, value, append_location, is_addition) in configs {
        result.push(LlmProviderConfig {
            id,
            name,
            llm_provider_id,
            value,
            append_location,
            is_addition,
        });
    }
    Ok(result)
}

#[tauri::command]
pub async fn get_llm_models(provider_id: String) -> Result<Vec<LlmModel>, String> {
    let db = LLMDatabase::new().map_err(|e| e.to_string())?;
    let models = db.get_llm_models(provider_id).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for (id, name, llm_provider_id, code, description, vision_support, audio_support, video_support) in models {
        result.push(LlmModel {
            id,
            name,
            llm_provider_id,
            code,
            description,
            vision_support,
            audio_support,
            video_support,
        });
    }
    Ok(result)
}
