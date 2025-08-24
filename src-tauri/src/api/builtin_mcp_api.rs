use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use std::time::Duration;
use crate::db::mcp_db::MCPDatabase;
use reqwest;
use tokio::process::Command as TokioCommand;
use std::path::PathBuf;
use playwright::Playwright;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest { pub query: String }
pub fn is_aipp_builtin_command(command: &str) -> bool { command.trim().starts_with("aipp:") }


#[derive(Debug, Serialize, Deserialize)]
pub struct FetchRequest { pub url: String }

/// 内置搜索工具处理器
#[derive(Clone)]
pub struct BuiltinSearchHandler { 
    app_handle: AppHandle,
}

impl BuiltinSearchHandler {
    pub fn new(app_handle: AppHandle) -> Self { 
        println!("[HTML] Creating BuiltinSearchHandler");
        Self { 
            app_handle,
        }
    }

    /// 使用 Playwright 启动系统浏览器（不下载），并通过“持久上下文”加载页面、返回渲染后的 HTML。
    /// 配置优先级（来自 aipp:search 环境变量）：
    /// - BROWSER_EXECUTABLE: 自定义浏览器可执行文件路径（优先）
    /// - USER_DATA_DIR: 复用的用户数据目录（可直接指向系统浏览器的配置目录；注意可能被占用）
    /// - HEADLESS: 是否无头模式（默认 true）
    /// - USER_AGENT: 可选自定义 UA
    /// - BYPASS_CSP: 是否绕过 CSP（默认 false）
    async fn playwright_fetch_html(&self, url: &str) -> Result<String, String> {
        let envs = get_env_map_for_aipp_command(&self.app_handle, "aipp:search").unwrap_or_default();

        let headless = envs
            .get("HEADLESS")
            .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(true);
        let bypass_csp = envs
            .get("BYPASS_CSP")
            .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);
        let ua_opt = envs.get("USER_AGENT").cloned();
        let proxy_opt = envs.get("PROXY_SERVER").cloned();

        // 选择用户数据目录：优先环境变量，否则使用应用数据目录下的专用 profile
        let user_data_dir = if let Some(p) = envs.get("USER_DATA_DIR") {
            PathBuf::from(p)
        } else {
            let base = self
                .app_handle
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app data dir: {}", e))?;
            base.join("playwright_profile")
        };
        if let Err(e) = fs::create_dir_all(&user_data_dir) {
            println!("[PW] Failed to create user_data_dir {:?}: {}", user_data_dir, e);
        }

        // 选择浏览器可执行文件：优先环境变量，其次自动查找系统 Edge/Chrome
        let browser_exe = if let Some(p) = envs.get("BROWSER_EXECUTABLE") {
            let pb = PathBuf::from(p);
            if !pb.exists() {
                return Err(format!("BROWSER_EXECUTABLE not found: {}", p));
            }
            Some(pb)
        } else {
            Self::find_headless_browser()
        };

        let playwright = Playwright::initialize()
            .await
            .map_err(|e| format!("Playwright init error: {}", e))?;
        // 不调用 prepare()，避免下载捆绑浏览器
        let chromium = playwright.chromium();

        let mut launcher = chromium.persistent_context_launcher(&user_data_dir);
        if let Some(exe) = &browser_exe {
            launcher = launcher.executable(exe.as_path());
        }
        launcher = launcher.headless(headless);
        if bypass_csp {
            launcher = launcher.bypass_csp(true);
        }
        if let Some(ua) = ua_opt.as_deref() {
            launcher = launcher.user_agent(ua);
        }
        if let Some(proxy) = proxy_opt.as_deref() {
            use playwright::api::ProxySettings;
            let proxy_settings = ProxySettings {
                server: proxy.to_string(),
                bypass: None,
                username: None,
                password: None,
            };
            launcher = launcher.proxy(proxy_settings);
        }

        let context = launcher
            .launch()
            .await
            .map_err(|e| format!("Playwright launch error: {}", e))?;
        let page = context
            .new_page()
            .await
            .map_err(|e| format!("Playwright new_page error: {}", e))?;
        page
            .goto_builder(url)
            .goto()
            .await
            .map_err(|e| format!("Playwright goto error: {}", e))?;

