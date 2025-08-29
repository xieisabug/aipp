use crate::db::mcp_db::{
    MCPDatabase, MCPServer, MCPServerPrompt, MCPServerResource, MCPServerTool,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: String,
    #[serde(rename = "isEnabled")]
    pub is_enabled: bool,
    #[serde(rename = "isAutoRun")]
    pub is_auto_run: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpProviderInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "transportType")]
    pub transport_type: String,
    #[serde(rename = "isEnabled")]
    pub is_enabled: bool,
    pub tools: Vec<McpToolInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MCPServerRequest {
    pub name: String,
    pub description: Option<String>,
    pub transport_type: String,
    pub command: Option<String>,
    pub environment_variables: Option<String>,
    pub url: Option<String>,
    pub timeout: Option<i32>,
    pub is_long_running: bool,
    pub is_enabled: bool,
    pub is_builtin: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MCPToolConfig {
    pub tool_name: String,
    pub is_enabled: bool,
    pub is_auto_run: bool,
}

#[tauri::command]
pub async fn get_mcp_servers(app_handle: tauri::AppHandle) -> Result<Vec<MCPServer>, String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e: rusqlite::Error| e.to_string())?;
    let servers = db.get_mcp_servers().map_err(|e| e.to_string())?;
    Ok(servers)
}

#[tauri::command]
pub async fn get_mcp_server(app_handle: tauri::AppHandle, id: i64) -> Result<MCPServer, String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e: rusqlite::Error| e.to_string())?;
    let server = db.get_mcp_server(id).map_err(|e| e.to_string())?;
    Ok(server)
}

#[tauri::command]
pub async fn add_mcp_server(
    app_handle: tauri::AppHandle,
    request: MCPServerRequest,
) -> Result<i64, String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e: rusqlite::Error| e.to_string())?;

    let server_id = db
        .upsert_mcp_server_with_builtin(
            &request.name,
            request.description.as_deref(),
            &request.transport_type,
            request.command.as_deref(),
            request.environment_variables.as_deref(),
            request.url.as_deref(),
            request.timeout,
            request.is_long_running,
            request.is_enabled,
            request.is_builtin.unwrap_or(false),
        )
        .map_err(|e| e.to_string())?;

    Ok(server_id)
}

#[tauri::command]
pub async fn update_mcp_server(
    app_handle: tauri::AppHandle,
    id: i64,
    request: MCPServerRequest,
) -> Result<(), String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e: rusqlite::Error| e.to_string())?;

    db.update_mcp_server_with_builtin(
        id,
        &request.name,
        request.description.as_deref(),
        &request.transport_type,
        request.command.as_deref(),
        request.environment_variables.as_deref(),
        request.url.as_deref(),
        request.timeout,
        request.is_long_running,
        request.is_enabled,
        request.is_builtin.unwrap_or(false),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn delete_mcp_server(app_handle: tauri::AppHandle, id: i64) -> Result<(), String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e: rusqlite::Error| e.to_string())?;
    db.delete_mcp_server(id).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn toggle_mcp_server(
    app_handle: tauri::AppHandle,
    id: i64,
    is_enabled: bool,
) -> Result<(), String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e: rusqlite::Error| e.to_string())?;
    db.toggle_mcp_server(id, is_enabled).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_mcp_server_tools(
    app_handle: tauri::AppHandle,
    server_id: i64,
) -> Result<Vec<MCPServerTool>, String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e: rusqlite::Error| e.to_string())?;
    let tools = db.get_mcp_server_tools(server_id).map_err(|e| e.to_string())?;
    Ok(tools)
}

#[tauri::command]
pub async fn update_mcp_server_tool(
    app_handle: tauri::AppHandle,
    tool_id: i64,
    is_enabled: bool,
    is_auto_run: bool,
) -> Result<(), String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e: rusqlite::Error| e.to_string())?;
    db.update_mcp_server_tool(tool_id, is_enabled, is_auto_run).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_mcp_server_resources(
    app_handle: tauri::AppHandle,
    server_id: i64,
) -> Result<Vec<MCPServerResource>, String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e: rusqlite::Error| e.to_string())?;
    let resources = db.get_mcp_server_resources(server_id).map_err(|e| e.to_string())?;
    Ok(resources)
}

