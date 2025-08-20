use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::sync::LazyLock;
use tauri::{AppHandle, Manager, Emitter};

use crate::artifacts::shared_components::{
    SharedPreviewUtils, TemplateCache, 
    kill_process_by_pid, kill_process_group_by_pid, kill_processes_by_port
};

// 全局共享的服务器映射
static GLOBAL_ARTIFACT_SERVERS: LazyLock<Arc<Mutex<HashMap<String, ArtifactServer>>>> = LazyLock::new(|| {
    Arc::new(Mutex::new(HashMap::new()))
});

#[derive(Debug, Clone)]
pub struct ArtifactServer {
    pub id: String,
    pub port: u16,
    pub process: Option<u32>, // PID
    pub template_path: PathBuf,
}

pub struct ReactArtifactRunner {
    app_handle: AppHandle,
    shared_utils: SharedPreviewUtils,
}

impl ReactArtifactRunner {
    pub fn new(app_handle: AppHandle) -> Self {
        let shared_utils = SharedPreviewUtils::new(app_handle.clone());
        Self {
            app_handle,
            shared_utils,
        }
    }

    /// 运行保存的React artifact
    pub async fn run_artifact(
        &self,
        artifact_id: i64,
        component_code: String,
        component_name: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let server_id = format!("react-artifact-{}", artifact_id);
        println!("🚀 [ReactRunner] 开始运行 React artifact, ID: {}", server_id);

        // 发送日志到artifact窗口
        if let Some(window) = self.app_handle.get_webview_window("artifact") {
            let _ = window.emit("artifact-log", "开始运行 React 组件...");
        }

        let port = self.shared_utils.find_available_port(3001, 4000)?;
        println!("🚀 [ReactRunner] 找到可用端口: {}", port);
        
        // 关闭已存在的artifact实例
        let _ = self.close_artifact(&server_id);

        let (template_path, need_install_deps) =
            self.setup_artifact_project(&server_id, &component_code, &component_name)?;
        println!("🚀 [ReactRunner] 组件项目已设置到: {:?}", template_path);

        let process_id = self.start_server(&template_path, port, need_install_deps).await?;
        println!("🚀 [ReactRunner] 服务器已启动, PID: {}", process_id);

        if let Some(window) = self.app_handle.get_webview_window("artifact") {
            let _ = window.emit("artifact-log", "React 组件服务启动完成");
        }

        let server = ArtifactServer {
            id: server_id.clone(),
            port,
            process: Some(process_id),
            template_path,
        };

        println!("🔧 [ReactRunner] 创建服务器对象: ID={}, Port={}, PID={:?}", server_id, port, process_id);
        
        GLOBAL_ARTIFACT_SERVERS
            .lock()
            .unwrap()
            .insert(server_id.clone(), server);

        // 等待服务器启动
        self.wait_for_server_ready(port).await?;

        let preview_url = format!("http://localhost:{}", port);
        println!("🚀 [ReactRunner] React 组件已准备完成: {}", preview_url);

        if let Some(window) = self.app_handle.get_webview_window("artifact") {
            let _ = window.emit("artifact-success", "React 组件已准备完成");
            let _ = window.emit("artifact-redirect", preview_url.clone());
        }

        Ok(preview_url)
    }

