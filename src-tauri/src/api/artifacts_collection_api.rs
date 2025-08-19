use crate::{
    api::artifacts_api::{
        is_react_component, extract_component_name, is_vue_component, extract_vue_component_name
    },
    artifacts::{
        react_runner::run_react_artifact,
        vue_runner::run_vue_artifact,
    },
    db::artifacts_db::{
        ArtifactCollection, ArtifactsDatabase, NewArtifactCollection, UpdateArtifactCollection,
    }, 
    utils::bun_utils::BunUtils
};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

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

/// Save a new artifact to collection
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

    let artifact_id = db
        .save_artifact(new_artifact)
        .map_err(|e| format!("Failed to save artifact: {}", e))?;

    // Emit events to update UI across all windows
    let windows_to_notify = ["artifact_collections", "ask", "chat_ui"];
    for window_name in windows_to_notify.iter() {
        if let Some(window) = app_handle.get_webview_window(window_name) {
            let _ = window.emit("artifact-collection-updated", artifact_id);
        }
    }

    Ok(artifact_id)
}

/// Get all artifacts with optional type filter
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

/// Get artifact by ID with full code content
#[tauri::command]
pub fn get_artifact_by_id(
    app_handle: tauri::AppHandle,
    id: i64,
) -> Result<Option<ArtifactCollection>, String> {
    let db = ArtifactsDatabase::new(&app_handle)
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let artifact = db
        .get_artifact_by_id(id)
        .map_err(|e| format!("Failed to get artifact: {}", e))?;

    Ok(artifact)
}

/// Search artifacts by name, description, or tags
#[tauri::command]
pub fn search_artifacts_collection(
    app_handle: tauri::AppHandle,
    query: String,
) -> Result<Vec<ArtifactCollectionItem>, String> {
    let db = ArtifactsDatabase::new(&app_handle)
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let artifacts = db
        .search_artifacts(&query)
        .map_err(|e| format!("Failed to search artifacts: {}", e))?;

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

/// Update artifact metadata (name, icon, description, tags)
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

    db.update_artifact(update)
        .map_err(|e| format!("Failed to update artifact: {}", e))?;

    // Emit events to update UI across all windows
    let windows_to_notify = ["artifact_collections", "ask", "chat_ui"];
    for window_name in windows_to_notify.iter() {
        if let Some(window) = app_handle.get_webview_window(window_name) {
            let _ = window.emit("artifact-collection-updated", request.id);
        }
    }
    
    Ok(())
}

/// Delete artifact by ID
#[tauri::command]
pub fn delete_artifact_collection(
    app_handle: tauri::AppHandle,
    id: i64,
) -> Result<bool, String> {
    let db = ArtifactsDatabase::new(&app_handle)
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let deleted = db
        .delete_artifact(id)
        .map_err(|e| format!("Failed to delete artifact: {}", e))?;

    if deleted {
        // Emit events to update UI across all windows
        let windows_to_notify = ["artifact_collections", "ask", "chat_ui"];
        for window_name in windows_to_notify.iter() {
            if let Some(window) = app_handle.get_webview_window(window_name) {
                let _ = window.emit("artifact-collection-updated", id);
            }
        }
    }

    Ok(deleted)
}

