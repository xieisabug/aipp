use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tauri::Emitter;
use tauri::Manager;

use crate::utils::bun_utils::BunUtils;
use crate::FeatureConfigState;

use crate::{
    artifacts::{
        applescript::run_applescript,
        powershell::run_powershell,
        react_preview::{create_react_preview, create_react_preview_for_artifact},
    },
    errors::AppError,
    window::{open_preview_html_window, open_preview_react_window, open_preview_vue_window},
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

    let nextjs_port = preview_config
        .get("nextjs_port")
        .and_then(|config| config.value.parse::<u16>().ok())
        .unwrap_or(3001); // 默认端口如果解析失败

    let nuxtjs_port = preview_config
        .get("nuxtjs_port")
        .and_then(|config| config.value.parse::<u16>().ok())
        .unwrap_or(3002); // 默认端口如果解析失败

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
        "xml" | "svg" | "html" => {
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-log", format!("准备预览 {} 内容...", lang));
            }
            let _ = open_preview_html_window(app_handle.clone(), input_str.to_string()).await;
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-success", "HTML/SVG/XML 预览已准备完成");
            }
            // 发送跳转事件，让前端窗口自动跳转
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-redirect", "预览已准备完成");
            }
        }
        "react" | "jsx" => {
            println!("🎯 [Artifacts] 处理 React/JSX 代码");

            // 检查是否是完整的组件代码
            if is_react_component(input_str) {
                println!("🎯 [Artifacts] 检测到完整的 React 组件，使用新预览");

                // 使用新的 React Component Preview
                let component_name = extract_component_name(input_str).unwrap_or_else(|| {
                    println!("🎯 [Artifacts] 无法提取组件名称，使用默认名称");
                    "UserComponent".to_string()
                });
                println!("🎯 [Artifacts] 组件名称: {}", component_name);

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
                println!("🎯 [Artifacts] 检测到代码片段，使用旧预览方式");
                if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("artifact-log", "检测到代码片段，使用传统预览方式...");
                }

                // 使用旧的预览方式（代码片段）
                let _ = open_preview_react_window(
                    app_handle.clone(),
                    input_str.to_string(),
                    nextjs_port,
                )
                .await;
                if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("artifact-success", "React 代码片段预览已准备完成");
                }
                // 发送跳转事件
                let preview_url = format!(
                    "http://preview.teafakedomain.com:{}/previews/react",
                    nextjs_port
                );
                if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("artifact-redirect", preview_url);
                }
            }
        }
        "vue" => {
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-log", "准备 Vue 预览...");
            }
            let _ = open_preview_vue_window(app_handle.clone(), input_str.to_string(), nuxtjs_port)
                .await;
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-success", "Vue 预览已准备完成");
            }
            // 发送跳转事件
            let preview_url = format!(
                "http://preview.teafakedomain.com:{}/previews/vue",
                nuxtjs_port
            );
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-redirect", preview_url);
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
pub fn check_uv_version() -> Result<String, String> {
    match Command::new("uv").arg("--version").output() {
        Ok(output) => {
            if output.status.success() {
                let version_info = String::from_utf8_lossy(&output.stdout).to_string();
                // Example output: `uv 0.2.8`
                let version = version_info.to_string();
                Ok(version)
            } else {
                Ok("Not Installed".to_string())
            }
        }
        Err(_) => Ok("Not Installed".to_string()),
    }
}

#[tauri::command]
pub fn install_bun(window: tauri::Window) -> Result<(), String> {
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

        window.emit("bun-install-log", "开始下载 Bun").unwrap();

        // 使用应用数据目录作为安装位置，避免依赖第三方目录库
        let app_data_dir = window
            .app_handle()
            .path()
            .app_data_dir()
            .expect("无法获取应用数据目录");
        let bun_install_dir = app_data_dir.join("bun");
        let bun_bin_dir = bun_install_dir.join("bin");
        std::fs::create_dir_all(&bun_bin_dir).unwrap();

        let zip_path = bun_install_dir.join("bun.zip");

        // Download
        let mut response = reqwest::blocking::get(&url).unwrap();
        let mut file = std::fs::File::create(&zip_path).unwrap();
        std::io::copy(&mut response, &mut file).unwrap();
        window.emit("bun-install-log", "下载完成.").unwrap();

        // Unzip 到安装目录
        window.emit("bun-install-log", "开始解压...").unwrap();
        let zip_file = std::fs::File::open(&zip_path).unwrap();
        zip_extract::extract(zip_file, &bun_install_dir, true).unwrap();
        window.emit("bun-install-log", "解压成功.").unwrap();

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

        let bun_executable_path = candidate_paths
            .iter()
            .find(|p| p.exists())
            .expect("未找到 bun 可执行文件")
            .to_path_buf();

        let dest_path = bun_bin_dir.join(&bun_executable_name);
        // 如果目标已存在则先删除
        if dest_path.exists() {
            std::fs::remove_file(&dest_path).unwrap();
        }
        std::fs::rename(bun_executable_path, &dest_path).unwrap();

        window.emit("bun-install-log", "Bun 安装成功.").unwrap();
        window.emit("bun-install-finished", true).unwrap();
    });

    Ok(())
}

#[tauri::command]
pub fn install_uv(window: tauri::Window) -> Result<(), String> {
    std::thread::spawn(move || {
        let max_retries = 3;
        let mut success = false;
        
        for attempt in 1..=max_retries {
            let log_msg = format!("正在尝试安装 uv (第 {} 次尝试)...", attempt);
            println!("uv-install-log: {}", log_msg);
            window.emit("uv-install-log", log_msg).unwrap();
            
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
                    println!("uv-install-log: {}", error_msg);
                    window.emit("uv-install-log", error_msg).unwrap();
                    continue;
                }
            };

            let mut has_critical_error = false;

            if let Some(stdout) = child.stdout.take() {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        println!("uv-install-log: {}", line);
                        window.emit("uv-install-log", line).unwrap();
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
                        
                        println!("uv-install-log: {}", line);
                        window.emit("uv-install-log", line).unwrap();
                    }
                }
            }

            match child.wait() {
                Ok(status) => {
                    // 即使退出码成功，但如果有关键错误，也需要重试
                    if status.success() && !has_critical_error {
                        success = true;
                        let success_msg = "uv 安装成功！";
                        println!("uv-install-log: {}", success_msg);
                        window.emit("uv-install-log", success_msg).unwrap();
                        break;
                    } else {
                        let error_msg = if has_critical_error {
                            format!("第 {} 次尝试失败：检测到网络错误", attempt)
                        } else {
                            format!("第 {} 次尝试失败，退出码: {}", attempt, status.code().unwrap_or(-1))
                        };
                        println!("uv-install-log: {}", error_msg);
                        window.emit("uv-install-log", error_msg).unwrap();
                        
                        if attempt < max_retries {
                            let retry_msg = format!("等待 2 秒后重试...");
                            println!("uv-install-log: {}", retry_msg);
                            window.emit("uv-install-log", retry_msg).unwrap();
                            std::thread::sleep(std::time::Duration::from_secs(2));
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("等待进程失败: {}", e);
                    println!("uv-install-log: {}", error_msg);
                    window.emit("uv-install-log", error_msg).unwrap();
                }
            }
        }

        if !success {
            let final_error = format!("经过 {} 次尝试后，uv 安装失败", max_retries);
            println!("uv-install-log: {}", final_error);
            window.emit("uv-install-log", final_error).unwrap();
        }

        println!("uv-install-finished: {}", success);
        window
            .emit("uv-install-finished", success)
            .unwrap();
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
