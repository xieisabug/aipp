use chrono::DateTime;
use tauri::{AppHandle, Manager, Url, WindowBuilder, WindowEvent, WindowUrl};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::fs;

pub fn create_ask_window(app: &AppHandle) {
    let window_builder = WindowBuilder::new(
        app,
        "ask",
        WindowUrl::App("index.html".into())
    )
        .title("Tea")
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
        },
        Err(e) => eprintln!("Failed to build window: {}", e),
    }
}

pub fn create_config_window(app: &AppHandle) {
    let window_builder = WindowBuilder::new(
        app,
        "config",
        WindowUrl::App("index.html".into())
    )
        .title("Tea")
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
        },
        Err(e) => eprintln!("Failed to build window: {}", e),
    }
}

pub fn create_chat_ui_window(app: &AppHandle) {
    let window_builder = WindowBuilder::new(
        app,
        "chat_ui",
        WindowUrl::App("index.html".into())
    )
        .title("Tea")
        .inner_size(1000.0, 800.0)
        .fullscreen(false)
        .resizable(true)
        .decorations(true)
        .disable_file_drop_handler()
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
        },
        Err(e) => eprintln!("Failed to build window: {}", e),
    }
}

#[tauri::command]
pub async fn open_config_window(app_handle: AppHandle) -> Result<(), String> {
    if app_handle.get_window("config").is_none() {
        println!("Creating window");

        create_config_window(&app_handle)
    } else if let Some(window) = app_handle.get_window("config") {
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
    if app_handle.get_window("chat_ui").is_none() {
        println!("Creating window");

        create_chat_ui_window(&app_handle);
        app_handle.get_window("ask").unwrap().close().unwrap();
    } else if let Some(window) = app_handle.get_window("config") {
        println!("Showing window");
        if window.is_minimized().unwrap_or(false) {
            window.unminimize().unwrap();
        }
        window.show().unwrap();
        window.set_focus().unwrap();
        app_handle.get_window("ask").unwrap().close().unwrap();
    }
    Ok(())
}

pub async fn open_preview_html_window(app_handle: AppHandle, html: String) -> Result<(), String> {
    let window_builder = WindowBuilder::new(
        &app_handle,
        "preview_html",
        WindowUrl::App("index.html".into())
    )
        .title("Tea")
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

            let window = app_handle.get_window("preview_html").unwrap();
            
            window.clone().once("preview-window-load", move |_| {
                window.emit("preview_html", html.clone()).unwrap();
            });
            
        },
        Err(e) => eprintln!("Failed to build window: {}", e),
    }

    Ok(())
}

#[derive(Serialize, Deserialize)]
struct ReactComponentPayload {
    code: String,
    css: String,
}

pub async fn open_preview_react_window(app_handle: AppHandle, html: String) -> Result<(), String> {
    let mut hasher = Sha256::new();
    hasher.update(html.clone());
    let result = hasher.finalize();
    let html_hash = format!("{:x}", result);

    let directory = "/Users/xiejingyang/Workspace/js/react-component-preview/pages/components/";
    let js_file_path = format!("{}{}.js", directory, html_hash);

    if !fs::metadata(&js_file_path).is_ok() {
        fs::write(&js_file_path, html.clone()).unwrap();
    }

    let url = Url::parse(&("http://preview.teafakedomain.com:3001/previews/".to_owned() + &html_hash)).map_err(|e| e.to_string())?;

    let window_builder = WindowBuilder::new(
        &app_handle,
        "preview_react",
        WindowUrl::External(url)
    )
        .title("Tea")
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

            let window = app_handle.get_window("preview_react").unwrap();
            
            window.clone().once("preview-window-load", move |_| {
                let payload = ReactComponentPayload { code: html.clone(), css: "".to_string() };
                let json_payload = serde_json::to_string(&payload).unwrap();
                window.emit("preview_react", json_payload).unwrap();
            });
            
        },
        Err(e) => eprintln!("Failed to build window: {}", e),
    }

    Ok(())
}

pub async fn open_preview_vue_window(app_handle: AppHandle, html: String) -> Result<(), String> {
    let mut hasher = Sha256::new();
    hasher.update(html.clone());
    let result = hasher.finalize();
    let html_hash = format!("{:x}", result);

    let directory = "/Users/xiejingyang/Workspace/js/vue-component-preview/components/";
    let js_file_path = format!("{}{}.vue", directory, html_hash);

    if !fs::metadata(&js_file_path).is_ok() {
        fs::write(&js_file_path, html.clone()).unwrap();
    }

    let url = Url::parse(&("http://preview.teafakedomain.com:3002/previews/".to_owned() + &html_hash)).map_err(|e| e.to_string())?;

    let window_builder = WindowBuilder::new(
        &app_handle,
        "preview_vue",
        WindowUrl::External(url)
    )
        .title("Tea")
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

            let window = app_handle.get_window("preview_vue").unwrap();
            
            window.clone().once("preview-window-load", move |_| {
                let payload = ReactComponentPayload { code: html.clone(), css: "".to_string() };
                let json_payload = serde_json::to_string(&payload).unwrap();
                window.emit("preview_react", json_payload).unwrap();
            });
            
        },
        Err(e) => eprintln!("Failed to build window: {}", e),
    }

    Ok(())
}