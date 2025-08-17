use crate::db::artifacts_db::ArtifactCollection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::Emitter;
use tauri::Listener;
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri::{LogicalPosition, LogicalSize};

/// 当按照显示器大小调整窗口尺寸时保留的屏幕占比（90%）
const SCREEN_MARGIN_RATIO: f64 = 0.9;

// 获取合适的窗口大小和位置
fn get_window_size_and_position(
    app: &AppHandle,
    default_width: f64,
    default_height: f64,
    reference_window_labels: &[&str],
) -> (LogicalSize<f64>, Option<LogicalPosition<f64>>) {
    // 预设窗口尺寸
    let mut window_size = LogicalSize::new(default_width, default_height);

    // 优先寻找参考窗口所在的显示器
    let mut target_monitor = None;

    // 为提升效率，提前获取一次显示器列表（若失败留空）
    let monitors_cache = app.available_monitors().unwrap_or_default();

    // 先收集窗口并按“可见优先”排序
    let mut visible_windows = Vec::new();
    let mut hidden_windows = Vec::new();

    for label in reference_window_labels {
        if let Some(w) = app.get_webview_window(label) {
            if w.is_visible().unwrap_or(false) {
                visible_windows.push(w);
            } else {
                hidden_windows.push(w);
            }
        }
    }

    // 可见窗口 → 隐藏窗口 两轮查找
    let search_lists = [visible_windows, hidden_windows];

    'search: for list in &search_lists {
        for w in list {
            // 1. 尝试使用 current_monitor()
            if let Ok(Some(m)) = w.current_monitor() {
                target_monitor = Some(m.clone());
                break 'search;
            }

            // 2. 如果失败，再根据窗口坐标匹配显示器
            if let Ok(pos) = w.outer_position() {
                for m in &monitors_cache {
                    let mp = m.position();
                    let ms = m.size();
                    // 判断窗口左上角是否位于该显示器范围内
                    if pos.x >= mp.x
                        && pos.x < mp.x + ms.width as i32
                        && pos.y >= mp.y
                        && pos.y < mp.y + ms.height as i32
                    {
                        target_monitor = Some(m.clone());
                        break 'search;
                    }
                }
            }
        }
    }

    // 如果仍未找到，则采用 primary_monitor() 兜底
    if target_monitor.is_none() {
        if let Ok(Some(m)) = app.primary_monitor() {
            target_monitor = Some(m.clone());
        }
    }

    // 计算合适的窗口位置
    if let Some(monitor) = target_monitor {
        // 将物理尺寸转换为逻辑尺寸，避免 HiDPI 误差
        let scale = monitor.scale_factor() as f64;

        let screen_width = monitor.size().width as f64 / scale;
        let screen_height = monitor.size().height as f64 / scale;

        // 留出边距
        let max_width = screen_width * SCREEN_MARGIN_RATIO;
        let max_height = screen_height * SCREEN_MARGIN_RATIO;

        window_size.width = window_size.width.min(max_width);
        window_size.height = window_size.height.min(max_height);

        // 居中到目标显示器（逻辑坐标）
        let monitor_pos_x = monitor.position().x as f64 / scale;
        let monitor_pos_y = monitor.position().y as f64 / scale;

        // 取整避免亚像素导致系统自动纠偏
        let center_x = (monitor_pos_x + (screen_width - window_size.width) / 2.0).round();
        let center_y = (monitor_pos_y + (screen_height - window_size.height) / 2.0).round();

        return (window_size, Some(LogicalPosition::new(center_x, center_y)));
    }

    // 若所有方案均失败，交给窗口构建器自行居中
    (window_size, None)
}

pub fn create_ask_window(app: &AppHandle) {
    let window_builder =
        WebviewWindowBuilder::new(app, "ask", WebviewUrl::App("index.html".into()))
            .title("Aipp")
            .inner_size(800.0, 450.0)
            .fullscreen(false)
            .resizable(false)
            .decorations(false)
            .center();

    #[cfg(not(target_os = "macos"))]
    let window_builder = window_builder.transparent(true);

    match window_builder.build() {
        Ok(window) => {
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { .. } = event {
                    window_clone.hide().unwrap();
                }
            });
        }
        Err(e) => eprintln!("Failed to build window: {}", e),
    }
}

