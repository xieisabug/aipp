use crate::db::artifacts_collection_db::{
    ArtifactCollection, ArtifactsCollectionDatabase, NewArtifactCollection, UpdateArtifactCollection,
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
    let db = ArtifactsCollectionDatabase::new(&app_handle)
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

    // Emit event to update UI
    if let Some(window) = app_handle.get_webview_window("artifact_collections") {
        let _ = window.emit("artifact-saved", artifact_id);
    }

    Ok(artifact_id)
}

/// Get all artifacts with optional type filter
#[tauri::command]
pub fn get_artifacts_collection(
    app_handle: tauri::AppHandle,
    artifact_type: Option<String>,
) -> Result<Vec<ArtifactCollectionItem>, String> {
    let db = ArtifactsCollectionDatabase::new(&app_handle)
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
    let db = ArtifactsCollectionDatabase::new(&app_handle)
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
    let db = ArtifactsCollectionDatabase::new(&app_handle)
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
    let db = ArtifactsCollectionDatabase::new(&app_handle)
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

    // Emit event to update UI
    if let Some(window) = app_handle.get_webview_window("artifact_collections") {
        let _ = window.emit("artifact-updated", request.id);
    }

    Ok(())
}

/// Delete artifact by ID
#[tauri::command]
pub fn delete_artifact_collection(
    app_handle: tauri::AppHandle,
    id: i64,
) -> Result<bool, String> {
    let db = ArtifactsCollectionDatabase::new(&app_handle)
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let deleted = db
        .delete_artifact(id)
        .map_err(|e| format!("Failed to delete artifact: {}", e))?;

    if deleted {
        // Emit event to update UI
        if let Some(window) = app_handle.get_webview_window("artifact_collections") {
            let _ = window.emit("artifact-deleted", id);
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
    let db = ArtifactsCollectionDatabase::new(&app_handle)
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
    crate::window::open_artifact_window(app_handle, artifact).await?;

    Ok(())
}

/// Get artifacts statistics
#[tauri::command]
pub fn get_artifacts_statistics(app_handle: tauri::AppHandle) -> Result<ArtifactStatistics, String> {
    let db = ArtifactsCollectionDatabase::new(&app_handle)
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