use crate::api::ai::events::{ConversationEvent, MCPToolCallUpdateEvent};
use crate::api::ai_api::tool_result_continue_ask_ai;
use crate::db::conversation_db::{ConversationDatabase, Repository};
use crate::db::mcp_db::{MCPDatabase, MCPServer, MCPToolCall};
use anyhow::Result;
use tauri::Emitter;

// MCP Tool Execution API

// 发送MCP工具调用状态更新事件
fn emit_mcp_tool_call_update(
    window: &tauri::Window,
    conversation_id: i64,
    tool_call: &MCPToolCall,
) {
    let update_event = ConversationEvent {
        r#type: "mcp_tool_call_update".to_string(),
        data: serde_json::to_value(MCPToolCallUpdateEvent {
            call_id: tool_call.id,
            conversation_id: tool_call.conversation_id,
            status: tool_call.status.clone(),
            result: tool_call.result.clone(),
            error: tool_call.error.clone(),
            started_time: tool_call.started_time.as_ref().map(|s| {
                chrono::DateTime::parse_from_rfc3339(s)
                    .unwrap_or_else(|_| chrono::Utc::now().into())
                    .with_timezone(&chrono::Utc)
            }),
            finished_time: tool_call.finished_time.as_ref().map(|s| {
                chrono::DateTime::parse_from_rfc3339(s)
                    .unwrap_or_else(|_| chrono::Utc::now().into())
                    .with_timezone(&chrono::Utc)
            }),
        })
        .unwrap(),
    };

    let _ = window.emit(
        format!("conversation_event_{}", conversation_id).as_str(),
        update_event,
    );
}

// 规范化从 LLM 返回的 parameters JSON，移除可能的代码块包裹
fn normalize_parameters_json(parameters: &str) -> String {
    let trimmed = parameters.trim();
    if trimmed.starts_with("```") {
        // 去掉首尾 ```，并移除可能的语言标识（如 ```json）
        let without_start = trimmed.trim_start_matches("```");
        // 可能存在语言标签，截到首个换行
        let without_lang = match without_start.find('\n') {
            Some(idx) => &without_start[idx + 1..],
            None => without_start,
        };
        let without_end = without_lang.trim_end_matches("```").trim();
        without_end.to_string()
    } else {
        trimmed.to_string()
    }
}
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
    let server = servers
        .iter()
        .find(|s| s.name == server_name && s.is_enabled)
        .ok_or_else(|| format!("Server '{}' not found or disabled", server_name))?;

    // Create the tool call record
    let tool_call = db
        .create_mcp_tool_call(
            conversation_id,
            message_id,
            server.id,
            &server_name,
            &tool_name,
            &parameters,
        )
        .map_err(|e| e.to_string())?;

    Ok(tool_call)
}

// 新的带 LLM ID 的创建方法
pub async fn create_mcp_tool_call_with_llm_id(
    app_handle: tauri::AppHandle,
    conversation_id: i64,
    message_id: Option<i64>,
    server_name: String,
    tool_name: String,
    parameters: String,
    llm_call_id: Option<&str>,
    assistant_message_id: Option<i64>,
) -> Result<MCPToolCall, String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    // Find the server by name
    let servers = db.get_mcp_servers().map_err(|e| e.to_string())?;
    let server = servers
        .iter()
        .find(|s| s.name == server_name && s.is_enabled)
        .ok_or_else(|| format!("Server '{}' not found or disabled", server_name))?;

    // Create the tool call record with LLM ID
    let tool_call = db
        .create_mcp_tool_call_with_llm_id(
            conversation_id,
            message_id,
            server.id,
            &server_name,
            &tool_name,
            &parameters,
            llm_call_id,
            assistant_message_id,
        )
        .map_err(|e| e.to_string())?;

    Ok(tool_call)
}

