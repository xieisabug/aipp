use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::{AppHandle, Manager};
use sha2::{Sha256, Digest};
use hex;

use crate::utils::bun_utils::BunUtils;
use crate::db::system_db::{SystemDatabase, FeatureConfig};

/// 模板缓存信息
#[derive(Debug, Clone)]
pub struct TemplateCache {
    pub files_hash: String,
    pub deps_hash: String,
}

/// 共享的预览服务器管理工具
pub struct SharedPreviewUtils {
    app_handle: AppHandle,
}

impl SharedPreviewUtils {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    /// 获取 bun 可执行文件路径
    pub fn get_bun_executable(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        BunUtils::get_bun_executable(&self.app_handle)
    }

    /// 查找可用端口
    pub fn find_available_port(&self, start_port: u16, end_port: u16) -> Result<u16, Box<dyn std::error::Error>> {
        use std::net::TcpListener;

        for port in start_port..end_port {
            // Check if port is available on both 127.0.0.1 and 0.0.0.0
            let localhost_available = TcpListener::bind(("127.0.0.1", port)).is_ok();
            let wildcard_available = TcpListener::bind(("0.0.0.0", port)).is_ok();
            
            if localhost_available && wildcard_available {
                return Ok(port);
            }
        }

        Err("No available port found".into())
    }

    /// 检查端口是否开放
    pub fn is_port_open(ip: &str, port: u16) -> bool {
        use std::net::{TcpStream, ToSocketAddrs};
        use std::time::Duration;
        
        let addr = format!("{}:{}", ip, port);
        
        // 尝试解析地址
        let socket_addrs: Vec<_> = match addr.to_socket_addrs() {
            Ok(addrs) => addrs.collect(),
            Err(e) => {
                println!("🔧 [SharedUtils] 地址解析失败 {}: {}", addr, e);
                return false;
            }
        };
        
        // 尝试连接到端口，设置较短的超时时间以快速检测
        for socket_addr in socket_addrs {
            match TcpStream::connect_timeout(&socket_addr, Duration::from_millis(200)) {
                Ok(_) => {
                    println!("🔧 [SharedUtils] 端口 {} 已开放", addr);
                    return true;
                }
                Err(_) => {
                    continue;
                }
            }
        }
        
        println!("🔧 [SharedUtils] 端口 {} 未开放", addr);
        false
    }

    /// 计算模板文件的哈希值
    pub fn calculate_template_files_hash(&self, template_path: &PathBuf, component_file: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut hasher = Sha256::new();
        
        // 排除的文件和目录
        let exclude_patterns = vec![
            "node_modules",
            ".git",
            "dist",
            "build",
            ".cache",
            ".tmp",
            ".temp",
            component_file,      // 组件文件会被动态替换，不参与哈希计算
            ".DS_Store",         // macOS 系统文件
            "Thumbs.db",         // Windows 系统文件
            ".gitignore",        // git 忽略文件可能变化
            "bun.lockb",         // bun 二进制锁文件
            ".vite",             // vite 缓存目录
            ".turbo",            // turbo 缓存目录
            "coverage",          // 测试覆盖率目录
        ];
        
        self.hash_directory_recursive(template_path, &mut hasher, &exclude_patterns)?;
        let result = hasher.finalize();
        Ok(hex::encode(result))
    }

