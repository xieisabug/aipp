use crate::api::assistant_api::{get_assistant_mcp_servers_with_tools, MCPServerWithTools};
use crate::api::mcp_execution_api::create_mcp_tool_call;
use crate::errors::AppError;

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
    let mcp_constraint_prompt = r#"
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

```xml
<mcp_tool_call>
<server_name>服务器名称</server_name>
<tool_name>工具名称</tool_name>
<parameters>
{
  "parameter1": "value1",
  "parameter2": "value2"
}
</parameters>
</mcp_tool_call>
```

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

pub async fn detect_and_process_mcp_calls(
    app_handle: &tauri::AppHandle,
    conversation_id: i64,
    message_id: i64,
    content: &str,
) -> Result<(), anyhow::Error> {
    let mcp_regex = regex::Regex::new(r"<mcp_tool_call>\s*<server_name>([^<]*)</server_name>\s*<tool_name>([^<]*)</tool_name>\s*<parameters>([\s\S]*?)</parameters>\s*</mcp_tool_call>").unwrap();
    for cap in mcp_regex.captures_iter(content) {
        let server_name = cap[1].trim().to_string();
        let tool_name = cap[2].trim().to_string();
        let parameters = cap[3].trim().to_string();
        match create_mcp_tool_call(
            app_handle.clone(),
            conversation_id,
            Some(message_id),
            server_name,
            tool_name,
            parameters,
        )
        .await
        {
            Ok(tool_call) => {
                println!("Created MCP tool call with ID: {}", tool_call.id);
            }
            Err(e) => {
                eprintln!("Failed to create MCP tool call: {}", e);
            }
        }
    }
    Ok(())
}

