use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State};

use crate::api::ai::config::{get_network_proxy_from_config, get_request_timeout_from_config};
use crate::api::genai_client;
use crate::artifacts::code_utils::{
    extract_component_name, extract_vue_component_name, is_react_component, is_vue_component,
};
use crate::artifacts::react_runner::run_react_artifact;
use crate::artifacts::vue_runner::run_vue_artifact;
use crate::db::llm_db::LLMDatabase;
use crate::FeatureConfigState;

use super::artifacts_db::{
    ArtifactCollection, ArtifactsDatabase, NewArtifactCollection, UpdateArtifactCollection,
};
use crate::utils::bun_utils::BunUtils;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArtifactCollectionItem {
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub description: String,
    pub artifact_type: String,
    pub tags: Option<String>,
    pub created_time: String,
    pub last_used_time: Option<String>,
    pub use_count: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SaveArtifactRequest {
    pub name: String,
    pub icon: String,
    pub description: String,
    pub artifact_type: String,
    pub code: String,
    pub tags: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateArtifactRequest {
    pub id: i64,
    pub name: Option<String>,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub tags: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArtifactStatistics {
    pub total_count: i64,
    pub total_uses: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArtifactMetadata {
    pub name: String,
    pub description: String,
    pub tags: String,
    pub emoji_category: String,
}

#[tauri::command]
pub fn save_artifact_to_collection(
    app_handle: tauri::AppHandle,
    request: SaveArtifactRequest,
) -> Result<i64, String> {
    let db = ArtifactsDatabase::new(&app_handle)
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let new_artifact = NewArtifactCollection {
        name: request.name,
        icon: request.icon,
        description: request.description,
        artifact_type: request.artifact_type,
        code: request.code,
        tags: request.tags,
    };

    let artifact_id =
        db.save_artifact(new_artifact).map_err(|e| format!("Failed to save artifact: {}", e))?;

    let windows_to_notify = ["artifact_collections", "ask", "chat_ui"];
    for window_name in windows_to_notify.iter() {
        if let Some(window) = app_handle.get_webview_window(window_name) {
            let _ = window.emit("artifact-collection-updated", artifact_id);
        }
    }

    Ok(artifact_id)
}

#[tauri::command]
pub fn get_artifacts_collection(
    app_handle: tauri::AppHandle,
    artifact_type: Option<String>,
) -> Result<Vec<ArtifactCollectionItem>, String> {
    let db = ArtifactsDatabase::new(&app_handle)
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let artifacts = db
        .get_artifacts(artifact_type.as_deref())
        .map_err(|e| format!("Failed to get artifacts: {}", e))?;

    let items: Vec<ArtifactCollectionItem> = artifacts
        .into_iter()
        .map(|artifact| ArtifactCollectionItem {
            id: artifact.id,
            name: artifact.name,
            icon: artifact.icon,
            description: artifact.description,
            artifact_type: artifact.artifact_type,
            tags: artifact.tags,
            created_time: artifact.created_time,
            last_used_time: artifact.last_used_time,
            use_count: artifact.use_count,
        })
        .collect();

    Ok(items)
}

#[tauri::command]
pub fn get_artifact_by_id(
    app_handle: tauri::AppHandle,
    id: i64,
) -> Result<Option<ArtifactCollection>, String> {
    let db = ArtifactsDatabase::new(&app_handle)
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let artifact =
        db.get_artifact_by_id(id).map_err(|e| format!("Failed to get artifact: {}", e))?;

    Ok(artifact)
}

#[tauri::command]
pub fn search_artifacts_collection(
    app_handle: tauri::AppHandle,
    query: String,
) -> Result<Vec<ArtifactCollectionItem>, String> {
    let db = ArtifactsDatabase::new(&app_handle)
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let artifacts =
        db.search_artifacts(&query).map_err(|e| format!("Failed to search artifacts: {}", e))?;

    let items: Vec<ArtifactCollectionItem> = artifacts
        .into_iter()
        .map(|artifact| ArtifactCollectionItem {
            id: artifact.id,
            name: artifact.name,
            icon: artifact.icon,
            description: artifact.description,
            artifact_type: artifact.artifact_type,
            tags: artifact.tags,
            created_time: artifact.created_time,
            last_used_time: artifact.last_used_time,
            use_count: artifact.use_count,
        })
        .collect();

    Ok(items)
}

#[tauri::command]
pub fn update_artifact_collection(
    app_handle: tauri::AppHandle,
    request: UpdateArtifactRequest,
) -> Result<(), String> {
    let db = ArtifactsDatabase::new(&app_handle)
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let update = UpdateArtifactCollection {
        id: request.id,
        name: request.name,
        icon: request.icon,
        description: request.description,
        tags: request.tags,
    };

    db.update_artifact(update).map_err(|e| format!("Failed to update artifact: {}", e))?;

    let windows_to_notify = ["artifact_collections", "ask", "chat_ui"];
    for window_name in windows_to_notify.iter() {
        if let Some(window) = app_handle.get_webview_window(window_name) {
            let _ = window.emit("artifact-collection-updated", request.id);
        }
    }

    Ok(())
}

#[tauri::command]
pub fn delete_artifact_collection(app_handle: tauri::AppHandle, id: i64) -> Result<bool, String> {
    let db = ArtifactsDatabase::new(&app_handle)
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let deleted =
        db.delete_artifact(id).map_err(|e| format!("Failed to delete artifact: {}", e))?;

    if deleted {
        let windows_to_notify = ["artifact_collections", "ask", "chat_ui"];
        for window_name in windows_to_notify.iter() {
            if let Some(window) = app_handle.get_webview_window(window_name) {
                let _ = window.emit("artifact-collection-updated", id);
            }
        }
    }

    Ok(deleted)
}

#[tauri::command]
pub async fn open_artifact_window(
    app_handle: tauri::AppHandle,
    artifact_id: i64,
) -> Result<(), String> {
    let db = ArtifactsDatabase::new(&app_handle)
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let artifact = db
        .get_artifact_by_id(artifact_id)
        .map_err(|e| format!("Failed to get artifact: {}", e))?
        .ok_or_else(|| "Artifact not found".to_string())?;

    db.increment_use_count(artifact_id)
        .map_err(|e| format!("Failed to increment use count: {}", e))?;

    crate::window::open_artifact_window(app_handle.clone(), artifact.clone()).await?;
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    match artifact.artifact_type.as_str() {
        "react" | "jsx" => {
            if is_react_component(artifact.code.as_str()) {
                let component_name =
                    extract_component_name(artifact.code.as_str()).unwrap_or_else(|| "UserComponent".to_string());
                if let Some(window) = app_handle.get_webview_window("artifact") {
                    let _ = window.emit(
                        "artifact-data",
                        serde_json::json!({
                            "id": artifact.id,
                            "name": artifact.name,
                            "icon": artifact.icon,
                            "description": artifact.description,
                            "type": "react",
                            "original_code": artifact.code.as_str(),
                            "tags": artifact.tags,
                            "created_time": artifact.created_time,
                            "last_used_time": artifact.last_used_time,
                            "use_count": artifact.use_count,
                        }),
                    );
                }
                let _url = run_react_artifact(
                    app_handle.clone(),
                    artifact_id,
                    artifact.code.as_str().to_string(),
                    component_name,
                )
                .await
                .map_err(|e| {
                    let error_msg = format!("React 组件运行失败: {}", e);
                    if let Some(window) = app_handle.get_webview_window("artifact") { let _ = window.emit("artifact-error", &error_msg); }
                    error_msg
                })?;
            } else if let Some(window) = app_handle.get_webview_window("artifact") {
                let _ = window.emit("artifact-error", "React 代码片段预览暂不支持，请提供完整的 React 组件代码。");
            }
        }
        "vue" => {
            let bun_version = BunUtils::get_bun_version(&app_handle);
            if bun_version.is_err() || bun_version.as_ref().unwrap_or(&String::new()).contains("Not Installed") {
                if let Some(window) = app_handle.get_webview_window("artifact") {
                    let _ = window.emit("environment-check", serde_json::json!({
                        "tool": "bun",
                        "message": "Vue 预览需要 bun 环境，但系统中未安装 bun。是否要自动安装？",
                        "lang": "vue",
                        "input_str": artifact.code.as_str()
                    }));
                }
                return Ok(());
            }
            if is_vue_component(artifact.code.as_str()) {
                let component_name = extract_vue_component_name(artifact.code.as_str()).unwrap_or_else(|| "UserComponent".to_string());
                if let Some(window) = app_handle.get_webview_window("artifact") {
                    let _ = window.emit(
                        "artifact-data",
                        serde_json::json!({
                            "id": artifact.id,
                            "name": artifact.name,
                            "icon": artifact.icon,
                            "description": artifact.description,
                            "type": "vue",
                            "original_code": artifact.code.as_str(),
                            "tags": artifact.tags,
                            "created_time": artifact.created_time,
                            "last_used_time": artifact.last_used_time,
                            "use_count": artifact.use_count,
                        }),
                    );
                }
                let _url = run_vue_artifact(
                    app_handle.clone(),
                    artifact_id,
                    artifact.code.as_str().to_string(),
                    component_name,
                )
                .await
                .map_err(|e| {
                    let error_msg = format!("Vue 组件运行失败: {}", e);
                    if let Some(window) = app_handle.get_webview_window("artifact") { let _ = window.emit("artifact-error", &error_msg); }
                    error_msg
                })?;
            } else if let Some(window) = app_handle.get_webview_window("artifact") {
                let _ = window.emit("artifact-error", "Vue 代码片段预览暂不支持，请提供完整的 Vue 组件代码。");
            }
        }
        "html" => {
            if let Some(window) = app_handle.get_webview_window("artifact") {
                let _ = window.emit("artifact-log", format!("准备预览 {} 内容...", "html"));
                let _ = window.emit(
                    "artifact-data",
                    serde_json::json!({
                        "id": artifact.id,
                        "name": artifact.name,
                        "icon": artifact.icon,
                        "description": artifact.description,
                        "type": "html",
                        "original_code": artifact.code.as_str(),
                        "tags": artifact.tags,
                        "created_time": artifact.created_time,
                        "last_used_time": artifact.last_used_time,
                        "use_count": artifact.use_count,
                    }),
                );
                let _ = window.emit("artifact-log", format!("html content: {}", artifact.code.as_str()));
                let _ = window.emit("artifact-success", format!("{} 预览已准备完成", "HTML"));
            }
        }
        _ => {}
    }
    Ok(())
}

#[tauri::command]
pub fn get_artifacts_statistics(
    app_handle: tauri::AppHandle,
) -> Result<ArtifactStatistics, String> {
    let db = ArtifactsDatabase::new(&app_handle)
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let (total_count, total_uses) =
        db.get_statistics().map_err(|e| format!("Failed to get statistics: {}", e))?;

    Ok(ArtifactStatistics { total_count, total_uses })
}

#[tauri::command]
pub fn get_artifacts_for_completion(
    app_handle: tauri::AppHandle,
) -> Result<Vec<ArtifactCollectionItem>, String> {
    get_artifacts_collection(app_handle, None)
}

#[tauri::command]
pub async fn generate_artifact_metadata(
    app_handle: tauri::AppHandle,
    feature_config_state: State<'_, FeatureConfigState>,
    artifact_type: String,
    code: String,
) -> Result<ArtifactMetadata, String> {
    let config_feature_map = feature_config_state.config_feature_map.lock().await;
    let feature_config = config_feature_map.get("conversation_summary");

    if let Some(config) = feature_config {
        let (provider_id, model_code) = if let Some(form_model) = config.get("form_autofill_model") {
            let form_model_value = &form_model.value;
            if form_model_value.contains("%%") && !form_model_value.starts_with("%%") {
                let parts: Vec<&str> = form_model_value.split("%%").collect();
                if parts.len() == 2 {
                    let provider_id = parts[0]
                        .parse::<i64>()
                        .map_err(|e| format!("表单填写模型provider_id解析失败: {}", e))?;
                    (provider_id, parts[1].to_string())
                } else { return Err("表单填写模型配置格式错误".to_string()); }
            } else { return Err("表单填写模型未配置，请在设置 -> 功能助手配置 -> AI总结 中配置表单填写模型".to_string()); }
        } else {
            return Err("表单填写模型未配置，请在设置 -> 功能助手配置 -> AI总结 中配置表单填写模型".to_string());
        };

        let system_prompt = r#"你是一个专业的代码分析助手。提供的代码是某个工具的核心组件，请根据代码对该工具生成适当的元数据。
你需要返回一个JSON格式的响应，包含以下字段：
{
  "name": "简洁的名称（中文，不超过10字）",
  "description": "对该工具的描述（中文，5-30字）",
  "tags": "相关标签（用英文逗号分隔）",
  "emoji_category": "emoji类型（选择以下之一：smileys_emotion, people_body, activities, objects, symbols, animals, food_drink, travel）"
}

请确保：
- 名称要简洁明了，体现代码的核心功能
- 描述要准确说明代码的用途和特点
- 标签要相关且有用，帮助分类和搜索
- emoji_category要根据代码内容选择最合适的类别"#;

        let user_prompt = format!("代码类型：{}\n\n代码内容：\n{}\n\n请分析上述代码并生成相应的元数据。", artifact_type, code);

        let llm_db = LLMDatabase::new(&app_handle).map_err(|e| format!("数据库连接失败: {}", e))?;
        let model_detail = llm_db
            .get_llm_model_detail(&provider_id, &model_code)
            .map_err(|e| format!("模型详情获取失败: {}", e))?;

        let network_proxy = get_network_proxy_from_config(&config_feature_map);
        let request_timeout = get_request_timeout_from_config(&config_feature_map);
        let proxy_enabled = false;

        let client = genai_client::create_client_with_config(
            &model_detail.configs,
            &model_detail.model.code,
            &model_detail.provider.api_type,
            network_proxy.as_deref(),
            proxy_enabled,
            Some(request_timeout),
        )
        .map_err(|e| format!("AI客户端创建失败: {}", e))?;

        let chat_messages = vec![
            genai::chat::ChatMessage::system(system_prompt),
            genai::chat::ChatMessage::user(&user_prompt),
        ];
        let chat_request = genai::chat::ChatRequest::new(chat_messages);
        let model_name = &model_detail.model.code;

        let response = client
            .exec_chat(model_name, chat_request.clone(), None)
            .await
            .map_err(|e| format!("AI请求失败: {}", e))?;

        let response_text = response.first_text().unwrap_or("").to_string();

        match serde_json::from_str::<ArtifactMetadata>(&response_text) {
            Ok(metadata) => Ok(metadata),
            Err(_) => {
                if let Some(json_start) = response_text.find('{') {
                    if let Some(json_end) = response_text.rfind('}') {
                        let json_part = &response_text[json_start..=json_end];
                        match serde_json::from_str::<ArtifactMetadata>(json_part) {
                            Ok(metadata) => Ok(metadata),
                            Err(_) => Ok(ArtifactMetadata {
                                name: format!("{} 代码", artifact_type),
                                description: format!("一个 {} 类型的代码片段，包含丰富的功能实现。", artifact_type),
                                tags: format!("{},代码,开发,工具", artifact_type.to_lowercase()),
                                emoji_category: "物品".to_string(),
                            }),
                        }
                    } else { Err("无法从AI响应中提取有效的JSON格式".to_string()) }
                } else { Err("AI响应不包含JSON格式的内容".to_string()) }
            }
        }
    } else {
        Err("conversation_summary配置未找到".to_string())
    }
}
