use crate::api::genai_client;
use crate::db::llm_db::LLMDatabase;
use crate::utils::share_utils::{decrypt_provider_data, encrypt_provider_data, ProviderShareData};
use genai::Modality;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct LlmProvider {
    pub id: i64,
    pub name: String,
    pub api_type: String,
    pub description: String,
    pub is_official: bool,
    pub is_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
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
    pub append_location: Option<String>,
    pub is_addition: Option<bool>,
}

#[tauri::command]
pub async fn get_llm_providers(app_handle: tauri::AppHandle) -> Result<Vec<LlmProvider>, String> {
    let db = LLMDatabase::new(&app_handle).map_err(|e: rusqlite::Error| e.to_string())?;
    let providers = db.get_llm_providers().map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for (id, name, api_type, description, is_official, is_enabled) in providers {
        result.push(LlmProvider { id, name, api_type, description, is_official, is_enabled });
    }
    Ok(result)
}

#[tauri::command]
pub async fn add_llm_provider(
    app: tauri::AppHandle,
    name: String,
    api_type: String,
) -> Result<(), String> {
    let db = LLMDatabase::new(&app).map_err(|e| e.to_string())?;
    db.add_llm_provider(&*name, &*api_type, "", false, false).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn update_llm_provider(
    app_handle: tauri::AppHandle,
    id: i64,
    name: String,
    api_type: String,
    description: String,
    is_enabled: bool,
) -> Result<(), String> {
    let db = LLMDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    db.update_llm_provider(id, &*name, &*api_type, &*description, is_enabled)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_llm_provider(
    app_handle: tauri::AppHandle,
    llm_provider_id: i64,
) -> Result<(), String> {
    let db = LLMDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    db.delete_llm_provider(llm_provider_id).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_llm_provider_config(
    app_handle: tauri::AppHandle,
    id: i64,
) -> Result<Vec<LlmProviderConfig>, String> {
    let db = LLMDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let configs = db.get_llm_provider_config(id).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for config in configs {
        result.push(LlmProviderConfig {
            id: config.id,
            name: config.name,
            llm_provider_id: config.llm_provider_id,
            value: config.value,
            append_location: Some(config.append_location),
            is_addition: Some(config.is_addition),
        });
    }
    Ok(result)
}

#[tauri::command]
pub async fn update_llm_provider_config(
    app_handle: tauri::AppHandle,
    llm_provider_id: i64,
    name: String,
    value: String,
) -> Result<(), String> {
    let db = LLMDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    db.update_llm_provider_config(llm_provider_id, &*name, &*value).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_llm_models(
    app_handle: tauri::AppHandle,
    llm_provider_id: String,
) -> Result<Vec<LlmModel>, String> {
    let db = LLMDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let models = db.get_llm_models(llm_provider_id).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for (
        id,
        name,
        llm_provider_id,
        code,
        description,
        vision_support,
        audio_support,
        video_support,
    ) in models
    {
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

#[tauri::command]
pub async fn fetch_model_list(
    app_handle: tauri::AppHandle,
    llm_provider_id: i64,
) -> Result<Vec<LlmModel>, String> {
    let db = LLMDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let llm_provider = db.get_llm_provider(llm_provider_id).map_err(|e| e.to_string())?;
    let llm_provider_config =
        db.get_llm_provider_config(llm_provider_id).map_err(|e| e.to_string())?;

    // 使用共用的客户端创建函数
    let client = genai_client::create_client_with_config(
        &llm_provider_config,
        "",
        &llm_provider.api_type,
        None,
        false,
        None,
    )
    .map_err(|e| e.to_string())?;

    let adapter_kind = genai_client::infer_adapter_kind_simple(&llm_provider.api_type);

    match client.all_models(adapter_kind).await {
        Ok(models) => {
            db.delete_llm_model_by_provider(llm_provider_id).map_err(|e| e.to_string())?;

            let mut result = Vec::new();
            for model in &models {
                let model = LlmModel {
                    id: 0,
                    name: model.name.to_string(),
                    llm_provider_id,
                    code: model.id.to_string(),
                    description: format!("Model: {}", model.name),
                    vision_support: model.supports_input_modality(&Modality::Image),
                    audio_support: model.supports_input_modality(&Modality::Audio),
                    video_support: model.supports_input_modality(&Modality::Video),
                };

                db.add_llm_model(
                    &model.name,
                    llm_provider_id,
                    &model.code,
                    &model.description,
                    model.vision_support,
                    model.audio_support,
                    model.video_support,
                )
                .map_err(|e| e.to_string())?;

                result.push(model);
            }

            Ok(result)
        }
        Err(e) => {
            eprintln!("获取模型列表错误: {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn add_llm_model(
    app_handle: tauri::AppHandle,
    llm_provider_id: i64,
    code: String,
) -> Result<(), String> {
    let db = LLMDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let code_str = code.as_str();
    db.add_llm_model(code_str, llm_provider_id, code_str, code_str, false, false, false)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_llm_model(
    app_handle: tauri::AppHandle,
    llm_provider_id: i64,
    code: String,
) -> Result<(), String> {
    let db = LLMDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let _ = db.delete_llm_model(llm_provider_id, code);
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct ModelForSelect {
    name: String,
    code: String,
    id: i64,
    llm_provider_id: i64,
}

#[derive(Serialize, Deserialize)]
pub struct ModelForSelection {
    pub name: String,
    pub code: String,
    pub description: String,
    pub vision_support: bool,
    pub audio_support: bool,
    pub video_support: bool,
    pub is_selected: bool, // 是否已在数据库中存在
}

#[derive(Serialize, Deserialize)]
pub struct ModelSelectionResponse {
    pub available_models: Vec<ModelForSelection>,
    pub missing_models: Vec<String>, // 在数据库中但远程不存在的模型
}

#[tauri::command]
pub async fn preview_model_list(
    app_handle: tauri::AppHandle,
    llm_provider_id: i64,
) -> Result<ModelSelectionResponse, String> {
    let db = LLMDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let llm_provider = db.get_llm_provider(llm_provider_id).map_err(|e| e.to_string())?;
    let llm_provider_config =
        db.get_llm_provider_config(llm_provider_id).map_err(|e| e.to_string())?;

    // 获取现有的模型列表
    let existing_models =
        db.get_llm_models(llm_provider_id.to_string()).map_err(|e| e.to_string())?;
    let existing_model_codes: std::collections::HashSet<String> =
        existing_models.iter().map(|(_, _, _, code, _, _, _, _)| code.clone()).collect();

    // 使用共用的客户端创建函数
    let client = genai_client::create_client_with_config(
        &llm_provider_config,
        "",
        &llm_provider.api_type,
        None,
        false,
        None,
    )
    .map_err(|e| e.to_string())?;

    let adapter_kind = genai_client::infer_adapter_kind_simple(&llm_provider.api_type);

    match client.all_models(adapter_kind).await {
        Ok(models) => {
            let mut available_models = Vec::new();
            let remote_model_codes: std::collections::HashSet<String> =
                models.iter().map(|model| model.id.to_string()).collect();

            // 构建可选择的模型列表
            for model in &models {
                let model_code = model.id.to_string();
                let is_selected = existing_model_codes.contains(&model_code);

                available_models.push(ModelForSelection {
                    name: model.name.to_string(),
                    code: model_code,
                    description: format!("Model: {}", model.name),
                    vision_support: model.supports_input_modality(&Modality::Image),
                    audio_support: model.supports_input_modality(&Modality::Audio),
                    video_support: model.supports_input_modality(&Modality::Video),
                    is_selected,
                });
            }

            // 找出在数据库中但远程不存在的模型
            let missing_models: Vec<String> =
                existing_model_codes.difference(&remote_model_codes).cloned().collect();

            Ok(ModelSelectionResponse { available_models, missing_models })
        }
        Err(e) => {
            eprintln!("获取模型列表错误: {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub fn get_models_for_select(app_handle: tauri::AppHandle) -> Result<Vec<ModelForSelect>, String> {
    let db = LLMDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let result = db.get_models_for_select().unwrap();
    let models = result
        .iter()
        .map(|(name, code, id, llm_provider_id)| ModelForSelect {
            name: name.clone(),
            code: code.clone(),
            id: *id,
            llm_provider_id: *llm_provider_id,
        })
        .collect();
    Ok(models)
}

#[tauri::command]
pub async fn update_selected_models(
    app_handle: tauri::AppHandle,
    llm_provider_id: i64,
    selected_models: Vec<ModelForSelection>,
) -> Result<(), String> {
    let db = LLMDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    // 删除所有该提供商的现有模型
    db.delete_llm_model_by_provider(llm_provider_id).map_err(|e| e.to_string())?;

    // 添加选中的模型
    for model in selected_models.iter().filter(|m| m.is_selected) {
        db.add_llm_model(
            &model.name,
            llm_provider_id,
            &model.code,
            &model.description,
            model.vision_support,
            model.audio_support,
            model.video_support,
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

// Share and Import LLM Provider Commands

#[tauri::command]
pub async fn export_llm_provider(
    app_handle: tauri::AppHandle,
    provider_id: i64,
    password: String,
) -> Result<String, String> {
    let db = LLMDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    // Get provider information
    let provider = db.get_llm_provider(provider_id).map_err(|e| e.to_string())?;
    let configs = db.get_llm_provider_config(provider_id).map_err(|e| e.to_string())?;

    // Extract endpoint and api_key from configs
    let mut endpoint = None;
    let mut api_key = String::new();

    for config in configs {
        match config.name.as_str() {
            "endpoint" | "base_url" => endpoint = Some(config.value),
            "api_key" => api_key = config.value,
            _ => {}
        }
    }

    if api_key.is_empty() {
        return Err("API Key is required for export".to_string());
    }

    // Create share data
    let share_data =
        ProviderShareData { name: provider.name, api_type: provider.api_type, endpoint, api_key };

    // Encrypt with password
    let encrypted_data =
        encrypt_provider_data(&share_data, &password).map_err(|e| e.to_string())?;

    // Return the base64 encoded string directly
    Ok(encrypted_data)
}

#[tauri::command]
pub async fn import_llm_provider(
    app_handle: tauri::AppHandle,
    share_code: String,
    password: String,
    new_name: Option<String>,
) -> Result<LlmProvider, String> {
    // Decrypt data directly from share code
    let provider_data = decrypt_provider_data(&share_code, &password)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    let db = LLMDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    // Use provided name or original name with suffix
    let provider_name = new_name.unwrap_or_else(|| format!("{} (导入)", provider_data.name));

    // Create new provider
    db.add_llm_provider(&provider_name, &provider_data.api_type, "Imported provider", false, true)
        .map_err(|e| e.to_string())?;

    // Get the newly created provider ID
    let providers = db.get_llm_providers().map_err(|e| e.to_string())?;
    let new_provider = providers
        .iter()
        .find(|(_, name, _, _, _, _)| name == &provider_name)
        .ok_or("Failed to find newly created provider")?;

    let provider_id = new_provider.0;

    // Add endpoint config if provided
    if let Some(endpoint) = provider_data.endpoint {
        db.add_llm_provider_config(provider_id, "endpoint", &endpoint, "header", false)
            .map_err(|e| e.to_string())?;
    }

    // Add API key config
    db.add_llm_provider_config(provider_id, "api_key", &provider_data.api_key, "header", false)
        .map_err(|e| e.to_string())?;

    // Return the created provider
    Ok(LlmProvider {
        id: provider_id,
        name: provider_name,
        api_type: provider_data.api_type,
        description: "Imported provider".to_string(),
        is_official: false,
        is_enabled: true,
    })
}
