use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::Emitter;

use crate::{
    db::conversation_db::{
        ConversationDatabase, Message, MessageAttachment, MessageDetail, Repository,
    },
    errors::AppError,
    NameCacheState,
};

/// 处理消息版本管理的纯函数 - 这是核心业务逻辑
/// 输入原始消息列表，返回经过版本管理处理的最终消息列表
pub fn process_message_versions(mut message_details: Vec<MessageDetail>) -> Vec<MessageDetail> {
    // 处理 regenerate 关系 - 支持 generation_group_id 系统
    let regenerate_map: HashMap<i64, Vec<MessageDetail>> = message_details
        .iter()
        .filter(|m| m.parent_id.is_some())
        .map(|m| (m.parent_id.unwrap(), m.clone()))
        .fold(HashMap::new(), |mut acc, (parent_id, message)| {
            acc.entry(parent_id).or_default().push(message);
            acc
        });

    // 为每个消息构建regenerate数组
    for message in &mut message_details {
        if let Some(regenerated) = regenerate_map.get(&message.id) {
            // 对regenerate消息按创建时间排序
            let mut sorted_regenerated = regenerated.clone();
            sorted_regenerated.sort_by_key(|m| m.created_time);
            message.regenerate = sorted_regenerated;
        }
    }

    // 过滤逻辑：显示最新版本的消息
    // 1. 如果消息没有parent_id，它是原始消息
    // 2. 如果消息有parent_id，它是某条消息的新版本
    // 3. 我们需要显示：原始消息（如果没有更新版本）或最新的更新版本

    // 构建parent_id到直接子消息的映射
    let mut direct_children: HashMap<i64, Vec<MessageDetail>> = HashMap::new();
    let mut child_message_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();

    for message in &message_details {
        if let Some(parent_id) = message.parent_id {
            child_message_ids.insert(message.id);
            direct_children.entry(parent_id).or_default().push(message.clone());
        }
    }

    // 对每个父消息的子消息按时间排序
    for children in direct_children.values_mut() {
        children.sort_by_key(|m| m.created_time);
    }

    // 递归查找最终的最新版本
    fn find_latest_version(
        message_id: i64,
        direct_children: &HashMap<i64, Vec<MessageDetail>>,
    ) -> Option<MessageDetail> {
        if let Some(children) = direct_children.get(&message_id) {
            if let Some(latest_child) = children.last() {
                // 递归查找这个子版本的最新版本
                find_latest_version(latest_child.id, direct_children)
                    .or_else(|| Some(latest_child.clone()))
            } else {
                None
            }
        } else {
            None
        }
    }

    // 构建最终显示的消息列表
    let mut final_messages: Vec<MessageDetail> = Vec::new();
    for message in message_details {
        if child_message_ids.contains(&message.id) {
            // 这是某个消息的子版本，跳过（会在后续处理中添加最新版本）
            continue;
        }

        // 检查是否有这个消息的更新版本（递归查找）
        if let Some(latest_version) = find_latest_version(message.id, &direct_children) {
            // 有更新版本，使用最新版本
            final_messages.push(latest_version);
        } else {
            // 没有更新版本，使用原消息
            final_messages.push(message);
        }
    }

    // 按创建时间排序
    final_messages.sort_by_key(|m| m.created_time);
    final_messages
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversationResult {
    pub id: i64,
    pub name: String,
    pub assistant_id: i64,
    pub assistant_name: String,
    pub created_time: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversationWithMessages {
    pub conversation: ConversationResult,
    pub messages: Vec<MessageDetail>,
}

#[tauri::command]
pub async fn list_conversations(
    app_handle: tauri::AppHandle,
    name_cache_state: tauri::State<'_, NameCacheState>,
    page: u32,
    page_size: u32,
) -> Result<Vec<ConversationResult>, AppError> {
    let db = ConversationDatabase::new(&app_handle).map_err(AppError::from)?;

    let conversations =
        db.conversation_repo().unwrap().list(page, page_size).map_err(|e| e.to_string());

    let mut conversation_results = Vec::new();
    let assistant_name_cache = name_cache_state.assistant_names.lock().await.clone();
    if let Ok(conversations) = &conversations {
        for conversation in conversations {
            let assistant_name = assistant_name_cache.get(&conversation.assistant_id.unwrap());
            conversation_results.push(ConversationResult {
                id: conversation.id,
                name: conversation.name.clone(),
                assistant_id: conversation.assistant_id.unwrap_or(0),
                assistant_name: assistant_name.unwrap_or(&"未知".to_string()).clone(),
                created_time: conversation.created_time,
            });
        }
    }
    Ok(conversation_results)
}

#[tauri::command]
pub async fn get_conversation_with_messages(
    app_handle: tauri::AppHandle,
    name_cache_state: tauri::State<'_, NameCacheState>,
    conversation_id: i64,
) -> Result<ConversationWithMessages, String> {
    let db = ConversationDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let conversation = db
        .conversation_repo()
        .unwrap()
        .read(conversation_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Conversation not found".to_string())?;

    let messages = db
        .message_repo()
        .unwrap()
        .list_by_conversation_id(conversation_id)
        .map_err(|e| e.to_string())?;

    let mut message_details: Vec<MessageDetail> = Vec::new();
    let mut attachment_map: HashMap<i64, Vec<MessageAttachment>> = HashMap::new();

    for (message, attachment) in messages.clone() {
        if let Some(attachment) = attachment {
            attachment_map.entry(message.id).or_default().push(attachment);
        }
    }

    // Convert messages to a HashMap to preserve it for the second pass
    let message_map: HashMap<i64, Message> =
        messages.clone().into_iter().map(|(message, _)| (message.id, message)).collect();

    // Second pass: Create MessageDetail with the collected attachments
    for (message_id, message) in message_map {
        let attachment_list = attachment_map.get(&message_id).cloned().unwrap_or_default();
        message_details.push(MessageDetail {
            id: message.id,
            parent_id: message.parent_id,
            conversation_id: message.conversation_id,
            message_type: message.message_type,
            content: message.content,
            llm_model_id: message.llm_model_id,
            created_time: message.created_time,
            start_time: message.start_time,
            finish_time: message.finish_time,
            token_count: message.token_count,
            generation_group_id: message.generation_group_id,
            parent_group_id: message.parent_group_id,
            tool_calls_json: message.tool_calls_json,
            attachment_list,
            regenerate: Vec::new(),
        });
    }

    // 处理消息版本管理逻辑
    let final_messages = process_message_versions(message_details);

    let assistant_name_cache = name_cache_state.assistant_names.lock().await;
    let assistant_name = assistant_name_cache
        .get(&conversation.assistant_id.unwrap_or(0))
        .cloned()
        .unwrap_or_else(|| "未知".to_string());

    Ok(ConversationWithMessages {
        conversation: ConversationResult {
            id: conversation.id,
            name: conversation.name,
            assistant_id: conversation.assistant_id.unwrap_or(0),
            assistant_name,
            created_time: conversation.created_time,
        },
        messages: final_messages,
    })
}

#[tauri::command]
pub fn delete_conversation(
    app_handle: tauri::AppHandle,
    conversation_id: i64,
) -> Result<(), String> {
    let db = ConversationDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    db.conversation_repo().unwrap().delete(conversation_id).map_err(|e| e.to_string())?;

    // 发送删除事件通知前端更新列表
    let _ = app_handle.emit("conversation_deleted", conversation_id);

    Ok(())
}

#[tauri::command]
pub fn update_conversation(
    app_handle: tauri::AppHandle,
    conversation_id: i64,
    name: String,
) -> Result<(), String> {
    let db = ConversationDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    let mut conversation = db
        .conversation_repo()
        .unwrap()
        .read(conversation_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Conversation not found".to_string())?;
    conversation.name = name.clone();
    db.conversation_repo().unwrap().update(&conversation).map_err(|e| e.to_string())?;

    let _ = app_handle.emit("title_change", (conversation_id, name));
    Ok(())
}

#[tauri::command]
pub fn update_message_content(
    app_handle: tauri::AppHandle,
    message_id: i64,
    content: String,
) -> Result<(), String> {
    let db = ConversationDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    db.message_repo().unwrap().update_content(message_id, &content).map_err(|e| e.to_string())
}