#[tauri::command]
pub async fn test_mcp_connection(
    app_handle: tauri::AppHandle,
    server_id: i64,
) -> Result<bool, String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e: rusqlite::Error| e.to_string())?;
    let server = db.get_mcp_server(server_id).map_err(|e| e.to_string())?;

    // 测试实际的MCP连接
    let test_result = match server.transport_type.as_str() {
        "stdio" => {
            if let Some(cmd) = &server.command {
                if crate::mcp::builtin_api::is_builtin_mcp_call(cmd) {
                    // 内置 aipp:* 不需要实际连接
                    Ok(())
                } else {
                    test_stdio_connection(&server).await
                }
            } else {
                test_stdio_connection(&server).await
            }
        }
        "sse" => test_sse_connection(&server).await,
        "http" => test_http_connection(&server).await,
        _ => Err(format!("Unsupported transport type: {}", server.transport_type)),
    };

    match test_result {
        Ok(_) => Ok(true),
        Err(e) => {
            println!("MCP connection test failed: {}", e);
            Ok(false)
        }
    }
}

// 测试stdio连接
async fn test_stdio_connection(server: &MCPServer) -> Result<(), String> {
    use rmcp::{
        transport::{ConfigureCommandExt, TokioChildProcess},
        ServiceExt,
    };
    use tokio::process::Command;

    let command = server.command.as_ref().ok_or("No command specified for stdio transport")?;
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty command".to_string());
    }

    // 简短的连接测试，超时时间更短
    let client_result = tokio::time::timeout(
        std::time::Duration::from_millis(5000), // 5秒超时
        async {
            let client = ()
                .serve(TokioChildProcess::new(Command::new(parts[0]).configure(|cmd| {
                    // 添加命令参数
                    if parts.len() > 1 {
                        cmd.args(&parts[1..]);
                    }

                    // 设置环境变量
                    if let Some(env_vars) = &server.environment_variables {
                        for line in env_vars.lines() {
                            if let Some((key, value)) = line.split_once('=') {
                                cmd.env(key.trim(), value.trim());
                            }
                        }
                    }
                }))?)
                .await?;

            // 测试成功，取消连接
            client.cancel().await?;
            Ok::<(), anyhow::Error>(())
        },
    )
    .await;

    match client_result {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(format!("Failed to create MCP client: {}", e)),
        Err(_) => Err("Timeout while connecting to MCP server".to_string()),
    }
}

// 测试SSE连接
async fn test_sse_connection(server: &MCPServer) -> Result<(), String> {
    use rmcp::{
        model::{ClientCapabilities, ClientInfo, Implementation},
        transport::SseClientTransport,
        ServiceExt,
    };

    let url = server.url.as_ref().ok_or("No URL specified for SSE transport")?;

    // 简短的连接测试，超时时间更短
    let client_result = tokio::time::timeout(
        std::time::Duration::from_millis(5000), // 5秒超时
        async {
            let transport = SseClientTransport::start(url.as_str()).await?;
            let client_info = ClientInfo {
                protocol_version: Default::default(),
                capabilities: ClientCapabilities::default(),
                client_info: Implementation {
                    name: "AIPP MCP SSE Test Client".to_string(),
                    version: "0.1.0".to_string(),
                },
            };
            let client = client_info.serve(transport).await?;

            // 测试成功，取消连接
            client.cancel().await?;
            Ok::<(), anyhow::Error>(())
        },
    )
    .await;

    match client_result {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(format!("Failed to create MCP SSE client: {}", e)),
        Err(_) => Err("Timeout while connecting to SSE server".to_string()),
    }
}

// 测试HTTP连接
async fn test_http_connection(server: &MCPServer) -> Result<(), String> {
    use rmcp::{
        model::{ClientCapabilities, ClientInfo, Implementation},
        transport::StreamableHttpClientTransport,
        ServiceExt,
    };

    let url = server.url.as_ref().ok_or("No URL specified for HTTP transport")?;

    // 创建StreamableHttpClientTransport传输
    let transport = StreamableHttpClientTransport::from_uri(url.as_str());

    // 创建客户端信息
    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "AIPP MCP Test Client".to_string(),
            version: "0.1.0".to_string(),
        },
    };

    // 简短的连接测试，超时时间更短
    let client_result = tokio::time::timeout(
        std::time::Duration::from_millis(server.timeout.unwrap_or(5000) as u64),
        async { client_info.serve(transport).await },
    )
    .await;

    match client_result {
        Ok(Ok(client)) => {
            // 测试成功，取消连接
            let _ = client.cancel().await;
            Ok(())
        }
        Ok(Err(e)) => Err(format!("Failed to create MCP client: {}", e)),
        Err(_) => Err("Timeout while connecting to HTTP server".to_string()),
    }
}