    /// 递归计算目录的哈希
    fn hash_directory_recursive(
        &self,
        dir: &PathBuf,
        hasher: &mut Sha256,
        exclude_patterns: &[&str],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !dir.exists() {
            return Ok(());
        }

        let mut entries: Vec<_> = fs::read_dir(dir)?
            .filter_map(|entry| entry.ok())
            .collect();
        
        // 按文件名排序以确保一致的哈希结果
        entries.sort_by_key(|entry| entry.file_name());

        for entry in entries {
            let path = entry.path();
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            // 检查是否应该排除
            if exclude_patterns.iter().any(|&pattern| file_name_str.contains(pattern)) {
                println!("🔍 [SharedHash] 排除文件: {:?}", path);
                continue;
            }

            if path.is_dir() {
                println!("🔍 [SharedHash] 处理目录: {:?}", path);
                // 递归处理子目录
                self.hash_directory_recursive(&path, hasher, exclude_patterns)?;
            } else if path.is_file() {
                println!("🔍 [SharedHash] 包含文件: {:?}", path);
                
                // 只添加相对路径到哈希，避免绝对路径差异
                if let Ok(relative_path) = path.strip_prefix(dir) {
                    hasher.update(relative_path.to_string_lossy().as_bytes());
                } else {
                    hasher.update(path.to_string_lossy().as_bytes());
                }
                
                // 添加文件内容到哈希
                if let Ok(content) = fs::read(&path) {
                    hasher.update(&content);
                }
            }
        }

        Ok(())
    }

    /// 计算依赖文件的哈希值（package.json 和 bun.lock）
    pub fn calculate_deps_hash(&self, template_path: &PathBuf) -> Result<String, Box<dyn std::error::Error>> {
        let mut hasher = Sha256::new();
        
        // 计算 package.json 的哈希
        let package_json = template_path.join("package.json");
        if package_json.exists() {
            let content = fs::read(&package_json)?;
            hasher.update(&content);
        }

        // 计算 bun.lock 的哈希（如果存在）
        let bun_lock = template_path.join("bun.lock");
        if bun_lock.exists() {
            let content = fs::read(&bun_lock)?;
            hasher.update(&content);
        }

        let result = hasher.finalize();
        Ok(hex::encode(result))
    }

    /// 从数据库获取模板缓存信息
    pub fn get_template_cache(&self, template_name: &str) -> Result<Option<TemplateCache>, Box<dyn std::error::Error>> {
        let db = SystemDatabase::new(&self.app_handle)?;
        
        println!("🔍 [SharedCache] 查询模板缓存: {}", template_name);
        
        // 查询文件哈希
        let files_hash_key = format!("{}_files_hash", template_name);
        let deps_hash_key = format!("{}_deps_hash", template_name);
        
        println!("🔍 [SharedCache] 查询文件哈希配置: feature_code='template_cache', key='{}'", files_hash_key);
        let files_hash_config = db.get_feature_config("template_cache", &files_hash_key)?;
        
        println!("🔍 [SharedCache] 查询依赖哈希配置: feature_code='template_cache', key='{}'", deps_hash_key);
        let deps_hash_config = db.get_feature_config("template_cache", &deps_hash_key)?;

        match (&files_hash_config, &deps_hash_config) {
            (Some(files_config), Some(deps_config)) => {
                println!("✅ [SharedCache] 找到缓存信息:");
                println!("  - 文件哈希: {}", files_config.value);
                println!("  - 依赖哈希: {}", deps_config.value);
                Ok(Some(TemplateCache {
                    files_hash: files_config.value.clone(),
                    deps_hash: deps_config.value.clone(),
                }))
            }
            (None, Some(_)) => {
                println!("⚠️ [SharedCache] 只找到依赖哈希配置，缺少文件哈希配置");
                Ok(None)
            }
            (Some(_), None) => {
                println!("⚠️ [SharedCache] 只找到文件哈希配置，缺少依赖哈希配置");
                Ok(None)
            }
            (None, None) => {
                println!("🔍 [SharedCache] 没有找到任何缓存配置");
                Ok(None)
            }
        }
    }

