use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tauri::Emitter;
use tauri::Manager;

use crate::utils::bun_utils::BunUtils;
use crate::utils::uv_utils::UvUtils;
use crate::FeatureConfigState;

use crate::{
    artifacts::{
        applescript::run_applescript,
        powershell::run_powershell,
        react_preview::{create_react_preview, create_react_preview_for_artifact},
        vue_preview::create_vue_preview_for_artifact,
    },
    errors::AppError,
    window::{open_preview_react_window, open_preview_vue_window},
};

// 检查是否是完整的 React 组件代码
fn is_react_component(code: &str) -> bool {
    // 检查代码是否包含 React 组件的特征
    let has_import = code.contains("import") && (code.contains("react") || code.contains("React"));
    let has_function_component = code.contains("function ") && code.contains("return");
    let has_arrow_component =
        code.contains("const ") && code.contains("=>") && code.contains("return");
    let has_export = code.contains("export");

    // 检查是否包含 JSX 返回语句
    let has_jsx_return = code.contains("return (") || code.contains("return <");

    (has_import || has_export) && (has_function_component || has_arrow_component) && has_jsx_return
}

// 从代码中提取组件名称
fn extract_component_name(code: &str) -> Option<String> {
    use regex::Regex;

    // 尝试匹配函数组件名称
    if let Ok(re) = Regex::new(r"function\s+([A-Z][a-zA-Z0-9_]*)\s*\(") {
        if let Some(caps) = re.captures(code) {
            if let Some(name) = caps.get(1) {
                return Some(name.as_str().to_string());
            }
        }
    }

    // 尝试匹配箭头函数组件名称
    if let Ok(re) = Regex::new(r"const\s+([A-Z][a-zA-Z0-9_]*)\s*[=:]") {
        if let Some(caps) = re.captures(code) {
            if let Some(name) = caps.get(1) {
                return Some(name.as_str().to_string());
            }
        }
    }

    // 尝试匹配 export 的组件名称
    if let Ok(re) = Regex::new(r"export\s+(?:default\s+)?(?:function\s+)?([A-Z][a-zA-Z0-9_]*)") {
        if let Some(caps) = re.captures(code) {
            if let Some(name) = caps.get(1) {
                return Some(name.as_str().to_string());
            }
        }
    }

    None
}

// 检查是否是完整的 Vue 组件代码
fn is_vue_component(code: &str) -> bool {
    // 检查代码是否包含 Vue 组件的特征
    let has_template = code.contains("<template>");
    let has_script = code.contains("<script");
    let _has_style = code.contains("<style");
    
    // 检查是否有 Vue 3 Composition API 或者 Options API
    let has_setup = code.contains("setup") || code.contains("defineComponent");
    let has_export_default = code.contains("export default");
    
    // 至少要有 template 和 script 标签
    has_template && has_script && (has_setup || has_export_default)
}

// 从 Vue 组件代码中提取组件名称
fn extract_vue_component_name(code: &str) -> Option<String> {
    use regex::Regex;
    
    // 尝试匹配 export default 中的 name 属性
    if let Ok(re) = Regex::new(r#"name\s*:\s*['"]([A-Z][a-zA-Z0-9_]*)['"]"#) {
        if let Some(caps) = re.captures(code) {
            if let Some(name) = caps.get(1) {
                return Some(name.as_str().to_string());
            }
        }
    }
    
    // 尝试匹配 defineComponent 中的 name
    if let Ok(re) = Regex::new(r#"defineComponent\s*\(\s*\{\s*name\s*:\s*['"]([A-Z][a-zA-Z0-9_]*)['"]"#) {
        if let Some(caps) = re.captures(code) {
            if let Some(name) = caps.get(1) {
                return Some(name.as_str().to_string());
            }
        }
    }
    
    None
}

