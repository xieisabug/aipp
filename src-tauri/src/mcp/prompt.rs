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
    enabled_servers_filter: Option<&Vec<String>>, // 可选过滤：服务器名称或ID（字符串）
) -> Result<MCPInfoForAssistant, AppError> {
    let use_native_toolcall = match crate::api::assistant_api::get_assistant_field_value(
        app_handle.clone(),
        assistant_id,
        "use_native_toolcall",
    ) {
        Ok(value) => value == "true",
        Err(e) => {
            println!("Failed to get native toolcall config: {}, using default (false)", e);
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

    // 根据传入的 enabled_servers_filter 选择服务器：
    // - 如果提供了 ID 列表，就按该列表精确选择对应服务器（忽略助手层面的启用状态）
    // - 如果未提供，则只保留助手配置里启用的服务器
    let enabled_servers: Vec<MCPServerWithTools> = if let Some(filters) = enabled_servers_filter {
        if !filters.is_empty() {
            use std::collections::HashSet;
            let filter_set: HashSet<&String> = filters.iter().collect();

            // 1. 先加入助手“已启用”的服务器（基础集合的一部分）
            let mut picked: Vec<MCPServerWithTools> = Vec::new();
            let mut existing_id_set: HashSet<i64> = HashSet::new();
            for server in &all_servers {
                if server.is_enabled {
                    picked.push(server.clone());
                    existing_id_set.insert(server.id);
                }
            }

            // 2. 过滤列表里出现的（ID 或 名称匹配） -> 并集：即便未启用也加入
            for server in &all_servers {
                let id_str = server.id.to_string();
                if (filter_set.contains(&id_str) || filter_set.contains(&server.name))
                    && !existing_id_set.contains(&server.id)
                {
                    picked.push(server.clone());
                    existing_id_set.insert(server.id);
                }
            }

            // 3. filters 里额外的 ID（不在 all_servers 中）再批量查
            let mut extra_ids: Vec<i64> = Vec::new();
            for raw in filters.iter() {
                if let Ok(id_val) = raw.parse::<i64>() {
                    if !existing_id_set.contains(&id_val) {
                        extra_ids.push(id_val);
                    }
                }
            }

            if !extra_ids.is_empty() {
                if let Ok(db) = crate::mcp::mcp_db::MCPDatabase::new(app_handle) {
                    if let Ok(pairs) = db.get_mcp_servers_with_tools_by_ids(&extra_ids) {
                        for (srv, tools_raw) in pairs {
                            if existing_id_set.contains(&srv.id) {
                                continue;
                            }
                            // 只保留启用工具
                            let tools_converted: Vec<crate::api::assistant_api::MCPToolInfo> =
                                tools_raw
                                    .into_iter()
                                    .filter(|t| t.is_enabled)
                                    .map(|t| crate::api::assistant_api::MCPToolInfo {
                                        id: t.id,
                                        name: t.tool_name,
                                        description: t.tool_description.unwrap_or_default(),
                                        is_enabled: t.is_enabled,
                                        is_auto_run: t.is_auto_run,
                                        parameters: t
                                            .parameters
                                            .unwrap_or_else(|| "{}".to_string()),
                                    })
                                    .collect();
                            picked.push(MCPServerWithTools {
                                id: srv.id,
                                name: srv.name,
                                is_enabled: srv.is_enabled,
                                tools: tools_converted,
                            });
                            existing_id_set.insert(srv.id);
                        }
                    }
                }
            }

            picked
        } else {
            // 空过滤列表 => 助手启用服务器
            all_servers.into_iter().filter(|s| s.is_enabled).collect()
        }
    } else {
        // 无过滤 => 助手启用服务器
        all_servers.into_iter().filter(|s| s.is_enabled).collect()
    };

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
        if let Some(enabled_server_id) = enabled_servers {
            if !enabled_server_id.contains(&server_details.id.to_string()) {
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