pub fn create_config_window(app: &AppHandle) {
    let (window_size, window_position) =
        get_window_size_and_position(app, 1300.0, 1000.0, &["ask", "chat_ui"]);

    let mut window_builder =
        WebviewWindowBuilder::new(app, "config", WebviewUrl::App("index.html".into()))
            .title("Aipp")
            .inner_size(window_size.width, window_size.height)
            .fullscreen(false)
            .resizable(true)
            .decorations(true);

    // macOS 若仍有偏差可考虑额外使用 parent(&window) 方案

    if let Some(position) = window_position {
        window_builder = window_builder.position(position.x, position.y);
    } else {
        window_builder = window_builder.center();
    }

    #[cfg(not(target_os = "macos"))]
    let window_builder = window_builder.transparent(false);

    match window_builder.build() {
        Ok(window) => {
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { .. } = event {
                    window_clone.hide().unwrap();
                }
            });
        }
        Err(e) => eprintln!("Failed to build window: {}", e),
    }
}

pub fn create_chat_ui_window(app: &AppHandle) {
    let (window_size, window_position) = get_window_size_and_position(app, 1000.0, 800.0, &["ask"]);

    let mut window_builder =
        WebviewWindowBuilder::new(app, "chat_ui", WebviewUrl::App("index.html".into()))
            .title("Aipp")
            .inner_size(window_size.width, window_size.height)
            .fullscreen(false)
            .resizable(true)
            .decorations(true)
            .disable_drag_drop_handler();

    // macOS 若仍有偏差可考虑额外使用 parent(&window) 方案

    if let Some(position) = window_position {
        window_builder = window_builder.position(position.x, position.y);
    } else {
        window_builder = window_builder.center();
    }

    #[cfg(not(target_os = "macos"))]
    let window_builder = window_builder.transparent(false);

    match window_builder.build() {
        Ok(window) => {
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { .. } = event {
                    window_clone.hide().unwrap();
                }
            });
            let _ = window.maximize();
        }
        Err(e) => eprintln!("Failed to build window: {}", e),
    }
}

pub fn create_plugin_window(app: &AppHandle) {
    let window_builder =
        WebviewWindowBuilder::new(app, "plugin", WebviewUrl::App("index.html".into()))
            .title("Aipp")
            .inner_size(1000.0, 800.0)
            .fullscreen(false)
            .resizable(true)
            .decorations(true)
            .center();

    #[cfg(not(target_os = "macos"))]
    let window_builder = window_builder.transparent(false);

    match window_builder.build() {
        Ok(window) => {
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { .. } = event {
                    window_clone.hide().unwrap();
                }
            });
        }
        Err(e) => eprintln!("Failed to build window: {}", e),
    }
}

pub fn create_artifact_preview_window(app: &AppHandle) {
    let (window_size, window_position) =
        get_window_size_and_position(app, 1000.0, 800.0, &["ask", "chat_ui"]);

    let mut window_builder =
        WebviewWindowBuilder::new(app, "artifact_preview", WebviewUrl::App("index.html".into()))
            .title("Artifact Preview - Aipp")
            .inner_size(window_size.width, window_size.height)
            .fullscreen(false)
            .resizable(true)
            .decorations(true);

    if let Some(position) = window_position {
        window_builder = window_builder.position(position.x, position.y);
    } else {
        window_builder = window_builder.center();
    }

    #[cfg(not(target_os = "macos"))]
    let window_builder = window_builder.transparent(false);

    match window_builder.build() {
        Ok(window) => {
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { .. } = event {
                    window_clone.hide().unwrap();
                }
            });
        }
        Err(e) => eprintln!("Failed to build window: {}", e),
    }
}