#[tauri::command]
pub async fn run_artifacts(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, FeatureConfigState>,
    lang: &str,
    input_str: &str,
) -> Result<String, AppError> {
    // Anthropic artifacts : code, markdown, html, svg, mermaid, react(引入了 lucid3-react, recharts, tailwind, shadcn/ui )
    // 加上 vue, nextjs 引入更多的前端库( echarts, antd, element-ui )

    // 先打开 artifact preview 窗口以显示友好的界面和日志
    let _ = crate::window::open_artifact_preview_window(app_handle.clone()).await;

    // 等待窗口加载（延长到 1 秒，避免日志在窗口完成加载前发送导致丢失）
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    let config_map = state.config_feature_map.lock().await;
    let preview_config = config_map
        .get("preview")
        .map(|c| c.to_owned())
        .unwrap_or_else(HashMap::new);

    match lang {
        "powershell" => {
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-log", "执行 PowerShell 脚本...");
            }
            return Ok(run_powershell(input_str).map_err(|e| {
                let error_msg = "PowerShell 脚本执行失败:".to_owned() + &e.to_string();
                if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("artifact-error", &error_msg);
                }
                AppError::RunCodeError(error_msg)
            })?);
        }
        "applescript" => {
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-log", "执行 AppleScript 脚本...");
            }
            return Ok(run_applescript(input_str).map_err(|e| {
                let error_msg = "AppleScript 脚本执行失败:".to_owned() + &e.to_string();
                if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("artifact-error", &error_msg);
                }
                AppError::RunCodeError(error_msg)
            })?);
        }
        "mermaid" => {
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-log", "准备预览 Mermaid 图表...");
            }
            
            // 发送 mermaid 内容到前端
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-data", serde_json::json!({
                    "type": "mermaid",
                    "original_code": input_str,
                }));
                let _ = window.emit("artifact-log", format!("mermaid content: {}", input_str));
                let _ = window.emit("artifact-success", "Mermaid 图表预览已准备完成");
            }
        }
        "xml" | "svg" | "html" | "markdown" | "md" => {
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-log", format!("准备预览 {} 内容...", lang));
                let _ = window.emit("artifact-data", serde_json::json!({
                    "type": lang,
                    "original_code": input_str,
                }));
                let _ = window.emit("artifact-log", format!("{} content: {}", lang, input_str));
                let _ = window.emit("artifact-success", format!("{} 预览已准备完成", lang.to_uppercase()));
            }
        }
        "react" | "jsx" => {
            println!("🎯 [Artifacts] 处理 React/JSX 代码");

            // 检查是否需要 bun 环境
            let bun_version = BunUtils::get_bun_version(&app_handle);
            if bun_version.is_err() || bun_version.as_ref().unwrap_or(&String::new()).contains("Not Installed") {
                println!("🎯 [Artifacts] 检测到需要 bun 环境但未安装");
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

            // 检查是否是完整的组件代码
            if is_react_component(input_str) {
                println!("🎯 [Artifacts] 检测到完整的 React 组件，使用新预览");

                // 使用新的 React Component Preview
                let component_name = extract_component_name(input_str).unwrap_or_else(|| {
                    println!("🎯 [Artifacts] 无法提取组件名称，使用默认名称");
                    "UserComponent".to_string()
                });
                println!("🎯 [Artifacts] 组件名称: {}", component_name);
                if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("artifact-data", serde_json::json!({
                        "type": "react",
                        "original_code": input_str,
                    }));
                }

                let preview_id = create_react_preview_for_artifact(
                    app_handle.clone(),
                    input_str.to_string(),
                    component_name,
                )
                .await
                .map_err(|e| {
                    println!("❌ [Artifacts] React 组件预览失败: {}", e);
                    let error_msg = format!("React 组件预览失败: {}", e);
                    if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                        let _ = window.emit("artifact-error", &error_msg);
                    }
                    AppError::RunCodeError(error_msg)
                })?;

                let success_msg = format!("React 组件预览已启动，预览 ID: {}", preview_id);

                return Ok(success_msg);
            } else {
                if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("artifact-error", "React 代码片段预览暂不支持，请提供完整的 React 组件代码。");
                }
            }
        }
        "vue" => {
            println!("🎯 [Artifacts] 处理 Vue 代码");

            // 检查是否需要 bun 环境
            let bun_version = BunUtils::get_bun_version(&app_handle);
            if bun_version.is_err() || bun_version.as_ref().unwrap_or(&String::new()).contains("Not Installed") {
                println!("🎯 [Artifacts] 检测到需要 bun 环境但未安装");
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

            // 检查是否是完整的组件代码
            if is_vue_component(input_str) {
                println!("🎯 [Artifacts] 检测到完整的 Vue 组件，使用新预览");

                // 使用新的 Vue Component Preview
                let component_name = extract_vue_component_name(input_str).unwrap_or_else(|| {
                    println!("🎯 [Artifacts] 无法提取组件名称，使用默认名称");
                    "UserComponent".to_string()
                });
                println!("🎯 [Artifacts] 组件名称: {}", component_name);
                if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("artifact-data", serde_json::json!({
                        "type": "vue",
                        "original_code": input_str,
                    }));
                }
                let preview_id = create_vue_preview_for_artifact(
                    app_handle.clone(),
                    input_str.to_string(),
                    component_name,
                )
                .await
                .map_err(|e| {
                    println!("❌ [Artifacts] Vue 组件预览失败: {}", e);
                    let error_msg = format!("Vue 组件预览失败: {}", e);
                    if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                        let _ = window.emit("artifact-error", &error_msg);
                    }
                    AppError::RunCodeError(error_msg)
                })?;

                let success_msg = format!("Vue 组件预览已启动，预览 ID: {}", preview_id);

                return Ok(success_msg);
            } else {
                if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("artifact-error", "Vue 代码片段预览暂不支持，请提供完整的 Vue 组件代码。");
                }
            }
        }
        _ => {
            // Handle other languages here
            let error_msg = "暂不支持该语言的代码执行".to_owned();
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-error", &error_msg);
            }
            return Err(AppError::RunCodeError(error_msg));
        }
    }
    Ok("".to_string())
}