    /// 保存模板缓存信息到数据库
    pub fn save_template_cache(&self, template_name: &str, cache: &TemplateCache) -> Result<(), Box<dyn std::error::Error>> {
        let db = SystemDatabase::new(&self.app_handle)?;

        // 保存文件哈希
        let files_hash_config = FeatureConfig {
            id: None,
            feature_code: "template_cache".to_string(),
            key: format!("{}_files_hash", template_name),
            value: cache.files_hash.clone(),
            data_type: "string".to_string(),
            description: Some(format!("{} 模板文件哈希值", template_name)),
        };

        // 保存依赖哈希
        let deps_hash_config = FeatureConfig {
            id: None,
            feature_code: "template_cache".to_string(),
            key: format!("{}_deps_hash", template_name),
            value: cache.deps_hash.clone(),
            data_type: "string".to_string(),
            description: Some(format!("{} 模板依赖哈希值", template_name)),
        };

        // 尝试插入或更新文件哈希
        println!("💾 [SharedCache] 尝试插入文件哈希配置...");
        match db.add_feature_config(&files_hash_config) {
            Ok(_) => {
                println!("✅ [SharedCache] 文件哈希配置插入成功");
            }
            Err(add_err) => {
                println!("⚠️ [SharedCache] 文件哈希配置插入失败: {}, 尝试更新现有记录", add_err);
                match db.update_feature_config(&files_hash_config) {
                    Ok(_) => {
                        println!("✅ [SharedCache] 文件哈希配置更新成功");
                    }
                    Err(update_err) => {
                        println!("❌ [SharedCache] 文件哈希配置更新失败: {}", update_err);
                        return Err(Box::new(update_err));
                    }
                }
            }
        }
        
        // 尝试插入或更新依赖哈希
        println!("💾 [SharedCache] 尝试插入依赖哈希配置...");
        match db.add_feature_config(&deps_hash_config) {
            Ok(_) => {
                println!("✅ [SharedCache] 依赖哈希配置插入成功");
            }
            Err(add_err) => {
                println!("⚠️ [SharedCache] 依赖哈希配置插入失败: {}, 尝试更新现有记录", add_err);
                match db.update_feature_config(&deps_hash_config) {
                    Ok(_) => {
                        println!("✅ [SharedCache] 依赖哈希配置更新成功");
                    }
                    Err(update_err) => {
                        println!("❌ [SharedCache] 依赖哈希配置更新失败: {}", update_err);
                        return Err(Box::new(update_err));
                    }
                }
            }
        }

        Ok(())
    }

    /// 递归复制目录
    pub fn copy_dir_recursively(
        &self,
        source: &PathBuf,
        dest: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let path = entry.path();
            let dest_path = dest.join(entry.file_name());

            if path.is_dir() {
                if entry.file_name() != "node_modules" && entry.file_name() != ".git" {
                    fs::create_dir_all(&dest_path)?;
                    self.copy_dir_recursively(&path, &dest_path)?;
                }
            } else {
                fs::copy(&path, &dest_path)?;
            }
        }
        Ok(())
    }

    /// 复制模板
    pub fn copy_template(
        &self,
        source: &PathBuf,
        dest: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if dest.exists() {
            fs::remove_dir_all(dest)?;
        }
        fs::create_dir_all(dest)?;

        // 递归复制文件
        self.copy_dir_recursively(source, dest)?;

        Ok(())
    }

    /// 获取应用数据目录下的预览目录路径
    pub fn get_preview_directory(&self, component_type: &str, preview_id: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let app_data_dir = self
            .app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("无法获取应用数据目录: {}", e))?;

        let preview_dir = app_data_dir
            .join("preview")
            .join("templates")
            .join(component_type)
            .join(preview_id);

        Ok(preview_dir)
    }

    /// 获取模板源路径
    pub fn get_template_source_path(&self, component_type: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let resource_dir = self.app_handle.path().resource_dir().unwrap_or_else(|_| {
            println!("⚠️ [SharedTemplate] 无法获取资源目录，使用当前目录");
            PathBuf::from(".")
        });

        let template_path = resource_dir
            .join("artifacts")
            .join("templates")
            .join(component_type);

        println!("📁 [SharedTemplate] 资源目录: {:?}", resource_dir);
        println!("📁 [SharedTemplate] 模板路径: {:?}", template_path);

        if !template_path.exists() {
            return Err(format!("模板源路径不存在: {:?}", template_path).into());
        }

        Ok(template_path)
    }

    /// 修改 bunfig.toml 中的缓存目录
    pub fn setup_bunfig_cache(&self, project_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let app_data_dir = self
            .app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("无法获取应用数据目录: {}", e))?;

        let bunfig_path = project_path.join("bunfig.toml");
        if bunfig_path.exists() {
            println!("🔧 [SharedSetup] 修改 bunfig.toml 中的 install.cache.dir 为应用数据目录");
            let mut bunfig_content = fs::read_to_string(&bunfig_path)?;
            let cache_path = app_data_dir
                .join("bun")
                .join("install")
                .join("cache");
            
            // For Windows, ensure double backslashes are used
            let cache_path_str = if cfg!(target_os = "windows") {
                cache_path.to_string_lossy().replace("\\", "\\\\")
            } else {
                cache_path.to_string_lossy().to_string()
            };
            
            bunfig_content = bunfig_content.replace(
                "~/.bun/install/cache",
                &cache_path_str,
            );
            fs::write(&bunfig_path, bunfig_content)?;
        }

        Ok(())
    }
}