#[tauri::command]
pub async fn get_mcp_server_prompts(
    app_handle: tauri::AppHandle,
    server_id: i64,
) -> Result<Vec<MCPServerPrompt>, String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e: rusqlite::Error| e.to_string())?;
    let prompts = db.get_mcp_server_prompts(server_id).map_err(|e| e.to_string())?;
    Ok(prompts)
}

#[tauri::command]
pub async fn update_mcp_server_prompt(
    app_handle: tauri::AppHandle,
    prompt_id: i64,
    is_enabled: bool,
) -> Result<(), String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e: rusqlite::Error| e.to_string())?;
    db.update_mcp_server_prompt(prompt_id, is_enabled).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn refresh_mcp_server_capabilities(
    app_handle: tauri::AppHandle,
    server_id: i64,
) -> Result<(Vec<MCPServerTool>, Vec<MCPServerResource>, Vec<MCPServerPrompt>), String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e: rusqlite::Error| e.to_string())?;
    let server = db.get_mcp_server(server_id).map_err(|e| e.to_string())?;

    // Use incremental updates instead of clearing existing data

    // Try to connect to MCP server and get capabilities
    let result = match server.transport_type.as_str() {
        "stdio" => {
            // If aipp builtin server, register tools directly
            if let Some(cmd) = &server.command {
                if crate::mcp::builtin_api::is_builtin_mcp_call(cmd) {
                    let db = MCPDatabase::new(&app_handle).map_err(|e| e.to_string())?;
                    for tool in crate::mcp::builtin_api::get_builtin_tools_for_command(cmd) {
                        let params_json = tool.input_schema.to_string();
                        let _ = db.upsert_mcp_server_tool(server_id, &tool.name, Some(&tool.description), Some(&params_json));
                    }
                    Ok(())
                } else {
                    get_stdio_capabilities(app_handle.clone(), server_id, server.clone()).await
                }
            } else {
                get_stdio_capabilities(app_handle.clone(), server_id, server.clone()).await
            }
        },
        "sse" => get_sse_capabilities(app_handle.clone(), server_id, server.clone()).await,
        "http" => get_http_capabilities(app_handle.clone(), server_id, server.clone()).await,
        _ => Err(format!("Unsupported transport type: {}", server.transport_type)),
    };

    match result {
        Ok(_) => {
            let tools = db.get_mcp_server_tools(server_id).map_err(|e| e.to_string())?;
            let resources = db.get_mcp_server_resources(server_id).map_err(|e| e.to_string())?;
            let prompts = db.get_mcp_server_prompts(server_id).map_err(|e| e.to_string())?;
            Ok((tools, resources, prompts))
        }
        Err(e) => {
            // If real MCP connection fails, add some placeholder data for testing
            println!("MCP connection failed: {}, adding placeholder data", e);
            Err(format!("获取 MCP 服务器工具错误: {}", e))
        }
    }
}

