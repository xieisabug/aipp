use crate::db::conversation_db::AttachmentType;
use crate::db::conversation_db::Repository;
use crate::db::conversation_db::{Conversation, ConversationDatabase, Message, MessageAttachment};
use crate::errors::AppError;
use base64::Engine;
use genai::chat::{ChatMessage, ToolCall, ToolResponse};
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub fn build_chat_messages(
    init_message_list: &[(String, String, Vec<MessageAttachment>)],
) -> Vec<ChatMessage> {
    build_chat_messages_with_context(init_message_list, None)
}

pub fn build_chat_messages_with_context(
    init_message_list: &[(String, String, Vec<MessageAttachment>)],
    current_tool_call_id: Option<String>,
) -> Vec<ChatMessage> {
    println!(
        "[[DEBUG - build_chat_messages_with_context]] current_tool_call_id: {:?}",
        current_tool_call_id
    );

    let mut chat_messages = Vec::new();
    for (message_type, content, attachment_list) in init_message_list.iter() {
        println!(
            "[[DEBUG]] Processing message type: {}, content preview: {}",
            message_type,
            content.chars().take(50).collect::<String>()
        );

        match message_type.as_str() {
            "system" => chat_messages.push(ChatMessage::system(content)),
            "user" => {
                if attachment_list.is_empty() {
                    chat_messages.push(ChatMessage::user(content));
                } else {
                    let mut parts = Vec::new();
                    parts.push(genai::chat::ContentPart::from_text(content));
                    for attachment in attachment_list {
                        // 优先处理图片附件（OpenAI 不支持 file:// 本地 URL，需要转为 base64）
                        if attachment.attachment_type == AttachmentType::Image {
                            // 1) 若 attachment_content 为 data:URL，直接解析
                            if let Some(content) = &attachment.attachment_content {
                                if content.starts_with("data:") {
                                    if let Some((mime, b64)) = parse_data_url(content) {
                                        parts.push(genai::chat::ContentPart::from_binary_base64(
                                            None, mime, b64,
                                        ));
                                        continue;
                                    }
                                }
                            }

                            // 2) 若 attachment_url 为 http/https，则可直接使用 URL
                            if let Some(url) = &attachment.attachment_url {
                                let url_lower = url.to_lowercase();
                                if url_lower.starts_with("http://") || url_lower.starts_with("https://") {
                                    let mime = infer_media_type_from_url(url);
                                    parts.push(genai::chat::ContentPart::from_binary_url(
                                        None,
                                        mime,
                                        url.clone(),
                                    ));
                                    continue;
                                }

                                // 3) 若 attachment_url 是 data:URL，则解析为 base64
                                if url_lower.starts_with("data:") {
                                    if let Some((mime, b64)) = parse_data_url(url) {
                                        parts.push(genai::chat::ContentPart::from_binary_base64(
                                            None, mime, b64,
                                        ));
                                        continue;
                                    }
                                }

                                // 4) 其他情况（如 file:// 或本地路径）：读取文件转 base64
                                let path = if url_lower.starts_with("file://") {
                                    // 去掉 file:// 前缀
                                    url.trim_start_matches("file://").to_string()
                                } else {
                                    url.clone()
                                };
                                // 尝试读取文件并转换
                                if let Ok(bytes) = std::fs::read(&path) {
                                    let mime = infer_media_type_from_url(url);
                                    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
                                    parts.push(genai::chat::ContentPart::from_binary_base64(
                                        None, mime, b64,
                                    ));
                                    continue;
                                } else {
                                    // 无法读取则跳过为安全
                                    eprintln!("[warn] Failed to read image file for attachment: {}", url);
                                }
                            }
                        }

                        // 非图片类型或图片回退处理
                        if let Some(attachment_content) = &attachment.attachment_content {
                            if matches!(
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
            "tool_result" => {
                println!("[[DEBUG]] Processing tool_result message");
                // Priority: 1. current_tool_call_id, 2. extracted from content, 3. random
                let tool_call_id = current_tool_call_id
                    .clone()
                    .or_else(|| extract_tool_call_id(content))
                    .unwrap_or_else(|| {
                        format!("tool_call_{}", Uuid::new_v4().to_string()[..8].to_string())
                    });

                println!("[[DEBUG]] Using tool_call_id: {}", tool_call_id);

                // Try to extract clean result, fallback to full content
                let tool_result =
                    extract_tool_result(content).unwrap_or_else(|| content.to_string());

                println!(
                    "[[DEBUG]] Tool result preview: {}",
                    tool_result.chars().take(100).collect::<String>()
                );

                // Create ToolResponse from genai crate
                let tool_response = ToolResponse::new(tool_call_id.clone(), tool_result);
                println!("[[DEBUG]] Created ToolResponse with call_id: {}", tool_call_id);
                chat_messages.push(ChatMessage::from(tool_response));
            }
            other => {
                // 将 response 统一视为 assistant 历史
                if other == "response" {
                    // 检查是否包含 MCP_TOOL_CALL 注释，如果有则需要重建包含工具调用的 assistant 消息
                    if let Some(assistant_with_calls) =
                        reconstruct_assistant_with_tool_calls_from_content(content)
                    {
                        chat_messages.push(assistant_with_calls);
                    } else {
                        chat_messages.push(ChatMessage::assistant(content));
                    }
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

// Helper function to extract tool call ID from tool result content
pub fn extract_tool_call_id(content: &str) -> Option<String> {
    // Expected format: "Tool execution completed:\n\nTool Call ID: {id}\nResult:\n{result}"
    if let Some(start) = content.find("Tool Call ID: ") {
        let start_pos = start + "Tool Call ID: ".len();
        if let Some(end) = content[start_pos..].find('\n') {
            return Some(content[start_pos..start_pos + end].to_string());
        } else {
            // If no newline found, take rest of string (shouldn't happen with our format)
            return Some(content[start_pos..].to_string());
        }
    }
    None
}

// Helper function to extract tool result from tool result content
pub fn extract_tool_result(content: &str) -> Option<String> {
    // Expected format: "Tool execution completed:\n\nTool Call ID: {id}\nResult:\n{result}"
    if let Some(start) = content.find("Result:\n") {
        let start_pos = start + "Result:\n".len();
        return Some(content[start_pos..].to_string());
    }
    None
}

// Helper function to reconstruct assistant message with tool calls from MCP_TOOL_CALL comments
pub fn reconstruct_assistant_with_tool_calls_from_content(content: &str) -> Option<ChatMessage> {
    // 查找所有 MCP_TOOL_CALL 注释
    let mcp_call_regex = Regex::new(r"<!-- MCP_TOOL_CALL:(.*?) -->").ok()?;
    let mut tool_calls = Vec::new();

    // 提取所有工具调用信息
    for capture in mcp_call_regex.captures_iter(content) {
        if let Ok(tool_data) = serde_json::from_str::<serde_json::Value>(&capture[1]) {
            if let (Some(server_name), Some(tool_name), Some(parameters)) = (
                tool_data["server_name"].as_str(),
                tool_data["tool_name"].as_str(),
                tool_data["parameters"].as_str(),
            ) {
                // 使用正确的格式：server__tool (双下划线)
                let fn_name = format!("{}__{}", server_name, tool_name);
                let fn_arguments =
                    serde_json::from_str(parameters).unwrap_or(serde_json::json!({}));

                // 优先使用 llm_call_id，如果没有则使用 call_id 转换为字符串
                let call_id = tool_data["llm_call_id"]
                    .as_str()
                    .map(|s| s.to_string())
                    .or_else(|| tool_data["call_id"].as_u64().map(|n| n.to_string()))
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                tool_calls.push(ToolCall { call_id, fn_name, fn_arguments });
            }
        }
    }

    if !tool_calls.is_empty() {
        // 创建包含工具调用的 assistant 消息（忽略文本内容以避免混合消息类型的复杂性）
        return Some(ChatMessage::from(tool_calls));
    }

    None
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
    skip_mcp_detection: bool,
) -> Result<(), anyhow::Error> {
    let end_time = chrono::Utc::now();
    let duration_ms = end_time.timestamp_millis() - start_time.timestamp_millis();

    conversation_db.message_repo()?.update_finish_time(message_id)?;

    if message_type == "response" && !skip_mcp_detection {
    if let Err(e) = crate::mcp::detect_and_process_mcp_calls(
            app_handle,
            window,
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
    let _ = window.emit(format!("conversation_event_{}", conversation_id).as_str(), type_end_event);

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
    let _ =
        window.emit(format!("conversation_event_{}", conversation_id).as_str(), final_update_event);

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
            conversation_db.message_repo().unwrap().update_finish_time(msg_id).unwrap();
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
            let _ = window
                .emit(format!("conversation_event_{}", conversation_id).as_str(), complete_event);
        }
    }

    if let Some(msg_id) = response_message_id {
        conversation_db.message_repo().unwrap().update_finish_time(msg_id).unwrap();
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
        let _ =
            window.emit(format!("conversation_event_{}", conversation_id).as_str(), complete_event);
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
                tool_calls_json: None,
            })
            .map_err(AppError::from)?;
        for attachment in attachment_list {
            let mut updated_attachment = attachment.clone();
            updated_attachment.message_id = message.id;
            db.attachment_repo().unwrap().update(&updated_attachment).map_err(AppError::from)?;
        }
        message_result_array.push(message.clone());
    }
    Ok((conversation_clone, message_result_array))
}
