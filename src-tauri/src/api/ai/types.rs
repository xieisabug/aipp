use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// MCP配置覆盖
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct McpOverrideConfig {
    // 覆盖特定工具的自动运行配置
    pub tool_auto_run: Option<HashMap<String, bool>>,  // "serverId/toolName" -> autoRun
    // 覆盖整个服务器的启用状态
    pub server_enabled: Option<HashMap<String, bool>>,  // "serverId" -> enabled
    // 覆盖是否使用原生工具调用
    pub use_native_toolcall: Option<bool>,
    // 自定义MCP工具调用超时时间
    pub tool_call_timeout: Option<u64>,
}

// MCP事件处理器配置（不能序列化函数，只传递是否有处理器的标记）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct McpHandlerConfig {
    pub has_detected_handler: bool,
    pub has_executing_handler: bool,
    pub has_result_handler: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AiRequest {
    pub conversation_id: String,
    pub assistant_id: i64,
    pub prompt: String,
    pub model: Option<String>,
    pub override_model_id: Option<String>,  // 覆盖助手的默认模型ID
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
    pub attachment_list: Option<Vec<i64>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AiResponse {
    pub conversation_id: i64,
    pub request_prompt_result_with_context: String,
}
