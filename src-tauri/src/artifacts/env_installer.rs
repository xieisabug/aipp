use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tauri::{Emitter, Manager};

#[tauri::command]
pub fn check_bun_version(app: tauri::AppHandle) -> Result<String, String> {
    crate::utils::bun_utils::BunUtils::get_bun_version(&app)
}

#[tauri::command]
pub fn check_uv_version(app: tauri::AppHandle) -> Result<String, String> {
    crate::utils::uv_utils::UvUtils::get_uv_version(&app)
}

#[tauri::command]
pub fn install_bun(
    app_handle: tauri::AppHandle,
    target_window: Option<String>,
) -> Result<(), String> {
    let (event_prefix, emit_to_window) = if let Some(ref window) = target_window {
        if window == "artifact_preview" { ("artifact", true) } else { ("bun-install", true) }
    } else { ("bun-install", false) };

    std::thread::spawn(move || {
        let bun_version = "1.2.18";
        let (os, arch) = if cfg!(target_os = "windows") {
            ("windows", "x64")
        } else if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") { ("darwin", "aarch64") } else { ("darwin", "x64") }
        } else { ("linux", "x64") };

        let url = format!(
            "https://registry.npmmirror.com/-/binary/bun/bun-v{}/bun-{}-{}.zip",
            bun_version, os, arch
        );

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

        let app_data_dir = app_handle.path().app_data_dir().expect("无法获取应用数据目录");
        let bun_install_dir = app_data_dir.join("bun");
        let bun_bin_dir = bun_install_dir.join("bin");

        if let Err(e) = std::fs::create_dir_all(&bun_bin_dir) { emit_error(&format!("创建目录失败: {}", e)); return; }
        let zip_path = bun_install_dir.join("bun.zip");

        match reqwest::blocking::get(&url) {
            Ok(mut response) => match std::fs::File::create(&zip_path) {
                Ok(mut file) => { if let Err(e) = std::io::copy(&mut response, &mut file) { emit_error(&format!("下载失败: {}", e)); return; } emit_log("下载完成"); }
                Err(e) => { emit_error(&format!("创建文件失败: {}", e)); return; }
            },
            Err(e) => { emit_error(&format!("下载失败: {}", e)); return; }
        }

        emit_log("开始解压...");
        match std::fs::File::open(&zip_path) {
            Ok(zip_file) => {
                if let Err(e) = zip_extract::extract(zip_file, &bun_install_dir, true) { emit_error(&format!("解压失败: {}", e)); return; }
                emit_log("解压成功");
            }
            Err(e) => { emit_error(&format!("打开压缩文件失败: {}", e)); return; }
        }

        let bun_executable_name = if cfg!(target_os = "windows") { "bun.exe" } else { "bun" };
        let candidate_paths = [
            bun_install_dir.join(&bun_executable_name),
            bun_install_dir.join(format!("bun-{}-{}", os, arch)).join(&bun_executable_name),
        ];
        let bun_executable_path = match candidate_paths.iter().find(|p| p.exists()) { Some(p) => p.to_path_buf(), None => { emit_error("未找到 bun 可执行文件"); return; } };

        let dest_path = bun_bin_dir.join(&bun_executable_name);
        if dest_path.exists() { if let Err(e) = std::fs::remove_file(&dest_path) { emit_error(&format!("删除旧文件失败: {}", e)); return; } }
        if let Err(e) = std::fs::rename(bun_executable_path, &dest_path) { emit_error(&format!("移动文件失败: {}", e)); return; }

        emit_success("Bun 安装成功");
    });

    Ok(())
}

#[tauri::command]
pub fn install_uv(
    app_handle: tauri::AppHandle,
    target_window: Option<String>,
) -> Result<(), String> {
    let (event_prefix, emit_to_window) = if let Some(ref window) = target_window {
        if window == "artifact_preview" { ("artifact", true) } else { ("uv-install", true) }
    } else { ("uv-install", false) };

    std::thread::spawn(move || {
        let max_retries = 3;
        let mut success = false;

        let emit_log = |msg: &str| {
            println!("uv-install-log: {}", msg);
            if emit_to_window { if let Some(ref window_name) = target_window { if let Some(window) = app_handle.get_webview_window(window_name) { let _ = window.emit(&format!("{}-log", event_prefix), msg); } } } else { let _ = app_handle.emit("uv-install-log", msg); }
        };
        let emit_error = |msg: &str| {
            println!("uv-install-log: {}", msg);
            if emit_to_window { if let Some(ref window_name) = target_window { if let Some(window) = app_handle.get_webview_window(window_name) { let _ = window.emit(&format!("{}-error", event_prefix), msg); } } } else { let _ = app_handle.emit("uv-install-log", msg); }
        };
        let emit_success = |msg: &str| {
            println!("uv-install-log: {}", msg);
            if emit_to_window { if let Some(ref window_name) = target_window { if let Some(window) = app_handle.get_webview_window(window_name) { let _ = window.emit(&format!("{}-success", event_prefix), msg); } } } else { let _ = app_handle.emit("uv-install-log", msg); }
        };

        for attempt in 1..=max_retries {
            emit_log(&format!("正在尝试安装 uv (第 {} 次尝试)...", attempt));

            let (command, args) = if cfg!(target_os = "windows") {
                ("powershell", vec!["-c", "irm https://astral.sh/uv/install.ps1 | iex"])
            } else {
                ("sh", vec!["-c", "curl -LsSf --retry 3 --retry-delay 2 https://astral.sh/uv/install.sh | sh"]) 
            };

            let mut child = match Command::new(command)
                .args(args)
                .env("UV_INSTALLER_GHE_BASE_URL", "https://ghfast.top/https://github.com")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            { Ok(child) => child, Err(e) => { emit_error(&format!("启动安装命令失败: {}", e)); continue; } };

            let mut has_critical_error = false;
            if let Some(stdout) = child.stdout.take() { let reader = BufReader::new(stdout); for line in reader.lines() { if let Ok(line) = line { emit_log(&line); } } }
            if let Some(stderr) = child.stderr.take() { let reader = BufReader::new(stderr); for line in reader.lines() { if let Ok(line) = line { if line.contains("curl:") && (line.contains("Error in the HTTP2 framing layer") || line.contains("Recv failure: Connection reset by peer") || line.contains("Failed to connect") || line.contains("Could not resolve host")) { has_critical_error = true; } emit_log(&line); } } }

            match child.wait() {
                Ok(status) => {
                    if status.success() && !has_critical_error { success = true; emit_success("uv 安装成功！"); break; }
                    else { emit_error(&if has_critical_error { format!("第 {} 次尝试失败：检测到网络错误", attempt) } else { format!("第 {} 次尝试失败，退出码: {}", attempt, status.code().unwrap_or(-1)) }); if attempt < max_retries { emit_log("等待 2 秒后重试..."); std::thread::sleep(std::time::Duration::from_secs(2)); } }
                }
                Err(e) => { emit_error(&format!("等待进程失败: {}", e)); }
            }
        }

        if !success { emit_error(&format!("经过 {} 次尝试后，uv 安装失败", max_retries)); }

        println!("uv-install-finished: {}", success);
        if emit_to_window { if let Some(ref window_name) = target_window { if let Some(window) = app_handle.get_webview_window(window_name) { let _ = window.emit("uv-install-finished", success); } } } else { let _ = app_handle.emit("uv-install-finished", success); }
    });

    Ok(())
}
