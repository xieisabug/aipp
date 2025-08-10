use crate::db::conversation_db::AttachmentType;
use crate::db::conversation_db::Repository;
use crate::db::conversation_db::{Conversation, ConversationDatabase, Message, MessageAttachment};
use crate::errors::AppError;
use genai::chat::ChatMessage;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;

pub fn build_chat_messages(
    init_message_list: &[(String, String, Vec<MessageAttachment>)],
) -> Vec<ChatMessage> {
    let mut chat_messages = Vec::new();
    for (message_type, content, attachment_list) in init_message_list.iter() {
        match message_type.as_str() {
            "system" => chat_messages.push(ChatMessage::system(content)),
            "user" => {
                if attachment_list.is_empty() {
                    chat_messages.push(ChatMessage::user(content));
                } else {
                    let mut parts = Vec::new();
                    parts.push(genai::chat::ContentPart::from_text(content));
                    for attachment in attachment_list {
                        if let Some(attachment_url) = &attachment.attachment_url {
                            match attachment.attachment_type {
                                AttachmentType::Image => {
                                    let mime = infer_media_type_from_url(attachment_url);
                                    parts.push(genai::chat::ContentPart::from_image_url(
                                        mime,
                                        attachment_url.clone(),
                                    ));
                                }
                                AttachmentType::Text => {
                                    // 文本作为上下文已经合并进 prompt
                                }
                                AttachmentType::PDF
                                | AttachmentType::Word
                                | AttachmentType::PowerPoint
                                | AttachmentType::Excel => {
                                    // 文档类型在下方用内容处理
                                }
                            }
                        }
                        if let Some(attachment_content) = &attachment.attachment_content {
                            if attachment.attachment_type == AttachmentType::Image
                                && attachment_content.starts_with("data:")
                            {
                                if let Some((mime, b64)) = parse_data_url(attachment_content) {
                                    parts.push(genai::chat::ContentPart::from_image_base64(
                                        mime, b64,
                                    ));
                                }
                            } else if matches!(
                                attachment.attachment_type,
                                AttachmentType::Text
                                    | AttachmentType::PDF
                                    | AttachmentType::Word
                                    | AttachmentType::PowerPoint
                                    | AttachmentType::Excel
                            ) {
                                let file_name =
                                    attachment.attachment_url.as_deref().unwrap_or("未知文档");
                                let file_type = match attachment.attachment_type {
                                    AttachmentType::PDF => "PDF文档",
                                    AttachmentType::Word => "Word文档",
                                    AttachmentType::PowerPoint => "PowerPoint文档",
                                    AttachmentType::Excel => "Excel文档",
                                    _ => "文档",
                                };
                                parts.push(genai::chat::ContentPart::from_text(format!(
                                    "\n\n[{}: {}]\n{}",
                                    file_type, file_name, attachment_content
                                )));
                            }
                        }
                    }
                    chat_messages.push(ChatMessage::user(parts));
                }
            }
            other => {
                // 将 response / tool_result 统一视为 assistant 历史
                if other == "response" || other == "tool_result" {
                    chat_messages.push(ChatMessage::assistant(content));
                } else if other == "system" {
                    chat_messages.push(ChatMessage::system(content));
                } else {
                    chat_messages.push(ChatMessage::assistant(content));
                }
            }
        }
    }
    chat_messages
}

pub fn infer_media_type_from_url(url: &str) -> String {
    let url_lower = url.to_lowercase();
    if url_lower.ends_with(".jpg") || url_lower.ends_with(".jpeg") {
        "image/jpeg".to_string()
    } else if url_lower.ends_with(".png") {
        "image/png".to_string()
    } else if url_lower.ends_with(".gif") {
        "image/gif".to_string()
    } else if url_lower.ends_with(".webp") {
        "image/webp".to_string()
    } else if url_lower.ends_with(".bmp") {
        "image/bmp".to_string()
    } else {
        "image/jpeg".to_string()
    }
}

pub fn parse_data_url(data_url: &str) -> Option<(String, String)> {
    if !data_url.starts_with("data:") {
        return None;
    }
    let parts: Vec<&str> = data_url.splitn(2, ',').collect();
    if parts.len() != 2 {
        return None;
    }
    let header = parts[0];
    let content = parts[1];
    let header_without_data = header.strip_prefix("data:")?;
    let mime_type = if let Some(semicolon_pos) = header_without_data.find(';') {
        &header_without_data[..semicolon_pos]
    } else {
        header_without_data
    };
    if !header.contains("base64") {
        return None;
    }
    Some((mime_type.to_string(), content.to_string()))
}

pub async fn cleanup_token(
    tokens: &Arc<tokio::sync::Mutex<HashMap<i64, CancellationToken>>>,
    message_id: i64,
) {
    let mut map = tokens.lock().await;
    map.remove(&message_id);
}

