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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversationResult {
    pub id: i64,
    pub name: String,
    pub assistant_id: i64,
    pub assistant_name: String,
    pub created_time: DateTime<Utc>,
}

#[tauri::command]
pub async fn list_conversations(
    app_handle: tauri::AppHandle,
    name_cache_state: tauri::State<'_, NameCacheState>,
    page: u32,
    page_size: u32,
) -> Result<Vec<ConversationResult>, AppError> {
    let db = ConversationDatabase::new(&app_handle).map_err(AppError::from)?;

    let conversations = db
        .conversation_repo()
        .unwrap()
        .list(page, page_size)
        .map_err(|e| e.to_string());

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
) -> Result<(ConversationResult, Vec<MessageDetail>), String> {
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
            attachment_map
                .entry(message.id)
                .or_default()
                .push(attachment);
        }
    }

    // Convert messages to a HashMap to preserve it for the second pass
    let message_map: HashMap<i64, Message> = messages
        .clone()
        .into_iter()
        .map(|(message, _)| (message.id, message))
        .collect();

    // Second pass: Create MessageDetail with the collected attachments
    for (message_id, message) in message_map {
        let attachment_list = attachment_map.get(&message_id).cloned().unwrap_or_default();
        message_details.push(MessageDetail {
            id: message.id,
            conversation_id: message.conversation_id,
            message_type: message.message_type,
            content: message.content,
            llm_model_id: message.llm_model_id,
            created_time: message.created_time,
            start_time: message.start_time,
            finish_time: message.finish_time,
            token_count: message.token_count,
            attachment_list,
            regenerate: Vec::new(),
            parent_id: message.parent_id,
        });
    }

    // 处理 regenerate 关系
    let regenerate_map: HashMap<i64, Vec<MessageDetail>> = message_details
        .iter()
        .filter(|m| m.parent_id.is_some())
        .map(|m| (m.parent_id.unwrap(), m.clone()))
        .fold(HashMap::new(), |mut acc, (parent_id, message)| {
            acc.entry(parent_id).or_default().push(message);
            acc
        });

    for message in &mut message_details {
        if let Some(regenerated) = regenerate_map.get(&message.id) {
            message.regenerate = regenerated.clone();
        }
    }

    // 过滤掉有 parent_id 的消息，并按 ID 排序
    message_details = message_details
        .into_iter()
        .filter(|m| m.parent_id.is_none())
        .collect();
    message_details.sort_by_key(|m| m.id);

    let assistant_name_cache = name_cache_state.assistant_names.lock().await;
    let assistant_name = assistant_name_cache
        .get(&conversation.assistant_id.unwrap_or(0))
        .cloned()
        .unwrap_or_else(|| "未知".to_string());

    Ok((
        ConversationResult {
            id: conversation.id,
            name: conversation.name,
            assistant_id: conversation.assistant_id.unwrap_or(0),
            assistant_name,
            created_time: conversation.created_time,
        },
        message_details,
    ))
}

#[tauri::command]
pub fn delete_conversation(
    app_handle: tauri::AppHandle,
    conversation_id: i64,
) -> Result<(), String> {
    let db = ConversationDatabase::new(&app_handle).map_err(|e| e.to_string())?;
    db.conversation_repo()
        .unwrap()
        .delete(conversation_id)
        .map_err(|e| e.to_string())
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
    db.conversation_repo()
        .unwrap()
        .update(&conversation)
        .map_err(|e| e.to_string())?;

    let _ = app_handle.emit("title_change", [conversation_id.to_string(), name]);
    Ok(())
}
