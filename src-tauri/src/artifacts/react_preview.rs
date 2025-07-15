use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, Emitter};
use sha2::{Sha256, Digest};
use hex;

use crate::utils::bun_utils::BunUtils;
use crate::db::system_db::{SystemDatabase, FeatureConfig};

#[derive(Debug, Clone)]
pub struct PreviewServer {
    pub id: String,
    pub port: u16,
    pub process: Option<u32>, // PID
    pub template_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct TemplateCache {
    pub files_hash: String,
    pub deps_hash: String,
}

pub struct ReactPreviewManager {
    servers: Arc<Mutex<HashMap<String, PreviewServer>>>,
    app_handle: AppHandle,
}

impl ReactPreviewManager {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            servers: Arc::new(Mutex::new(HashMap::new())),
            app_handle,
        }
    }

    // 获取 bun 可执行文件路径
    fn get_bun_executable(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        BunUtils::get_bun_executable(&self.app_handle)
    }

    // 计算模板文件的 MD5 哈希值
    fn calculate_template_files_hash(&self, template_path: &PathBuf) -> Result<String, Box<dyn std::error::Error>> {
        let mut hasher = Sha256::new();
        
        // 排除的文件和目录
        let exclude_patterns = vec![
            "node_modules",
            ".git",
            "dist",
            "build",
            ".cache",
            "UserComponent.tsx" // 这个文件会被动态替换，不参与哈希计算
        ];
        
        self.hash_directory_recursive(template_path, &mut hasher, &exclude_patterns)?;
        let result = hasher.finalize();
        Ok(hex::encode(result))
    }

    // 递归计算目录的哈希
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
                continue;
            }

            if path.is_dir() {
                // 递归处理子目录
                self.hash_directory_recursive(&path, hasher, exclude_patterns)?;
            } else if path.is_file() {
                // 添加文件路径到哈希
                hasher.update(path.to_string_lossy().as_bytes());
                
                // 添加文件内容到哈希
                if let Ok(content) = fs::read(&path) {
                    hasher.update(&content);
                }
            }
        }

        Ok(())
    }

    // 计算依赖文件的 MD5 哈希值（package.json 和 bun.lock）
    fn calculate_deps_hash(&self, template_path: &PathBuf) -> Result<String, Box<dyn std::error::Error>> {
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

    // 从数据库获取模板缓存信息
    fn get_template_cache(&self, template_name: &str) -> Result<Option<TemplateCache>, Box<dyn std::error::Error>> {
        let db = SystemDatabase::new(&self.app_handle)?;
        
        // 查询文件哈希
        let files_hash_config = db.get_feature_config("template_cache", &format!("{}_files_hash", template_name))?;
        let deps_hash_config = db.get_feature_config("template_cache", &format!("{}_deps_hash", template_name))?;

        if let (Some(files_config), Some(deps_config)) = (files_hash_config, deps_hash_config) {
            Ok(Some(TemplateCache {
                files_hash: files_config.value,
                deps_hash: deps_config.value,
            }))
        } else {
            Ok(None)
        }
    }

    // 保存模板缓存信息到数据库
    fn save_template_cache(&self, template_name: &str, cache: &TemplateCache) -> Result<(), Box<dyn std::error::Error>> {
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

        // 尝试更新或插入
        if let Err(_) = db.update_feature_config(&files_hash_config) {
            db.add_feature_config(&files_hash_config)?;
        }
        
        if let Err(_) = db.update_feature_config(&deps_hash_config) {
            db.add_feature_config(&deps_hash_config)?;
        }

        Ok(())
    }

    pub fn create_preview_for_artifact(
        &self,
        component_code: String,
        component_name: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let preview_id = "react".to_string();
        println!("🚀 [React Preview] 开始创建预览, ID: {}", preview_id);
        if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("artifact-log", "开始创建 React 预览...");
        }

        let port = self.find_available_port()?;
        println!("🚀 [React Preview] 找到可用端口: {}", port);
        if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("artifact-log", format!("找到可用端口: {}", port));
        }

        // 关闭已存在的预览实例
        let _ = self.close_preview(&preview_id);

        // 设置项目目录
        if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("artifact-log", "设置项目模板...");
        }
        let (template_path, need_install_deps) =
            self.setup_template_project(&preview_id, &component_code, &component_name)?;
        println!("🚀 [React Preview] 模板项目已设置到: {:?}", template_path);
        if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("artifact-log", "模板项目设置完成");
        }

        // 启动开发服务器
        if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("artifact-log", "启动开发服务器...");
        }
        let process_id = self.start_dev_server(&template_path, port, need_install_deps)?;
        println!("🚀 [React Preview] 开发服务器已启动, PID: {}", process_id);
        if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("artifact-log", format!("开发服务器已启动, PID: {}", process_id));
        }

        let server = PreviewServer {
            id: preview_id.clone(),
            port,
            process: Some(process_id),
            template_path,
        };

        self.servers
            .lock()
            .unwrap()
            .insert(preview_id.clone(), server);

        // 等待开发服务器启动
        let app_handle = self.app_handle.clone();
        std::thread::spawn(move || {
            // 等待服务器启动
            println!("🚀 [React Preview] 等待服务器启动...");
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-log", "等待服务器启动...");
            }
            std::thread::sleep(std::time::Duration::from_secs(3));
            
            let preview_url = format!("http://localhost:{}", port);
            println!("🚀 [React Preview] 预览已准备完成: {}", preview_url);
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-success", "预览服务器已启动完成");
            }
            
            // 发送跳转事件，让前端窗口自动跳转到预览页面
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-redirect", preview_url);
            }
        });

        println!("🚀 [React Preview] 预览创建成功, ID: {}", preview_id);
        if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("artifact-log", "React 预览创建成功");
        }
        Ok(preview_id)
    }

    pub fn create_preview(
        &self,
        component_code: String,
        component_name: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let preview_id = "react".to_string();
        println!("🚀 [React Preview] 开始创建预览, ID: {}", preview_id);
        if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("artifact-log", "开始创建 React 预览...");
        }

        let port = self.find_available_port()?;
        println!("🚀 [React Preview] 找到可用端口: {}", port);
        if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("artifact-log", format!("找到可用端口: {}", port));
        }

        // 关闭已存在的预览实例
        let _ = self.close_preview(&preview_id);

        // 设置项目目录
        if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("artifact-log", "设置项目模板...");
        }
        let (template_path, need_install_deps) =
            self.setup_template_project(&preview_id, &component_code, &component_name)?;
        println!("🚀 [React Preview] 模板项目已设置到: {:?}", template_path);
        if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("artifact-log", "模板项目设置完成");
        }

        // 启动开发服务器
        if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("artifact-log", "启动开发服务器...");
        }
        let process_id = self.start_dev_server(&template_path, port, need_install_deps)?;
        println!("🚀 [React Preview] 开发服务器已启动, PID: {}", process_id);
        if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("artifact-log", format!("开发服务器已启动, PID: {}", process_id));
        }

        let server = PreviewServer {
            id: preview_id.clone(),
            port,
            process: Some(process_id),
            template_path,
        };

        self.servers
            .lock()
            .unwrap()
            .insert(preview_id.clone(), server);

        // 延迟打开预览窗口，等待服务器启动
        let app_handle = self.app_handle.clone();
        let preview_id_clone = preview_id.clone();
        std::thread::spawn(move || {
            // 等待服务器启动
            println!("🚀 [React Preview] 等待服务器启动...");
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-log", "等待服务器启动...");
            }
            std::thread::sleep(std::time::Duration::from_secs(3));
            println!("🚀 [React Preview] 尝试打开预览窗口");
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-log", "打开预览窗口...");
            }
            let _ = Self::open_preview_window_static(&app_handle, &preview_id_clone, port);
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-success", format!("预览窗口已打开: http://localhost:{}", port));
            }
        });

        println!("🚀 [React Preview] 预览创建成功, ID: {}", preview_id);
        if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("artifact-log", "React 预览创建成功");
        }
        Ok(preview_id)
    }

    pub fn close_preview(&self, preview_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut servers = self.servers.lock().unwrap();

        if let Some(server) = servers.remove(preview_id) {
            // 终止进程
            if let Some(pid) = server.process {
                let _ = self.kill_process(pid);
            }
            // 注意：不再删除文件夹，保留模板以便重用
        }

        Ok(())
    }

    fn setup_template_project(
        &self,
        preview_id: &str,
        component_code: &str,
        _component_name: &str,
    ) -> Result<(PathBuf, bool), Box<dyn std::error::Error>> {
        // 使用应用数据目录，类似 bun 二进制存放位置
        let app_data_dir = self
            .app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("无法获取应用数据目录: {}", e))?;

        let preview_dir = app_data_dir
            .join("preview")
            .join("templates")
            .join(preview_id);
        println!("🛠️ [Setup] 设置预览目录: {:?}", preview_dir);

        // 获取模板源路径
        let template_source = self.get_template_path();
        println!("🛠️ [Setup] 模板源路径: {:?}", template_source);

        if !template_source.exists() {
            let error_msg = format!("模板源路径不存在: {:?}", template_source);
            println!("❌ [Setup] {}", error_msg);
            return Err(error_msg.into());
        }

        // 计算当前模板的哈希值
        let current_files_hash = self.calculate_template_files_hash(&template_source)?;
        let current_deps_hash = self.calculate_deps_hash(&template_source)?;
        
        println!("🔍 [Setup] 当前模板文件哈希: {}", current_files_hash);
        println!("🔍 [Setup] 当前依赖哈希: {}", current_deps_hash);

        // 检查缓存
        let cached_info = self.get_template_cache("react");
        let mut need_copy_files = true;
        let mut need_install_deps = true;

        if let Ok(Some(cache)) = cached_info {
            println!("🔍 [Setup] 缓存文件哈希: {}", cache.files_hash);
            println!("🔍 [Setup] 缓存依赖哈希: {}", cache.deps_hash);
            
            // 检查文件是否需要更新
            if cache.files_hash == current_files_hash && preview_dir.exists() {
                need_copy_files = false;
                println!("✅ [Setup] 模板文件无变化，跳过复制");
                if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("artifact-log", "模板文件无变化，跳过复制");
                }
            }
            
            // 检查依赖是否需要更新
            if cache.deps_hash == current_deps_hash && preview_dir.join("node_modules").exists() {
                need_install_deps = false;
                println!("✅ [Setup] 依赖文件无变化，跳过安装");
                if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("artifact-log", "依赖文件无变化，跳过安装");
                }
            }
        } else {
            println!("🔍 [Setup] 没有找到缓存信息，需要初始化");
            if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-log", "首次设置模板，需要初始化");
            }
        }

        // 如果需要复制文件
        if need_copy_files {
            println!("📂 [Setup] 开始复制模板文件...");
            if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-log", "开始复制模板文件...");
            }
            self.copy_template(&template_source, &preview_dir)?;
            println!("✅ [Setup] 模板文件复制完成");
            if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-log", "模板文件复制完成");
            }
        }

        // 如果需要安装依赖
        if need_install_deps {
            println!("📦 [Setup] 需要安装/更新依赖");
            if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-log", "需要安装/更新依赖");
            }
            // 删除现有的 node_modules（如果存在）
            let node_modules_dir = preview_dir.join("node_modules");
            if node_modules_dir.exists() {
                println!("🗑️ [Setup] 删除现有的 node_modules");
                if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
                    let _ = window.emit("artifact-log", "删除现有的 node_modules");
                }
                let _ = fs::remove_dir_all(&node_modules_dir);
            }
        }

        // 保存新的缓存信息
        let new_cache = TemplateCache {
            files_hash: current_files_hash,
            deps_hash: current_deps_hash,
        };
        
        if let Err(e) = self.save_template_cache("react", &new_cache) {
            println!("⚠️ [Setup] 保存缓存信息失败: {}", e);
        } else {
            println!("✅ [Setup] 缓存信息已更新");
        }

        // 写入组件代码到 UserComponent.tsx
        let component_file = preview_dir.join("src").join("UserComponent.tsx");
        println!("🛠️ [Setup] 写入组件文件到: {:?}", component_file);

        fs::write(&component_file, component_code)?;
        println!("🛠️ [Setup] 组件文件写入完成");

        // 返回预览目录和是否需要安装依赖的标志
        Ok((preview_dir, need_install_deps))
    }

    fn copy_template(
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

    fn copy_dir_recursively(
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

    fn start_dev_server(
        &self,
        project_path: &PathBuf,
        port: u16,
        force_install: bool,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        println!(
            "🔧 [DevServer] 在项目路径启动开发服务器: {:?}",
            project_path
        );
        println!("🔧 [DevServer] 使用端口: {}", port);

        // 获取 bun 可执行文件路径
        let bun_executable = self.get_bun_executable()?;
        println!("🔧 [DevServer] Bun 可执行文件: {:?}", bun_executable);

        // 检查 bun 版本
        match Command::new(&bun_executable).arg("--version").output() {
            Ok(output) => {
                let version = String::from_utf8_lossy(&output.stdout);
                println!("🔧 [DevServer] Bun 版本: {}", version.trim());
            }
            Err(e) => {
                let error_msg = format!("无法获取 Bun 版本: {}", e);
                println!("❌ [DevServer] {}", error_msg);
                return Err(error_msg.into());
            }
        }

        // 检查项目路径和package.json
        let package_json = project_path.join("package.json");
        if !package_json.exists() {
            let error_msg = format!("package.json 不存在: {:?}", package_json);
            println!("❌ [DevServer] {}", error_msg);
            return Err(error_msg.into());
        }
        println!("🔧 [DevServer] package.json 存在: {:?}", package_json);

        // 修改 bunfig.toml 中的 install.cache.dir 为应用数据目录
        // 使用应用数据目录，类似 bun 二进制存放位置
        let app_data_dir = self
            .app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("无法获取应用数据目录: {}", e))?;

        let bunfig_path = project_path.join("bunfig.toml");
        if bunfig_path.exists() {
            println!("🔧 [DevServer] 修改 bunfig.toml 中的 install.cache.dir 为应用数据目录");
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

        // 先安装依赖（如果需要的话）
        if force_install || !project_path.join("node_modules").exists() {
            println!("🔧 [DevServer] 开始安装依赖...");
            let install_result = Command::new(&bun_executable)
                .args(&["install", "--force"])
                .current_dir(project_path)
                .output();

            match install_result {
                Ok(output) => {
                    if output.status.success() {
                        println!("✅ [DevServer] 依赖安装成功");
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if !stdout.is_empty() {
                            println!("🔧 [DevServer] Bun install 输出: {}", stdout.trim());
                        }
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let error_msg =
                            format!("Bun install 失败:\nStderr: {}\nStdout: {}", stderr, stdout);
                        println!("❌ [DevServer] {}", error_msg);
                        return Err(error_msg.into());
                    }
                }
                Err(e) => {
                    let error_msg = format!("无法执行 bun install: {}", e);
                    println!("❌ [DevServer] {}", error_msg);
                    return Err(error_msg.into());
                }
            }
        } else {
            println!("✅ [DevServer] 依赖已存在，跳过安装");
        }

        // 启动 Vite 开发服务器
        println!("🔧 [DevServer] 启动 Vite 开发服务器...");

        // 首先尝试使用 bunx vite
        let mut vite_command = Command::new(&bun_executable);
        vite_command
            .args(&[
                "x",
                "vite",
                "--port",
                &port.to_string(),
                "--host",
                "127.0.0.1",
            ])
            .current_dir(project_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let child = vite_command.spawn();

        match child {
            Ok(child) => {
                let pid = child.id();
                println!("✅ [DevServer] Vite 服务器启动成功, PID: {}", pid);

                // 让子进程在后台运行
                std::mem::forget(child);

                Ok(pid)
            }
            Err(e) => {
                let error_msg = format!("无法启动 Vite 服务器: {}", e);
                println!("❌ [DevServer] {}", error_msg);
                Err(error_msg.into())
            }
        }
    }

    fn open_preview_window_static(
        app_handle: &AppHandle,
        preview_id: &str,
        port: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("http://localhost:{}", port);
        println!("🪟 [Window] 准备打开预览窗口: {}", url);

        let window = WebviewWindowBuilder::new(
            app_handle,
            format!("preview-{}", preview_id),
            WebviewUrl::External(url.parse().unwrap()),
        )
        .title("Component Preview - AIPP")
        .inner_size(1024.0, 768.0)
        .center()
        .resizable(true)
        .build();

        match window {
            Ok(_) => {
                println!("✅ [Window] 预览窗口创建成功");
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("预览窗口创建失败: {}", e);
                println!("❌ [Window] {}", error_msg);
                Err(error_msg.into())
            }
        }
    }

    fn find_available_port(&self) -> Result<u16, Box<dyn std::error::Error>> {
        use std::net::TcpListener;

        for port in 3001..4000 {
            // Check if port is available on both 127.0.0.1 and 0.0.0.0
            let localhost_available = TcpListener::bind(("127.0.0.1", port)).is_ok();
            let wildcard_available = TcpListener::bind(("0.0.0.0", port)).is_ok();
            
            if localhost_available && wildcard_available {
                return Ok(port);
            }
        }

        Err("No available port found".into())
    }

    fn get_template_path(&self) -> PathBuf {
        // 获取应用资源目录中的模板路径
        let resource_dir = self.app_handle.path().resource_dir().unwrap_or_else(|_| {
            println!("⚠️ [Template] 无法获取资源目录，使用当前目录");
            PathBuf::from(".")
        });

        let template_path = resource_dir
            .join("artifacts")
            .join("templates")
            .join("react");

        println!("📁 [Template] 资源目录: {:?}", resource_dir);
        println!("📁 [Template] 模板路径: {:?}", template_path);

        template_path
    }

    fn kill_process(&self, pid: u32) -> Result<(), Box<dyn std::error::Error>> {
        kill_process_by_pid(pid)
    }
}

#[cfg(target_os = "windows")]
fn kill_process_by_pid(pid: u32) -> Result<(), Box<dyn std::error::Error>> {
    Command::new("taskkill")
        .args(&["/F", "/PID", &pid.to_string()])
        .output()?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn kill_process_by_pid(pid: u32) -> Result<(), Box<dyn std::error::Error>> {
    Command::new("kill")
        .args(&["-9", &pid.to_string()])
        .output()?;
    Ok(())
}

// Tauri 命令接口
#[tauri::command]
pub async fn create_react_preview_for_artifact(
    app_handle: AppHandle,
    component_code: String,
    component_name: String,
) -> Result<String, String> {
    let manager = ReactPreviewManager::new(app_handle);
    manager
        .create_preview_for_artifact(component_code, component_name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_react_preview(
    app_handle: AppHandle,
    component_code: String,
    component_name: String,
) -> Result<String, String> {
    let manager = ReactPreviewManager::new(app_handle);
    manager
        .create_preview(component_code, component_name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn close_react_preview(app_handle: AppHandle, preview_id: String) -> Result<(), String> {
    println!("🔧 [ReactPreview] 关闭预览窗口: {}", preview_id);
    let manager = ReactPreviewManager::new(app_handle);
    manager
        .close_preview(&preview_id)
        .map_err(|e| e.to_string())
}