#[tauri::command]
pub async fn open_artifact_preview_window(app_handle: AppHandle) -> Result<(), String> {
    if app_handle.get_webview_window("artifact_preview").is_none() {
        println!("Creating artifact preview window");

        create_artifact_preview_window(&app_handle);
    } else if let Some(window) = app_handle.get_webview_window("artifact_preview") {
        println!("Showing artifact preview window");
        if window.is_minimized().unwrap_or(false) {
            window.unminimize().unwrap();
        }
        window.show().unwrap();
        window.set_focus().unwrap();
    }
    Ok(())
}

pub fn create_preview_frontend_window(app: &AppHandle) {
    let (window_size, window_position) =
        get_window_size_and_position(app, 1200.0, 900.0, &["ask", "chat_ui"]);

    let mut window_builder =
        WebviewWindowBuilder::new(app, "preview_frontend", WebviewUrl::App("index.html".into()))
            .title("Component Preview - Aipp")
            .inner_size(window_size.width, window_size.height)
            .fullscreen(false)
            .resizable(true)
            .decorations(true);

    if let Some(position) = window_position {
        window_builder = window_builder.position(position.x, position.y);
    } else {
        window_builder = window_builder.center();
    }

    #[cfg(not(target_os = "macos"))]
    let window_builder = window_builder.transparent(false);

    match window_builder.build() {
        Ok(window) => {
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { .. } = event {
                    window_clone.hide().unwrap();
                }
            });
        }
        Err(e) => eprintln!("Failed to build window: {}", e),
    }
}

#[tauri::command]
pub async fn open_config_window(app_handle: AppHandle) -> Result<(), String> {
    if app_handle.get_webview_window("config").is_none() {
        println!("Creating window");

        create_config_window(&app_handle)
    } else if let Some(window) = app_handle.get_webview_window("config") {
        println!("Showing window");
        if window.is_minimized().unwrap_or(false) {
            window.unminimize().unwrap();
        }
        window.show().unwrap();
        window.set_focus().unwrap();
    }
    Ok(())
}

#[tauri::command]
pub async fn open_chat_ui_window(app_handle: AppHandle) -> Result<(), String> {
    if app_handle.get_webview_window("chat_ui").is_none() {
        println!("Creating window");

        create_chat_ui_window(&app_handle);
        app_handle
            .get_webview_window("ask")
            .unwrap()
            .hide()
            .unwrap();
    } else if let Some(window) = app_handle.get_webview_window("chat_ui") {
        println!("Showing window");
        if window.is_minimized().unwrap_or(false) {
            window.unminimize().unwrap();
        }
        window.show().unwrap();
        window.set_focus().unwrap();
        app_handle
            .get_webview_window("ask")
            .unwrap()
            .hide()
            .unwrap();
    }
    Ok(())
}

#[tauri::command]
pub async fn open_plugin_window(app_handle: AppHandle) -> Result<(), String> {
    if app_handle.get_webview_window("plugin").is_none() {
        println!("Creating window");

        create_plugin_window(&app_handle);
    } else if let Some(window) = app_handle.get_webview_window("plugin") {
        println!("Showing window");
        if window.is_minimized().unwrap_or(false) {
            window.unminimize().unwrap();
        }
        window.show().unwrap();
        window.set_focus().unwrap();
    }
    Ok(())
}

