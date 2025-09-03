use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;
use tauri::Manager; // for AppHandle.state
use crate::db::conversation_db::Repository; // for repository.read

// 对话级别的 MCP 执行状态管理
type ConversationMcpState = Arc<Mutex<HashMap<i64, u32>>>;

static CONVERSATION_MCP_DEPTH: OnceLock<ConversationMcpState> = OnceLock::new();

const MAX_MCP_RECURSION_DEPTH: u32 = 3;

/// MCP 工具调用检测结果
#[derive(Debug, Clone)]
pub struct McpCallDetection {
    pub server_name: String,
    pub tool_name: String,
    pub parameters: String,
}

/// 检测内容中的所有 MCP 工具调用（通用函数，供子任务复用）
pub fn detect_mcp_calls_in_content(content: &str) -> Vec<McpCallDetection> {
    let mcp_regex = regex::Regex::new(r"<mcp_tool_call>\s*<server_name>([^<]*)</server_name>\s*<tool_name>([^<]*)</tool_name>\s*<parameters>([\s\S]*?)</parameters>\s*</mcp_tool_call>").unwrap();
    
    let mut calls = Vec::new();
    for cap in mcp_regex.captures_iter(content) {
        let server_name = cap[1].trim().to_string();
        let tool_name = cap[2].trim().to_string();
        let parameters = cap[3].trim().to_string();

        calls.push(McpCallDetection {
            server_name,
            tool_name,
            parameters,
        });
    }
    
    calls
}

/// 过滤 MCP 调用（根据启用的服务器和工具，供子任务复用）
pub fn filter_mcp_calls(
    calls: Vec<McpCallDetection>,
    enabled_servers: &[String],
    enabled_tools: &Option<HashMap<String, Vec<String>>>,
) -> Vec<McpCallDetection> {
    calls
        .into_iter()
        .filter(|call| {
            // 检查服务器是否启用
            if !enabled_servers.contains(&call.server_name) {
                return false;
            }

            // 检查工具是否启用
            if let Some(ref tools_map) = enabled_tools {
                if let Some(allowed_tools) = tools_map.get(&call.server_name) {
                    if !allowed_tools.contains(&call.tool_name) {
                        return false;
                    }
                }
            }

            true
        })
        .collect()
}

/// 专为子任务设计的 MCP 调用检测和处理函数（复用核心逻辑）
pub async fn detect_and_process_mcp_calls_for_subtask(
    app_handle: &tauri::AppHandle,
    conversation_id: i64,
    subtask_id: i64,
    content: &str,
    enabled_servers: &[String],
    enabled_tools: &Option<HashMap<String, Vec<String>>>,
) -> Result<Vec<crate::mcp::mcp_db::MCPToolCall>, anyhow::Error> {
    let mcp_regex = regex::Regex::new(r"<mcp_tool_call>\s*<server_name>([^<]*)</server_name>\s*<tool_name>([^<]*)</tool_name>\s*<parameters>([\s\S]*?)</parameters>\s*</mcp_tool_call>").unwrap();
    
    let mut executed_calls = Vec::new();
    
    // 处理所有匹配的 MCP 调用
    for cap in mcp_regex.captures_iter(content) {
        let server_name = cap[1].trim().to_string();
        let tool_name = cap[2].trim().to_string();
        let parameters = cap[3].trim().to_string();

        println!(
            "Detected MCP call for subtask: server={}, tool={}, conversation_id={}, subtask_id={}",
            server_name, tool_name, conversation_id, subtask_id
        );

        // 检查服务器是否在允许列表中
        if !enabled_servers.contains(&server_name) {
            println!("Server '{}' not in enabled list for subtask", server_name);
            continue;
        }

        // 检查工具是否在允许列表中
        if let Some(ref tools_map) = enabled_tools {
            if let Some(allowed_tools) = tools_map.get(&server_name) {
                if !allowed_tools.contains(&tool_name) {
                    println!("Tool '{}' not in enabled list for server '{}'", tool_name, server_name);
                    continue;
                }
            }
        }

        // 查找服务器（复用原逻辑）
        let mcp_db = crate::mcp::mcp_db::MCPDatabase::new(app_handle)?;
        let servers = mcp_db.get_mcp_servers()?;
        let server_opt = servers.iter().find(|s| s.name == server_name && s.is_enabled);

        if let Some(server) = server_opt {
            // 创建工具调用记录（用于子任务）
            let tool_call = mcp_db.create_mcp_tool_call_for_subtask(
                conversation_id,
                subtask_id,
                server.id,
                &server_name,
                &tool_name,
                &parameters,
                None,
            )?;

            // 直接执行工具调用（复用现有执行逻辑）
            let execution_result = crate::mcp::execution_api::execute_tool_by_transport(
                app_handle,
                server,
                &tool_name,
                &parameters,
            ).await;

            // 更新工具调用状态
            match execution_result {
                Ok(result) => {
                    let _ = mcp_db.update_mcp_tool_call_status(
                        tool_call.id,
                        "success",
                        Some(&result),
                        None,
                    );
                    println!("MCP tool call executed successfully for subtask");
                },
                Err(error) => {
                    let _ = mcp_db.update_mcp_tool_call_status(
                        tool_call.id,
                        "failed",
                        None,
                        Some(&error),
                    );
                    println!("MCP tool call failed for subtask: {}", error);
                }
            }

            executed_calls.push(tool_call);
        } else {
            println!("Server '{}' not found or disabled", server_name);
        }
    }

    Ok(executed_calls)
}

