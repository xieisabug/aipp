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

    let _ = window.emit(format!("conversation_event_{}", conversation_id).as_str(), update_event);
}

// 验证工具调用是否可以执行
fn validate_tool_call_execution(tool_call: &MCPToolCall) -> Result<bool, String> {
    let is_retry = tool_call.status == "failed";

    if tool_call.status != "pending" && tool_call.status != "failed" {
        return Err(format!("工具调用状态为 {} 时无法重新执行", tool_call.status));
    }

    Ok(is_retry)
}

// 验证服务器状态
fn validate_server_status(server: &MCPServer) -> Result<(), String> {
    if !server.is_enabled {
        return Err("MCP服务器已禁用".to_string());
    }
    Ok(())
}

// 处理工具执行结果
async fn handle_tool_execution_result(
    app_handle: &tauri::AppHandle,
    state: &tauri::State<'_, crate::AppState>,
    feature_config_state: &tauri::State<'_, crate::FeatureConfigState>,
    message_token_manager: &tauri::State<'_, crate::state::message_token::MessageTokenManager>,
    window: &tauri::Window,
    call_id: i64,
    mut tool_call: MCPToolCall,
    execution_result: Result<String, String>,
    is_retry: bool,
) -> Result<MCPToolCall, String> {
    let db = MCPDatabase::new(app_handle).map_err(|e| format!("初始化数据库失败: {}", e))?;

    match execution_result {
        Ok(result) => {
            println!("✅ 工具调用 {} 执行成功", tool_call.id);

            db.update_mcp_tool_call_status(call_id, "success", Some(&result), None)
                .map_err(|e| format!("更新工具调用状态失败: {}", e))?;

            tool_call.status = "success".to_string();
            tool_call.result = Some(result.clone());
            tool_call.error = None;

            emit_mcp_tool_call_update(window, tool_call.conversation_id, &tool_call);

            // 处理对话继续逻辑
            if let Err(e) = handle_tool_success_continuation(
                app_handle,
                state,
                feature_config_state,
                message_token_manager,
                window,
                &tool_call,
                &result,
                is_retry,
            )
            .await
            {
                println!("⚠️ 工具执行成功但触发对话继续失败: {}", e);
            }
        }
        Err(error) => {
            println!("❌ 工具调用 {} 执行失败: {}", tool_call.id, error);

            db.update_mcp_tool_call_status(call_id, "failed", None, Some(&error))
                .map_err(|e| format!("更新工具调用状态失败: {}", e))?;

            tool_call.status = "failed".to_string();
            tool_call.error = Some(error);
            tool_call.result = None;

            emit_mcp_tool_call_update(window, tool_call.conversation_id, &tool_call);
        }
    }

    Ok(tool_call)
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
    llm_call_id: Option<String>,
    assistant_message_id: Option<i64>,
) -> Result<MCPToolCall, String> {
    let db = MCPDatabase::new(&app_handle).map_err(|e| format!("初始化数据库失败: {}", e))?;

    // 查找并验证服务器
    let servers = db.get_mcp_servers().map_err(|e| format!("获取MCP服务器列表失败: {}", e))?;
    let server = servers
        .iter()
        .find(|s| s.name == server_name && s.is_enabled)
        .ok_or_else(|| format!("服务器 '{}' 未找到或已禁用", server_name))?;

    // 根据是否提供 llm_call_id 选择相应的创建方法
    let tool_call = if llm_call_id.is_some() || assistant_message_id.is_some() {
        db.create_mcp_tool_call_with_llm_id(
            conversation_id,
            message_id,
            server.id,
            &server_name,
            &tool_name,
            &parameters,
            llm_call_id.as_deref(),
            assistant_message_id,
        )
    } else {
        db.create_mcp_tool_call(
            conversation_id,
            message_id,
            server.id,
            &server_name,
            &tool_name,
            &parameters,
        )
    };

    let result = tool_call.map_err(|e| format!("创建MCP工具调用失败: {}", e))?;

    Ok(result)
}

// 为了向后兼容，提供一个不带LLM ID的创建函数
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
    create_mcp_tool_call(
        app_handle,
        conversation_id,
        message_id,
        server_name,
        tool_name,
        parameters,
        llm_call_id.map(|s| s.to_string()),
        assistant_message_id,
    )
    .await
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
    let db = MCPDatabase::new(&app_handle).map_err(|e| format!("初始化数据库失败: {}", e))?;

    // 获取工具调用信息
    let mut tool_call =
        db.get_mcp_tool_call(call_id).map_err(|e| format!("获取工具调用信息失败: {}", e))?;

    // 验证工具调用状态
    let is_retry = validate_tool_call_execution(&tool_call)?;

    // 获取并验证服务器状态
    let server = db
        .get_mcp_server(tool_call.server_id)
        .map_err(|e| format!("获取MCP服务器信息失败: {}", e))?;
    validate_server_status(&server)?;

    // 原子性地将状态转为执行中，避免并发重复执行
    if !db
        .mark_mcp_tool_call_executing_if_pending(call_id)
        .map_err(|e| format!("更新工具调用状态失败: {}", e))?
    {
        let current = db
            .get_mcp_tool_call(call_id)
            .map_err(|e| format!("获取当前工具调用状态失败: {}", e))?;
        return Ok(current);
    }

    // 重新加载工具调用以获取更新后的状态并发送事件
    tool_call =
        db.get_mcp_tool_call(call_id).map_err(|e| format!("重新加载工具调用信息失败: {}", e))?;
    emit_mcp_tool_call_update(&window, tool_call.conversation_id, &tool_call);

    // 执行工具
    let execution_result =
        execute_tool_by_transport(&app_handle, &server, &tool_call.tool_name, &tool_call.parameters)
            .await;

    // 处理执行结果
    handle_tool_execution_result(
        &app_handle,
        &state,
        &feature_config_state,
        &message_token_manager,
        &window,
        call_id,
        tool_call,
        execution_result,
        is_retry,
    )
    .await
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

