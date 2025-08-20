use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::sync::LazyLock;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, Emitter};

use crate::artifacts::shared_components::{
    SharedPreviewUtils, TemplateCache, 
    kill_process_by_pid, kill_process_group_by_pid, kill_processes_by_port
};

// 全局共享的服务器映射
static GLOBAL_SERVERS: LazyLock<Arc<Mutex<HashMap<String, PreviewServer>>>> = LazyLock::new(|| {
    Arc::new(Mutex::new(HashMap::new()))
});

#[derive(Debug, Clone)]
pub struct PreviewServer {
    pub id: String,
    pub port: u16,
    pub process: Option<u32>, // PID
    pub template_path: PathBuf,
}

#[derive(Debug, Clone)]
enum PreviewMode {
    Artifact,
    Window,
}

pub struct ReactPreviewManager {
    app_handle: AppHandle,
    shared_utils: SharedPreviewUtils,
}

impl ReactPreviewManager {
    pub fn new(app_handle: AppHandle) -> Self {
        let shared_utils = SharedPreviewUtils::new(app_handle.clone());
        Self {
            app_handle,
            shared_utils,
        }
    }

    // 获取 bun 可执行文件路径
    fn get_bun_executable(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        self.shared_utils.get_bun_executable()
    }

    pub fn create_preview_for_artifact(
        &self,
        component_code: String,
        component_name: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.create_preview_internal(component_code, component_name, PreviewMode::Artifact)
    }

    pub fn create_preview(
        &self,
        component_code: String,
        component_name: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.create_preview_internal(component_code, component_name, PreviewMode::Window)
    }

    fn create_preview_internal(
        &self,
        component_code: String,
        component_name: String,
        mode: PreviewMode,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let preview_id = "react".to_string();
        println!("🚀 [React Preview] 开始创建预览, ID: {}", preview_id);
        if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("artifact-preview-log", "开始创建 React 预览...");
        }

        let port = self.find_available_port()?;
        println!("🚀 [React Preview] 找到可用端口: {}", port);
        
        // 关闭已存在的预览实例
        let _ = self.close_preview(&preview_id);

        let (template_path, need_install_deps) =
            self.setup_template_project(&preview_id, &component_code, &component_name)?;
        println!("🚀 [React Preview] 模板项目已设置到: {:?}", template_path);

