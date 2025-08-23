use crate::db::mcp_db::MCPDatabase;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest { pub query: String }

#[derive(Debug, Serialize, Deserialize)]
pub struct FetchRequest { pub url: String }

/// 内置搜索工具处理器
#[derive(Clone)]
pub struct BuiltinSearchHandler { app_handle: AppHandle }

impl BuiltinSearchHandler {
    pub fn new(app_handle: AppHandle) -> Self { Self { app_handle } }

    async fn search_web(&self, query: &str) -> Result<serde_json::Value, String> {
        if let Err(e) = crate::window::ensure_hidden_search_window(self.app_handle.clone()).await {
            return Err(format!("Failed to create hidden search window: {}", e));
        }
        let window = self
            .app_handle
            .get_webview_window("hidden_search")
            .ok_or("Hidden search window not found")?;

        let search_url = format!("https://duckduckgo.com/lite/?q={}", urlencoding::encode(query));
        window
            .navigate(search_url.parse().unwrap())
            .map_err(|e| format!("Failed to navigate to search URL: {}", e))?;

        tokio::time::sleep(Duration::from_secs(2)).await;

        Ok(serde_json::json!({
            "query": query,
            "results": [],
            "message": "Search executed",
            "source": "DuckDuckGo Lite"
        }))
    }
    
    async fn fetch_url(&self, url: &str) -> Result<serde_json::Value, String> {
        if let Err(e) = crate::window::ensure_hidden_search_window(self.app_handle.clone()).await {
            return Err(format!("Failed to create hidden search window: {}", e));
        }
        let window = self
            .app_handle
            .get_webview_window("hidden_search")
            .ok_or("Hidden search window not found")?;

        window
            .navigate(url.parse().map_err(|e| format!("Invalid URL: {}", e))?)
            .map_err(|e| format!("Failed to navigate to URL: {}", e))?;

        tokio::time::sleep(Duration::from_secs(2)).await;

        Ok(serde_json::json!({
            "url": url,
            "status": "navigated",
            "message": "Fetch executed"
        }))
    }
}

// ---- Builtin registry & templates (stdio + aipp:* scheme) ----

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
            tip: Some("用于发起搜索的URL，比如 https://www.google.com/search?q=".into()),
        }],
    }]
}

pub fn is_aipp_builtin_command(command: &str) -> bool { command.trim().starts_with("aipp:") }

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