// 处理重试成功的情况：更新现有工具结果消息并触发新的AI响应
async fn handle_retry_success_continuation(
    app_handle: &tauri::AppHandle,
    state: &tauri::State<'_, crate::AppState>,
    feature_config_state: &tauri::State<'_, crate::FeatureConfigState>,
    message_token_manager: &tauri::State<'_, crate::state::message_token::MessageTokenManager>,
    window: &tauri::Window,
    tool_call: &MCPToolCall,
    result: &str,
) -> Result<(), String> {
    let conversation_db = ConversationDatabase::new(app_handle)
        .map_err(|e| format!("初始化对话数据库失败: {}", e))?;

    // 更新现有的 tool_result 消消息在数据库中（用于记录保存）
    let messages = conversation_db
        .message_repo()
        .unwrap()
        .list_by_conversation_id(tool_call.conversation_id)
        .map_err(|e| format!("获取对话消息列表失败: {}", e))?;

    // 查找与此工具调用匹配的现有 tool_result 消息
    let existing_tool_message = messages.into_iter().find(|(msg, _)| {
        msg.message_type == "tool_result"
            && msg.content.contains(&format!("Tool Call ID: {}", tool_call.id))
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
            // 更新现有的 tool_result 消息在数据库中
            existing_msg.content = updated_tool_result_content;
            conversation_db
                .message_repo()
                .unwrap()
                .update(&existing_msg)
                .map_err(|e| format!("更新工具结果消息失败: {}", e))?;

            // 重试成功后也应该触发AI对话继续
            return trigger_conversation_continuation(
                app_handle,
                state,
                feature_config_state,
                message_token_manager,
                window,
                tool_call,
                result,
            )
            .await;
        }
        None => {
            // 即使未找到现有消息，也应该触发对话继续
            return trigger_conversation_continuation(
                app_handle,
                state,
                feature_config_state,
                message_token_manager,
                window,
                tool_call,
                result,
            )
            .await;
        }
    }
}

