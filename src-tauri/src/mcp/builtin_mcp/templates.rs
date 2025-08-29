use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use crate::mcp::mcp_db::MCPDatabase;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinTemplateEnvVar {
    pub key: String,
    pub label: String,
    pub required: bool,
    pub tip: Option<String>,
    pub field_type: String, // "text", "select", "boolean", "number"
    pub default_value: Option<String>,
    pub placeholder: Option<String>,
    pub options: Option<Vec<EnvVarOption>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVarOption {
    pub label: String,
    pub value: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

fn builtin_templates() -> Vec<BuiltinTemplateInfo> {
    vec![BuiltinTemplateInfo {
        id: "search".into(),
        name: "搜索工具".into(),
        description: "内置的网络搜索和网页访问工具，支持多种搜索引擎和浏览器，可以通过搜索引擎获取到相关信息，并且可以调用访问工具进一步获取页面的具体信息".into(),
        command: "aipp:search".into(),
        transport_type: "stdio".into(),
        required_envs: vec![
            BuiltinTemplateEnvVar {
                key: "BROWSER_TYPE".into(),
                label: "浏览器类型".into(),
                required: false,
                tip: Some("搜索使用的浏览器类型，默认使用 Chrome，如果不可用则降级为 Edge".into()),
                field_type: "select".into(),
                default_value: Some("chrome".into()),
                placeholder: None,
                options: Some(vec![
                    EnvVarOption { label: "Chrome".into(), value: "chrome".into() },
                    EnvVarOption { label: "Edge".into(), value: "edge".into() },
                ]),
            },
            BuiltinTemplateEnvVar {
                key: "SEARCH_ENGINE".into(),
                label: "搜索引擎".into(),
                required: true,
                tip: Some("搜索使用的搜索引擎，默认使用 Google，如果不可用则降级为 Bing".into()),
                field_type: "select".into(),
                default_value: Some("google".into()),
                placeholder: None,
                options: Some(vec![
                    EnvVarOption { label: "Google".into(), value: "google".into() },
                    EnvVarOption { label: "Bing".into(), value: "bing".into() },
                    EnvVarOption { label: "DuckDuckGo".into(), value: "duckduckgo".into() },
                    EnvVarOption { label: "Kagi".into(), value: "kagi".into() },
                ]),
            },
            BuiltinTemplateEnvVar {
                key: "USER_DATA_DIR".into(),
                label: "浏览器数据目录".into(),
                required: false,
                tip: Some("使用的浏览器 profile 目录，用于共享登录状态和配置".into()),
                field_type: "text".into(),
                default_value: None,
                placeholder: Some("/path/to/browser/profile".into()),
                options: None,
            },
            BuiltinTemplateEnvVar {
                key: "PROXY_SERVER".into(),
                label: "代理服务器".into(),
                required: false,
                tip: Some("代理服务器地址，支持 HTTP 和 SOCKS5 协议".into()),
                field_type: "text".into(),
                default_value: None,
                placeholder: Some("http://proxy:port 或 socks5://proxy:port".into()),
                options: None,
            },
            BuiltinTemplateEnvVar {
                key: "HEADLESS".into(),
                label: "无头模式".into(),
                required: false,
                tip: Some("启用后浏览器在后台运行，关闭后会显示浏览器窗口（用于调试）".into()),
                field_type: "boolean".into(),
                default_value: Some("true".into()),
                placeholder: None,
                options: None,
            },
            BuiltinTemplateEnvVar {
                key: "WAIT_SELECTORS".into(),
                label: "等待元素选择器".into(),
                required: false,
                tip: Some("等待页面指定元素加载完成的 CSS 选择器，多个选择器用逗号分隔，程序已经对常用的搜索引擎进行了适配，如果发现适配出现问题，可以使用该属性进行覆盖".into()),
                field_type: "text".into(),
                default_value: None,
                placeholder: Some("#search-results, .content-area".into()),
                options: None,
            },
            BuiltinTemplateEnvVar {
                key: "WAIT_TIMEOUT_MS".into(),
                label: "等待超时时间".into(),
                required: false,
                tip: Some("等待页面元素加载的超时时间（毫秒）".into()),
                field_type: "number".into(),
                default_value: Some("15000".into()),
                placeholder: Some("15000".into()),
                options: None,
            },
        ],
    }]
}

pub fn get_builtin_tools_for_command(command: &str) -> Vec<BuiltinToolInfo> {
    match super::builtin_command_id(command).as_deref() {
        Some("search") => vec![
            BuiltinToolInfo {
                name: "search_web".into(),
                description: "搜索网络内容，支持多种结果格式。可以返回原始HTML、Markdown格式或结构化搜索结果。当搜索的结果较为简单但判断该结果有可用性时，可以进一步通过fetch_url工具获取到页面完整的信息。".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string", 
                            "description": "搜索查询关键词，支持中英文和各种搜索语法"
                        },
                        "result_type": {
                            "type": "string",
                            "enum": ["html", "markdown", "items"],
                            "default": "html",
                            "description": "结果格式类型：\n- html: 返回原始HTML内容，适合需要完整页面信息的场景\n- markdown: 将HTML转换为Markdown格式，便于阅读和处理\n- items: 返回结构化的搜索结果列表，包含标题、URL、摘要等字段"
                        }
                    },
                    "required": ["query"]
                }),
            },
            BuiltinToolInfo {
                name: "fetch_url".into(),
                description: "获取网页内容，支持多种结果格式。可以返回原始HTML或Markdown格式的网页内容。".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string", 
                            "description": "要获取内容的URL"
                        },
                        "result_type": {
                            "type": "string",
                            "enum": ["html", "markdown"],
                            "default": "html",
                            "description": "结果格式类型：\\n- html: 返回原始HTML内容，适合需要完整页面信息的场景\\n- markdown: 将HTML转换为Markdown格式，便于阅读和处理"
                        }
                    },
                    "required": ["url"]
                }),
            },
        ],
        _ => vec![],
    }
}

#[tauri::command]
pub async fn list_aipp_builtin_templates() -> Result<Vec<BuiltinTemplateInfo>, String> {
    Ok(builtin_templates())
}

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