#[tauri::command]
pub async fn execute_mcp_tool_call(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
    feature_config_state: tauri::State<'_, crate::FeatureConfigState>,
    message_token_manager: tauri::State<'_, crate::state::message_token::MessageTokenManager>,
    window: tauri::Window,
    call_id: i64,
) -> Result<MCPToolCall, String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e| e.to_string())?;

    // Get the tool call
    let mut tool_call = db.get_mcp_tool_call(call_id).map_err(|e| e.to_string())?;

    // Check if this is a retry of a failed call
    let is_retry = tool_call.status == "failed";

    // Only allow re-execution if the call failed, or if it's not started yet
    if tool_call.status != "pending" && tool_call.status != "failed" {
        return Err(format!(
            "Cannot re-execute tool call with status: {}",
            tool_call.status
        ));
    }

    // Get the server
    let server = db
        .get_mcp_server(tool_call.server_id)
        .map_err(|e| e.to_string())?;

    if !server.is_enabled {
        return Err("Server is disabled".to_string());
    }

    // Conditionally transition to executing to avoid concurrent duplicate executions
    if !db
        .mark_mcp_tool_call_executing_if_pending(call_id)
        .map_err(|e| e.to_string())?
    {
        // Someone else has taken this call (or it already started). Reload and return current state.
        let current = db.get_mcp_tool_call(call_id).map_err(|e| e.to_string())?;
        return Ok(current);
    }

    // Reload tool call to get updated status and emit event
    tool_call = db.get_mcp_tool_call(call_id).map_err(|e| e.to_string())?;
    emit_mcp_tool_call_update(&window, tool_call.conversation_id, &tool_call);

    // Execute the tool based on transport type
    let execution_result = match server.transport_type.as_str() {
        "stdio" => execute_stdio_tool(&server, &tool_call.tool_name, &tool_call.parameters).await,
        "sse" => execute_sse_tool(&server, &tool_call.tool_name, &tool_call.parameters).await,
        "http" => execute_http_tool(&server, &tool_call.tool_name, &tool_call.parameters).await,
        _ => Err(format!(
            "Unsupported transport type: {}",
            server.transport_type
        )),
    };

    // Update the tool call with the result
    match execution_result {
        Ok(result) => {
            db.update_mcp_tool_call_status(call_id, "success", Some(&result), None)
                .map_err(|e| e.to_string())?;
            tool_call.status = "success".to_string();
            tool_call.result = Some(result.clone());
            tool_call.error = None; // Clear any previous error

            // Emit status update event
            emit_mcp_tool_call_update(&window, tool_call.conversation_id, &tool_call);

            // Auto-trigger conversation continuation after successful tool execution
            // For retries, we need to handle this differently to avoid duplicate messages
            if let Err(e) = handle_tool_success_continuation(
                &app_handle,
                &state,
                &feature_config_state,
                &message_token_manager,
                &window,
                &tool_call,
                &result,
                is_retry,
            )
            .await
            {
                println!("Failed to trigger conversation continuation: {}", e);
            }
        }
        Err(error) => {
            db.update_mcp_tool_call_status(call_id, "failed", None, Some(&error))
                .map_err(|e| e.to_string())?;
            tool_call.status = "failed".to_string();
            tool_call.error = Some(error);
            tool_call.result = None; // Clear any previous result

            // Emit status update event
            emit_mcp_tool_call_update(&window, tool_call.conversation_id, &tool_call);
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
    db.get_mcp_tool_calls_by_conversation(conversation_id)
        .map_err(|e| e.to_string())
}

// Handle tool execution success and conversation continuation
// Different logic for first-time execution vs retry
async fn handle_tool_success_continuation(
    app_handle: &tauri::AppHandle,
    state: &tauri::State<'_, crate::AppState>,
    feature_config_state: &tauri::State<'_, crate::FeatureConfigState>,
    message_token_manager: &tauri::State<'_, crate::state::message_token::MessageTokenManager>,
    window: &tauri::Window,
    tool_call: &MCPToolCall,
    result: &str,
    is_retry: bool,
) -> Result<(), String> {
    if is_retry {
        // For retries, we need to update the existing tool_result message instead of creating a new one
        handle_retry_success_continuation(
            app_handle,
            state,
            feature_config_state,
            message_token_manager,
            window,
            tool_call,
            result,
        )
        .await
    } else {
        // For first-time execution, use the original logic
        trigger_conversation_continuation(
            app_handle,
            state,
            feature_config_state,
            message_token_manager,
            window,
            tool_call,
            result,
        )
        .await
    }
}

// Handle retry success by updating existing tool_result message and triggering new AI response
async fn handle_retry_success_continuation(
    app_handle: &tauri::AppHandle,
    _state: &tauri::State<'_, crate::AppState>,
    _feature_config_state: &tauri::State<'_, crate::FeatureConfigState>,
    _message_token_manager: &tauri::State<'_, crate::state::message_token::MessageTokenManager>,
    _window: &tauri::Window,
    tool_call: &MCPToolCall,
    result: &str,
) -> Result<(), String> {
    println!(
        "Handling retry success continuation for tool call {}",
        tool_call.id
    );

    // 对于重试，我们只更新工具结果，不触发新的AI对话
    // 避免重试导致的多次AI回复
    let conversation_db = ConversationDatabase::new(app_handle).map_err(|e| e.to_string())?;

    // Update existing tool_result message in database (for record keeping)
    let messages = conversation_db
        .message_repo()
        .unwrap()
        .list_by_conversation_id(tool_call.conversation_id)
        .map_err(|e| e.to_string())?;

    // Look for existing tool_result message that matches this tool call
    let existing_tool_message = messages.into_iter().find(|(msg, _)| {
        msg.message_type == "tool_result"
            && msg
                .content
                .contains(&format!("Tool Call ID: {}", tool_call.id))
    });

    let updated_tool_result_content = format!(
        "Tool execution completed:\n\nTool Call ID: {}\nTool: {}\nServer: {}\nParameters: {}\nResult:\n{}",
        tool_call.id,
        tool_call.tool_name,
        tool_call.server_name,
        tool_call.parameters,
        result
    );

    match existing_tool_message {
        Some((mut existing_msg, _)) => {
            // Update the existing tool_result message in database
            existing_msg.content = updated_tool_result_content;
            conversation_db
                .message_repo()
                .unwrap()
                .update(&existing_msg)
                .map_err(|e| e.to_string())?;

            println!("Updated existing tool result message {} in database for retry (no AI continuation triggered)", existing_msg.id);
        }
        None => {
            println!("No existing tool result message found for retry tool call {}, skipping AI continuation to avoid duplicates", tool_call.id);
        }
    }

    Ok(())
}

// Trigger conversation continuation after tool execution (genai style)
// Tool results are stored in database and included in AI conversation history but not shown in UI
async fn trigger_conversation_continuation(
    app_handle: &tauri::AppHandle,
    state: &tauri::State<'_, crate::AppState>,
    feature_config_state: &tauri::State<'_, crate::FeatureConfigState>,
    message_token_manager: &tauri::State<'_, crate::state::message_token::MessageTokenManager>,
    window: &tauri::Window,
    tool_call: &MCPToolCall,
    result: &str,
) -> Result<(), String> {
    let conversation_db = ConversationDatabase::new(app_handle).map_err(|e| e.to_string())?;

    // Get the conversation details
    let conversation = conversation_db
        .conversation_repo()
        .unwrap()
        .read(tool_call.conversation_id)
        .map_err(|e| e.to_string())?
        .ok_or("Conversation not found")?;

    let assistant_id = conversation
        .assistant_id
        .ok_or("No assistant associated with conversation")?;

    // Use the new tool_result_continue_ask_ai function instead of creating user message
    // 使用数据库中保存的 llm_call_id（若存在），否则退回到兼容格式
    let tool_call_id = tool_call.llm_call_id
        .clone()
        .unwrap_or_else(|| format!("mcp_tool_call_{}", tool_call.id));

    // Call tool_result_continue_ask_ai to continue the conversation with tool result
    match tool_result_continue_ask_ai(
        app_handle.clone(),
        state.clone(),
        feature_config_state.clone(),
        message_token_manager.clone(),
        window.clone(),
        tool_call.conversation_id.to_string(),
        assistant_id,
        tool_call_id,
        result.to_string(),
    )
    .await
    {
        Ok(_) => {
            println!(
                "Successfully triggered conversation continuation for tool call {}",
                tool_call.id
            );
            Ok(())
        }
        Err(e) => {
            println!("Failed to trigger conversation continuation: {:?}", e);
            Err(format!(
                "Failed to trigger conversation continuation: {:?}",
                e
            ))
        }
    }
}

// Tool execution implementations
async fn execute_stdio_tool(
    server: &MCPServer,
    tool_name: &str,
    parameters: &str,
) -> Result<String, String> {
    use rmcp::{
        model::CallToolRequestParam,
        transport::{ConfigureCommandExt, TokioChildProcess},
        ServiceExt,
    };
    use tokio::process::Command;

    let command = server
        .command
        .as_ref()
        .ok_or("No command specified for stdio transport")?;
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty command".to_string());
    }

    let client_result = tokio::time::timeout(
        std::time::Duration::from_millis(server.timeout.unwrap_or(30000) as u64),
        async {
            let client = (()) // This is a placeholder for the actual client initialization
                .serve(
                    TokioChildProcess::new(Command::new(parts[0]).configure(|cmd| {
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
                    }))
                    .map_err(|e| format!("Failed to create child process: {}", e))?,
                )
                .await
                .map_err(|e| format!("Failed to initialize client: {}", e))?;

            // Parse parameters as JSON（容错：移除 ``` 包裹）
            let params_clean = normalize_parameters_json(parameters);
            let params_value: serde_json::Value = serde_json::from_str(&params_clean)
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

            let response = client
                .call_tool(request_param)
                .await
                .map_err(|e| format!("Tool call failed: {}", e))?;

            // Cancel the client connection
            client
                .cancel()
                .await
                .map_err(|e| format!("Failed to cancel client: {}", e))?;

            // Return the result content
            if response.is_error.unwrap_or(false) {
                Err(format!("Tool execution error: {:?}", response.content))
            } else {
                serde_json::to_string(&response.content)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            }
        },
    )
    .await;

    match client_result {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(format!("Tool execution failed: {}", e)),
        Err(_) => Err("Timeout while executing tool".to_string()),
    }
}