/// Open artifact in dedicated window
#[tauri::command]
pub async fn open_artifact_window(
    app_handle: tauri::AppHandle,
    artifact_id: i64,
) -> Result<(), String> {
    let db = ArtifactsDatabase::new(&app_handle)
        .map_err(|e| format!("Database connection failed: {}", e))?;

    // Get artifact details
    let artifact = db
        .get_artifact_by_id(artifact_id)
        .map_err(|e| format!("Failed to get artifact: {}", e))?
        .ok_or_else(|| "Artifact not found".to_string())?;

    // Increment use count
    db.increment_use_count(artifact_id)
        .map_err(|e| format!("Failed to increment use count: {}", e))?;

    // Open artifact window
    crate::window::open_artifact_window(app_handle.clone(), artifact.clone()).await?;

    // 等待窗口加载（延长到 1 秒，避免日志在窗口完成加载前发送导致丢失）
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    match artifact.artifact_type.as_str() {
        "react" | "jsx" => {
            println!("🎯 [Artifacts] 处理 React/JSX 代码");

            // 检查是否是完整的组件代码
            if is_react_component(artifact.code.as_str()) {
                println!("🎯 [Artifacts] 检测到完整的 React 组件，使用新预览");

                // 使用新的 React Component Preview
                let component_name = extract_component_name(artifact.code.as_str()).unwrap_or_else(|| {
                    println!("🎯 [Artifacts] 无法提取组件名称，使用默认名称");
                    "UserComponent".to_string()
                });
                println!("🎯 [Artifacts] 组件名称: {}", component_name);
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

                let preview_url = run_react_artifact(
                    app_handle.clone(),
                    artifact_id,
                    artifact.code.as_str().to_string(),
                    component_name,
                )
                .await
                .map_err(|e| {
                    println!("❌ [Artifacts] React 组件运行失败: {}", e);
                    let error_msg = format!("React 组件运行失败: {}", e);
                    if let Some(window) = app_handle.get_webview_window("artifact") {
                        let _ = window.emit("artifact-error", &error_msg);
                    }
                    error_msg
                })?;

                println!("✅ React 组件已启动，访问地址: {}", preview_url);
            } else {
                if let Some(window) = app_handle.get_webview_window("artifact") {
                    let _ = window.emit(
                        "artifact-error",
                        "React 代码片段预览暂不支持，请提供完整的 React 组件代码。",
                    );
                }
            }
        }
        "vue" => {
            println!("🎯 [Artifacts] 处理 Vue 代码");

            // 检查是否需要 bun 环境
            let bun_version = BunUtils::get_bun_version(&app_handle);
            if bun_version.is_err()
                || bun_version
                    .as_ref()
                    .unwrap_or(&String::new())
                    .contains("Not Installed")
            {
                println!("🎯 [Artifacts] 检测到需要 bun 环境但未安装");
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

            // 检查是否是完整的组件代码
            if is_vue_component(artifact.code.as_str()) {
                println!("🎯 [Artifacts] 检测到完整的 Vue 组件，使用新预览");

                // 使用新的 Vue Component Preview
                let component_name = extract_vue_component_name(artifact.code.as_str()).unwrap_or_else(|| {
                    println!("🎯 [Artifacts] 无法提取组件名称，使用默认名称");
                    "UserComponent".to_string()
                });
                println!("🎯 [Artifacts] 组件名称: {}", component_name);
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
                let preview_url = run_vue_artifact(
                    app_handle.clone(),
                    artifact_id,
                    artifact.code.as_str().to_string(),
                    component_name,
                )
                .await
                .map_err(|e| {
                    println!("❌ [Artifacts] Vue 组件运行失败: {}", e);
                    let error_msg = format!("Vue 组件运行失败: {}", e);
                    if let Some(window) = app_handle.get_webview_window("artifact") {
                        let _ = window.emit("artifact-error", &error_msg);
                    }
                    error_msg
                })?;

                println!("✅ Vue 组件已启动，访问地址: {}", preview_url);
            } else {
                if let Some(window) = app_handle.get_webview_window("artifact") {
                    let _ = window.emit(
                        "artifact-error",
                        "Vue 代码片段预览暂不支持，请提供完整的 Vue 组件代码。",
                    );
                }
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
                let _ = window.emit(
                    "artifact-success",
                    format!("{} 预览已准备完成", "HTML"),
                );
            }
        }
        _ => {
            // Handle other artifact types if needed, or do nothing
        }
    }


    Ok(())
}

/// Get artifacts statistics
#[tauri::command]
pub fn get_artifacts_statistics(app_handle: tauri::AppHandle) -> Result<ArtifactStatistics, String> {
    let db = ArtifactsDatabase::new(&app_handle)
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let (total_count, total_uses) = db
        .get_statistics()
        .map_err(|e| format!("Failed to get statistics: {}", e))?;

    Ok(ArtifactStatistics {
        total_count,
        total_uses,
    })
}

/// Get artifacts for completion suggestions (used by InputArea # trigger)
#[tauri::command]
pub fn get_artifacts_for_completion(
    app_handle: tauri::AppHandle,
) -> Result<Vec<ArtifactCollectionItem>, String> {
    // Return artifacts sorted by use frequency for completion
    get_artifacts_collection(app_handle, None)
}