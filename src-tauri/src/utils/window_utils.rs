use crate::api::ai::events::ERROR_NOTIFICATION_EVENT;
use tauri::{Emitter, Manager, Window};

/// 智能发送错误到合适的窗口
/// 如果 ChatUI 窗口打开且可见，优先发送给 ChatUI
/// 否则发送给 Ask 窗口
pub fn send_error_to_appropriate_window(window: &Window, error_message: &str) {
    // 获取 ChatUI 窗口
    if let Some(chat_ui_window) = window.app_handle().get_webview_window("chat_ui") {
        // 检查 ChatUI 窗口是否可见
        if let Ok(is_visible) = chat_ui_window.is_visible() {
            if is_visible {
                // ChatUI 窗口可见，发送错误给 ChatUI
                let _ = chat_ui_window.emit(ERROR_NOTIFICATION_EVENT, error_message);
                return;
            }
        }
    }
    
    // ChatUI 窗口不存在或不可见，发送给 Ask 窗口
    if let Some(ask_window) = window.app_handle().get_webview_window("ask") {
        let _ = ask_window.emit(ERROR_NOTIFICATION_EVENT, error_message);
    } else {
        // 回退到全局广播（保险起见）
        let _ = window.emit(ERROR_NOTIFICATION_EVENT, error_message);
    }
}