        // 等待选择器：优先使用环境变量 WAIT_SELECTORS，否则使用默认示例选择器
        let selectors: Vec<String> = envs
            .get("WAIT_SELECTORS")
            .map(|raw| {
                raw.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec![
                "#b_content > main".to_string(),
                "#search".to_string(),
                "body > div.app-box-center".to_string(),
            ]);

        if !selectors.is_empty() {
            let timeout_ms: u64 = envs
                .get("WAIT_TIMEOUT_MS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(15000);
            let poll_ms: u64 = envs
                .get("WAIT_POLL_MS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(250);

            let start = std::time::Instant::now();
            let sels_json = serde_json::to_string(&selectors).unwrap_or("[]".to_string());
            let script = format!(
                "() => {{ const sels = {}; for (const s of sels) {{ if (document.querySelector(s)) return s; }} return null; }}",
                sels_json
            );
            let mut matched: Option<String> = None;
            loop {
                let found: Option<String> = page
                    .eval(&script)
                    .await
                    .map_err(|e| format!("Playwright wait eval error: {}", e))?;
                if let Some(sel) = found {
                    matched = Some(sel);
                    break;
                }
                if start.elapsed() >= Duration::from_millis(timeout_ms) {
                    break;
                }
                page.wait_for_timeout(poll_ms as f64).await;
            }
            if let Some(sel) = matched {
                println!("[PW] Waited selectors matched: {}", sel);
            } else {
                println!("[PW] Waited selectors not found within {} ms", timeout_ms);
            }
        } else {
            // 默认轻量等待，避免过长阻塞
            page.wait_for_timeout(800.0).await;
        }

        let html: String = page
            .eval("() => document.documentElement.outerHTML")
            .await
            .map_err(|e| format!("Playwright eval error: {}", e))?;

        if html.trim().is_empty() {
            return Err("Empty HTML from Playwright".to_string());
        }
        Ok(html)
    }

    /// 查找可用的无头浏览器（优先 Edge，其次 Chrome）
    fn find_headless_browser() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            let candidates = [
                // Chrome
                r"C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
                r"C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
                "chrome.exe",
                // Edge
                r"C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
                r"C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
                "msedge.exe",
            ];
            for path in candidates.iter() {
                let p = PathBuf::from(path);
                if p.is_file() {
                    return Some(p);
                }
                // try calling it from PATH
                if path.ends_with(".exe") && which::which(path).is_ok() {
                    return which::which(path).ok();
                }
            }
            None
        }
        #[cfg(not(target_os = "windows"))]
        {
            let candidates = ["chromium", "google-chrome", "chrome", "edge", "msedge", "chromium-browser"]; 
            for name in candidates.iter() {
                if let Ok(p) = which::which(name) {
                    return Some(p);
                }
            }
            None
        }
    }