// Stdio transport implementation
async fn get_stdio_capabilities(
    app_handle: tauri::AppHandle,
    server_id: i64,
    server: MCPServer,
) -> Result<(), String> {
    use rmcp::{
        transport::{ConfigureCommandExt, TokioChildProcess},
        ServiceExt,
    };
    use tokio::process::Command;

    let db = MCPDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    // 获取命令，如果没有则返回错误
    let command = server.command.ok_or("No command specified for stdio transport")?;

    // 解析命令和参数
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty command".to_string());
    }

    // 创建MCP客户端 - 使用正确的API模式
    let client_result = tokio::time::timeout(
        std::time::Duration::from_millis(server.timeout.unwrap_or(30000) as u64),
        async {
            let client = ()
                .serve(TokioChildProcess::new(Command::new(parts[0]).configure(|cmd| {
                    // 添加命令参数
                    if parts.len() > 1 {
                        cmd.args(&parts[1..]);
                    }

                    // 设置环境变量
                    if let Some(env_vars) = &server.environment_variables {
                        for line in env_vars.lines() {
                            if let Some((key, value)) = line.split_once('=') {
                                cmd.env(key.trim(), value.trim());
                            }
                        }
                    }
                }))?)
                .await?;

            Ok::<_, anyhow::Error>(client)
        },
    )
    .await;

    let client = match client_result {
        Ok(Ok(client)) => client,
        Ok(Err(e)) => {
            return Err(format!("Failed to create MCP client: {}", e));
        }
        Err(_) => {
            return Err("Timeout while connecting to MCP server".to_string());
        }
    };

    // 获取服务器信息
    let _server_info = client.peer_info();

    // 获取能力 - 使用便捷方法
    let capabilities_result = tokio::time::timeout(
        std::time::Duration::from_millis(10000), // 10秒超时
        async {
            let tools_result = client.list_all_tools().await;
            let resources_result = client.list_all_resources().await;
            let prompts_result = client.list_all_prompts().await;

            (tools_result, resources_result, prompts_result)
        },
    )
    .await;

    let (tools_result, resources_result, prompts_result) = match capabilities_result {
        Ok(results) => results,
        Err(_) => {
            return Err("Timeout while getting MCP server capabilities".to_string());
        }
    };

    // 处理工具
    if let Ok(tools) = tools_result {
        for tool in tools {
            let params_json =
                serde_json::to_string(&tool.input_schema).unwrap_or_else(|_| "{}".to_string());

            if let Err(e) = db.upsert_mcp_server_tool(
                server_id,
                &tool.name,
                tool.description.as_deref(),
                Some(&params_json),
            ) {
                println!("Failed to add tool {}: {}", tool.name, e);
            }
        }
    }

    // 处理资源
    if let Ok(resources) = resources_result {
        for resource in resources {
            if let Err(e) = db.upsert_mcp_server_resource(
                server_id,
                &resource.uri,
                &resource.name,
                &resource.mime_type.as_ref().unwrap_or(&"unknown".to_string()),
                resource.description.as_deref(),
            ) {
                println!("Failed to add resource {}: {}", resource.name, e);
            }
        }
    }

    // 处理提示
    if let Ok(prompts) = prompts_result {
        for prompt in prompts {
            let args_json = if let Some(args) = prompt.arguments {
                serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string())
            } else {
                "{}".to_string()
            };

            if let Err(e) = db.upsert_mcp_server_prompt(
                server_id,
                &prompt.name,
                prompt.description.as_deref(),
                Some(&args_json),
            ) {
                println!("Failed to add prompt {}: {}", prompt.name, e);
            }
        }
    }

    // 取消客户端连接
    let _ = client.cancel().await;

    Ok(())
}

async fn get_sse_capabilities(
    app_handle: tauri::AppHandle,
    server_id: i64,
    server: MCPServer,
) -> Result<(), String> {
    use rmcp::{
        model::{ClientCapabilities, ClientInfo, Implementation},
        transport::SseClientTransport,
        ServiceExt,
    };

    let db = MCPDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    // 获取URL，如果没有则返回错误
    let url = server.url.ok_or("No URL specified for SSE transport")?;

    // 创建SSE传输和客户端
    let client_result = tokio::time::timeout(
        std::time::Duration::from_millis(server.timeout.unwrap_or(30000) as u64),
        async {
            let transport = SseClientTransport::start(url.as_str()).await?;
            let client_info = ClientInfo {
                protocol_version: Default::default(),
                capabilities: ClientCapabilities::default(),
                client_info: Implementation {
                    name: "AIPP MCP SSE Client".to_string(),
                    version: "0.1.0".to_string(),
                },
            };
            let client = client_info.serve(transport).await?;
            Ok::<_, anyhow::Error>(client)
        },
    )
    .await;

    let client = match client_result {
        Ok(Ok(client)) => client,
        Ok(Err(e)) => {
            return Err(format!("Failed to create MCP SSE client: {}", e));
        }
        Err(_) => {
            return Err("Timeout while connecting to SSE server".to_string());
        }
    };

    // 获取服务器信息
    let _server_info = client.peer_info();

    // 获取能力
    let capabilities_result = tokio::time::timeout(
        std::time::Duration::from_millis(10000), // 10秒超时
        async {
            let tools_result = client.list_tools(Default::default()).await;
            let resources_result = client.list_resources(Default::default()).await;
            let prompts_result = client.list_prompts(Default::default()).await;

            (tools_result, resources_result, prompts_result)
        },
    )
    .await;

    let (tools_result, resources_result, prompts_result) = match capabilities_result {
        Ok(results) => results,
        Err(_) => {
            return Err("Timeout while getting MCP SSE server capabilities".to_string());
        }
    };

    // 处理工具
    if let Ok(tools_response) = tools_result {
        for tool in tools_response.tools {
            let params_json =
                serde_json::to_string(&tool.input_schema).unwrap_or_else(|_| "{}".to_string());

            if let Err(e) = db.upsert_mcp_server_tool(
                server_id,
                &tool.name,
                tool.description.as_deref(),
                Some(&params_json),
            ) {
                println!("Failed to add tool {}: {}", tool.name, e);
            }
        }
    }

    // 处理资源
    if let Ok(resources_response) = resources_result {
        for resource in resources_response.resources {
            if let Err(e) = db.upsert_mcp_server_resource(
                server_id,
                &resource.uri,
                &resource.name,
                &resource.mime_type.as_ref().unwrap_or(&"unknown".to_string()),
                resource.description.as_deref(),
            ) {
                println!("Failed to add resource {}: {}", resource.name, e);
            }
        }
    }

    // 处理提示
    if let Ok(prompts_response) = prompts_result {
        for prompt in prompts_response.prompts {
            let args_json = if let Some(args) = prompt.arguments {
                serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string())
            } else {
                "{}".to_string()
            };

            if let Err(e) = db.upsert_mcp_server_prompt(
                server_id,
                &prompt.name,
                prompt.description.as_deref(),
                Some(&args_json),
            ) {
                println!("Failed to add prompt {}: {}", prompt.name, e);
            }
        }
    }

    // 取消客户端连接
    let _ = client.cancel().await;

    Ok(())
}

