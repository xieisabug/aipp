// Central MCP module: prompt building, detection, execution API, and builtin wrappers

pub mod prompt;
pub mod detection;
pub mod execution_api;
pub mod builtin_mcp;
pub mod registry_api;
pub mod mcp_db;

// Re-exports for convenience to minimize callsite churn
pub use prompt::{MCPInfoForAssistant, collect_mcp_info_for_assistant, format_mcp_prompt};
pub use detection::detect_and_process_mcp_calls;
