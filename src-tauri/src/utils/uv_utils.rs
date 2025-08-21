use dirs;
use std::process::Command;
use tauri::AppHandle;

/// Uv 可执行文件工具函数
pub struct UvUtils;

impl UvUtils {
    /// 获取 Uv 可执行文件名（根据操作系统）
    fn get_uv_executable_name() -> &'static str {
        if cfg!(target_os = "windows") {
            "uv.exe"
        } else {
            "uv"
        }
    }

    /// 获取 Uv 版本信息
    pub fn get_uv_version(_app_handle: &AppHandle) -> Result<String, String> {
        let uv_executable_name = Self::get_uv_executable_name();

        let get_version = |exe: &std::path::Path| -> Option<String> {
            if exe.exists() {
                match Command::new(exe).arg("--version").output() {
                    Ok(output) if output.status.success() => {
                        let version_info =
                            String::from_utf8_lossy(&output.stdout).trim().to_string();
                        // The output is like "uv 0.2.8", we just need the version number.
                        if let Some(version) = version_info.split_whitespace().last() {
                            Some(version.to_string())
                        } else {
                            Some(version_info)
                        }
                    }
                    _ => None,
                }
            } else {
                None
            }
        };

        // 1. 检查 $HOME/.local/bin/uv
        if let Some(home_dir) = dirs::home_dir() {
            let local_bin_path = home_dir.join(".local").join("bin").join(uv_executable_name);
            if let Some(ver) = get_version(&local_bin_path) {
                return Ok(ver);
            }
        }

        // 2. 检查 $HOME/.cargo/bin/uv
        if let Some(home_dir) = dirs::home_dir() {
            let cargo_bin_path = home_dir.join(".cargo").join("bin").join(uv_executable_name);
            if let Some(ver) = get_version(&cargo_bin_path) {
                return Ok(ver);
            }
        }

        // 3. 再试系统 PATH
        if let Some(ver) = get_version(std::path::Path::new(uv_executable_name)) {
            return Ok(ver);
        }

        Ok("Not Installed".to_string())
    }
}
