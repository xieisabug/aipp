use std::path::PathBuf;
use std::process::Command;
use tauri::{AppHandle, Manager};

/// Bun 可执行文件工具函数
pub struct BunUtils;

impl BunUtils {
    /// 获取 Bun 可执行文件名（根据操作系统）
    fn get_bun_executable_name() -> &'static str {
        if cfg!(target_os = "windows") {
            "bun.exe"
        } else {
            "bun"
        }
    }

    /// 测试指定路径的 Bun 可执行文件是否可用
    fn test_bun_executable(exe: &std::path::Path) -> bool {
        if exe.exists() {
            match Command::new(exe).arg("--version").output() {
                Ok(output) if output.status.success() => true,
                _ => false,
            }
        } else {
            false
        }
    }

    /// 获取 Bun 可执行文件路径
    /// 优先检查自定义应用数据目录，然后检查系统 PATH
    pub fn get_bun_executable(
        app_handle: &AppHandle,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let bun_executable_name = Self::get_bun_executable_name();

        // 优先检查我们应用下载的 Bun 位置
        let custom_bun_path = app_handle
            .path()
            .app_data_dir()
            .map(|p| p.join("bun").join("bin").join(&bun_executable_name));

        // 1. 先试自定义路径
        if let Ok(custom_path) = &custom_bun_path {
            if Self::test_bun_executable(custom_path) {
                return Ok(custom_path.clone());
            }
        }

        // 2. 再试系统 PATH
        let system_bun = PathBuf::from(&bun_executable_name);
        if Self::test_bun_executable(&system_bun) {
            return Ok(system_bun);
        }

        Err("Bun 可执行文件未找到".into())
    }

    /// 获取 Bun 版本信息
    /// 优先检查自定义应用数据目录，然后检查系统 PATH
    pub fn get_bun_version(app_handle: &AppHandle) -> Result<String, String> {
        let bun_executable_name = Self::get_bun_executable_name();

        // 优先检查我们应用下载的 Bun 位置
        let custom_bun_path = app_handle
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
}