    /// 使用系统浏览器以无头模式打开并导出 DOM（支持部分动态渲染，较接近“真实浏览器”）
    async fn headless_dump_dom(&self, url: &str) -> Result<String, String> {
        let browser = Self::find_headless_browser().ok_or_else(|| "No headless browser (Edge/Chrome) found".to_string())?;
        println!("[HEADLESS] Using browser: {}", browser.display());

        let mut cmd = TokioCommand::new(browser);
        let _ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";
        cmd.arg("--headless=new")
            .arg("--disable-gpu")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-dev-shm-usage")
            .arg("--disable-extensions")
            .arg("--disable-blink-features=AutomationControlled")
            .arg("--virtual-time-budget=15000")
            .arg("--timeout=45000")
            .arg("--hide-scrollbars")
            .arg("--window-size=1280,800")
            .arg("--dump-dom")
            .arg(url);

        let output = cmd.output().await.map_err(|e| format!("Failed to run headless browser: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Headless browser exited with code {:?}: {}", output.status.code(), stderr.trim()));
        }
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        if stdout.trim().is_empty() {
            return Err("Empty DOM output from headless browser".to_string());
        }
        println!("[HEADLESS] Dumped {} bytes", stdout.len());
        Ok(stdout)
    }

    /// 使用 HTTP 直接抓取页面内容，避免 WebView JS 注入与 CSP/同源等限制
    async fn http_fetch_html(&self, url: &str, user_agent: Option<&str>) -> Result<String, String> {
        let envs = get_env_map_for_aipp_command(&self.app_handle, "aipp:search").unwrap_or_default();
        let ua = user_agent.unwrap_or("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36");
        
        let mut client_builder = reqwest::Client::builder()
            .user_agent(ua)
            .redirect(reqwest::redirect::Policy::limited(10))
            .timeout(Duration::from_secs(15));
            
        // 添加代理支持
        if let Some(proxy_server) = envs.get("PROXY_SERVER") {
            let proxy = reqwest::Proxy::all(proxy_server)
                .map_err(|e| format!("Invalid proxy configuration: {}", e))?;
            client_builder = client_builder.proxy(proxy);
        }
        
        let client = client_builder
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let resp = client
            .get(url)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await
            .map_err(|e| format!("HTTP request error: {}", e))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(format!("HTTP status {} when fetching {}", status.as_u16(), url));
        }

        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let text = resp.text().await.map_err(|e| format!("Failed to read body: {}", e))?;
        if text.trim().is_empty() {
            return Err("Empty response body".to_string());
        }

        // 简单判断：不是 HTML 也放行，由上层决定如何处理
        println!("[HTTP] Fetched {} bytes (content-type: {})", text.len(), content_type);
        Ok(text)
    }

    // 旧的 WebView 注入方案已删除（跨域与平台差异导致不稳定）

    async fn search_web(&self, query: &str) -> Result<serde_json::Value, String> {
        println!("[SEARCH] Starting search for query: {}", query);
        
        // 从环境变量构建搜索 URL
        let envs = get_env_map_for_aipp_command(&self.app_handle, "aipp:search").unwrap_or_default();
        let base = envs.get("SEARCH_URL").map(|s| s.as_str()).unwrap_or("https://duckduckgo.com/html/?q=");
        let search_url = build_search_url(base, query);
        
        println!("[SEARCH] Search URL: {}", search_url);

        // 首选：Playwright（系统浏览器、可复用会话/更强动态能力）
        match self.playwright_fetch_html(&search_url).await {
            Ok(html) => {
                println!("[SEARCH][PW] HTML fetched successfully");
                return Ok(serde_json::json!({
                    "query": query,
                    "request_url": search_url,
                    "source": base,
                    "via": "playwright",
                    "html_content": html,
                    "message": "Search completed via Playwright",
                }));
            }
            Err(pw_err) => {
                println!("[SEARCH][PW] Failed: {}", pw_err);
            }
        }

        // 次选：系统浏览器 --dump-dom（无需下载，较轻量）
        match self.headless_dump_dom(&search_url).await {
            Ok(html) => {
                println!("[SEARCH][HEADLESS] DOM dumped successfully");
                return Ok(serde_json::json!({
                    "query": query,
                    "request_url": search_url,
                    "source": base,
                    "via": "headless",
                    "html_content": html,
                    "message": "Search completed via headless browser",
                }));
            }
            Err(h_err) => println!("[SEARCH][HEADLESS] Failed: {}", h_err),
        }

        // 次选：HTTP 直接抓取（某些站点仍可用）
        if let Ok(html) = self.http_fetch_html(&search_url, None).await {
            println!("[SEARCH][HTTP] HTML fetched successfully");
            return Ok(serde_json::json!({
                "query": query,
                "request_url": search_url,
                "source": base,
                "via": "http",
                "html_content": html,
                "message": "Search completed via HTTP fetch",
            }));
        }

        // 兜底：尝试 WebView 导航（不再承诺提取 HTML，仅返回状态信息）
        if let Err(e) = crate::window::ensure_hidden_search_window(self.app_handle.clone()).await {
            println!("[SEARCH][WebView] Failed to create hidden search window: {}", e);
        } else if let Some(window) = self.app_handle.get_webview_window("hidden_search") {
            let _ = window.navigate(search_url.parse().map_err(|e| format!("Invalid URL: {}", e))?);
            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        Ok(serde_json::json!({
            "query": query,
            "request_url": search_url,
            "source": base,
            "via": "webview",
            "html_content": "",
            "message": "HTTP fetch failed; navigated WebView but extraction is disabled",
        }))
    }
    
    async fn fetch_url(&self, url: &str) -> Result<serde_json::Value, String> {
        // 首选：Playwright（系统浏览器、可复用会话/更强动态能力）
        match self.playwright_fetch_html(url).await {
            Ok(html) => {
                return Ok(serde_json::json!({
                    "url": url,
                    "status": "success",
                    "via": "playwright",
                    "html_content": html,
                    "message": "Fetched via Playwright",
                }));
            }
            Err(pw_err) => {
                println!("[FETCH][PW] Failed: {}", pw_err);
            }
        }

        // 次选：系统浏览器 --dump-dom
        match self.headless_dump_dom(url).await {
            Ok(html) => {
                return Ok(serde_json::json!({
                    "url": url,
                    "status": "success",
                    "via": "headless",
                    "html_content": html,
                    "message": "Fetched via headless browser",
                }));
            }
            Err(h_err) => println!("[FETCH][HEADLESS] Failed: {}", h_err),
        }

        // 次选：HTTP 直接抓取
        if let Ok(html) = self.http_fetch_html(url, None).await {
            return Ok(serde_json::json!({
                "url": url,
                "status": "success",
                "via": "http",
                "html_content": html,
                "message": "Fetched via HTTP",
            }));
        }

        // 兜底：WebView 导航但不提取
        if let Ok(()) = crate::window::ensure_hidden_search_window(self.app_handle.clone()).await {
            if let Some(window) = self.app_handle.get_webview_window("hidden_search") {
                let _ = window.navigate(url.parse().map_err(|e| format!("Invalid URL: {}", e))?);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }

        Ok(serde_json::json!({
            "url": url,
            "status": "partial_success",
            "via": "webview",
            "html_content": "",
            "message": "HTTP fetch failed; navigated WebView but extraction is disabled",
        }))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinTemplateEnvVar {
    pub key: String,
    pub required: bool,
    pub tip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinTemplateInfo {
    pub id: String,           // unique key, e.g., "search"
    pub name: String,         // 显示名
    pub description: String,  // 描述
    pub command: String,      // aipp:search
    pub transport_type: String, // 固定 "stdio"
    pub required_envs: Vec<BuiltinTemplateEnvVar>,
}

fn builtin_templates() -> Vec<BuiltinTemplateInfo> {
    vec![BuiltinTemplateInfo {
        id: "search".into(),
        name: "内置搜索工具".into(),
        description: "内置的网络搜索和网页访问工具".into(),
        command: "aipp:search".into(),
        transport_type: "stdio".into(),
        required_envs: vec![
            BuiltinTemplateEnvVar {
                key: "SEARCH_URL".into(),
                required: true,
                tip: Some("用于发起搜索的URL，例如 https://duckduckgo.com/html/?q= 或 https://www.google.com/search?q=（也可使用 {} 占位符）".into()),
            },
            BuiltinTemplateEnvVar {
                key: "USER_DATA_DIR".into(),
                required: false,
                tip: Some("浏览器用户数据目录，可以指向用户日常使用的浏览器profile目录以共享配置".into()),
            },
            BuiltinTemplateEnvVar {
                key: "PROXY_SERVER".into(),
                required: false,
                tip: Some("代理服务器地址，格式：http://proxy:port 或 socks5://proxy:port".into()),
            },
            BuiltinTemplateEnvVar {
                key: "BROWSER_EXECUTABLE".into(),
                required: false,
                tip: Some("自定义浏览器可执行文件路径，优先级高于自动查找".into()),
            },
            BuiltinTemplateEnvVar {
                key: "HEADLESS".into(),
                required: false,
                tip: Some("是否无头模式运行浏览器（默认true），设置为false可显示浏览器窗口".into()),
            },
            BuiltinTemplateEnvVar {
                key: "USER_AGENT".into(),
                required: false,
                tip: Some("自定义User-Agent字符串".into()),
            },
            BuiltinTemplateEnvVar {
                key: "BYPASS_CSP".into(),
                required: false,
                tip: Some("是否绕过CSP内容安全策略（默认false）".into()),
            },
            BuiltinTemplateEnvVar {
                key: "WAIT_SELECTORS".into(),
                required: false,
                tip: Some("等待页面元素的CSS选择器，多个用逗号分隔".into()),
            },
            BuiltinTemplateEnvVar {
                key: "WAIT_TIMEOUT_MS".into(),
                required: false,
                tip: Some("等待页面元素的超时时间（毫秒），默认15000ms".into()),
            },
        ],
    }]
}

pub fn aipp_command_id(command: &str) -> Option<String> {
    if is_aipp_builtin_command(command) {
        Some(command.trim().trim_start_matches("aipp:").to_string())
    } else {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

pub fn get_builtin_tools_for_command(command: &str) -> Vec<BuiltinToolInfo> {
    match aipp_command_id(command).as_deref() {
        Some("search") => vec![
            BuiltinToolInfo {
                name: "search_web".into(),
                description: "搜索网络内容".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {"query": {"type": "string", "description": "搜索查询关键词"}},
                    "required": ["query"]
                }),
            },
            BuiltinToolInfo {
                name: "fetch_url".into(),
                description: "获取网页内容".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {"url": {"type": "string", "description": "要获取内容的URL"}},
                    "required": ["url"]
                }),
            },
        ],
        _ => vec![],
    }
}

// 列出内置模板
#[tauri::command]
pub async fn list_aipp_builtin_templates() -> Result<Vec<BuiltinTemplateInfo>, String> {
    Ok(builtin_templates())
}

// 创建/更新一个内置服务器实例（保存到 DB）
#[tauri::command]
pub async fn add_or_update_aipp_builtin_server(
    app_handle: AppHandle,
    template_id: String,
    name: Option<String>,
    description: Option<String>,
    envs: Option<std::collections::HashMap<String, String>>,
) -> Result<i64, String> {
    let templates = builtin_templates();
    let tpl = templates
        .into_iter()
        .find(|t| t.id == template_id)
        .ok_or_else(|| format!("Unknown builtin template id: {}", template_id))?;

    // envs to multiline string
    let env_str = envs.map(|m| {
        m.into_iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("\n")
    });

    let db = MCPDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let server_id = db
        .upsert_mcp_server_with_builtin(
            name.as_deref().unwrap_or(&tpl.name),
            description.as_deref().or(Some(&tpl.description)),
            &tpl.transport_type, // stdio
            Some(&tpl.command),   // aipp:*
            env_str.as_deref(),
            None,
            Some(20000),
            false,
            true,
            true,
        )
        .map_err(|e| e.to_string())?;

    // 注册工具
    for tool in get_builtin_tools_for_command(&tpl.command) {
        db.upsert_mcp_server_tool(
            server_id,
            &tool.name,
            Some(&tool.description),
            Some(&tool.input_schema.to_string()),
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(server_id)
}

/// 执行内置搜索工具
#[tauri::command]
pub async fn execute_aipp_builtin_tool(
    app_handle: AppHandle,
    server_command: String,
    tool_name: String,
    parameters: String,
) -> Result<String, String> {
    let handler = BuiltinSearchHandler::new(app_handle.clone());
    let args: serde_json::Value = serde_json::from_str(&parameters)
        .map_err(|e| format!("Invalid parameters: {}", e))?;

    // 路由到具体内置实现
    let cmd_id = aipp_command_id(&server_command).ok_or("Not an aipp builtin command")?;
    let result_value = match (cmd_id.as_str(), tool_name.as_str()) {
        ("search", "search_web") => {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: query".to_string())?;
            match handler.search_web(query).await {
                Ok(v) => serde_json::json!({
                    "content": [{"type": "json", "json": v}],
                    "isError": false
                }),
                Err(e) => serde_json::json!({
                    "content": [{"type": "text", "text": e}],
                    "isError": true
                }),
            }
        }
        ("search", "fetch_url") => {
            let url = args
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: url".to_string())?;
            match handler.fetch_url(url).await {
                Ok(v) => serde_json::json!({
                    "content": [{"type": "json", "json": v}],
                    "isError": false
                }),
                Err(e) => serde_json::json!({
                    "content": [{"type": "text", "text": e}],
                    "isError": true
                }),
            }
        }
        _ => serde_json::json!({
            "content": [{"type": "text", "text": format!("Unknown builtin/tool: {} / {}", cmd_id, tool_name)}],
            "isError": true
        }),
    };

    Ok(serde_json::to_string(&result_value).unwrap_or_else(|_| "{}".to_string()))
}
/// 检查是否为内置MCP工具（基于命令 aipp:*）
pub fn is_builtin_mcp_call(command: &str) -> bool { is_aipp_builtin_command(command) }

// ---------------- helpers -----------------

/// 从数据库获取指定 aipp 命令的环境变量，解析为 HashMap
fn get_env_map_for_aipp_command(
    app_handle: &AppHandle,
    command: &str,
) -> Option<std::collections::HashMap<String, String>> {
    let db = MCPDatabase::new(app_handle).ok()?;
    // 查询内置服务器（is_builtin=1）且命令匹配
    let mut stmt = db
        .conn
        .prepare("SELECT environment_variables FROM mcp_server WHERE command = ? AND is_builtin = 1 LIMIT 1")
        .ok()?;
    let env_text: Option<String> = match stmt.query_row([command], |row| row.get::<_, Option<String>>(0)) {
        Ok(v) => v,
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(_) => return None,
    };
    let mut map = std::collections::HashMap::new();
    if let Some(text) = env_text {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            if let Some((k, v)) = line.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    Some(map)
}

fn build_search_url(base: &str, query: &str) -> String {
    let encoded = urlencoding::encode(query);
    if base.contains("{}") {
        base.replace("{}", &encoded)
    } else if base.ends_with('=') || base.ends_with('/') || base.ends_with('?') || base.ends_with('&') {
        format!("{}{}", base, encoded)
    } else {
        format!("{}{}{}", base, if base.contains('?') { "&q=" } else { "?q=" }, encoded)
    }
}

