use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::Emitter;
use tauri::Listener;
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri::{LogicalPosition, LogicalSize};

// 获取合适的窗口大小和位置
fn get_window_size_and_position(
    app: &AppHandle,
    default_width: f64,
    default_height: f64,
    reference_window_labels: &[&str],
) -> (LogicalSize<f64>, Option<LogicalPosition<f64>>) {
    let mut window_size = LogicalSize::new(default_width, default_height);
    let mut window_position: Option<LogicalPosition<f64>> = None;

    // 按优先级尝试获取参考窗口信息
    for ref_label in reference_window_labels {
        if let Some(ref_window) = app.get_webview_window(ref_label) {
            // 检查窗口是否可见
            if let Ok(is_visible) = ref_window.is_visible() {
                if is_visible {
                    // 获取参考窗口所在的屏幕
                    if let Ok(current_monitor) = ref_window.current_monitor() {
                        if let Some(monitor) = current_monitor {
                            let monitor_size = monitor.size();
                            let monitor_position = monitor.position();

                            // 调整窗口大小以适应屏幕
                            let screen_width = monitor_size.width as f64;
                            let screen_height = monitor_size.height as f64;

                            // 留出一些边距（10%）
                            let max_width = screen_width * 0.9;
                            let max_height = screen_height * 0.9;

                            window_size.width = window_size.width.min(max_width);
                            window_size.height = window_size.height.min(max_height);

                            // 计算窗口位置（居中到参考窗口所在屏幕）
                            let center_x = monitor_position.x as f64
                                + (screen_width - window_size.width) / 2.0;
                            let center_y = monitor_position.y as f64
                                + (screen_height - window_size.height) / 2.0;

                            window_position = Some(LogicalPosition::new(center_x, center_y));
                            break; // 找到可见的参考窗口后停止搜索
                        }
                    }
                }
            }
        }
    }

    // 如果没有参考窗口，使用主屏幕
    if window_position.is_none() {
        if let Ok(Some(primary_monitor)) = app.primary_monitor() {
            let monitor_size = primary_monitor.size();
            let monitor_position = primary_monitor.position();

            // 调整窗口大小以适应屏幕
            let screen_width = monitor_size.width as f64;
            let screen_height = monitor_size.height as f64;

            // 留出一些边距（10%）
            let max_width = screen_width * 0.9;
            let max_height = screen_height * 0.9;

            window_size.width = window_size.width.min(max_width);
            window_size.height = window_size.height.min(max_height);

            // 计算窗口位置（居中到主屏幕）
            let center_x = monitor_position.x as f64 + (screen_width - window_size.width) / 2.0;
            let center_y = monitor_position.y as f64 + (screen_height - window_size.height) / 2.0;

            window_position = Some(LogicalPosition::new(center_x, center_y));
        }
    }

    (window_size, window_position)
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

pub async fn open_preview_html_window(app_handle: AppHandle, html: String) -> Result<(), String> {
    let window_builder = WebviewWindowBuilder::new(
        &app_handle,
        "preview_html",
        WebviewUrl::App("index.html".into()),
    )
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

            let window = app_handle.get_webview_window("preview_html").unwrap();

            window.clone().once("preview-window-load", move |_| {
                window.emit("preview_html", html.clone()).unwrap();
            });
        }
        Err(e) => eprintln!("Failed to build window: {}", e),
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
