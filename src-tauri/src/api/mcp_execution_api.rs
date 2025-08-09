use crate::db::mcp_db::{MCPDatabase, MCPServer, MCPToolCall};
use anyhow::Result;

// MCP Tool Execution API
#[tauri::command]
pub async fn create_mcp_tool_call(
    app_handle: tauri::AppHandle,
    conversation_id: i64,
    message_id: Option<i64>,
    server_name: String,
    tool_name: String,
    parameters: String,
) -> Result<MCPToolCall, String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    
    // Find the server by name
    let servers = db.get_mcp_servers().map_err(|e| e.to_string())?;
    let server = servers.iter()
        .find(|s| s.name == server_name && s.is_enabled)
        .ok_or_else(|| format!("Server '{}' not found or disabled", server_name))?;
    
    // Create the tool call record
    let tool_call = db.create_mcp_tool_call(
        conversation_id,
        message_id,
        server.id,
        &server_name,
        &tool_name,
        &parameters,
    ).map_err(|e| e.to_string())?;
    
    Ok(tool_call)
}

#[tauri::command]
pub async fn execute_mcp_tool_call(
    app_handle: tauri::AppHandle,
    call_id: i64,
) -> Result<MCPToolCall, String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    
    // Get the tool call
    let mut tool_call = db.get_mcp_tool_call(call_id).map_err(|e| e.to_string())?;
    
    // Get the server
    let server = db.get_mcp_server(tool_call.server_id).map_err(|e| e.to_string())?;
    
    if !server.is_enabled {
        return Err("Server is disabled".to_string());
    }
    
    // Update status to executing
    db.update_mcp_tool_call_status(call_id, "executing", None, None)
        .map_err(|e| e.to_string())?;
    
    // Execute the tool based on transport type
    let execution_result = match server.transport_type.as_str() {
        "stdio" => execute_stdio_tool(&server, &tool_call.tool_name, &tool_call.parameters).await,
        "sse" => execute_sse_tool(&server, &tool_call.tool_name, &tool_call.parameters).await,
        "http" => execute_http_tool(&server, &tool_call.tool_name, &tool_call.parameters).await,
        _ => Err(format!("Unsupported transport type: {}", server.transport_type))
    };
    
    // Update the tool call with the result
    match execution_result {
        Ok(result) => {
            db.update_mcp_tool_call_status(call_id, "success", Some(&result), None)
                .map_err(|e| e.to_string())?;
            tool_call.status = "success".to_string();
            tool_call.result = Some(result);
        }
        Err(error) => {
            db.update_mcp_tool_call_status(call_id, "failed", None, Some(&error))
                .map_err(|e| e.to_string())?;
            tool_call.status = "failed".to_string();
            tool_call.error = Some(error);
        }
    }
    
    Ok(tool_call)
}

#[tauri::command]
pub async fn get_mcp_tool_call(
    app_handle: tauri::AppHandle,
    call_id: i64,
) -> Result<MCPToolCall, String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    db.get_mcp_tool_call(call_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_mcp_tool_calls_by_conversation(
    app_handle: tauri::AppHandle,
    conversation_id: i64,
) -> Result<Vec<MCPToolCall>, String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    db.get_mcp_tool_calls_by_conversation(conversation_id).map_err(|e| e.to_string())
}

// Tool execution implementations
async fn execute_stdio_tool(server: &MCPServer, tool_name: &str, parameters: &str) -> Result<String, String> {
    use rmcp::{
        ServiceExt,
        transport::{ConfigureCommandExt, TokioChildProcess},
        model::CallToolRequestParam,
    };
    use tokio::process::Command;
    
    let command = server.command.as_ref().ok_or("No command specified for stdio transport")?;
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty command".to_string());
    }
    
    let client_result = tokio::time::timeout(
        std::time::Duration::from_millis(server.timeout.unwrap_or(30000) as u64),
        async {
            let client = (())
                .serve(TokioChildProcess::new(Command::new(parts[0]).configure(
                    |cmd| {
                        if parts.len() > 1 {
                            cmd.args(&parts[1..]);
                        }
                        
                        if let Some(env_vars) = &server.environment_variables {
                            for line in env_vars.lines() {
                                if let Some((key, value)) = line.split_once('=') {
                                    cmd.env(key.trim(), value.trim());
                                }
                            }
                        }
                    },
                )).map_err(|e| format!("Failed to create child process: {}", e))?)
                .await
                .map_err(|e| format!("Failed to initialize client: {}", e))?;
            
            // Parse parameters as JSON
            let params_value: serde_json::Value = serde_json::from_str(parameters)
                .map_err(|e| format!("Invalid parameters JSON: {}", e))?;
            
            // Convert Value to Map<String, Value>
            let params_map = match params_value {
                serde_json::Value::Object(map) => map,
                _ => return Err("Parameters must be a JSON object".to_string()),
            };
            
            // Call the tool with correct API
            let request_param = CallToolRequestParam {
                name: tool_name.to_string().into(),
                arguments: Some(params_map),
            };
            
            let response = client.call_tool(request_param).await
                .map_err(|e| format!("Tool call failed: {}", e))?;
            
            // Cancel the client connection
            client.cancel().await
                .map_err(|e| format!("Failed to cancel client: {}", e))?;
            
            // Return the result content
            if response.is_error.unwrap_or(false) {
                Err(format!("Tool execution error: {:?}", response.content))
            } else {
                serde_json::to_string(&response.content)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            }
        }
    ).await;
    
    match client_result {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(format!("Tool execution failed: {}", e)),
        Err(_) => Err("Timeout while executing tool".to_string()),
    }
}