// 进程管理函数

/// 终止进程 (跨平台)
pub fn kill_process_by_pid(pid: u32) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 [SharedProcess] 执行 kill_process PID: {}", pid);
    
    #[cfg(target_os = "windows")]
    {
        println!("🔧 [Windows] 尝试终止进程 PID: {}", pid);
        let output = Command::new("taskkill")
            .args(&["/F", "/PID", &pid.to_string()])
            .output()?;
        
        if output.status.success() {
            println!("✅ [Windows] taskkill 成功");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("❌ [Windows] taskkill 失败: {}", stderr);
            return Err(format!("taskkill 失败: {}", stderr).into());
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("🔧 [Unix] 尝试终止进程 PID: {}", pid);
        
        // 先检查进程是否存在
        if !process_exists(pid) {
            println!("✅ [Unix] 进程 PID {} 不存在或已终止", pid);
            return Ok(());
        }
        
        // 发送 TERM 信号
        let output = Command::new("kill")
            .args(&["-TERM", &pid.to_string()])
            .output()?;
        
        if output.status.success() {
            println!("✅ [Unix] kill -TERM 成功");
            // 等待进程终止
            std::thread::sleep(std::time::Duration::from_millis(500));
            
            // 检查进程是否已经终止
            if !process_exists(pid) {
                println!("✅ [Unix] 进程 PID {} 已成功终止", pid);
                return Ok(());
            }
            
            // 进程仍然存在，发送 SIGKILL
            println!("🔧 [Unix] 进程仍在运行，发送 SIGKILL");
            let output = Command::new("kill")
                .args(&["-9", &pid.to_string()])
                .output()?;
            
            if output.status.success() {
                println!("✅ [Unix] kill -9 成功");
                // 再次检查进程状态
                std::thread::sleep(std::time::Duration::from_millis(200));
                if !process_exists(pid) {
                    println!("✅ [Unix] 进程 PID {} 已被强制终止", pid);
                } else {
                    println!("⚠️ [Unix] 进程 PID {} 可能仍在运行", pid);
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("❌ [Unix] kill -9 失败: {}", stderr);
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("❌ [Unix] kill -TERM 失败: {}", stderr);
            return Err(format!("kill 失败: {}", stderr).into());
        }
    }
    
    Ok(())
}

/// 检查进程是否存在 (Unix only)
#[cfg(not(target_os = "windows"))]
fn process_exists(pid: u32) -> bool {
    // 使用 kill -0 检查进程是否存在
    Command::new("kill")
        .args(&["-0", &pid.to_string()])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// 终止进程组
pub fn kill_process_group_by_pid(pid: u32) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 [SharedProcess] 执行 kill_process_group PID: {}", pid);

    #[cfg(target_os = "windows")]
    {
        println!("🔧 [Windows] 尝试终止进程树 PID: {}", pid);
        let output = Command::new("taskkill")
            .args(&["/F", "/T", "/PID", &pid.to_string()])
            .output()?;
        
        if output.status.success() {
            println!("✅ [Windows] taskkill 进程树成功");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("❌ [Windows] taskkill 进程树失败: {}", stderr);
            return Err(format!("taskkill 进程树失败: {}", stderr).into());
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("🔧 [Unix] 尝试终止进程组 PID: {}", pid);
        
        // 先检查进程组是否存在
        if !process_exists(pid) {
            println!("✅ [Unix] 进程组 PID {} 不存在或已终止", pid);
            return Ok(());
        }
        
        // 先尝试终止整个进程组
        let output = Command::new("kill")
            .args(&["-TERM", &format!("-{}", pid)])
            .output()?;
        
        if output.status.success() {
            println!("✅ [Unix] kill -TERM 进程组成功");
            // 等待进程组终止
            std::thread::sleep(std::time::Duration::from_millis(500));
            
            // 检查进程组是否已经终止
            if !process_exists(pid) {
                println!("✅ [Unix] 进程组 PID {} 已成功终止", pid);
                return Ok(());
            }
            
            // 进程组仍然存在，强制终止
            println!("🔧 [Unix] 进程组仍在运行，强制终止");
            let output = Command::new("kill")
                .args(&["-9", &format!("-{}", pid)])
                .output()?;
            
            if output.status.success() {
                println!("✅ [Unix] kill -9 进程组成功");
                // 再次检查进程组状态
                std::thread::sleep(std::time::Duration::from_millis(200));
                if !process_exists(pid) {
                    println!("✅ [Unix] 进程组 PID {} 已被强制终止", pid);
                } else {
                    println!("⚠️ [Unix] 进程组 PID {} 可能仍在运行", pid);
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("❌ [Unix] kill -9 进程组失败: {}", stderr);
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("❌ [Unix] kill -TERM 进程组失败: {}", stderr);
            return Err(format!("kill 进程组失败: {}", stderr).into());
        }
    }
    
    Ok(())
}

/// 根据端口查找并终止进程
pub fn kill_processes_by_port(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 [SharedProcess] 根据端口 {} 查找并终止进程", port);

    #[cfg(target_os = "windows")]
    {
        println!("🔧 [Windows] 查找端口 {} 上的进程", port);
        
        // 使用 netstat 查找占用端口的进程
        let output = Command::new("netstat")
            .args(&["-ano"])
            .output()?;
        
        if !output.status.success() {
            return Err("netstat 命令失败".into());
        }
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut pids_to_kill = Vec::new();
        
        for line in output_str.lines() {
            if line.contains(&format!(":{}", port)) && line.contains("LISTENING") {
                // 解析 PID（最后一列）
                if let Some(pid_str) = line.split_whitespace().last() {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        pids_to_kill.push(pid);
                        println!("🔧 [Windows] 找到占用端口 {} 的进程 PID: {}", port, pid);
                    }
                }
            }
        }
        
        // 终止所有找到的进程
        for pid in pids_to_kill {
            println!("🔧 [Windows] 终止端口 {} 相关进程 PID: {}", port, pid);
            let _ = kill_process_by_pid(pid); // 继续处理其他进程，即使某个失败
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("🔧 [Unix] 查找端口 {} 上的进程", port);
        
        // 使用 lsof 查找占用端口的进程
        let output = Command::new("lsof")
            .args(&["-ti", &format!(":{}", port)])
            .output()?;
        
        if !output.status.success() {
            println!("⚠️ [Unix] lsof 未找到端口 {} 上的进程", port);
            return Ok(());
        }
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut pids_to_kill = Vec::new();
        
        for line in output_str.lines() {
            if let Ok(pid) = line.trim().parse::<u32>() {
                pids_to_kill.push(pid);
                println!("🔧 [Unix] 找到占用端口 {} 的进程 PID: {}", port, pid);
            }
        }
        
        // 终止所有找到的进程
        for pid in pids_to_kill {
            println!("🔧 [Unix] 终止端口 {} 相关进程 PID: {}", port, pid);
            let _ = kill_process_by_pid(pid); // 继续处理其他进程，即使某个失败
        }
    }
    
    Ok(())
}