        let process_id = self.start_dev_server(&template_path, port, need_install_deps)?;
        println!("🚀 [React Preview] 开发服务器已启动, PID: {}", process_id);
        if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
            let _ = window.emit("artifact-preview-log", "预览服务启动");
        }

        let server = PreviewServer {
            id: preview_id.clone(),
            port,
            process: Some(process_id),
            template_path,
        };

        println!("🔧 [ReactPreview] 创建服务器对象: ID={}, Port={}, PID={:?}", preview_id, port, process_id);
        
        GLOBAL_SERVERS
            .lock()
            .unwrap()
            .insert(preview_id.clone(), server);

        // 等待开发服务器启动并执行相应操作
        let app_handle = self.app_handle.clone();
        let preview_id_clone = preview_id.clone();
        std::thread::spawn(move || {
            // 等待服务器启动
            println!("🚀 [React Preview] 等待服务器启动...");
            if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-preview-log", "等待服务器启动完毕...");
            }

            // 检测本地的端口是否已经启动完毕
            let mut retries = 20;
            while retries > 0 {
                if Self::is_port_open("127.0.0.1", port) {
                    println!("🚀 [React Preview] 服务器已检测到完毕");
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(300));
                retries -= 1;
            }

            // std::thread::sleep(std::time::Duration::from_secs(3));
            
            match mode {
                PreviewMode::Artifact => {
                    let preview_url = format!("http://localhost:{}", port);
                    println!("🚀 [React Preview] 预览已准备完成: {}", preview_url);
                    if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                        let _ = window.emit("artifact-preview-success", "预览服务器已启动完成");
                    }
                    
                    // 发送跳转事件，让前端窗口自动跳转到预览页面
                    if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                        let _ = window.emit("artifact-preview-redirect", preview_url);
                    }
                }
                PreviewMode::Window => {
                    println!("🚀 [React Preview] 尝试打开预览窗口");
                    if let Some(window) = app_handle.get_webview_window("artifact_preview") {
                        let _ = window.emit("artifact-preview-log", "打开预览窗口...");
                    }
                    let _ = Self::open_preview_window_static(&app_handle, &preview_id_clone, port);
                }
            }
        });

        println!("🚀 [React Preview] 预览创建成功, ID: {}", preview_id);
        Ok(preview_id)
    }

    pub fn close_preview(&self, preview_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut servers = GLOBAL_SERVERS.lock().unwrap();
        
        // 调试信息：显示当前所有服务器
        println!("🔧 [ReactPreview] 当前服务器列表:");
        for (id, server) in servers.iter() {
            println!("  - ID: {}, Port: {}, PID: {:?}", id, server.port, server.process);
        }
        println!("🔧 [ReactPreview] 尝试关闭服务器 ID: {}", preview_id);

        if let Some(server) = servers.remove(preview_id) {
            println!("🔧 [ReactPreview] 找到预览服务器: {}", preview_id);
            
            // 优先使用PID终止进程
            if let Some(pid) = server.process {
                println!("🔧 [ReactPreview] 准备终止进程 PID: {}", pid);
                match self.kill_process(pid) {
                    Ok(_) => {
                        println!("✅ [ReactPreview] 成功终止进程 PID: {}", pid);
                        // PID终止成功，无需再按端口清理
                    }
                    Err(e) => {
                        println!("❌ [ReactPreview] 终止进程失败 PID: {}, 错误: {}", pid, e);
                        // 尝试强制终止进程组
                        match self.kill_process_group(pid) {
                            Ok(_) => {
                                println!("✅ [ReactPreview] 成功强制终止进程组");
                            }
                            Err(e2) => {
                                println!("❌ [ReactPreview] 强制终止进程组也失败: {}", e2);
                                // 作为最后手段，尝试根据端口清理
                                println!("🔧 [ReactPreview] 尝试根据端口 {} 清理进程", server.port);
                                if let Err(e3) = self.kill_processes_by_port(server.port) {
                                    println!("❌ [ReactPreview] 根据端口清理进程失败: {}", e3);
                                } else {
                                    println!("✅ [ReactPreview] 成功根据端口清理进程");
                                }
                            }
                        }
                    }
                }
            } else {
                println!("⚠️ [ReactPreview] 服务器记录中没有进程 PID，尝试根据端口清理");
                // 没有PID记录，只能根据端口清理
                if let Err(e) = self.kill_processes_by_port(server.port) {
                    println!("❌ [ReactPreview] 根据端口清理进程失败: {}", e);
                } else {
                    println!("✅ [ReactPreview] 成功根据端口清理进程");
                }
            }
        } else {
            println!("⚠️ [ReactPreview] 未找到预览服务器: {}", preview_id);
            println!("🔧 [ReactPreview] 可能的原因:");
            println!("  1. 服务器创建失败");
            println!("  2. 服务器已被其他地方清理");
            println!("  3. 竞态条件导致数据不一致");
        }

        // 显示清理后的服务器列表
        println!("🔧 [ReactPreview] 清理后的服务器列表:");
        for (id, server) in servers.iter() {
            println!("  - ID: {}, Port: {}, PID: {:?}", id, server.port, server.process);
        }

        Ok(())
    }

    fn setup_template_project(
        &self,
        preview_id: &str,
        component_code: &str,
        _component_name: &str,
    ) -> Result<(PathBuf, bool), Box<dyn std::error::Error>> {
        let preview_dir = self.shared_utils.get_preview_directory("react", preview_id)?;
        println!("🛠️ [Setup] 设置预览目录: {:?}", preview_dir);

        // 获取模板源路径
        let template_source = self.shared_utils.get_template_source_path("react")?;
        println!("🛠️ [Setup] 模板源路径: {:?}", template_source);

        // 计算当前模板的哈希值
        let current_files_hash = self.shared_utils.calculate_template_files_hash(&template_source, "UserComponent.tsx")?;
        let current_deps_hash = self.shared_utils.calculate_deps_hash(&template_source)?;
        
        println!("🔍 [Setup] 当前模板文件哈希: {}", current_files_hash);
        println!("🔍 [Setup] 当前依赖哈希: {}", current_deps_hash);

        // 检查缓存
        let cached_info = self.shared_utils.get_template_cache("react");
        let mut need_copy_files = true;
        let mut need_install_deps = true;

        if let Ok(Some(cache)) = cached_info {
            println!("🔍 [Setup] 缓存文件哈希: {}", cache.files_hash);
            println!("🔍 [Setup] 缓存依赖哈希: {}", cache.deps_hash);
            
            // 检查文件是否需要更新
            if cache.files_hash == current_files_hash && preview_dir.exists() {
                need_copy_files = false;
                println!("✅ [Setup] 模板文件无变化，跳过复制");
            }
            
            // 检查依赖是否需要更新
            if cache.deps_hash == current_deps_hash && preview_dir.join("node_modules").exists() {
                need_install_deps = false;
                println!("✅ [Setup] 依赖文件无变化，跳过安装");
            }
        } else {
            println!("🔍 [Setup] 没有找到缓存信息，需要初始化");
        }

        // 如果需要复制文件
        if need_copy_files {
            println!("📂 [Setup] 开始复制模板文件...");
            self.shared_utils.copy_template(&template_source, &preview_dir)?;
            println!("✅ [Setup] 模板文件复制完成");
        }

        // 如果需要安装依赖
        if need_install_deps {
            println!("📦 [Setup] 需要安装/更新依赖");
            if let Some(window) = self.app_handle.get_webview_window("artifact_preview") {
                let _ = window.emit("artifact-preview-log", "安装/更新依赖");
            }
            // 删除现有的 node_modules（如果存在）
            let node_modules_dir = preview_dir.join("node_modules");
            if node_modules_dir.exists() {
                println!("🗑️ [Setup] 删除现有的 node_modules");
                let _ = fs::remove_dir_all(&node_modules_dir);
            }
        }

        // 保存新的缓存信息
        let new_cache = TemplateCache {
            files_hash: current_files_hash,
            deps_hash: current_deps_hash,
        };
        
        if let Err(e) = self.shared_utils.save_template_cache("react", &new_cache) {
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

        // 设置 bunfig.toml 缓存
        self.shared_utils.setup_bunfig_cache(project_path)?;

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

        // 为 Unix 系统创建新的进程组
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            vite_command.process_group(0); // 创建新的进程组
            println!("🔧 [DevServer] 为 Unix 系统创建新进程组");
        }

        // 为 Windows 系统创建新的进程组
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            vite_command.creation_flags(0x00000200); // CREATE_NEW_PROCESS_GROUP
            println!("🔧 [DevServer] 为 Windows 系统创建新进程组");
        }

        let child = vite_command.spawn();

        match child {
            Ok(mut child) => {
                let pid = child.id();
                println!("✅ [DevServer] Vite 服务器启动成功, PID: {}", pid);

                // 在后台线程中管理子进程生命周期，避免僵尸进程
                std::thread::spawn(move || {
                    // 等待子进程结束或者被终止
                    match child.wait() {
                        Ok(status) => {
                            println!("🔧 [DevServer] Vite 进程 PID {} 已结束，状态: {}", pid, status);
                        }
                        Err(e) => {
                            println!("⚠️ [DevServer] 等待 Vite 进程 PID {} 结束时出错: {}", pid, e);
                        }
                    }
                });

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
        self.shared_utils.find_available_port(3001, 4000)
    }

    fn kill_process(&self, pid: u32) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔧 [ReactPreview] 执行 kill_process PID: {}", pid);
        kill_process_by_pid(pid)
    }

    fn kill_process_group(&self, pid: u32) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔧 [ReactPreview] 执行 kill_process_group PID: {}", pid);
        kill_process_group_by_pid(pid)
    }

    fn kill_processes_by_port(&self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔧 [ReactPreview] 根据端口 {} 查找并终止进程", port);
        kill_processes_by_port(port)
    }

    fn is_port_open(ip: &str, port: u16) -> bool {
        SharedPreviewUtils::is_port_open(ip, port)
    }
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
