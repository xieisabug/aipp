use tauri::{Emitter, Manager};

use crate::artifacts::code_utils::{
    extract_component_name, extract_vue_component_name, is_react_component, is_vue_component,
};
use crate::artifacts::react_preview::{create_react_preview, create_react_preview_for_artifact};
use crate::artifacts::vue_preview::create_vue_preview_for_artifact;
use crate::artifacts::{applescript::run_applescript, powershell::run_powershell};
use crate::errors::AppError;
use crate::utils::bun_utils::BunUtils;

#[tauri::command]
pub async fn run_artifacts(
    app_handle: tauri::AppHandle,
    lang: &str,
    input_str: &str,
) -> Result<String, AppError> {
    let _ = crate::window::open_artifact_preview_window(app_handle.clone()).await;
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    match lang {
        "powershell" => {
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-preview-log", "执行 PowerShell 脚本...");
            }
            return Ok(run_powershell(input_str).map_err(|e| {
                let error_msg = "PowerShell 脚本执行失败:".to_owned() + &e.to_string();
                if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("artifact-preview-error", &error_msg);
                }
                AppError::RunCodeError(error_msg)
            })?);
        }
        "applescript" => {
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-preview-log", "执行 AppleScript 脚本...");
            }
            return Ok(run_applescript(input_str).map_err(|e| {
                let error_msg = "AppleScript 脚本执行失败:".to_owned() + &e.to_string();
                if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("artifact-preview-error", &error_msg);
                }
                AppError::RunCodeError(error_msg)
            })?);
        }
        "mermaid" => {
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-preview-log", "准备预览 Mermaid 图表...");
            }
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit(
                    "artifact-preview-data",
                    serde_json::json!({ "type": "mermaid", "original_code": input_str }),
                );
                let _ = window.emit("artifact-preview-log", format!("mermaid content: {}", input_str));
                let _ = window.emit("artifact-preview-success", "Mermaid 图表预览已准备完成");
            }
        }
        "xml" | "svg" | "html" | "markdown" | "md" => {
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-preview-log", format!("准备预览 {} 内容...", lang));
                let _ = window.emit(
                    "artifact-preview-data",
                    serde_json::json!({ "type": lang, "original_code": input_str }),
                );
                let _ = window.emit("artifact-preview-log", format!("{} content: {}", lang, input_str));
                let _ = window.emit("artifact-preview-success", format!("{} 预览已准备完成", lang.to_uppercase()));
            }
        }
        "react" | "jsx" => {
            let bun_version = BunUtils::get_bun_version(&app_handle);
            if bun_version.is_err() || bun_version.as_ref().unwrap_or(&String::new()).contains("Not Installed") {
                if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("environment-check", serde_json::json!({
                        "tool": "bun",
                        "message": "React 预览需要 bun 环境，但系统中未安装 bun。是否要自动安装？",
                        "lang": lang,
                        "input_str": input_str
                    }));
                }
                return Ok("等待用户确认安装环境".to_string());
            }

            if is_react_component(input_str) {
                let component_name = extract_component_name(input_str).unwrap_or_else(|| "UserComponent".to_string());
                if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("artifact-preview-data", serde_json::json!({ "type": "react", "original_code": input_str }));
                }
                let preview_id = create_react_preview_for_artifact(app_handle.clone(), input_str.to_string(), component_name).await.map_err(|e| {
                    let error_msg = format!("React 组件预览失败: {}", e);
                    if let Some(window) = app_handle.get_webview_window("artifact_preview") { let _ = window.emit("artifact-preview-error", &error_msg); }
                    AppError::RunCodeError(error_msg)
                })?;
                return Ok(format!("React 组件预览已启动，预览 ID: {}", preview_id));
            } else {
                if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("artifact-preview-error", "React 代码片段预览暂不支持，请提供完整的 React 组件代码。");
                }
            }
        }
        "vue" => {
            let bun_version = BunUtils::get_bun_version(&app_handle);
            if bun_version.is_err() || bun_version.as_ref().unwrap_or(&String::new()).contains("Not Installed") {
                if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("environment-check", serde_json::json!({
                        "tool": "bun",
                        "message": "Vue 预览需要 bun 环境，但系统中未安装 bun。是否要自动安装？",
                        "lang": lang,
                        "input_str": input_str
                    }));
                }
                return Ok("等待用户确认安装环境".to_string());
            }

            if is_vue_component(input_str) {
                let component_name = extract_vue_component_name(input_str).unwrap_or_else(|| "UserComponent".to_string());
                if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("artifact-preview-data", serde_json::json!({ "type": "vue", "original_code": input_str }));
                }
                let preview_id = create_vue_preview_for_artifact(app_handle.clone(), input_str.to_string(), component_name).await.map_err(|e| {
                    let error_msg = format!("Vue 组件预览失败: {}", e);
                    if let Some(window) = app_handle.get_webview_window("artifact_preview") { let _ = window.emit("artifact-preview-error", &error_msg); }
                    AppError::RunCodeError(error_msg)
                })?;
                return Ok(format!("Vue 组件预览已启动，预览 ID: {}", preview_id));
            } else {
                if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("artifact-preview-error", "Vue 代码片段预览暂不支持，请提供完整的 Vue 组件代码。");
                }
            }
        }
        _ => {
            let error_msg = "暂不支持该语言的代码执行".to_owned();
            if let Some(window) = app_handle.get_webview_window("artifact_preview") { let _ = window.emit("artifact-preview-error", &error_msg); }
            return Err(AppError::RunCodeError(error_msg));
        }
    }
    Ok(String::new())
}

#[tauri::command]
pub async fn preview_react_component(
    app_handle: tauri::AppHandle,
    component_code: String,
    component_name: Option<String>,
) -> Result<String, String> {
    let name = component_name.unwrap_or_else(|| {
        extract_component_name(&component_code).unwrap_or_else(|| "UserComponent".to_string())
    });
    create_react_preview(app_handle, component_code, name).await
}

#[tauri::command]
pub async fn confirm_environment_install(
    app_handle: tauri::AppHandle,
    tool: String,
    confirmed: bool,
    lang: String,
    input_str: String,
) -> Result<String, String> {
    if !confirmed {
        if let Some(window) = app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("artifact-preview-error", "用户取消了环境安装，预览已停止");
        }
        return Ok("用户取消安装".to_string());
    }

    if let Some(window) = app_handle.get_webview_window("artifact_preview") {
        let _ = window.emit("artifact-preview-log", format!("开始安装 {} 环境...", tool));
        if tool == "bun" { let _ = crate::artifacts::env_installer::install_bun(app_handle.clone(), Some("artifact_preview".to_string())); }
        else if tool == "uv" { let _ = crate::artifacts::env_installer::install_uv(app_handle.clone(), Some("artifact_preview".to_string())); }
        if let Some(window) = app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit(
                "environment-install-started",
                serde_json::json!({ "tool": tool, "lang": lang, "input_str": input_str }),
            );
        }
    }
    Ok("开始安装环境".to_string())
}

#[tauri::command]
pub async fn retry_preview_after_install(
    app_handle: tauri::AppHandle,
    lang: String,
    input_str: String,
) -> Result<String, String> {
    match run_artifacts(app_handle.clone(), &lang, &input_str).await {
        Ok(result) => Ok(result),
        Err(e) => Err(e.to_string()),
    }
}