#[tauri::command]
pub async fn open_preview_frontend_window(app_handle: AppHandle) -> Result<(), String> {
    if app_handle.get_webview_window("preview_frontend").is_none() {
        println!("Creating preview frontend window");

        create_preview_frontend_window(&app_handle);
    } else if let Some(window) = app_handle.get_webview_window("preview_frontend") {
        println!("Showing preview frontend window");
        if window.is_minimized().unwrap_or(false) {
            window.unminimize().unwrap();
        }
        window.show().unwrap();
        window.set_focus().unwrap();
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct ReactComponentPayload {
    code: String,
    css: String,
}

async fn open_preview_window(
    app_handle: AppHandle,
    html: String,
    port: u16,
    window_id: &str,
    file_extension: &str,
    event_name: String,
) -> Result<(), String> {
    let mut hasher = Sha256::new();
    hasher.update(html.clone());
    let result = hasher.finalize();
    let html_hash = format!("{:x}", result);
    let file_name = format!("{}.{}", html_hash, file_extension);

    let file_content = html.clone();

    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "http://preview.teafakedomain.com:{}/api/saveFile",
            port
        ))
        .json(&serde_json::json!({
            "fileName": file_name,
            "fileContent": file_content
        }))
        .send()
        .await;

    if let Ok(response) = response {
        if response.status().is_success() {
            let url = Url::parse(&format!(
                "http://preview.teafakedomain.com:{}/previews/{}",
                port, html_hash
            ))
            .map_err(|e| e.to_string())?;

            let window_builder =
                WebviewWindowBuilder::new(&app_handle, window_id, WebviewUrl::External(url))
                    .title("Aipp")
                    .inner_size(1000.0, 800.0)
                    .fullscreen(false)
                    .resizable(true)
                    .decorations(true)
                    .center();

            #[cfg(not(target_os = "macos"))]
            let window_builder = window_builder.transparent(false);

            match window_builder.build() {
                Ok(window) => {
                    let window_clone = window.clone();
                    window.on_window_event(move |event| {
                        if let WindowEvent::CloseRequested { .. } = event {
                            window_clone.hide().unwrap();
                        }
                    });

                    let window = app_handle.get_webview_window(window_id).unwrap();

                    window.clone().once("preview-window-load", move |_| {
                        let payload = ReactComponentPayload {
                            code: html.clone(),
                            css: "".to_string(),
                        };
                        let json_payload = serde_json::to_string(&payload).unwrap();
                        window.emit(&event_name, json_payload).unwrap();
                    });
                }
                Err(e) => eprintln!("Failed to build window: {}", e),
            }
        } else {
            eprintln!("Failed to save file: {}", response.status());
        }
    } else {
        eprintln!("Failed to send request: {:?}", response);
    }

    Ok(())
}

pub async fn open_preview_react_window(
    app_handle: AppHandle,
    html: String,
    port: u16,
) -> Result<(), String> {
    open_preview_window(
        app_handle,
        html,
        port,
        "preview_react",
        "js",
        "preview_react".to_string(),
    )
    .await
}

pub async fn open_preview_vue_window(
    app_handle: AppHandle,
    html: String,
    port: u16,
) -> Result<(), String> {
    open_preview_window(
        app_handle,
        html,
        port,
        "preview_vue",
        "vue",
        "preview_vue".to_string(),
    )
    .await
}

pub fn handle_open_ask_window(app_handle: &AppHandle) {
    use chrono::Local;
    
    let ask_window = app_handle.get_webview_window("ask");

    match ask_window {
        None => {
            println!(
                "Creating ask window, at time: {}",
                &Local::now().to_string()
            );
            create_ask_window(app_handle);
        }
        Some(window) => {
            println!(
                "Focusing ask window, at time: {}",
                &Local::now().to_string()
            );
            if window.is_minimized().unwrap_or(false) {
                window.unminimize().unwrap();
            }
            window.show().unwrap();
            window.set_focus().unwrap();
        }
    }
}

pub fn awaken_aipp(app_handle: &AppHandle) {
    use chrono::Local;
    
    let ask_window = app_handle.get_webview_window("ask");
    let chat_ui_window = app_handle.get_webview_window("chat_ui");

    // 优先检查 chat_ui 窗口
    if let Some(window) = chat_ui_window {
        println!(
            "Focusing chat_ui window, at time: {}",
            &Local::now().to_string()
        );
        if window.is_minimized().unwrap_or(false) {
            window.unminimize().unwrap();
        }
        window.show().unwrap();
        window.set_focus().unwrap();
        return;
    }

    // 其次检查 ask 窗口
    if let Some(window) = ask_window {
        println!(
            "Focusing ask window, at time: {}",
            &Local::now().to_string()
        );
        if window.is_minimized().unwrap_or(false) {
            window.unminimize().unwrap();
        }
        window.show().unwrap();
        window.set_focus().unwrap();
        return;
    }

    // 最后创建 ask 窗口
    println!(
        "Creating ask window, at time: {}",
        &Local::now().to_string()
    );
    create_ask_window(app_handle);
}

