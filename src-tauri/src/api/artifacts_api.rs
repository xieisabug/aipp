use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tauri::Emitter;
use tauri::Manager;

use crate::FeatureConfigState;

use crate::{
    artifacts::{applescript::run_applescript, powershell::run_powershell},
    errors::AppError,
    window::{open_preview_html_window, open_preview_react_window, open_preview_vue_window},
};
#[tauri::command]
pub async fn run_artifacts(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, FeatureConfigState>,
    lang: &str,
    input_str: &str,
) -> Result<String, AppError> {
    // Anthropic artifacts : code, markdown, html, svg, mermaid, react(引入了 lucid3-react, recharts, tailwind, shadcn/ui )
    // 加上 vue, nextjs 引入更多的前端库( echarts, antd, element-ui )

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
            return Ok(run_powershell(input_str).map_err(|e| {
                AppError::RunCodeError("PowerShell 脚本执行失败:".to_owned() + &e.to_string())
            })?);
        }
        "applescript" => {
            return Ok(run_applescript(input_str).map_err(|e| {
                AppError::RunCodeError("AppleScript 脚本执行失败:".to_owned() + &e.to_string())
            })?);
        }
        "xml" | "svg" | "html" => {
            let _ = open_preview_html_window(app_handle, input_str.to_string()).await;
        }
        "react" | "jsx" => {
            let _ = open_preview_react_window(app_handle, input_str.to_string(), nextjs_port).await;
        }
        "vue" => {
            let _ = open_preview_vue_window(app_handle, input_str.to_string(), nuxtjs_port).await;
        }
        _ => {
            // Handle other languages here
            return Err(AppError::RunCodeError(
                "暂不支持该语言的代码执行".to_owned(),
            ));
        }
    }
    Ok("".to_string())
}

#[tauri::command]
pub fn check_bun_version(app: tauri::AppHandle) -> Result<String, String> {
    let bun_executable_name = if cfg!(target_os = "windows") {
        "bun.exe"
    } else {
        "bun"
    };

    // 优先检查我们应用下载的 Bun 位置
    let custom_bun_path = app
        .path()
        .app_data_dir()
        .map(|p| p.join("bun").join("bin").join(&bun_executable_name));

    // 定义一个闭包，尝试执行指定路径的 Bun --version
    let get_version = |exe: &std::path::Path| -> Option<String> {
        if exe.exists() {
            match Command::new(exe).arg("--version").output() {
                Ok(output) if output.status.success() => {
                    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
                }
                _ => None,
            }
        } else {
            None
        }
    };

    // 1. 先试自定义路径
    if let Ok(custom_path) = &custom_bun_path {
        if let Some(ver) = get_version(custom_path) {
            return Ok(ver);
        }
    }

    // 2. 再试系统 PATH
    if let Some(ver) = get_version(std::path::Path::new(&bun_executable_name)) {
        return Ok(ver);
    }

    Ok("Not Installed".to_string())
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
        let bun_version = "1.2.9";
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
        let (command, args) = if cfg!(target_os = "windows") {
            (
                "powershell",
                vec!["-c", "irm https://astral.sh/uv/install.ps1 | iex"],
            )
        } else {
            (
                "sh",
                vec!["-c", "curl -LsSf https://astral.sh/uv/install.sh | sh"],
            )
        };

        let mut child = Command::new(command)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn command");

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
                    println!("uv-install-log: {}", line);
                    window.emit("uv-install-log", line).unwrap();
                }
            }
        }

        let status = child.wait().expect("Failed to wait on child");
        println!("uv-install-finished: {}", status.success());
        window
            .emit("uv-install-finished", status.success())
            .unwrap();
    });

    Ok(())
}
