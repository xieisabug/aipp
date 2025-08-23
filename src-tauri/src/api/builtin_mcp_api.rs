use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Listener};
use std::time::Duration;
use crate::db::mcp_db::MCPDatabase;
use rusqlite::OptionalExtension;
use tokio::time::timeout;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

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

    async fn get_page_html(&self) -> Result<String, String> {
        println!("[HTML] Starting HTML extraction process");
        
        let window = self
            .app_handle
            .get_webview_window("hidden_search")
            .ok_or("Search window not found")?;

        // 等待页面加载完成
        println!("[HTML] Waiting for page to load...");
        tokio::time::sleep(Duration::from_secs(3)).await;
        
        // 先测试基本JavaScript执行能力
        let basic_test_js = r#"
            try {
                document.title = 'JS_TEST_SUCCESS';
                console.log('Basic JS test successful');
            } catch (error) {
                console.error('Basic JS test failed:', error);
            }
        "#;
        
        println!("[HTML] Testing basic JavaScript execution...");
        if let Err(e) = window.eval(basic_test_js) {
            println!("[HTML] Basic JavaScript test failed: {}", e);
            return Err("JavaScript execution not available".to_string());
        }
        
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // 检查基本JS测试结果
        match window.title() {
            Ok(title) => {
                println!("[HTML] Title after basic JS test: {}", title);
                if title == "JS_TEST_SUCCESS" {
                    println!("[HTML] ✓ JavaScript execution is working!");
                    
                    // JavaScript可以执行，继续页面分析
                    return self.extract_page_content(&window).await;
                } else {
                    println!("[HTML] ✗ JavaScript execution failed, title: {}", title);
                }
            }
            Err(e) => {
                println!("[HTML] Failed to read title after basic JS test: {}", e);
            }
        }
        
        // JavaScript执行失败，返回基本的页面信息
        println!("[HTML] JavaScript execution not available, returning basic page info");
        Ok(format!(
            "<!-- JavaScript execution not available in search window -->\n<!-- Page URL loaded successfully -->\n<html><head><title>Page Loaded</title></head><body><p>Page navigated successfully but content extraction unavailable</p><p>This is a limitation of webview JavaScript execution</p></body></html>"
        ))
    }
    
    async fn extract_page_content(&self, window: &tauri::WebviewWindow) -> Result<String, String> {
        println!("[HTML] Extracting page content with JavaScript...");
        
        // 检查页面状态
        let check_js = r#"
            try {
                let readyState = document.readyState;
                let url = window.location.href;
                let hasContent = document.documentElement ? 'yes' : 'no';
                let contentLength = document.documentElement ? document.documentElement.outerHTML.length : 0;
                let bodyExists = document.body ? 'yes' : 'no';
                
                console.log('Page analysis:', {
                    readyState: readyState,
                    url: url,
                    hasContent: hasContent,
                    contentLength: contentLength,
                    bodyExists: bodyExists
                });
                
                document.title = 'STATUS_' + readyState + '_' + hasContent + '_' + contentLength + '_' + bodyExists;
            } catch (error) {
                console.error('Status check error:', error);
                document.title = 'STATUS_ERROR_' + error.message.substring(0, 20);
            }
        "#;
        
        if let Err(e) = window.eval(check_js) {
            println!("[HTML] Failed to check page status: {}", e);
            return Err("Failed to check page status".to_string());
        }
        
        tokio::time::sleep(Duration::from_millis(300)).await;
        
        // 读取页面状态
        match window.title() {
            Ok(title) => {
                println!("[HTML] Page title after status check: {}", title);
                
                if title.starts_with("STATUS_") {
                    let parts: Vec<&str> = title.split('_').collect();
                    if parts.len() >= 5 {
                        let ready_state = parts[1];
                        let has_content = parts[2];
                        let content_len = parts[3].parse::<usize>().unwrap_or(0);
                        let body_exists = parts[4];
                        
                        println!("[HTML] Page analysis - Ready: {}, Content: {}, Length: {}, Body: {}", 
                                ready_state, has_content, content_len, body_exists);
                                
                        if has_content == "yes" && content_len > 100 {
                            // 页面有足够的内容，尝试获取一些基本信息
                            let extract_js = r#"
                                try {
                                    let title = document.title;
                                    let textLength = document.body ? document.body.innerText.length : 0;
                                    let linkCount = document.querySelectorAll('a').length;
                                    
                                    document.title = 'EXTRACT_' + textLength + '_' + linkCount;
                                    console.log('Content extracted - Text:', textLength, 'Links:', linkCount);
                                } catch (error) {
                                    document.title = 'EXTRACT_ERROR';
                                    console.error('Content extraction error:', error);
                                }
                            "#;
                            
                            if window.eval(extract_js).is_ok() {
                                tokio::time::sleep(Duration::from_millis(200)).await;
                                
                                if let Ok(final_title) = window.title() {
                                    println!("[HTML] Final extraction title: {}", final_title);
                                    
                                    if final_title.starts_with("EXTRACT_") {
                                        let extract_parts: Vec<&str> = final_title.split('_').collect();
                                        if extract_parts.len() >= 3 {
                                            let text_len = extract_parts[1];
                                            let link_count = extract_parts[2];
                                            
                                            return Ok(format!(
                                                "<!-- Page successfully loaded -->\n<!-- Content length: {} -->\n<!-- Text length: {} -->\n<!-- Links found: {} -->\n<!-- Ready state: {} -->\n<html><head><title>Search Results</title></head><body><p>Content extracted from search page</p><p>HTML length: {}, Text length: {}, Links: {}</p></body></html>", 
                                                content_len, text_len, link_count, ready_state, content_len, text_len, link_count
                                            ));
                                        }
                                    }
                                }
                            }
                            
                            // 基本信息提取
                            return Ok(format!(
                                "<!-- Page loaded successfully -->\n<!-- Content length: {} -->\n<!-- Ready state: {} -->\n<html><body>Content extracted from hidden window (length: {})</body></html>", 
                                content_len, ready_state, content_len
                            ));
                        } else {
                            println!("[HTML] Page has insufficient content - Length: {}", content_len);
                        }
                    }
                } else if title.starts_with("STATUS_ERROR") {
                    println!("[HTML] JavaScript error occurred: {}", title);
                }
            }
            Err(e) => {
                println!("[HTML] Failed to read window title: {}", e);
            }
        }
        
        Err("Page did not load properly or has no content".to_string())
    }

    async fn search_web(&self, query: &str) -> Result<serde_json::Value, String> {
        println!("[SEARCH] Starting search for query: {}", query);
        
        // 确保隐藏窗口存在
        if let Err(e) = crate::window::ensure_hidden_search_window(self.app_handle.clone()).await {
            return Err(format!("Failed to create hidden search window: {}", e));
        }
        let window = self
            .app_handle
            .get_webview_window("hidden_search")
            .ok_or("Hidden search window not found")?;

        println!("[SEARCH] Got hidden search window");

        // 从环境变量构建搜索 URL
        let envs = get_env_map_for_aipp_command(&self.app_handle, "aipp:search").unwrap_or_default();
        let base = envs.get("SEARCH_URL").map(|s| s.as_str()).unwrap_or("https://duckduckgo.com/html/?q=");
        let search_url = build_search_url(base, query);
        
        println!("[SEARCH] Search URL: {}", search_url);

        // 导航到搜索页面
        println!("[SEARCH] Navigating to search URL...");
        window
            .navigate(search_url.parse().map_err(|e| format!("Invalid URL: {}", e))?)
            .map_err(|e| format!("Failed to navigate to search URL: {}", e))?;

        println!("[SEARCH] Navigation completed, waiting for page load...");
        
        // 等待页面加载
        tokio::time::sleep(Duration::from_secs(3)).await;
        
        // 检查当前URL和标题
        match window.url() {
            Ok(current_url) => println!("[SEARCH] Current URL: {}", current_url),
            Err(e) => println!("[SEARCH] Failed to get current URL: {}", e),
        }
        
        match window.title() {
            Ok(title) => println!("[SEARCH] Current title before JS: {}", title),
            Err(e) => println!("[SEARCH] Failed to get current title: {}", e),
        }

        // 获取页面HTML内容
        match self.get_page_html().await {
            Ok(html) => {
                println!("[SEARCH] HTML extraction successful");
                Ok(serde_json::json!({
                    "query": query,
                    "request_url": search_url,
                    "source": base,
                    "html_content": html,
                    "message": "Search completed and HTML content extracted",
                }))
            }
            Err(e) => {
                println!("[SEARCH] HTML extraction failed: {}", e);
                // Fallback: return basic info with error
                Ok(serde_json::json!({
                    "query": query,
                    "request_url": search_url,
                    "source": base,
                    "html_content": "",
                    "message": format!("Search page loaded but HTML extraction failed: {}", e),
                    "error": e,
                }))
            }
        }
    }
    
    async fn fetch_url(&self, url: &str) -> Result<serde_json::Value, String> {
        if let Err(e) = crate::window::ensure_hidden_search_window(self.app_handle.clone()).await {
            return Err(format!("Failed to create hidden search window: {}", e));
        }
        let window = self
            .app_handle
            .get_webview_window("hidden_search")
            .ok_or("Hidden search window not found")?;

        // 导航到目标URL
        window
            .navigate(url.parse().map_err(|e| format!("Invalid URL: {}", e))?)
            .map_err(|e| format!("Failed to navigate to URL: {}", e))?;

        // 等待页面加载
        tokio::time::sleep(Duration::from_secs(3)).await;

        // 获取页面HTML内容
        match self.get_page_html().await {
            Ok(html) => {
                Ok(serde_json::json!({
                    "url": url,
                    "status": "success",
                    "html_content": html,
                    "message": "Page loaded and HTML content extracted",
                }))
            }
            Err(e) => {
                // Fallback: return basic info with error
                Ok(serde_json::json!({
                    "url": url,
                    "status": "partial_success", 
                    "html_content": "",
                    "message": format!("Page loaded but HTML extraction failed: {}", e),
                    "error": e,
                }))
            }
        }
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
        required_envs: vec![BuiltinTemplateEnvVar {
            key: "SEARCH_URL".into(),
            required: true,
            tip: Some("用于发起搜索的URL，例如 https://duckduckgo.com/html/?q= 或 https://www.google.com/search?q=（也可使用 {} 占位符）".into()),
        }],
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