async fn get_http_capabilities(
    app_handle: tauri::AppHandle,
    server_id: i64,
    server: MCPServer,
) -> Result<(), String> {
    use rmcp::{
        model::{ClientCapabilities, ClientInfo, Implementation},
        transport::StreamableHttpClientTransport,
        ServiceExt,
    };

    let db = MCPDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    // 获取URL，如果没有则返回错误
    let url = server.url.ok_or("No URL specified for HTTP transport")?;

    // 创建StreamableHttpClientTransport传输
    let transport = StreamableHttpClientTransport::from_uri(url.as_str());

    // 创建客户端信息
    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "AIPP MCP Client".to_string(),
            version: "0.1.0".to_string(),
        },
    };

    // 创建MCP客户端 - 使用正确的API模式
    let client_result = tokio::time::timeout(
        std::time::Duration::from_millis(server.timeout.unwrap_or(30000) as u64),
        async { client_info.serve(transport).await },
    )
    .await;

    let client = match client_result {
        Ok(Ok(client)) => client,
        Ok(Err(e)) => {
            return Err(format!("Failed to create MCP client: {}", e));
        }
        Err(_) => {
            return Err("Timeout while connecting to HTTP server".to_string());
        }
    };

    // 获取服务器信息
    let _server_info = client.peer_info();

    // 获取能力 - 使用正确的API
    let capabilities_result = tokio::time::timeout(
        std::time::Duration::from_millis(10000), // 10秒超时
        async {
            let tools_result = client.list_tools(Default::default()).await;
            let resources_result = client.list_resources(Default::default()).await;
            let prompts_result = client.list_prompts(Default::default()).await;

            (tools_result, resources_result, prompts_result)
        },
    )
    .await
    .map_err(|_| "Timeout while getting MCP server capabilities".to_string())?;

    let (tools_result, resources_result, prompts_result) = capabilities_result;

    // 处理工具
    if let Ok(tools_response) = tools_result {
        for tool in tools_response.tools {
            let params_json =
                serde_json::to_string(&tool.input_schema).unwrap_or_else(|_| "{}".to_string());

            if let Err(e) = db.upsert_mcp_server_tool(
                server_id,
                &tool.name,
                tool.description.as_deref(),
                Some(&params_json),
            ) {
                println!("Failed to add tool {}: {}", tool.name, e);
            }
        }
    }

    // 处理资源
    if let Ok(resources_response) = resources_result {
        for resource in resources_response.resources {
            if let Err(e) = db.upsert_mcp_server_resource(
                server_id,
                &resource.uri,
                &resource.name,
                &resource.mime_type.as_ref().unwrap_or(&"unknown".to_string()),
                resource.description.as_deref(),
            ) {
                println!("Failed to add resource {}: {}", resource.name, e);
            }
        }
    }

    // 处理提示
    if let Ok(prompts_response) = prompts_result {
        for prompt in prompts_response.prompts {
            let args_json = if let Some(args) = prompt.arguments {
                serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string())
            } else {
                "{}".to_string()
            };

            if let Err(e) = db.upsert_mcp_server_prompt(
                server_id,
                &prompt.name,
                prompt.description.as_deref(),
                Some(&args_json),
            ) {
                println!("Failed to add prompt {}: {}", prompt.name, e);
            }
        }
    }

    // 取消客户端连接
    let _ = client.cancel().await;

    Ok(())
}

