use crate::api::assistant_api::{get_assistant_mcp_servers_with_tools, MCPServerWithTools};
use crate::errors::AppError;

#[derive(Debug, Clone)]
pub struct MCPInfoForAssistant {
    pub enabled_servers: Vec<MCPServerWithTools>,
    pub use_native_toolcall: bool,
}

pub async fn collect_mcp_info_for_assistant(
    app_handle: &tauri::AppHandle,
    assistant_id: i64,
    mcp_override_config: Option<&crate::api::ai::types::McpOverrideConfig>,
) -> Result<MCPInfoForAssistant, AppError> {
    let use_native_toolcall = match crate::api::assistant_api::get_assistant_field_value(
        app_handle.clone(), assistant_id, "use_native_toolcall",
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

    // Apply override configuration for use_native_toolcall if provided
    let final_use_native_toolcall = mcp_override_config
        .and_then(|config| config.use_native_toolcall)
        .unwrap_or(use_native_toolcall);

    let all_servers = get_assistant_mcp_servers_with_tools(app_handle.clone(), assistant_id)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get MCP servers: {}", e)))?;

    // 只保留启用的服务器
    let enabled_servers: Vec<MCPServerWithTools> =
        all_servers.into_iter().filter(|server| server.is_enabled).collect();

    Ok(MCPInfoForAssistant { enabled_servers, use_native_toolcall: final_use_native_toolcall })
}

pub async fn format_mcp_prompt(
    assistant_prompt_result: String,
    mcp_info: &MCPInfoForAssistant,
) -> String {
    format_mcp_prompt_with_filters(assistant_prompt_result, mcp_info, None, None).await
}

pub async fn format_mcp_prompt_with_filters(
    assistant_prompt_result: String,
    mcp_info: &MCPInfoForAssistant,
    enabled_servers: Option<&Vec<String>>,
    enabled_tools: Option<&std::collections::HashMap<String, Vec<String>>>,
) -> String {
    let mcp_constraint_prompt: &str = r#"
# MCP (Model Context Protocol) 工具使用规范

作为 AI 助手，你可以使用以下 MCP 工具来执行各种任务。请严格遵守以下规则：

## 使用原则
1. 你只能调用系统明确提供的 MCP 工具，不得虚构或调用未提及的工具
2. 仅在有助于完成任务时调用工具；能靠自身知识完成时不调用
3. 信任工具的返回结果（除非工具明确报错、超时或返回无效数据）
4. 一次只调用一个工具；如需多个步骤，请分多轮依次调用
5. 工具调用必须放在本条消息的最后

## 输出格式
当需要调用 MCP 工具时，请使用以下 XML 格式，注意不需要代码块包裹：

<mcp_tool_call>
  <server_name>服务器名称</server_name>
  <tool_name>工具名称</tool_name>
  <parameters>{"parameter1":"value1"}</parameters>
  </mcp_tool_call>

## 重要注意事项
- 参数必须是有效的 JSON 格式
- 如果工具不需要参数，parameters 标签内应该为空对象 {}
- 不得伪造工具响应或猜测未返回的数据
"#;

    let mut tools_info = String::from("\n## 可用的 MCP 工具\n\n");
    
    for server_details in &mcp_info.enabled_servers {
        // Check if this server is in the enabled servers list
        if let Some(enabled_server_names) = enabled_servers {
            if !enabled_server_names.contains(&server_details.name) {
                continue;
            }
        }

        tools_info.push_str(&format!("### 服务器: {}\n", server_details.name));
        tools_info.push_str("\n#### 可用工具:\n\n");

        for tool in &server_details.tools {
            // Check if this tool is enabled for this server
            if let Some(enabled_tools_map) = enabled_tools {
                if let Some(allowed_tools) = enabled_tools_map.get(&server_details.name) {
                    if !allowed_tools.contains(&tool.name) {
                        continue;
                    }
                }
            }

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