pub async fn detect_and_process_mcp_calls(
    app_handle: &tauri::AppHandle,
    window: &tauri::Window,
    conversation_id: i64,
    message_id: i64,
    content: &str,
) -> Result<(), anyhow::Error> {
    // Check conversation-level recursion depth to prevent infinite loops
    let depth_state = CONVERSATION_MCP_DEPTH.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));
    let mut depth_map = depth_state.lock().await;
    let current_depth = *depth_map.get(&conversation_id).unwrap_or(&0);

    if current_depth >= MAX_MCP_RECURSION_DEPTH {
        println!(
            "MCP recursion depth limit reached for conversation {} (depth: {}), skipping detection",
            conversation_id, current_depth
        );
        return Ok(());
    }

    // Increment conversation-level recursion depth
    depth_map.insert(conversation_id, current_depth + 1);
    drop(depth_map); // 释放锁

    let result = async {
        let mcp_regex = regex::Regex::new(r"<mcp_tool_call>\s*<server_name>([^<]*)</server_name>\s*<tool_name>([^<]*)</tool_name>\s*<parameters>([\s\S]*?)</parameters>\s*</mcp_tool_call>").unwrap();

        // 只处理第一个匹配的 MCP 调用，避免单次回复中执行多个工具
        if let Some(cap) = mcp_regex.captures_iter(content).next() {
            let server_name = cap[1].trim().to_string();
            let tool_name = cap[2].trim().to_string();
            let parameters = cap[3].trim().to_string();

            println!(
                "Detected MCP call: server={}, tool={}, conversation_id={}, message_id={}",
                server_name, tool_name, conversation_id, message_id
            );

            // 避免重复：若已存在相同 message_id/server/tool/parameters 的 pending/failed/success 记录，则复用
            let existing_call_opt = {
                let db = crate::mcp::mcp_db::MCPDatabase::new(app_handle).ok();
                db.and_then(|db| db.get_mcp_tool_calls_by_conversation(conversation_id).ok())
                    .and_then(|calls| {
                        calls.into_iter().find(|c| {
                            c.message_id == Some(message_id)
                                && c.server_name == server_name
                                && c.tool_name == tool_name
                                && c.parameters.trim() == parameters.trim()
                        })
                    })
            };

            let create_result = if let Some(existing) = existing_call_opt {
                Ok(existing)
            } else {
                crate::mcp::execution_api::create_mcp_tool_call_with_llm_id(
                    app_handle.clone(),
                    conversation_id,
                    Some(message_id),
                    server_name.clone(),
                    tool_name.clone(),
                    parameters.clone(),
                    None,
                    None,
                )
                .await
            };

            match create_result {
                Ok(tool_call) => {
                    println!("Created MCP tool call with ID: {}", tool_call.id);

                    // 尝试根据助手配置自动执行（is_auto_run）
                    if let Ok(conversation_db) = crate::db::conversation_db::ConversationDatabase::new(app_handle) {
                        if let Ok(repository) = conversation_db.conversation_repo() {
                            if let Ok(Some(conversation)) = repository.read(conversation_id) {
                                if let Some(assistant_id) = conversation.assistant_id {
                                    match crate::api::assistant_api::get_assistant_mcp_servers_with_tools(
                                        app_handle.clone(),
                                        assistant_id,
                                    )
                                    .await
                                    {
                                        Ok(servers_with_tools) => {
                                            let mut should_auto_run = false;
                                            for s in servers_with_tools.iter() {
                                                if s.name == server_name && s.is_enabled {
                                                    if let Some(tool) = s.tools.iter().find(|t| t.name == tool_name && t.is_enabled) {
                                                        if tool.is_auto_run {
                                                            should_auto_run = true;
                                                        }
                                                    }
                                                }
                                            }

                                            if should_auto_run {
                                                let state = app_handle.state::<crate::AppState>();
                                                let feature_config_state = app_handle.state::<crate::FeatureConfigState>();
                                                let message_token_manager = app_handle.state::<crate::state::message_token::MessageTokenManager>();
                                                if let Err(e) = crate::mcp::execution_api::execute_mcp_tool_call(
                                                    app_handle.clone(),
                                                    state,
                                                    feature_config_state,
                                                    message_token_manager,
                                                    window.clone(),
                                                    tool_call.id,
                                                )
                                                .await
                                                {
                                                    eprintln!(
                                                        "Auto-execute MCP tool failed (call_id={}): {}",
                                                        tool_call.id, e
                                                    );
                                                }
                                            } else {
                                                println!(
                                                    "MCP tool auto-run is disabled for {}/{}",
                                                    server_name, tool_name
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "Failed to load assistant MCP configs for auto-run: {}",
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to create MCP tool call: {}", e);
                }
            }
        } else {
            println!("No MCP tool calls detected in message content");
        }
        Ok(())
    }
    .await;

    // Decrement conversation-level recursion depth
    let depth_state = CONVERSATION_MCP_DEPTH.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));
    let mut depth_map = depth_state.lock().await;
    if let Some(depth) = depth_map.get_mut(&conversation_id) {
        if *depth > 0 {
            *depth -= 1;
        }
        if *depth == 0 {
            depth_map.remove(&conversation_id);
        }
    }

    result
}