// Create artifact collections window to manage saved artifacts
fn create_artifact_collections_window(app_handle: &AppHandle) {
    let (window_size, window_position) = get_window_size_and_position(
        app_handle,
        1200.0,
        800.0,
        &["chat_ui", "ask", "config"],
    );

    let builder = WebviewWindowBuilder::new(
        app_handle,
        "artifact_collections",
        WebviewUrl::App("artifacts_collections.html".into()),
    )
    .title("Artifacts 合集管理")
    .inner_size(window_size.width, window_size.height)
    .resizable(true)
    .minimizable(true)
    .maximizable(true)
    .center();

    let builder = if let Some(position) = window_position {
        builder.position(position.x, position.y)
    } else {
        builder.center()
    };

    match builder.build() {
        Ok(_window) => {
            println!("Artifact collections window created successfully");
        }
        Err(e) => {
            eprintln!("Failed to create artifact collections window: {}", e);
        }
    }
}

// Create artifact window to display a single artifact
fn create_artifact_window(app_handle: &AppHandle, artifact: &ArtifactCollection) {
    let window_label = format!("artifact_{}", artifact.id);
    
    let (window_size, window_position) = get_window_size_and_position(
        app_handle,
        1000.0,
        700.0,
        &["chat_ui", "ask", "artifact_collections"],
    );

    let builder = WebviewWindowBuilder::new(
        app_handle,
        &window_label,
        WebviewUrl::App("index.html".into()),
    )
    .title(&format!("{} - {}", artifact.name, artifact.artifact_type.to_uppercase()))
    .inner_size(window_size.width, window_size.height)
    .resizable(true)
    .minimizable(true)
    .maximizable(true)
    .center();

    let builder = if let Some(position) = window_position {
        builder.position(position.x, position.y)
    } else {
        builder.center()
    };

    match builder.build() {
        Ok(_window) => {
            println!("Artifact window created successfully: {}", window_label);
            // 窗口会根据自己的 label 自动加载对应的 artifact 数据
        }
        Err(e) => {
            eprintln!("Failed to create artifact window: {}", e);
        }
    }
}

/// Open artifact collections management window
#[tauri::command]
pub async fn open_artifact_collections_window(app_handle: AppHandle) -> Result<(), String> {
    if app_handle.get_webview_window("artifact_collections").is_none() {
        println!("Creating artifact collections window");
        create_artifact_collections_window(&app_handle);
    } else if let Some(window) = app_handle.get_webview_window("artifact_collections") {
        println!("Showing artifact collections window");
        if window.is_minimized().unwrap_or(false) {
            window.unminimize().unwrap();
        }
        window.show().unwrap();
        window.set_focus().unwrap();
    }
    Ok(())
}

/// Open artifact window to display a single artifact
pub async fn open_artifact_window(
    app_handle: AppHandle,
    artifact: ArtifactCollection,
) -> Result<(), String> {
    let window_label = format!("artifact_{}", artifact.id);
    
    if app_handle.get_webview_window(&window_label).is_none() {
        println!("Creating artifact window: {}", window_label);
        create_artifact_window(&app_handle, &artifact);
    } else if let Some(window) = app_handle.get_webview_window(&window_label) {
        println!("Showing artifact window: {}", window_label);
        if window.is_minimized().unwrap_or(false) {
            window.unminimize().unwrap();
        }
        window.show().unwrap();
        window.set_focus().unwrap();
        
        // Update the artifact data in case it has changed
        println!("Sending updated artifact data to existing window: {}", window_label);
        let _ = window.emit("artifact-data", &artifact);
    }
    Ok(())
}