#[tauri::command]
pub async fn get_mcp_provider(
    app_handle: tauri::AppHandle,
    provider_id: String,
) -> Result<Option<McpProviderInfo>, String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e: rusqlite::Error| e.to_string())?;
    
    // Parse provider_id as server ID
    let server_id: i64 = provider_id.parse().map_err(|_| "Invalid provider ID format".to_string())?;
    
    // Get server information
    let server = match db.get_mcp_server(server_id) {
        Ok(server) => server,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    
    // Get server tools
    let server_tools = db.get_mcp_server_tools(server_id).map_err(|e| e.to_string())?;
    
    // Convert to McpProviderInfo format
    let tools: Vec<McpToolInfo> = server_tools
        .into_iter()
        .map(|tool| McpToolInfo {
            name: tool.tool_name,
            description: tool.tool_description.unwrap_or_default(),
            parameters: tool.parameters.unwrap_or_else(|| "{}".to_string()),
            is_enabled: tool.is_enabled,
            is_auto_run: tool.is_auto_run,
        })
        .collect();
    
    let provider_info = McpProviderInfo {
        id: server.id.to_string(),
        name: server.name,
        description: if server.description.is_empty() { None } else { Some(server.description) },
        transport_type: server.transport_type,
        is_enabled: server.is_enabled,
        tools,
    };
    
    Ok(Some(provider_info))
}

#[tauri::command]
pub async fn build_mcp_prompt(
    app_handle: tauri::AppHandle,
    provider_ids: Vec<String>,
) -> Result<String, String> {
    use crate::mcp::format_mcp_prompt;
    use crate::api::assistant_api::{MCPServerWithTools, MCPToolInfo};
    
    let db = MCPDatabase::new(&app_handle).map_err(|e: rusqlite::Error| e.to_string())?;
    
    let mut enabled_servers = Vec::new();
    
    // Process each provider ID to build enabled servers list
    for provider_id in provider_ids {
        let server_id: i64 = match provider_id.parse() {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Invalid provider ID format: {}", provider_id);
                continue;
            }
        };
        
        // Get server information
        let server = match db.get_mcp_server(server_id) {
            Ok(server) if server.is_enabled => server,
            _ => continue, // Skip disabled or non-existent servers
        };
        
        // Get server tools
        let server_tools = match db.get_mcp_server_tools(server_id) {
            Ok(tools) => tools,
            Err(_) => continue,
        };
        
        // Only include enabled tools and convert to the expected format
        let enabled_tools: Vec<MCPToolInfo> = server_tools
            .into_iter()
            .filter(|tool| tool.is_enabled)
            .map(|tool| MCPToolInfo {
                id: tool.id,
                name: tool.tool_name,
                description: tool.tool_description.unwrap_or_default(),
                is_enabled: tool.is_enabled,
                is_auto_run: tool.is_auto_run,
                parameters: tool.parameters.unwrap_or_else(|| "{}".to_string()),
            })
            .collect();
        
        if enabled_tools.is_empty() {
            continue;
        }
        
        // Build MCPServerWithTools
        let server_with_tools = MCPServerWithTools {
            id: server.id,
            name: server.name,
            is_enabled: server.is_enabled,
            tools: enabled_tools,
        };
        
        enabled_servers.push(server_with_tools);
    }
    
    if enabled_servers.is_empty() {
        return Ok("No MCP tools available for the specified providers.".to_string());
    }
    
    // Build MCPInfoForAssistant structure
    let mcp_info = crate::mcp::MCPInfoForAssistant {
        enabled_servers,
        use_native_toolcall: false, // For prompt generation, we use prompt-based mode
    };
    
    // Use existing format_mcp_prompt function
    let result = format_mcp_prompt("".to_string(), &mcp_info).await;
    Ok(result)
}
