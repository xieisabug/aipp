use serde::{Deserialize, Serialize};
use tauri::AppHandle;

// Re-export everything from the new modularized structure
pub use crate::api::builtin_mcp::{
    is_builtin_command as is_aipp_builtin_command,
    execute_aipp_builtin_tool,
    list_aipp_builtin_templates,
    add_or_update_aipp_builtin_server,
    get_builtin_tools_for_command,
    BuiltinTemplateEnvVar, BuiltinTemplateInfo, BuiltinToolInfo,
    SearchHandler,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest { 
    pub query: String 
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FetchRequest { 
    pub url: String 
}

/// 内置搜索工具处理器（为了兼容性保留）
#[deprecated(note = "Use SearchHandler from builtin_mcp::search module instead")]
#[derive(Clone)]
pub struct BuiltinSearchHandler {
    app_handle: AppHandle,
}

#[allow(deprecated)]
impl BuiltinSearchHandler {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    pub async fn search_web(&self, query: &str) -> Result<serde_json::Value, String> {
        let handler = SearchHandler::new(self.app_handle.clone());
        handler.search_web(query).await
    }

    pub async fn fetch_url(&self, url: &str) -> Result<serde_json::Value, String> {
        let handler = SearchHandler::new(self.app_handle.clone());
        handler.fetch_url(url).await
    }
}

// Legacy function aliases for backward compatibility
pub fn is_builtin_mcp_call(command: &str) -> bool { 
    is_aipp_builtin_command(command) 
}