    /// 关闭artifact服务器
    pub fn close_artifact(&self, server_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut servers = GLOBAL_ARTIFACT_SERVERS.lock().unwrap();
        
        println!("🔧 [ReactRunner] 尝试关闭服务器 ID: {}", server_id);

        if let Some(server) = servers.remove(server_id) {
            println!("🔧 [ReactRunner] 找到artifact服务器: {}", server_id);
            
            // 优先使用PID终止进程
            if let Some(pid) = server.process {
                println!("🔧 [ReactRunner] 准备终止进程 PID: {}", pid);
                match kill_process_by_pid(pid) {
                    Ok(_) => {
                        println!("✅ [ReactRunner] 成功终止进程 PID: {}", pid);
                    }
                    Err(e) => {
                        println!("❌ [ReactRunner] 终止进程失败 PID: {}, 错误: {}", pid, e);
                        // 尝试强制终止进程组
                        match kill_process_group_by_pid(pid) {
                            Ok(_) => {
                                println!("✅ [ReactRunner] 成功强制终止进程组");
                            }
                            Err(e2) => {
                                println!("❌ [ReactRunner] 强制终止进程组也失败: {}", e2);
                                // 作为最后手段，尝试根据端口清理
                                println!("🔧 [ReactRunner] 尝试根据端口 {} 清理进程", server.port);
                                if let Err(e3) = kill_processes_by_port(server.port) {
                                    println!("❌ [ReactRunner] 根据端口清理进程失败: {}", e3);
                                } else {
                                    println!("✅ [ReactRunner] 成功根据端口清理进程");
                                }
                            }
                        }
                    }
                }
            } else {
                println!("⚠️ [ReactRunner] 服务器记录中没有进程 PID，尝试根据端口清理");
                if let Err(e) = kill_processes_by_port(server.port) {
                    println!("❌ [ReactRunner] 根据端口清理进程失败: {}", e);
                } else {
                    println!("✅ [ReactRunner] 成功根据端口清理进程");
                }
            }
        } else {
            println!("⚠️ [ReactRunner] 未找到artifact服务器: {}", server_id);
        }

        Ok(())
    }

    /// 设置artifact项目
    fn setup_artifact_project(
        &self,
        server_id: &str,
        component_code: &str,
        _component_name: &str,
    ) -> Result<(PathBuf, bool), Box<dyn std::error::Error>> {
        let artifact_dir = self.shared_utils.get_preview_directory("react-artifacts", server_id)?;
        println!("🛠️ [ReactRunner] 设置artifact目录: {:?}", artifact_dir);

        // 获取模板源路径
        let template_source = self.shared_utils.get_template_source_path("react")?;
        println!("🛠️ [ReactRunner] 模板源路径: {:?}", template_source);

        // 计算当前模板的哈希值
        let current_files_hash = self.shared_utils.calculate_template_files_hash(&template_source, "UserComponent.tsx")?;
        let current_deps_hash = self.shared_utils.calculate_deps_hash(&template_source)?;

        // 检查缓存（使用独立的缓存key）
        let cached_info = self.shared_utils.get_template_cache("react-artifacts");
        let mut need_copy_files = true;
        let mut need_install_deps = true;

        if let Ok(Some(cache)) = cached_info {
            // 检查文件是否需要更新
            if cache.files_hash == current_files_hash && artifact_dir.exists() {
                need_copy_files = false;
                println!("✅ [ReactRunner] 模板文件无变化，跳过复制");
            }
            
            // 检查依赖是否需要更新
            if cache.deps_hash == current_deps_hash && artifact_dir.join("node_modules").exists() {
                need_install_deps = false;
                println!("✅ [ReactRunner] 依赖文件无变化，跳过安装");
            }
        }

        // 如果需要复制文件
        if need_copy_files {
            println!("📂 [ReactRunner] 开始复制模板文件...");
            self.shared_utils.copy_template(&template_source, &artifact_dir)?;
            println!("✅ [ReactRunner] 模板文件复制完成");
        }

        // 如果需要安装依赖
        if need_install_deps {
            println!("📦 [ReactRunner] 需要安装/更新依赖");
            if let Some(window) = self.app_handle.get_webview_window("artifact") {
                let _ = window.emit("artifact-log", "安装/更新依赖");
            }
            // 删除现有的 node_modules（如果存在）
            let node_modules_dir = artifact_dir.join("node_modules");
            if node_modules_dir.exists() {
                println!("🗑️ [ReactRunner] 删除现有的 node_modules");
                let _ = fs::remove_dir_all(&node_modules_dir);
            }
        }

        // 保存新的缓存信息
        let new_cache = TemplateCache {
            files_hash: current_files_hash,
            deps_hash: current_deps_hash,
        };
        
        if let Err(e) = self.shared_utils.save_template_cache("react-artifacts", &new_cache) {
            println!("⚠️ [ReactRunner] 保存缓存信息失败: {}", e);
        } else {
            println!("✅ [ReactRunner] 缓存信息已更新");
        }

        // 写入组件代码到 UserComponent.tsx
        let component_file = artifact_dir.join("src").join("UserComponent.tsx");
        println!("🛠️ [ReactRunner] 写入组件文件到: {:?}", component_file);

        fs::write(&component_file, component_code)?;
        println!("🛠️ [ReactRunner] 组件文件写入完成");

        Ok((artifact_dir, need_install_deps))
    }