pub async fn handle_message_type_end(
    message_id: i64,
    message_type: &str,
    content: &str,
    start_time: chrono::DateTime<chrono::Utc>,
    conversation_db: &ConversationDatabase,
    window: &tauri::Window,
    conversation_id: i64,
    app_handle: &tauri::AppHandle,
) -> Result<(), anyhow::Error> {
    let end_time = chrono::Utc::now();
    let duration_ms = end_time.timestamp_millis() - start_time.timestamp_millis();

    conversation_db
        .message_repo()?
        .update_finish_time(message_id)?;

    if message_type == "response" {
        if let Err(e) = crate::api::ai::mcp::detect_and_process_mcp_calls(
            app_handle,
            conversation_id,
            message_id,
            content,
        )
        .await
        {
            eprintln!("Failed to detect MCP calls: {}", e);
        }
    }

    let type_end_event = crate::api::ai::events::ConversationEvent {
        r#type: "message_type_end".to_string(),
        data: serde_json::to_value(crate::api::ai::events::MessageTypeEndEvent {
            message_id,
            message_type: message_type.to_string(),
            duration_ms,
            end_time,
        })
        .unwrap(),
    };
    let _ = window.emit(
        format!("conversation_event_{}", conversation_id).as_str(),
        type_end_event,
    );

    let final_update_event = crate::api::ai::events::ConversationEvent {
        r#type: "message_update".to_string(),
        data: serde_json::to_value(crate::api::ai::events::MessageUpdateEvent {
            message_id,
            message_type: message_type.to_string(),
            content: content.to_string(),
            is_done: true,
        })
        .unwrap(),
    };
    let _ = window.emit(
        format!("conversation_event_{}", conversation_id).as_str(),
        final_update_event,
    );

    Ok(())
}

pub fn finish_stream_messages(
    conversation_db: &ConversationDatabase,
    reasoning_message_id: Option<i64>,
    response_message_id: Option<i64>,
    reasoning_content: &str,
    response_content: &str,
    window: &tauri::Window,
    conversation_id: i64,
) -> Result<(), anyhow::Error> {
    if let Some(msg_id) = reasoning_message_id {
        if response_message_id.is_none() {
            conversation_db
                .message_repo()
                .unwrap()
                .update_finish_time(msg_id)
                .unwrap();
            let complete_event = crate::api::ai::events::ConversationEvent {
                r#type: "message_update".to_string(),
                data: serde_json::to_value(crate::api::ai::events::MessageUpdateEvent {
                    message_id: msg_id,
                    message_type: "reasoning".to_string(),
                    content: reasoning_content.to_string(),
                    is_done: true,
                })
                .unwrap(),
            };
            let _ = window.emit(
                format!("conversation_event_{}", conversation_id).as_str(),
                complete_event,
            );
        }
    }

    if let Some(msg_id) = response_message_id {
        conversation_db
            .message_repo()
            .unwrap()
            .update_finish_time(msg_id)
            .unwrap();
        let complete_event = crate::api::ai::events::ConversationEvent {
            r#type: "message_update".to_string(),
            data: serde_json::to_value(crate::api::ai::events::MessageUpdateEvent {
                message_id: msg_id,
                message_type: "response".to_string(),
                content: response_content.to_string(),
                is_done: true,
            })
            .unwrap(),
        };
        let _ = window.emit(
            format!("conversation_event_{}", conversation_id).as_str(),
            complete_event,
        );
    }
    Ok(())
}

pub fn init_conversation(
    app_handle: &tauri::AppHandle,
    assistant_id: i64,
    llm_model_id: i64,
    llm_model_code: String,
    messages: &Vec<(String, String, Vec<MessageAttachment>)>,
) -> Result<(Conversation, Vec<Message>), AppError> {
    let db = ConversationDatabase::new(app_handle).map_err(AppError::from)?;
    let conversation = db
        .conversation_repo()
        .unwrap()
        .create(&Conversation {
            id: 0,
            name: "新对话".to_string(),
            assistant_id: Some(assistant_id),
            created_time: chrono::Utc::now(),
        })
        .map_err(AppError::from)?;
    let conversation_clone = conversation.clone();
    let conversation_id = conversation_clone.id;
    let mut message_result_array = vec![];
    for (message_type, content, attachment_list) in messages {
        let message = db
            .message_repo()
            .unwrap()
            .create(&Message {
                id: 0,
                parent_id: None,
                conversation_id,
                message_type: message_type.clone(),
                content: content.clone(),
                llm_model_id: Some(llm_model_id),
                llm_model_name: Some(llm_model_code.clone()),
                created_time: chrono::Utc::now(),
                start_time: None,
                finish_time: None,
                token_count: 0,
                generation_group_id: None,
                parent_group_id: None,
            })
            .map_err(AppError::from)?;
        for attachment in attachment_list {
            let mut updated_attachment = attachment.clone();
            updated_attachment.message_id = message.id;
            db.attachment_repo()
                .unwrap()
                .update(&updated_attachment)
                .map_err(AppError::from)?;
        }
        message_result_array.push(message.clone());
    }
    Ok((conversation_clone, message_result_array))
}
