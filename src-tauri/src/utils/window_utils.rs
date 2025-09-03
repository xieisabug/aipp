use crate::api::ai::events::{ConversationEvent, ERROR_NOTIFICATION_EVENT};
use tauri::{Emitter, Manager, Window};

/// 检查chat和ask窗口是否有任何一个聚焦
/// 如果有任何一个窗口聚焦，返回true；否则返回false
pub fn is_chat_or_ask_window_focused(app_handle: &tauri::AppHandle) -> bool {
    // 检查 ask 窗口是否聚焦
    if let Some(ask_window) = app_handle.get_webview_window("ask") {
        if let Ok(is_visible) = ask_window.is_visible() {
            if is_visible {
                if let Ok(is_focused) = ask_window.is_focused() {
                    if is_focused {
                        return true;
                    }
                }
            }
        }
    }

    // 检查 chat_ui 窗口是否聚焦
    if let Some(chat_ui_window) = app_handle.get_webview_window("chat_ui") {
        if let Ok(is_visible) = chat_ui_window.is_visible() {
            if is_visible {
                if let Ok(is_focused) = chat_ui_window.is_focused() {
                    if is_focused {
                        return true;
                    }
                }
            }
        }
    }

    false
}

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

/// 向对话相关窗口发送对话事件
/// 同时向 ask 和 chat_ui 窗口发送对话事件，确保所有相关界面都能收到通知
pub fn send_conversation_event_to_chat_windows(
    app_handle: &tauri::AppHandle,
    conversation_id: i64,
    event: ConversationEvent,
) {
    let event_name = format!("conversation_event_{}", conversation_id);
    
    // 发送给 Ask 窗口
    if let Some(ask_window) = app_handle.get_webview_window("ask") {
        let _ = ask_window.emit(&event_name, &event);
    }
    
    // 发送给 Chat UI 窗口
    if let Some(chat_ui_window) = app_handle.get_webview_window("chat_ui") {
        let _ = chat_ui_window.emit(&event_name, &event);
    }
}