async fn execute_sse_tool(
    server: &MCPServer,
    tool_name: &str,
    parameters: &str,
) -> Result<String, String> {
    use rmcp::{
        model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation},
        transport::SseClientTransport,
        ServiceExt,
    };

    let url = server
        .url
        .as_ref()
        .ok_or("No URL specified for SSE transport")?;

    let client_result = tokio::time::timeout(
        std::time::Duration::from_millis(server.timeout.unwrap_or(30000) as u64),
        async {
            let transport = SseClientTransport::start(url.as_str())
                .await
                .map_err(|e| format!("Failed to start SSE transport: {}", e))?;
            let client_info = ClientInfo {
                protocol_version: Default::default(),
                capabilities: ClientCapabilities::default(),
                client_info: Implementation {
                    name: "AIPP MCP SSE Client".to_string(),
                    version: "0.1.0".to_string(),
                },
            };
            let client = client_info
                .serve(transport)
                .await
                .map_err(|e| format!("Failed to initialize SSE client: {}", e))?;

            // Parse parameters as JSON（容错：移除 ``` 包裹）
            let params_clean = normalize_parameters_json(parameters);
            let params_value: serde_json::Value = serde_json::from_str(&params_clean)
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

            let response = client
                .call_tool(request_param)
                .await
                .map_err(|e| format!("Tool call failed: {}", e))?;

            // Cancel the client connection
            client
                .cancel()
                .await
                .map_err(|e| format!("Failed to cancel client: {}", e))?;

            // Return the result content
            if response.is_error.unwrap_or(false) {
                Err(format!("Tool execution error: {:?}", response.content))
            } else {
                serde_json::to_string(&response.content)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            }
        },
    )
    .await;

    match client_result {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(format!("Tool execution failed: {}", e)),
        Err(_) => Err("Timeout while executing tool".to_string()),
    }
}