#[tauri::command]
pub fn check_bun_version(app: tauri::AppHandle) -> Result<String, String> {
    BunUtils::get_bun_version(&app)
}

#[tauri::command]
pub fn check_uv_version(app: tauri::AppHandle) -> Result<String, String> {
    UvUtils::get_uv_version(&app)
}

#[tauri::command]
pub fn install_bun(app_handle: tauri::AppHandle, target_window: Option<String>) -> Result<(), String> {
    // 获取事件前缀和目标窗口
    let (event_prefix, emit_to_window) = if let Some(ref window) = target_window {
        if window == "artifact_preview" {
            ("artifact", true)
        } else {
            ("bun-install", true)
        }
    } else {
        ("bun-install", false)
    };
    
    std::thread::spawn(move || {
        let bun_version = "1.2.18";
        let (os, arch) = if cfg!(target_os = "windows") {
            ("windows", "x64")
        } else if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") {
                ("darwin", "aarch64")
            } else {
                ("darwin", "x64")
            }
        } else {
            // linux 不只x64
            ("linux", "x64")
        };

        let url = format!(
            "https://registry.npmmirror.com/-/binary/bun/bun-v{}/bun-{}-{}.zip",
            bun_version, os, arch
        );

        // 发送日志到指定窗口或全局
        let emit_log = |msg: &str| {
            if emit_to_window {
                if let Some(ref window_name) = target_window {
                    if let Some(window) = app_handle.get_webview_window(window_name) {
                        let _ = window.emit(&format!("{}-log", event_prefix), msg);
                    }
                }
            } else {
                let _ = app_handle.emit("bun-install-log", msg);
            }
        };

        let emit_error = |msg: &str| {
            if emit_to_window {
                if let Some(ref window_name) = target_window {
                    if let Some(window) = app_handle.get_webview_window(window_name) {
                        let _ = window.emit(&format!("{}-error", event_prefix), msg);
                        let _ = window.emit("bun-install-finished", false);
                    }
                }
            } else {
                let _ = app_handle.emit("bun-install-log", msg);
                let _ = app_handle.emit("bun-install-finished", false);
            }
        };

        let emit_success = |msg: &str| {
            if emit_to_window {
                if let Some(ref window_name) = target_window {
                    if let Some(window) = app_handle.get_webview_window(window_name) {
                        let _ = window.emit(&format!("{}-success", event_prefix), msg);
                        let _ = window.emit("bun-install-finished", true);
                    }
                }
            } else {
                let _ = app_handle.emit("bun-install-log", msg);
                let _ = app_handle.emit("bun-install-finished", true);
            }
        };

        emit_log("开始下载 Bun");

        // 使用应用数据目录作为安装位置，避免依赖第三方目录库
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .expect("无法获取应用数据目录");
        let bun_install_dir = app_data_dir.join("bun");
        let bun_bin_dir = bun_install_dir.join("bin");
        
        if let Err(e) = std::fs::create_dir_all(&bun_bin_dir) {
            emit_error(&format!("创建目录失败: {}", e));
            return;
        }

        let zip_path = bun_install_dir.join("bun.zip");

        // Download
        match reqwest::blocking::get(&url) {
            Ok(mut response) => {
                match std::fs::File::create(&zip_path) {
                    Ok(mut file) => {
                        if let Err(e) = std::io::copy(&mut response, &mut file) {
                            emit_error(&format!("下载失败: {}", e));
                            return;
                        }
                        emit_log("下载完成");
                    }
                    Err(e) => {
                        emit_error(&format!("创建文件失败: {}", e));
                        return;
                    }
                }
            }
            Err(e) => {
                emit_error(&format!("下载失败: {}", e));
                return;
            }
        }

        // Unzip 到安装目录
        emit_log("开始解压...");
        
        match std::fs::File::open(&zip_path) {
            Ok(zip_file) => {
                if let Err(e) = zip_extract::extract(zip_file, &bun_install_dir, true) {
                    emit_error(&format!("解压失败: {}", e));
                    return;
                }
                emit_log("解压成功");
            }
            Err(e) => {
                emit_error(&format!("打开压缩文件失败: {}", e));
                return;
            }
        }

        // Move executable
        let bun_executable_name = if cfg!(target_os = "windows") {
            "bun.exe"
        } else {
            "bun"
        };

        // 可能的可执行文件路径（不同版本的压缩包结构不同）
        let candidate_paths = [
            bun_install_dir.join(&bun_executable_name),
            bun_install_dir
                .join(format!("bun-{}-{}", os, arch))
                .join(&bun_executable_name),
        ];

        let bun_executable_path = match candidate_paths
            .iter()
            .find(|p| p.exists()) {
                Some(path) => path.to_path_buf(),
                None => {
                    emit_error("未找到 bun 可执行文件");
                    return;
                }
            };

        let dest_path = bun_bin_dir.join(&bun_executable_name);
        // 如果目标已存在则先删除
        if dest_path.exists() {
            if let Err(e) = std::fs::remove_file(&dest_path) {
                emit_error(&format!("删除旧文件失败: {}", e));
                return;
            }
        }
        
        if let Err(e) = std::fs::rename(bun_executable_path, &dest_path) {
            emit_error(&format!("移动文件失败: {}", e));
            return;
        }

        emit_success("Bun 安装成功");
    });

    Ok(())
}