    /// 启动服务器（简化版，专注稳定运行）
    async fn start_server(
        &self,
        project_path: &PathBuf,
        port: u16,
        force_install: bool,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        println!("🔧 [ReactRunner] 在项目路径启动服务器: {:?}", project_path);

        // 获取 bun 可执行文件路径
        let bun_executable = self.shared_utils.get_bun_executable()?;
        println!("🔧 [ReactRunner] Bun 可执行文件: {:?}", bun_executable);

        // 检查项目路径和package.json
        let package_json = project_path.join("package.json");
        if !package_json.exists() {
            return Err(format!("package.json 不存在: {:?}", package_json).into());
        }

        // 设置 bunfig.toml 缓存
        self.shared_utils.setup_bunfig_cache(project_path)?;

        // 安装依赖（如果需要的话）
        if force_install || !project_path.join("node_modules").exists() {
            println!("🔧 [ReactRunner] 开始安装依赖...");
            let install_result = Command::new(&bun_executable)
                .args(&["install", "--force"])
                .current_dir(project_path)
                .output();

            match install_result {
                Ok(output) => {
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        return Err(format!("Bun install 失败:\\nStderr: {}\\nStdout: {}", stderr, stdout).into());
                    }
                    println!("✅ [ReactRunner] 依赖安装成功");
                }
                Err(e) => {
                    return Err(format!("无法执行 bun install: {}", e).into());
                }
            }
        } else {
            println!("✅ [ReactRunner] 依赖已存在，跳过安装");
        }

        // 启动 Vite 开发服务器
        println!("🔧 [ReactRunner] 启动 Vite 服务器...");

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
            vite_command.process_group(0);
        }

        // 为 Windows 系统创建新的进程组
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            vite_command.creation_flags(0x00000200);
        }

        let child = vite_command.spawn()?;
        let pid = child.id();
        println!("✅ [ReactRunner] Vite 服务器启动成功, PID: {}", pid);

        // 在后台线程中管理子进程生命周期
        std::thread::spawn(move || {
            let mut child = child;
            match child.wait() {
                Ok(status) => {
                    println!("🔧 [ReactRunner] Vite 进程 PID {} 已结束，状态: {}", pid, status);
                }
                Err(e) => {
                    println!("⚠️ [ReactRunner] 等待 Vite 进程 PID {} 结束时出错: {}", pid, e);
                }
            }
        });

        Ok(pid)
    }

    /// 等待服务器准备就绪
    async fn wait_for_server_ready(&self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 [ReactRunner] 等待服务器启动...");
        if let Some(window) = self.app_handle.get_webview_window("artifact") {
            let _ = window.emit("artifact-log", "等待服务器启动完毕...");
        }

        let mut retries = 20;
        while retries > 0 {
            if SharedPreviewUtils::is_port_open("127.0.0.1", port) {
                println!("🚀 [ReactRunner] 服务器已检测到完毕");
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            retries -= 1;
        }

        if retries == 0 {
            return Err("服务器启动超时".into());
        }

        Ok(())
    }
}

// Tauri 命令接口
#[tauri::command]
pub async fn run_react_artifact(
    app_handle: AppHandle,
    artifact_id: i64,
    component_code: String,
    component_name: String,
) -> Result<String, String> {
    let runner = ReactArtifactRunner::new(app_handle);
    runner
        .run_artifact(artifact_id, component_code, component_name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn close_react_artifact(app_handle: AppHandle, artifact_id: i64) -> Result<(), String> {
    let server_id = format!("react-artifact-{}", artifact_id);
    println!("🔧 [ReactRunner] 关闭artifact服务器: {}", server_id);
    let runner = ReactArtifactRunner::new(app_handle);
    runner
        .close_artifact(&server_id)
        .map_err(|e| e.to_string())
}