async fn execute_http_tool(
    server: &MCPServer,
    tool_name: &str,
    parameters: &str,
) -> Result<String, String> {
    use rmcp::{
        model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation},
        transport::StreamableHttpClientTransport,
        ServiceExt,
    };

    let url = server
        .url
        .as_ref()
        .ok_or("No URL specified for HTTP transport")?;

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
            let client = client_info
                .serve(transport)
                .await
                .map_err(|e| format!("Failed to initialize HTTP client: {}", e))?;

            // Parse parameters as JSON（容错：移除 ``` 包裹）
            let params_clean = normalize_parameters_json(parameters);
            let params_value: serde_json::Value = serde_json::from_str(&params_clean)
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

            let response = client
                .call_tool(request_param)
                .await
                .map_err(|e| format!("Tool call failed: {}", e))?;

            // Cancel the client connection
            client
                .cancel()
                .await
                .map_err(|e| format!("Failed to cancel client: {}", e))?;

            // Return the result content
            if response.is_error.unwrap_or(false) {
                Err(format!("Tool execution error: {:?}", response.content))
            } else {
                serde_json::to_string(&response.content)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            }
        },
    )
    .await;

    match client_result {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(format!("Tool execution failed: {}", e)),
        Err(_) => Err("Timeout while executing tool".to_string()),
    }
}
