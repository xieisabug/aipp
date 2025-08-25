use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use search::types::{SearchRequest, SearchResultType};

pub mod search;
pub mod templates;

pub use templates::{
    BuiltinTemplateEnvVar, BuiltinTemplateInfo, BuiltinToolInfo,
    list_aipp_builtin_templates, add_or_update_aipp_builtin_server,
    get_builtin_tools_for_command
};
pub use search::SearchHandler;

pub fn is_builtin_command(command: &str) -> bool {
    command.trim().starts_with("aipp:")
}

pub fn builtin_command_id(command: &str) -> Option<String> {
    if is_builtin_command(command) {
        Some(command.trim().trim_start_matches("aipp:").to_string())
    } else {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinExecutionResult {
    pub content: Vec<serde_json::Value>,
    pub is_error: bool,
}

#[tauri::command]
pub async fn execute_aipp_builtin_tool(
    app_handle: AppHandle,
    server_command: String,
    tool_name: String,
    parameters: String,
) -> Result<String, String> {
    let args: serde_json::Value = serde_json::from_str(&parameters)
        .map_err(|e| format!("Invalid parameters: {}", e))?;

    let cmd_id = builtin_command_id(&server_command)
        .ok_or("Not a builtin command")?;
    
    let result_value = match cmd_id.as_str() {
        "search" => {
            let handler = SearchHandler::new(app_handle.clone());
            match tool_name.as_str() {
                "search_web" => {
                    let query = args
                        .get("query")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Missing required parameter: query".to_string())?;
                    
                    // 获取result_type参数，默认为html
                    let result_type_str = args
                        .get("result_type")
                        .and_then(|v| v.as_str());
                    
                    let result_type = SearchResultType::from_str(result_type_str);
                    let request = SearchRequest {
                        query: query.to_string(),
                        result_type,
                    };
                    
                    match handler.search_web_with_type(request).await {
                        Ok(response) => {
                            // 根据result_type返回不同格式的内容
                            match response {
                                search::types::SearchResponse::Html { html_content, .. } => {
                                    serde_json::json!({
                                        "content": [{"type": "text", "text": html_content}],
                                        "isError": false
                                    })
                                }
                                search::types::SearchResponse::Markdown { markdown_content, .. } => {
                                    serde_json::json!({
                                        "content": [{"type": "text", "text": markdown_content}],
                                        "isError": false
                                    })
                                }
                                search::types::SearchResponse::Items(search_results) => {
                                    serde_json::json!({
                                        "content": [{"type": "json", "json": search_results}],
                                        "isError": false
                                    })
                                }
                                search::types::SearchResponse::ItemsOnly(items) => {
                                    serde_json::json!({
                                        "content": [{"type": "json", "json": items}],
                                        "isError": false
                                    })
                                }
                            }
                        }
                        Err(e) => serde_json::json!({
                            "content": [{"type": "text", "text": e}],
                            "isError": true
                        }),
                    }
                }
                "fetch_url" => {
                    let url = args
                        .get("url")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Missing required parameter: url".to_string())?;
                    
                    // 获取result_type参数，默认为html
                    let result_type = args
                        .get("result_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("html");
                    
                    match handler.fetch_url_with_type(url, result_type).await {
                        Ok(v) => serde_json::json!({
                            "content": [{"type": "text", "text": v}],
                            "isError": false
                        }),
                        Err(e) => serde_json::json!({
                            "content": [{"type": "text", "text": e}],
                            "isError": true
                        }),
                    }
                }
                _ => serde_json::json!({
                    "content": [{"type": "text", "text": format!("Unknown search tool: {}", tool_name)}],
                    "isError": true
                }),
            }
        }
        _ => serde_json::json!({
            "content": [{"type": "text", "text": format!("Unknown builtin command: {}", cmd_id)}],
            "isError": true
        }),
    };

    Ok(serde_json::to_string(&result_value).unwrap_or_else(|_| "{}".to_string()))
}