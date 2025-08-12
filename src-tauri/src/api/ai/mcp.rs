use crate::api::assistant_api::{get_assistant_mcp_servers_with_tools, MCPServerWithTools};
use crate::db::conversation_db::Repository;
use tauri::Manager;
use crate::errors::AppError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct MCPInfoForAssistant {
    pub enabled_servers: Vec<MCPServerWithTools>,
    pub use_native_toolcall: bool,
}

pub async fn collect_mcp_info_for_assistant(
    app_handle: &tauri::AppHandle,
    assistant_id: i64,
) -> Result<MCPInfoForAssistant, AppError> {
    let use_native_toolcall = match super::super::assistant_api::get_assistant_field_value(
        app_handle.clone(),
        assistant_id,
        "use_native_toolcall",
    ) {
        Ok(value) => value == "true",
        Err(e) => {
            println!(
                "Failed to get native toolcall config: {}, using default (false)",
                e
            );
            false
        }
    };

    let enabled_servers = get_assistant_mcp_servers_with_tools(app_handle.clone(), assistant_id)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get MCP servers: {}", e)))?;

    Ok(MCPInfoForAssistant {
        enabled_servers,
        use_native_toolcall,
    })
}

pub async fn format_mcp_prompt(
    assistant_prompt_result: String,
    mcp_info: &MCPInfoForAssistant,
) -> String {
    let mcp_constraint_prompt: &str = r#"
# MCP (Model Context Protocol) 工具使用规范

作为 AI 助手，你可以使用以下 MCP 工具来执行各种任务。请严格遵守以下规则：

## 使用原则
1. 你仅能调用以下提供的工具，不能够调用未提及的工具
2. 优先使用最适合任务的工具
3. 使用工具你可以获取到你需要的数据，不要怀疑工具的返回结果
4. 每次只调用一个工具，等待之后返回的结果
5. 工具调用放置在你回复的最后

## 输出格式
当需要调用 MCP 工具时，请使用以下 XML 格式：

<mcp_tool_call>
  <server_name>服务器名称</server_name>
  <tool_name>工具名称</tool_name>
  <parameters>{"parameter1":"value1"}</parameters>
</mcp_tool_call>

## 重要注意事项
- 参数必须是有效的 JSON 格式
- 如果工具不需要参数，parameters 标签内应该为空对象 {}
"#;

    let mut tools_info = String::from("\n## 可用的 MCP 工具\n\n");
    for server_details in &mcp_info.enabled_servers {
        tools_info.push_str(&format!("### 服务器: {}\n", server_details.name));
        tools_info.push_str("\n#### 可用工具:\n\n");

        for tool in &server_details.tools {
            tools_info.push_str(&format!("**{}** \n", tool.name));
            tools_info.push_str(&format!(" - description: {}\n", tool.description));
            tools_info.push_str(&format!(" - parameters: {}\n", tool.parameters));
            tools_info.push_str("\n\n");
        }
        tools_info.push_str("\n---\n\n");
    }

    format!(
        "{}\n{}\n{}\n{}",
        mcp_constraint_prompt, tools_info, "## 原始助手指令\n", assistant_prompt_result
    )
}

/// 为原生 ToolCall 模式提供轻量提示，帮助模型更倾向调用工具
pub fn format_native_mcp_hint(mcp_info: &MCPInfoForAssistant) -> String {
    let mut s = String::from("\n\n# Tools Available\n\nYou have access to the following tools. When they are helpful, prefer to call them via function calling. Call only one tool at a time and wait for the result before continuing.\n\n");
    for server in &mcp_info.enabled_servers {
        s.push_str(&format!("- Server: {}\n", server.name));
        for tool in &server.tools {
            s.push_str(&format!(
                "  - Tool: {} (call name: {}__{})\n    Description: {}\n",
                tool.name,
                server.name,
                tool.name,
                tool.description
            ));
        }
        s.push('\n');
    }
    s
}

// 对话级别的 MCP 执行状态管理
type ConversationMcpState = Arc<Mutex<HashMap<i64, u32>>>;

lazy_static::lazy_static! {
    static ref CONVERSATION_MCP_DEPTH: ConversationMcpState = Arc::new(Mutex::new(HashMap::new()));
}

const MAX_MCP_RECURSION_DEPTH: u32 = 3;

pub async fn detect_and_process_mcp_calls(
    app_handle: &tauri::AppHandle,
    window: &tauri::Window,
    conversation_id: i64,
    message_id: i64,
    content: &str,
) -> Result<(), anyhow::Error> {
    // Check conversation-level recursion depth to prevent infinite loops
    let mut depth_map = CONVERSATION_MCP_DEPTH.lock().await;
    let current_depth = *depth_map.get(&conversation_id).unwrap_or(&0);
    
    if current_depth >= MAX_MCP_RECURSION_DEPTH {
        println!("MCP recursion depth limit reached for conversation {} (depth: {}), skipping detection", 
                conversation_id, current_depth);
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
        
        println!("Detected MCP call: server={}, tool={}, conversation_id={}, message_id={}", 
                server_name, tool_name, conversation_id, message_id);
        
        // 避免重复：若已存在相同 message_id/server/tool/parameters 的 pending/failed/success 记录，则复用
        let existing_call_opt = {
            let db = crate::db::mcp_db::MCPDatabase::new(app_handle).ok();
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
            crate::api::mcp_execution_api::create_mcp_tool_call_with_llm_id(
                app_handle.clone(),
                conversation_id,
                Some(message_id),
                server_name.clone(),
                tool_name.clone(),
                parameters.clone(),
                None, // llm_call_id - MCP检测创建的调用没有LLM call ID
                None, // assistant_message_id - MCP检测创建的调用没有关联的assistant消息
            )
            .await
        };

        match create_result {
            Ok(tool_call) => {
                println!("Created MCP tool call with ID: {}", tool_call.id);

                // 尝试根据助手配置自动执行（is_auto_run）
                // 1) 获取该对话的助手ID
                if let Ok(conversation_db) = crate::db::conversation_db::ConversationDatabase::new(app_handle) {
                    if let Ok(repository) = conversation_db.conversation_repo() {
                        if let Ok(Some(conversation)) = repository.read(conversation_id) {
                            if let Some(assistant_id) = conversation.assistant_id {
                            // 2) 读取助手的 MCP 服务器与工具配置
                            match crate::api::assistant_api::get_assistant_mcp_servers_with_tools(
                                app_handle.clone(),
                                assistant_id,
                            )
                            .await
                            {
                                Ok(servers_with_tools) => {
                                    // 3) 定位到当前 server/tool 的配置
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
                                        if let Err(e) = crate::api::mcp_execution_api::execute_mcp_tool_call(
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
                                        println!("MCP tool auto-run is disabled for {}/{}", server_name, tool_name);
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
    }.await;

    // Decrement conversation-level recursion depth
    let mut depth_map = CONVERSATION_MCP_DEPTH.lock().await;
    if let Some(depth) = depth_map.get_mut(&conversation_id) {
        if *depth > 0 {
            *depth -= 1;
        }
        // 如果深度为0，移除记录以节省内存
        if *depth == 0 {
            depth_map.remove(&conversation_id);
        }
    }

    result
}

