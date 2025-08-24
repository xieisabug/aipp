use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use crate::db::mcp_db::MCPDatabase;

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
                required: false,
                tip: Some("浏览器类型：chrome 或 edge（默认：chrome，降级：edge）".into()),
            },
            BuiltinTemplateEnvVar {
                key: "SEARCH_ENGINE".into(),
                required: false,
                tip: Some("搜索引擎：google、bing、duckduckgo、kagi（默认：google，降级：bing）".into()),
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
                tip: Some("等待页面元素的CSS选择器，多个用逗号分隔（搜索引擎会提供默认值）".into()),
            },
            BuiltinTemplateEnvVar {
                key: "WAIT_TIMEOUT_MS".into(),
                required: false,
                tip: Some("等待页面元素的超时时间（毫秒），默认15000ms".into()),
            },
        ],
    }]
}

pub fn get_builtin_tools_for_command(command: &str) -> Vec<BuiltinToolInfo> {
    match super::builtin_command_id(command).as_deref() {
        Some("search") => vec![
            BuiltinToolInfo {
                name: "search_web".into(),
                description: "搜索网络内容，支持多种结果格式。可以返回原始HTML、Markdown格式或结构化搜索结果。适合大型语言模型处理和理解搜索内容。".into(),
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