#[tauri::command]
pub fn install_uv(app_handle: tauri::AppHandle, target_window: Option<String>) -> Result<(), String> {
    // 获取事件前缀和目标窗口
    let (event_prefix, emit_to_window) = if let Some(ref window) = target_window {
        if window == "artifact_preview" {
            ("artifact", true)
        } else {
            ("uv-install", true)
        }
    } else {
        ("uv-install", false)
    };

    std::thread::spawn(move || {
        let max_retries = 3;
        let mut success = false;
        
        // 发送日志到指定窗口或全局
        let emit_log = |msg: &str| {
            println!("uv-install-log: {}", msg);
            if emit_to_window {
                if let Some(ref window_name) = target_window {
                    if let Some(window) = app_handle.get_webview_window(window_name) {
                        let _ = window.emit(&format!("{}-log", event_prefix), msg);
                    }
                }
            } else {
                let _ = app_handle.emit("uv-install-log", msg);
            }
        };

        let emit_error = |msg: &str| {
            println!("uv-install-log: {}", msg);
            if emit_to_window {
                if let Some(ref window_name) = target_window {
                    if let Some(window) = app_handle.get_webview_window(window_name) {
                        let _ = window.emit(&format!("{}-error", event_prefix), msg);
                    }
                }
            } else {
                let _ = app_handle.emit("uv-install-log", msg);
            }
        };

        let emit_success = |msg: &str| {
            println!("uv-install-log: {}", msg);
            if emit_to_window {
                if let Some(ref window_name) = target_window {
                    if let Some(window) = app_handle.get_webview_window(window_name) {
                        let _ = window.emit(&format!("{}-success", event_prefix), msg);
                    }
                }
            } else {
                let _ = app_handle.emit("uv-install-log", msg);
            }
        };
        
        for attempt in 1..=max_retries {
            let log_msg = format!("正在尝试安装 uv (第 {} 次尝试)...", attempt);
            emit_log(&log_msg);
            
            let (command, args) = if cfg!(target_os = "windows") {
                (
                    "powershell",
                    vec!["-c", "irm https://astral.sh/uv/install.ps1 | iex"],
                )
            } else {
                (
                    "sh",
                    vec!["-c", "curl -LsSf --retry 3 --retry-delay 2 https://astral.sh/uv/install.sh | sh"],
                )
            };

            let mut child = match Command::new(command)
                .args(args)
                .env("UV_INSTALLER_GHE_BASE_URL", "https://ghfast.top/https://github.com")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(child) => child,
                Err(e) => {
                    let error_msg = format!("启动安装命令失败: {}", e);
                    emit_error(&error_msg);
                    continue;
                }
            };

            let mut has_critical_error = false;

            if let Some(stdout) = child.stdout.take() {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        emit_log(&line);
                    }
                }
            }

            if let Some(stderr) = child.stderr.take() {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        // 检查是否有关键错误
                        if line.contains("curl:") && (
                            line.contains("Error in the HTTP2 framing layer") ||
                            line.contains("Recv failure: Connection reset by peer") ||
                            line.contains("Failed to connect") ||
                            line.contains("Could not resolve host")
                        ) {
                            has_critical_error = true;
                        }
                        
                        emit_log(&line);
                    }
                }
            }

            match child.wait() {
                Ok(status) => {
                    // 即使退出码成功，但如果有关键错误，也需要重试
                    if status.success() && !has_critical_error {
                        success = true;
                        let success_msg = "uv 安装成功！";
                        emit_success(success_msg);
                        break;
                    } else {
                        let error_msg = if has_critical_error {
                            format!("第 {} 次尝试失败：检测到网络错误", attempt)
                        } else {
                            format!("第 {} 次尝试失败，退出码: {}", attempt, status.code().unwrap_or(-1))
                        };
                        emit_error(&error_msg);
                        
                        if attempt < max_retries {
                            let retry_msg = format!("等待 2 秒后重试...");
                            emit_log(&retry_msg);
                            std::thread::sleep(std::time::Duration::from_secs(2));
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("等待进程失败: {}", e);
                    emit_error(&error_msg);
                }
            }
        }

        if !success {
            let final_error = format!("经过 {} 次尝试后，uv 安装失败", max_retries);
            emit_error(&final_error);
        }

        println!("uv-install-finished: {}", success);
        // 发送安装完成事件
        if emit_to_window {
            if let Some(ref window_name) = target_window {
                if let Some(window) = app_handle.get_webview_window(window_name) {
                    let _ = window.emit("uv-install-finished", success);
                }
            }
        } else {
            let _ = app_handle.emit("uv-install-finished", success);
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn open_react_component_preview(app_handle: tauri::AppHandle) -> Result<(), String> {
    use crate::window::open_preview_frontend_window;
    open_preview_frontend_window(app_handle).await
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
        // 用户选择取消，停止预览
        if let Some(window) = app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("artifact-error", "用户取消了环境安装，预览已停止");
        }
        return Ok("用户取消安装".to_string());
    }

    // 用户确认安装，开始安装过程
    if let Some(window) = app_handle.get_webview_window("artifact_preview") {
        let _ = window.emit("artifact-log", format!("开始安装 {} 环境...", tool));
        
        if tool == "bun" {
            let _ = install_bun(app_handle.clone(), Some("artifact_preview".to_string()));
        } else if tool == "uv" {
            let _ = install_uv(app_handle.clone(), Some("artifact_preview".to_string()));
        }
        
        // 保存原始参数，等待安装完成后重新调用
        if let Some(window) = app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("environment-install-started", serde_json::json!({
                "tool": tool,
                "lang": lang,
                "input_str": input_str
            }));
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
    println!("🔧 [ArtifactsAPI] 收到重新启动预览事件: lang={}, input_str={}", lang, input_str);
    // 重新调用 run_artifacts 函数
    match run_artifacts(
        app_handle.clone(),
        app_handle.state::<FeatureConfigState>(),
        &lang,
        &input_str,
    ).await {
        Ok(result) => Ok(result),
        Err(e) => Err(e.to_string()),
    }
}