async fn execute_sse_tool(server: &MCPServer, tool_name: &str, parameters: &str) -> Result<String, String> {
    use rmcp::{
        ServiceExt,
        model::{ClientCapabilities, ClientInfo, Implementation, CallToolRequestParam},
        transport::SseClientTransport,
    };

    let url = server.url.as_ref().ok_or("No URL specified for SSE transport")?;

    let client_result = tokio::time::timeout(
        std::time::Duration::from_millis(server.timeout.unwrap_or(30000) as u64),
        async {
            let transport = SseClientTransport::start(url.as_str()).await
                .map_err(|e| format!("Failed to start SSE transport: {}", e))?;
            let client_info = ClientInfo {
                protocol_version: Default::default(),
                capabilities: ClientCapabilities::default(),
                client_info: Implementation {
                    name: "AIPP MCP SSE Client".to_string(),
                    version: "0.1.0".to_string(),
                },
            };
            let client = client_info.serve(transport).await
                .map_err(|e| format!("Failed to initialize SSE client: {}", e))?;
            
            // Parse parameters as JSON
            let params_value: serde_json::Value = serde_json::from_str(parameters)
                .map_err(|e| format!("Invalid parameters JSON: {}", e))?;
            
            // Convert Value to Map<String, Value>
            let params_map = match params_value {
                serde_json::Value::Object(map) => map,
                _ => return Err("Parameters must be a JSON object".to_string()),
            };
            
            // Call the tool with correct API
            let request_param = CallToolRequestParam {
                name: tool_name.to_string().into(),
                arguments: Some(params_map),
            };
            
            let response = client.call_tool(request_param).await
                .map_err(|e| format!("Tool call failed: {}", e))?;
            
            // Cancel the client connection
            client.cancel().await
                .map_err(|e| format!("Failed to cancel client: {}", e))?;
            
            // Return the result content
            if response.is_error.unwrap_or(false) {
                Err(format!("Tool execution error: {:?}", response.content))
            } else {
                serde_json::to_string(&response.content)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            }
        }
    ).await;

    match client_result {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(format!("Tool execution failed: {}", e)),
        Err(_) => Err("Timeout while executing tool".to_string()),
    }
}

async fn execute_http_tool(server: &MCPServer, tool_name: &str, parameters: &str) -> Result<String, String> {
    use rmcp::{
        ServiceExt,
        model::{ClientCapabilities, ClientInfo, Implementation, CallToolRequestParam},
        transport::StreamableHttpClientTransport,
    };
    
    let url = server.url.as_ref().ok_or("No URL specified for HTTP transport")?;
    
    let transport = StreamableHttpClientTransport::from_uri(url.as_str());
    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "AIPP MCP HTTP Client".to_string(),
            version: "0.1.0".to_string(),
        },
    };
    
    let client_result = tokio::time::timeout(
        std::time::Duration::from_millis(server.timeout.unwrap_or(30000) as u64),
        async {
            let client = client_info.serve(transport).await
                .map_err(|e| format!("Failed to initialize HTTP client: {}", e))?;
            
            // Parse parameters as JSON
            let params_value: serde_json::Value = serde_json::from_str(parameters)
                .map_err(|e| format!("Invalid parameters JSON: {}", e))?;
            
            // Convert Value to Map<String, Value>
            let params_map = match params_value {
                serde_json::Value::Object(map) => map,
                _ => return Err("Parameters must be a JSON object".to_string()),
            };
            
            // Call the tool with correct API
            let request_param = CallToolRequestParam {
                name: tool_name.to_string().into(),
                arguments: Some(params_map),
            };
            
            let response = client.call_tool(request_param).await
                .map_err(|e| format!("Tool call failed: {}", e))?;
            
            // Cancel the client connection
            client.cancel().await
                .map_err(|e| format!("Failed to cancel client: {}", e))?;
            
            // Return the result content
            if response.is_error.unwrap_or(false) {
                Err(format!("Tool execution error: {:?}", response.content))
            } else {
                serde_json::to_string(&response.content)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            }
        }
    ).await;
    
    match client_result {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(format!("Tool execution failed: {}", e)),
        Err(_) => Err("Timeout while executing tool".to_string()),
    }
}