// 触发工具执行后的对话继续（genai 风格）
// 工具结果存储在数据库中并包含在AI对话历史中，但不在UI中显示
async fn trigger_conversation_continuation(
    app_handle: &tauri::AppHandle,
    state: &tauri::State<'_, crate::AppState>,
    feature_config_state: &tauri::State<'_, crate::FeatureConfigState>,
    message_token_manager: &tauri::State<'_, crate::state::message_token::MessageTokenManager>,
    window: &tauri::Window,
    tool_call: &MCPToolCall,
    result: &str,
) -> Result<(), String> {
    let conversation_db = ConversationDatabase::new(app_handle)
        .map_err(|e| format!("初始化对话数据库失败: {}", e))?;

    // 获取对话详情
    let conversation = conversation_db
        .conversation_repo()
        .unwrap()
        .read(tool_call.conversation_id)
        .map_err(|e| format!("获取对话信息失败: {}", e))?
        .ok_or("未找到对话")?;

    let assistant_id = conversation.assistant_id.ok_or("对话未关联助手")?;

    // 使用数据库中保存的 llm_call_id（若存在），否则退回到兼容格式
    let tool_call_id =
        tool_call.llm_call_id.clone().unwrap_or_else(|| format!("mcp_tool_call_{}", tool_call.id));

    // 调用 tool_result_continue_ask_ai 以工具结果继续对话
    tool_result_continue_ask_ai(
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
    .map_err(|e| format!("触发对话继续失败: {:?}", e))?;

    Ok(())
}

// 统一的工具执行函数，根据传输类型选择相应的执行策略
async fn execute_tool_by_transport(
    app_handle: &tauri::AppHandle,
    server: &MCPServer,
    tool_name: &str,
    parameters: &str,
) -> Result<String, String> {
    match server.transport_type.as_str() {
        // If stdio but command is aipp:*, route to builtin executor
        "stdio" => {
            if let Some(cmd) = &server.command {
                if crate::mcp::builtin_api::is_builtin_mcp_call(cmd) {
                    execute_builtin_tool(app_handle, server, tool_name, parameters).await
                } else {
                    execute_stdio_tool(app_handle, server, tool_name, parameters).await
                }
            } else {
                execute_stdio_tool(app_handle, server, tool_name, parameters).await
            }
        },
        "sse" => execute_sse_tool(app_handle, server, tool_name, parameters).await,
        "http" => execute_http_tool(app_handle, server, tool_name, parameters).await,
        // Legacy builtin type is no longer used, but keep for backward compatibility
        "builtin" => execute_builtin_tool(app_handle, server, tool_name, parameters).await,
        _ => {
            let error_msg = format!("不支持的传输类型: {}", server.transport_type);
            Err(error_msg)
        }
    }
}
async fn execute_stdio_tool(
    app_handle: &tauri::AppHandle,
    server: &MCPServer,
    tool_name: &str,
    parameters: &str,
) -> Result<String, String> {
    // If command is aipp:*, delegate to builtin executor
    if let Some(cmd) = &server.command {
        if crate::mcp::builtin_api::is_builtin_mcp_call(cmd) {
            return execute_builtin_tool(app_handle, server, tool_name, parameters).await;
        }
    }
    use rmcp::{
        model::CallToolRequestParam,
        transport::{ConfigureCommandExt, TokioChildProcess},
        ServiceExt,
    };
    use tokio::process::Command;

    let command = server.command.as_ref().ok_or("未为 stdio 传输指定命令")?;
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("命令为空".to_string());
    }

    let timeout_ms = server.timeout.unwrap_or(30000) as u64;

    let client_result = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), async {
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
                .map_err(|e| format!("创建子进程失败: {}", e))?,
            )
            .await
            .map_err(|e| format!("初始化客户端失败: {}", e))?;

        // 解析参数为 JSON（容错：移除 ``` 包裹）
        let params_clean = normalize_parameters_json(parameters);
        let params_value: serde_json::Value =
            serde_json::from_str(&params_clean).map_err(|e| format!("无效的参数 JSON: {}", e))?;

        // Convert Value to Map<String, Value>
        let params_map = match params_value {
            serde_json::Value::Object(map) => map,
            _ => return Err("参数必须是 JSON 对象".to_string()),
        };

        // Call the tool with correct API
        let request_param = CallToolRequestParam {
            name: tool_name.to_string().into(),
            arguments: Some(params_map),
        };

        let response =
            client.call_tool(request_param).await.map_err(|e| format!("工具调用失败: {}", e))?;

        // Cancel the client connection
        client.cancel().await.map_err(|e| format!("关闭客户端连接失败: {}", e))?;

        // Return the result content
        if response.is_error.unwrap_or(false) {
            Err(format!("工具执行错误: {:?}", response.content))
        } else {
            serde_json::to_string(&response.content).map_err(|e| format!("序列化结果失败: {}", e))
        }
    })
    .await;

    match client_result {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(format!("工具执行失败: {}", e)),
        Err(_) => Err("工具执行超时".to_string()),
    }
}

async fn execute_sse_tool(
    _app_handle: &tauri::AppHandle,
    server: &MCPServer,
    tool_name: &str,
    parameters: &str,
) -> Result<String, String> {
    use rmcp::{
        model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation},
        transport::SseClientTransport,
        ServiceExt,
    };

    let url = server.url.as_ref().ok_or("No URL specified for SSE transport")?;

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
            client.cancel().await.map_err(|e| format!("Failed to cancel client: {}", e))?;

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

// 执行内置工具
async fn execute_builtin_tool(
    app_handle: &tauri::AppHandle,
    server: &MCPServer,
    tool_name: &str,
    parameters: &str,
) -> Result<String, String> {
    use crate::mcp::builtin_api::{execute_aipp_builtin_tool, is_builtin_mcp_call};

    // 验证是否为内置工具调用
    let command = server.command.clone().unwrap_or_default();
    if !is_builtin_mcp_call(&command) {
        return Err(format!("Unknown builtin tool: {} for command: {}", tool_name, command));
    }

    // 通过 rmcp 的内置服务执行工具，并规范化返回为 content JSON 字符串
    let params_clean = normalize_parameters_json(parameters);
    let raw = execute_aipp_builtin_tool(app_handle.clone(), command.clone(), tool_name.to_string(), params_clean).await?;

    // raw 是序列化后的 ToolResult，提取其中的 content 字段以与其他传输保持一致
    let v: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| format!("解析内置工具结果失败: {}", e))?;
    let is_error = v
        .get("is_error")
        .or_else(|| v.get("isError"))
        .and_then(|x| x.as_bool())
        .unwrap_or(false);
    if is_error {
        return Err(format!("工具执行错误: {}", v.get("content").unwrap_or(&serde_json::Value::Null)));
    }
    let content = v.get("content").cloned().unwrap_or(serde_json::Value::Null);
    serde_json::to_string(&content).map_err(|e| format!("序列化结果失败: {}", e))
}

async fn execute_http_tool(
    _app_handle: &tauri::AppHandle,
    server: &MCPServer,
    tool_name: &str,
    parameters: &str,
) -> Result<String, String> {
    use rmcp::{
        model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation},
        transport::StreamableHttpClientTransport,
        ServiceExt,
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
            client.cancel().await.map_err(|e| format!("Failed to cancel client: {}", e))?;

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
