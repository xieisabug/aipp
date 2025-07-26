use crate::api::assistant_api::get_assistant;
use crate::api::genai_client;
use crate::db::assistant_db::AssistantModelConfig;
use crate::db::conversation_db::{AttachmentType, Repository};
use crate::db::conversation_db::{Conversation, ConversationDatabase, Message, MessageAttachment};
use crate::db::llm_db::LLMDatabase;
use crate::db::system_db::FeatureConfig;
use crate::errors::AppError;
use crate::state::message_token::MessageTokenManager;
use crate::template_engine::TemplateEngine;
use crate::{AppState, FeatureConfigState};
use anyhow::Context;
use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tauri::Emitter;
use tauri::State;
use tokio_util::sync::CancellationToken;


/// Conversation事件数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConversationEvent {
    r#type: String,
    data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MessageAddEvent {
    message_id: i64,
    message_type: String,
    temp_message_id: i64, // 用于取消操作的临时ID
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MessageUpdateEvent {
    message_id: i64,
    message_type: String,
    content: String,
    is_done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MessageTypeEndEvent {
    message_id: i64,
    message_type: String,
    duration_ms: i64,
    end_time: chrono::DateTime<chrono::Utc>,
}

use futures::StreamExt;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, ContentPart};
use genai::Client;
use tokio::time::{sleep, Duration};

use super::assistant_api::AssistantDetail;

// 事件名称常量
const _MESSAGE_FINISH_EVENT: &str = "Tea::Event::MessageFinish";
const TITLE_CHANGE_EVENT: &str = "title_change";
const ERROR_NOTIFICATION_EVENT: &str = "conversation-window-error-notification";

/// 重试配置
const MAX_RETRY_ATTEMPTS: u32 = 1;
const RETRY_DELAY_MS: u64 = 1000;

/// AI聊天配置
#[derive(Debug, Clone)]
struct ChatConfig {
    model_name: String,
    stream: bool,
    chat_options: ChatOptions,
    client: Client,
}


/// 配置构建器
struct ConfigBuilder;

impl ConfigBuilder {
    /// 构建聊天选项
    fn build_chat_options(config_map: &HashMap<String, String>) -> ChatOptions {
        let mut chat_options = ChatOptions::default();

        if let Some(temp_str) = config_map.get("temperature") {
            if let Ok(temp) = temp_str.parse::<f64>() {
                chat_options = chat_options.with_temperature(temp);
            }
        }

        if let Some(max_tokens_str) = config_map.get("max_tokens") {
            if let Ok(max_tokens) = max_tokens_str.parse::<u32>() {
                chat_options = chat_options.with_max_tokens(max_tokens);
            }
        }

        if let Some(top_p_str) = config_map.get("top_p") {
            if let Ok(top_p) = top_p_str.parse::<f64>() {
                chat_options = chat_options.with_top_p(top_p);
            }
        }

        chat_options
    }

    /// 合并模型配置
    fn merge_model_configs(
        base_configs: Vec<AssistantModelConfig>,
        model_detail: &crate::db::llm_db::ModelDetail,
        override_configs: Option<HashMap<String, serde_json::Value>>,
    ) -> Vec<AssistantModelConfig> {
        let mut model_config_clone = base_configs;
        model_config_clone.push(AssistantModelConfig {
            id: 0,
            assistant_id: model_detail.model.id, // 使用正确的字段
            assistant_model_id: model_detail.model.id,
            name: "model".to_string(),
            value: Some(model_detail.model.code.clone()),
            value_type: "string".to_string(),
        });

        if let Some(override_configs) = override_configs {
            for (key, value) in override_configs {
                let value_type = match &value {
                    serde_json::Value::String(_) => "string",
                    serde_json::Value::Number(_) => "number",
                    serde_json::Value::Bool(_) => "boolean",
                    serde_json::Value::Array(_) => "array",
                    serde_json::Value::Object(_) => "object",
                    serde_json::Value::Null => "null",
                };

                let value_str = match value {
                    serde_json::Value::String(s) => s,
                    other => other.to_string(),
                };

                // 查找是否已存在该配置
                if let Some(existing_config) = model_config_clone.iter_mut().find(|c| c.name == key)
                {
                    existing_config.value = Some(value_str);
                    existing_config.value_type = value_type.to_string();
                } else {
                    // 添加新配置
                    model_config_clone.push(AssistantModelConfig {
                        id: 0,
                        assistant_id: model_detail.model.id,
                        assistant_model_id: model_detail.model.id,
                        name: key,
                        value: Some(value_str),
                        value_type: value_type.to_string(),
                    });
                }
            }
        }

        model_config_clone
    }
}

/// 将消息列表转换为 ChatMessage 格式，并处理多媒体附件
/// 过滤掉推理类型的消息，只保留对话相关的消息
fn build_chat_messages(
    init_message_list: &[(String, String, Vec<MessageAttachment>)],
) -> Vec<ChatMessage> {
    let mut chat_messages = Vec::new();

    for (message_type, content, attachments) in init_message_list {
        // 过滤掉推理类型的消息
        if message_type == "reasoning" {
            continue;
        }
        
        // 将 response 类型转换为 assistant 角色
        let role = if message_type == "response" {
            "assistant"
        } else {
            message_type.as_str()
        };
        
        // 如果没有附件，使用简单的文本消息
        if attachments.is_empty() {
            match role {
                "system" => chat_messages.push(ChatMessage::system(content)),
                "user" => chat_messages.push(ChatMessage::user(content)),
                "assistant" => chat_messages.push(ChatMessage::assistant(content)),
                _ => {}
            }
            continue;
        }

        // 如果有附件，使用 ContentPart 来构建消息
        let mut content_parts = vec![ContentPart::from_text(content)];

        // 处理各种类型的附件
        for attachment in attachments {
            match attachment.attachment_type {
                crate::db::conversation_db::AttachmentType::Image => {
                    // 图像附件
                    if let Some(content) = &attachment.attachment_content {
                        // 解析 data URL 格式的内容，提取 MIME type 和纯 base64 内容
                        if let Some((content_type, base64_content)) = parse_data_url(content) {
                            content_parts.push(ContentPart::from_image_base64(
                                &content_type,
                                &*base64_content,
                            ));
                        }
                    } else if let Some(url) = &attachment.attachment_url {
                        // 推断图像的媒体类型
                        let media_type = infer_media_type_from_url(url);
                        content_parts.push(ContentPart::from_image_url(&media_type, url.as_str()));
                    }
                }
                crate::db::conversation_db::AttachmentType::Text => {
                    // 文本附件
                    if let Some(attachment_content) = &attachment.attachment_content {
                        let file_name = attachment.attachment_url.as_deref().unwrap_or("未知文件");
                        content_parts.push(ContentPart::from_text(&format!(
                            "\n\n[文本附件: {}]\n{}",
                            file_name, attachment_content
                        )));
                    }
                }
                crate::db::conversation_db::AttachmentType::PDF
                | crate::db::conversation_db::AttachmentType::Word
                | crate::db::conversation_db::AttachmentType::PowerPoint
                | crate::db::conversation_db::AttachmentType::Excel => {
                    // 其他文档类型，作为文本内容处理
                    if let Some(attachment_content) = &attachment.attachment_content {
                        let file_name = attachment.attachment_url.as_deref().unwrap_or("未知文档");
                        let file_type = match attachment.attachment_type {
                            crate::db::conversation_db::AttachmentType::PDF => "PDF文档",
                            crate::db::conversation_db::AttachmentType::Word => "Word文档",
                            crate::db::conversation_db::AttachmentType::PowerPoint => {
                                "PowerPoint文档"
                            }
                            crate::db::conversation_db::AttachmentType::Excel => "Excel文档",
                            _ => "文档",
                        };
                        content_parts.push(ContentPart::from_text(&format!(
                            "\n\n[{}: {}]\n{}",
                            file_type, file_name, attachment_content
                        )));
                    }
                }
            }
        }

        // 创建包含多个内容部分的消息
        match role {
            "system" => {
                // 系统消息通常不支持多媒体内容，将所有内容合并为文本
                let combined_text = content_parts
                    .iter()
                    .map(|part| {
                        // 注意：这里假设 ContentPart 有某种方式提取文本，
                        // 实际情况可能需要根据 genai 库的具体实现调整
                        match part {
                            // 这里需要根据实际的 ContentPart API 来实现
                            _ => content.clone(), // 临时处理
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("");
                chat_messages.push(ChatMessage::system(&combined_text));
            }
            "user" => {
                chat_messages.push(ChatMessage::user(content_parts));
            }
            "assistant" => {
                // 助手消息也通常是纯文本，将内容合并
                let combined_text = content_parts
                    .iter()
                    .map(|_| content.clone()) // 临时处理
                    .collect::<Vec<_>>()
                    .join("");
                chat_messages.push(ChatMessage::assistant(&combined_text));
            }
            _ => {}
        }
    }
    println!("================================ Chat Messages (Filtered) ===============================================");
    println!("{:?}", chat_messages);
    println!("================================ Chat Messages End ===============================================");
    chat_messages
}

/// 根据URL推断图像的媒体类型
fn infer_media_type_from_url(url: &str) -> String {
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
        "image/jpeg".to_string() // 默认值
    }
}

/// 解析 data URL 格式的内容，提取 MIME type 和纯 base64 内容
/// 支持格式：data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAA...
fn parse_data_url(data_url: &str) -> Option<(String, String)> {
    if !data_url.starts_with("data:") {
        return None;
    }

    let parts: Vec<&str> = data_url.splitn(2, ',').collect();
    if parts.len() != 2 {
        return None;
    }

    let header = parts[0];
    let content = parts[1];

    // 提取 MIME type
    let header_without_data = header.strip_prefix("data:")?;
    let mime_type = if let Some(semicolon_pos) = header_without_data.find(';') {
        &header_without_data[..semicolon_pos]
    } else {
        header_without_data
    };

    // 检查是否包含 base64 标识
    if !header.contains("base64") {
        return None;
    }

    Some((mime_type.to_string(), content.to_string()))
}

/// 清理消息令牌
async fn cleanup_token(
    tokens: &Arc<tokio::sync::Mutex<HashMap<i64, CancellationToken>>>,
    message_id: i64,
) {
    let mut map = tokens.lock().await;
    map.remove(&message_id);
}

/// 处理消息类型结束事件
fn handle_message_type_end(
    message_id: i64,
    message_type: &str,
    content: &str,
    start_time: chrono::DateTime<chrono::Utc>,
    conversation_db: &ConversationDatabase,
    window: &tauri::Window,
    conversation_id: i64,
) -> Result<(), anyhow::Error> {
    let end_time = chrono::Utc::now();
    let duration_ms = end_time.timestamp_millis() - start_time.timestamp_millis();
    
    // 更新数据库的finish_time
    conversation_db
        .message_repo()
        .unwrap()
        .update_finish_time(message_id)?;
    
    // 发送类型结束事件
    let type_end_event = ConversationEvent {
        r#type: "message_type_end".to_string(),
        data: serde_json::to_value(MessageTypeEndEvent {
            message_id,
            message_type: message_type.to_string(),
            duration_ms,
            end_time,
        }).unwrap(),
    };
    let _ = window.emit(
        format!("conversation_event_{}", conversation_id).as_str(),
        type_end_event
    );
    
    // 发送最终的更新事件，标记为完成
    let final_update_event = ConversationEvent {
        r#type: "message_update".to_string(),
        data: serde_json::to_value(MessageUpdateEvent {
            message_id,
            message_type: message_type.to_string(),
            content: content.to_string(),
            is_done: true,
        }).unwrap(),
    };
    let _ = window.emit(
        format!("conversation_event_{}", conversation_id).as_str(),
        final_update_event
    );
    
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AiRequest {
    conversation_id: String,
    assistant_id: i64,
    prompt: String,
    model: Option<String>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    max_tokens: Option<u32>,
    stream: Option<bool>,
    attachment_list: Option<Vec<i64>>,
}

#[derive(Serialize, Deserialize)]
pub struct AiResponse {
    conversation_id: i64,
    add_message_id: i64,
    request_prompt_result_with_context: String,
}

/// 处理流式聊天
async fn handle_stream_chat(
    client: &Client,
    model_name: &str,
    chat_request: &ChatRequest,
    chat_options: &ChatOptions,
    conversation_id: i64,
    initial_message_id: i64,
    cancel_token: &CancellationToken,
    conversation_db: &ConversationDatabase,
    tokens: &Arc<tokio::sync::Mutex<HashMap<i64, CancellationToken>>>,
    window: &tauri::Window,
    app_handle: &tauri::AppHandle,
    need_generate_title: bool,
    user_prompt: String,
    config_feature_map: HashMap<String, HashMap<String, FeatureConfig>>,
    generation_group_id_override: Option<String>, // 新增参数，用于regenerate时复用group_id
    parent_group_id_override: Option<String>, // 新增参数，用于设置parent_group_id
    llm_model_id: i64, // 模型ID
    llm_model_name: String, // 模型名称
) -> Result<(), anyhow::Error> {
    // 添加重试逻辑
    let mut attempts = 0;
    let chat_stream_result = loop {
        match client
            .exec_chat_stream(model_name, chat_request.clone(), Some(chat_options))
            .await
        {
            Ok(response) => break Ok(response),
            Err(e) => {
                attempts += 1;
                if attempts > MAX_RETRY_ATTEMPTS {
                    eprintln!("Chat stream failed after {} attempts: {}", attempts, e);
                    break Err(e);
                }
                eprintln!("Chat stream attempt {} failed: {}, retrying...", attempts, e);
                sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
            }
        }
    };

    match chat_stream_result {
        Ok(chat_stream_response) => {
            let mut chat_stream = chat_stream_response.stream;
            let mut reasoning_content = String::new();
            let mut response_content = String::new();
            let mut reasoning_message_id: Option<i64> = None;
            let mut response_message_id: Option<i64> = None;
            
            // 为这次生成确定组ID：优先使用传入的group_id，否则创建新的
            let generation_group_id = generation_group_id_override
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            
            // 检查是否为重新生成操作（如果有parent_group_id_override则为重新生成）
            let is_regeneration = parent_group_id_override.is_some();
            let mut group_merge_event_emitted = false;
            
            // 状态跟踪变量
            let mut current_output_type: Option<String> = None;
            let mut reasoning_start_time: Option<chrono::DateTime<chrono::Utc>> = None;
            let mut response_start_time: Option<chrono::DateTime<chrono::Utc>> = None;

            loop {
                tokio::select! {
                    stream_result = chat_stream.next() => {
                        match stream_result {
                            Some(Ok(stream_event)) => {
                                use genai::chat::ChatStreamEvent;
                                match stream_event {
                                    ChatStreamEvent::Start => {
                                        // 流开始，暂时不做处理
                                    }
                                    ChatStreamEvent::Chunk(chunk) => {
                                        // 检查是否需要结束reasoning状态
                                        if current_output_type == Some("reasoning".to_string()) {
                                            if let (Some(msg_id), Some(start_time)) = (reasoning_message_id, reasoning_start_time) {
                                                handle_message_type_end(
                                                    msg_id,
                                                    "reasoning",
                                                    &reasoning_content,
                                                    start_time,
                                                    &conversation_db,
                                                    &window,
                                                    conversation_id,
                                                ).unwrap_or_else(|e| {
                                                    eprintln!("Failed to handle reasoning type end: {}", e);
                                                });
                                            }
                                        }
                                        
                                        // 切换到response状态
                                        if current_output_type != Some("response".to_string()) {
                                            current_output_type = Some("response".to_string());
                                        }
                                        
                                        // 处理正常回答内容
                                        response_content.push_str(&chunk.content);
                                        
                                        // 如果还没有创建 response 消息，创建一个
                                        if response_message_id.is_none() {
                                            let now = chrono::Utc::now();
                                            response_start_time = Some(now);
                                            
                                            let new_message = conversation_db
                                                .message_repo()
                                                .unwrap()
                                                .create(&Message {
                                                    id: 0,
                                                    parent_id: None,
                                                    conversation_id,
                                                    message_type: "response".to_string(),
                                                    content: response_content.clone(),
                                                    llm_model_id: Some(llm_model_id),
                                                    llm_model_name: Some(llm_model_name.clone()),
                                                    created_time: now,
                                                    start_time: Some(now),
                                                    finish_time: None,
                                                    token_count: 0,
                                                    generation_group_id: Some(generation_group_id.clone()),
                                                    parent_group_id: parent_group_id_override.clone(),
                                                })
                                                .unwrap();
                                            response_message_id = Some(new_message.id);
                                            
                                            // 发送消息添加事件
                                            let add_event = ConversationEvent {
                                                r#type: "message_add".to_string(),
                                                data: serde_json::to_value(MessageAddEvent {
                                                    message_id: new_message.id,
                                                    message_type: "response".to_string(),
                                                    temp_message_id: initial_message_id,
                                                }).unwrap(),
                                            };
                                            let _ = window.emit(
                                                format!("conversation_event_{}", conversation_id).as_str(),
                                                add_event
                                            );
                                            
                                            // 如果是重新生成并且还没有发送组合并事件，则发送
                                            if is_regeneration && !group_merge_event_emitted {
                                                if let Some(ref parent_group_id) = parent_group_id_override {
                                                    let group_merge_event = serde_json::json!({
                                                        "type": "group_merge",
                                                        "data": {
                                                            "original_group_id": parent_group_id,
                                                            "new_group_id": generation_group_id.clone(),
                                                            "is_regeneration": true,
                                                            "first_message_id": new_message.id,
                                                            "conversation_id": conversation_id
                                                        }
                                                    });
                                                    let _ = window.emit(
                                                        format!("conversation_event_{}", conversation_id).as_str(),
                                                        group_merge_event
                                                    );
                                                    group_merge_event_emitted = true;
                                                }
                                            }
                                        }
                                        
                                        // 更新消息内容
                                        if let Some(msg_id) = response_message_id {
                                            let mut message = conversation_db
                                                .message_repo()
                                                .unwrap()
                                                .read(msg_id)
                                                .unwrap()
                                                .unwrap();
                                            message.content = response_content.clone();
                                            conversation_db
                                                .message_repo()
                                                .unwrap()
                                                .update(&message)
                                                .unwrap();
                                            
                                            // 发送消息更新事件
                                            let update_event = ConversationEvent {
                                                r#type: "message_update".to_string(),
                                                data: serde_json::to_value(MessageUpdateEvent {
                                                    message_id: msg_id,
                                                    message_type: "response".to_string(),
                                                    content: response_content.clone(),
                                                    is_done: false,
                                                }).unwrap(),
                                            };
                                            let _ = window.emit(
                                                format!("conversation_event_{}", conversation_id).as_str(),
                                                update_event
                                            );
                                        }
                                    }
                                    ChatStreamEvent::ReasoningChunk(reasoning_chunk) => {
                                        // 切换到reasoning状态
                                        if current_output_type != Some("reasoning".to_string()) {
                                            current_output_type = Some("reasoning".to_string());
                                        }
                                        
                                        // 处理推理内容
                                        reasoning_content.push_str(&reasoning_chunk.content);
                                        
                                        // 如果还没有创建 reasoning 消息，创建一个
                                        if reasoning_message_id.is_none() {
                                            let now = chrono::Utc::now();
                                            reasoning_start_time = Some(now);
                                            
                                            let new_message = conversation_db
                                                .message_repo()
                                                .unwrap()
                                                .create(&Message {
                                                    id: 0,
                                                    parent_id: None,
                                                    conversation_id,
                                                    message_type: "reasoning".to_string(),
                                                    content: reasoning_content.clone(),
                                                    llm_model_id: Some(llm_model_id),
                                                    llm_model_name: Some(llm_model_name.clone()),
                                                    created_time: now,
                                                    start_time: Some(now),
                                                    finish_time: None,
                                                    token_count: 0,
                                                    generation_group_id: Some(generation_group_id.clone()),
                                                    parent_group_id: parent_group_id_override.clone(),
                                                })
                                                .unwrap();
                                            reasoning_message_id = Some(new_message.id);
                                            
                                            // 发送消息添加事件
                                            let add_event = ConversationEvent {
                                                r#type: "message_add".to_string(),
                                                data: serde_json::to_value(MessageAddEvent {
                                                    message_id: new_message.id,
                                                    message_type: "reasoning".to_string(),
                                                    temp_message_id: initial_message_id,
                                                }).unwrap(),
                                            };
                                            let _ = window.emit(
                                                format!("conversation_event_{}", conversation_id).as_str(),
                                                add_event
                                            );
                                            
                                            // 如果是重新生成并且还没有发送组合并事件，则发送
                                            if is_regeneration && !group_merge_event_emitted {
                                                if let Some(ref parent_group_id) = parent_group_id_override {
                                                    let group_merge_event = serde_json::json!({
                                                        "type": "group_merge",
                                                        "data": {
                                                            "original_group_id": parent_group_id,
                                                            "new_group_id": generation_group_id.clone(),
                                                            "is_regeneration": true,
                                                            "first_message_id": new_message.id,
                                                            "conversation_id": conversation_id
                                                        }
                                                    });
                                                    let _ = window.emit(
                                                        format!("conversation_event_{}", conversation_id).as_str(),
                                                        group_merge_event
                                                    );
                                                    group_merge_event_emitted = true;
                                                }
                                            }
                                        }
                                        
                                        // 更新消息内容
                                        if let Some(msg_id) = reasoning_message_id {
                                            let mut message = conversation_db
                                                .message_repo()
                                                .unwrap()
                                                .read(msg_id)
                                                .unwrap()
                                                .unwrap();
                                            message.content = reasoning_content.clone();
                                            conversation_db
                                                .message_repo()
                                                .unwrap()
                                                .update(&message)
                                                .unwrap();
                                            
                                            // 发送消息更新事件
                                            let update_event = ConversationEvent {
                                                r#type: "message_update".to_string(),
                                                data: serde_json::to_value(MessageUpdateEvent {
                                                    message_id: msg_id,
                                                    message_type: "reasoning".to_string(),
                                                    content: reasoning_content.clone(),
                                                    is_done: false,
                                                }).unwrap(),
                                            };
                                            let _ = window.emit(
                                                format!("conversation_event_{}", conversation_id).as_str(),
                                                update_event
                                            );
                                        }
                                    }
                                    ChatStreamEvent::ToolCallChunk(_tool_chunk) => {
                                        // 工具调用块，暂时忽略
                                    }
                                    ChatStreamEvent::End(_stream_end) => {
                                        // 流结束，处理当前活跃的消息类型
                                        match current_output_type.as_deref() {
                                            Some("reasoning") => {
                                                if let (Some(msg_id), Some(start_time)) = (reasoning_message_id, reasoning_start_time) {
                                                    handle_message_type_end(
                                                        msg_id,
                                                        "reasoning",
                                                        &reasoning_content,
                                                        start_time,
                                                        &conversation_db,
                                                        &window,
                                                        conversation_id,
                                                    ).unwrap_or_else(|e| {
                                                        eprintln!("Failed to handle reasoning type end: {}", e);
                                                    });
                                                }
                                            }
                                            Some("response") => {
                                                if let (Some(msg_id), Some(start_time)) = (response_message_id, response_start_time) {
                                                    handle_message_type_end(
                                                        msg_id,
                                                        "response",
                                                        &response_content,
                                                        start_time,
                                                        &conversation_db,
                                                        &window,
                                                        conversation_id,
                                                    ).unwrap_or_else(|e| {
                                                        eprintln!("Failed to handle response type end: {}", e);
                                                    });
                                                    
                                                    // 在 response 完成后自动生成标题
                                                    if need_generate_title && !response_content.is_empty() {
                                                        let app_handle_clone = app_handle.clone();
                                                        let user_prompt_clone = user_prompt.clone();
                                                        let response_content_clone = response_content.clone();
                                                        let config_feature_map_clone = config_feature_map.clone();
                                                        let window_clone = window.clone();
                                                        
                                                        tokio::spawn(async move {
                                                            if let Err(e) = generate_title(
                                                                &app_handle_clone,
                                                                conversation_id,
                                                                user_prompt_clone,
                                                                response_content_clone,
                                                                config_feature_map_clone,
                                                                window_clone,
                                                            ).await {
                                                                eprintln!("Failed to generate title: {}", e);
                                                            }
                                                        });
                                                    }
                                                }
                                            }
                                            _ => {
                                                // 没有活跃的消息类型，使用原有逻辑
                                                return finish_stream_messages(
                                                    &conversation_db,
                                                    reasoning_message_id,
                                                    response_message_id,
                                                    &reasoning_content,
                                                    &response_content,
                                                    &window,
                                                    conversation_id,
                                                );
                                            }
                                        }
                                        return Ok(());
                                    }
                                }
                            },
                            Some(Err(e)) => {
                                eprintln!("Stream error: {}", e);
                                cleanup_token(tokens, initial_message_id).await;
                                let err_msg = format!("Chat stream error: {}", e);
                                
                                // 发送错误事件到任一存在的消息
                                if let Some(msg_id) = response_message_id.or(reasoning_message_id) {
                                    let error_event = ConversationEvent {
                                        r#type: "message_update".to_string(),
                                        data: serde_json::to_value(MessageUpdateEvent {
                                            message_id: msg_id,
                                            message_type: "error".to_string(),
                                            content: err_msg,
                                            is_done: true,
                                        }).unwrap(),
                                    };
                                    let _ = window.emit(
                                        format!("conversation_event_{}", conversation_id).as_str(),
                                        error_event
                                    );
                                }
                                
                                return Err(anyhow::anyhow!("Stream error: {}", e));
                            },
                            None => {
                                // 流结束，处理当前活跃的消息类型
                                match current_output_type.as_deref() {
                                    Some("reasoning") => {
                                        if let (Some(msg_id), Some(start_time)) = (reasoning_message_id, reasoning_start_time) {
                                            handle_message_type_end(
                                                msg_id,
                                                "reasoning",
                                                &reasoning_content,
                                                start_time,
                                                &conversation_db,
                                                &window,
                                                conversation_id,
                                            ).unwrap_or_else(|e| {
                                                eprintln!("Failed to handle reasoning type end: {}", e);
                                            });
                                        }
                                    }
                                    Some("response") => {
                                        if let (Some(msg_id), Some(start_time)) = (response_message_id, response_start_time) {
                                            handle_message_type_end(
                                                msg_id,
                                                "response",
                                                &response_content,
                                                start_time,
                                                &conversation_db,
                                                &window,
                                                conversation_id,
                                            ).unwrap_or_else(|e| {
                                                eprintln!("Failed to handle response type end: {}", e);
                                            });
                                            
                                            // 在 response 完成后自动生成标题
                                            if need_generate_title && !response_content.is_empty() {
                                                let app_handle_clone = app_handle.clone();
                                                let user_prompt_clone = user_prompt.clone();
                                                let response_content_clone = response_content.clone();
                                                let config_feature_map_clone = config_feature_map.clone();
                                                let window_clone = window.clone();
                                                
                                                tokio::spawn(async move {
                                                    if let Err(e) = generate_title(
                                                        &app_handle_clone,
                                                        conversation_id,
                                                        user_prompt_clone,
                                                        response_content_clone,
                                                        config_feature_map_clone,
                                                        window_clone,
                                                    ).await {
                                                        eprintln!("Failed to generate title: {}", e);
                                                    }
                                                });
                                            }
                                        }
                                    }
                                    _ => {
                                        // 没有活跃的消息类型，使用原有逻辑
                                        return finish_stream_messages(
                                            &conversation_db,
                                            reasoning_message_id,
                                            response_message_id,
                                            &reasoning_content,
                                            &response_content,
                                            &window,
                                            conversation_id,
                                        );
                                    }
                                }
                                return Ok(());
                            }
                        }
                    }
                    _ = cancel_token.cancelled() => {
                        println!("Chat stream cancelled");
                        return Ok(());
                    }
                }
            }
        }
        Err(e) => {
            cleanup_token(tokens, initial_message_id).await;
            let err_msg = format!("Chat stream error: {}", e);
            
            // 发送错误事件
            let error_event = ConversationEvent {
                r#type: "message_update".to_string(),
                data: serde_json::to_value(MessageUpdateEvent {
                    message_id: initial_message_id,
                    message_type: "error".to_string(),
                    content: err_msg,
                    is_done: true,
                }).unwrap(),
            };
            let _ = window.emit(
                format!("conversation_event_{}", conversation_id).as_str(),
                error_event
            );
            eprintln!("Chat stream error: {}", e);
            return Err(anyhow::anyhow!("Chat stream error: {}", e));
        }
    }
}

/// 处理非流式聊天
async fn handle_non_stream_chat(
    client: &Client,
    model_name: &str,
    chat_request: &ChatRequest,
    chat_options: &ChatOptions,
    conversation_id: i64,
    initial_message_id: i64,
    cancel_token: &CancellationToken,
    conversation_db: &ConversationDatabase,
    tokens: &Arc<tokio::sync::Mutex<HashMap<i64, CancellationToken>>>,
    window: &tauri::Window,
    app_handle: &tauri::AppHandle,
    need_generate_title: bool,
    user_prompt: String,
    config_feature_map: HashMap<String, HashMap<String, FeatureConfig>>,
    generation_group_id_override: Option<String>, // 新增参数，用于regenerate时复用group_id
    parent_group_id_override: Option<String>, // 新增参数，用于设置parent_group_id
    llm_model_id: i64, // 模型ID
    llm_model_name: String, // 模型名称
) -> Result<(), anyhow::Error> {
    // 为这次生成确定组ID：优先使用传入的group_id，否则创建新的
    let generation_group_id = generation_group_id_override
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    
    // 创建一个 response 类型的消息
    let response_message = conversation_db
        .message_repo()
        .unwrap()
        .create(&Message {
            id: 0,
            parent_id: None,
            conversation_id,
            message_type: "response".to_string(),
            content: String::new(),
            llm_model_id: Some(llm_model_id),
            llm_model_name: Some(llm_model_name.clone()),
            created_time: chrono::Utc::now(),
            start_time: Some(chrono::Utc::now()),
            finish_time: None,
            token_count: 0,
            generation_group_id: Some(generation_group_id),
            parent_group_id: parent_group_id_override,
        })
        .unwrap();
    let response_message_id = response_message.id;

    // 发送消息添加事件
    let add_event = ConversationEvent {
        r#type: "message_add".to_string(),
        data: serde_json::to_value(MessageAddEvent {
            message_id: response_message_id,
            message_type: "response".to_string(),
            temp_message_id: initial_message_id,
        }).unwrap(),
    };
    let _ = window.emit(
        format!("conversation_event_{}", conversation_id).as_str(),
        add_event
    );

    let chat_result = tokio::select! {
        result = async {
            // 添加重试逻辑
            let mut attempts = 0;
            loop {
                match client.exec_chat(model_name, chat_request.clone(), Some(chat_options)).await {
                    Ok(response) => break Ok(response),
                    Err(e) => {
                        attempts += 1;
                        if attempts > MAX_RETRY_ATTEMPTS {
                            eprintln!("Chat request failed after {} attempts: {}", attempts, e);
                            break Err(e);
                        }
                        eprintln!("Chat request attempt {} failed: {}, retrying...", attempts, e);
                        sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                    }
                }
            }
        } => result,
        _ = cancel_token.cancelled() => {
            cleanup_token(tokens, initial_message_id).await;
            return Err(anyhow::anyhow!("Request cancelled"));
        }
    };

    match chat_result {
        Ok(chat_response) => {
            let content = chat_response.first_text().unwrap_or("").to_string();
            println!("Chat content: {}", content.clone());

            // 更新消息内容
            let mut message = conversation_db
                .message_repo()
                .unwrap()
                .read(response_message_id)
                .unwrap()
                .unwrap();
            message.content = content.clone();
            conversation_db
                .message_repo()
                .unwrap()
                .update(&message)
                .unwrap();

            conversation_db
                .message_repo()
                .unwrap()
                .update_finish_time(response_message_id)
                .unwrap();
            
            // 发送消息更新事件（完成状态）
            let update_event = ConversationEvent {
                r#type: "message_update".to_string(),
                data: serde_json::to_value(MessageUpdateEvent {
                    message_id: response_message_id,
                    message_type: "response".to_string(),
                    content: content.clone(),
                    is_done: true,
                }).unwrap(),
            };
            let _ = window.emit(
                format!("conversation_event_{}", conversation_id).as_str(),
                update_event
            );
            
            // 在非流式消息完成后自动生成标题
            if need_generate_title && !content.is_empty() {
                let app_handle_clone = app_handle.clone();
                let user_prompt_clone = user_prompt.clone();
                let content_clone = content.clone();
                let config_feature_map_clone = config_feature_map.clone();
                let window_clone = window.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = generate_title(
                        &app_handle_clone,
                        conversation_id,
                        user_prompt_clone,
                        content_clone,
                        config_feature_map_clone,
                        window_clone,
                    ).await {
                        eprintln!("Failed to generate title: {}", e);
                    }
                });
            }
            
            Ok(())
        }
        Err(e) => {
            cleanup_token(tokens, initial_message_id).await;
            let err_msg = format!("Chat error: {}", e);
            
            // 发送错误事件
            let error_event = ConversationEvent {
                r#type: "message_update".to_string(),
                data: serde_json::to_value(MessageUpdateEvent {
                    message_id: response_message_id,
                    message_type: "error".to_string(),
                    content: err_msg,
                    is_done: true,
                }).unwrap(),
            };
            let _ = window.emit(
                format!("conversation_event_{}", conversation_id).as_str(),
                error_event
            );
            
            eprintln!("Chat error: {}", e);
            Err(anyhow::anyhow!("Chat error: {}", e))
        }
    }
}


#[tauri::command]
pub async fn ask_ai(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    feature_config_state: State<'_, FeatureConfigState>,
    message_token_manager: State<'_, MessageTokenManager>,
    window: tauri::Window,
    request: AiRequest,
    override_model_config: Option<HashMap<String, serde_json::Value>>,
    override_prompt: Option<String>,
) -> Result<AiResponse, AppError> {
    println!("================================ Ask AI Start ===============================================");
    println!(
        "ask_ai: {:?}, override_model_config: {:?}, override_prompt: {:?}",
        request, override_model_config, override_prompt
    );
    let template_engine = TemplateEngine::new();
    let mut template_context = HashMap::new();

    let selected_text = state.inner().selected_text.lock().await.clone();
    template_context.insert("selected_text".to_string(), selected_text);

    let app_handle_clone = app_handle.clone();
    let assistant_detail = get_assistant(app_handle_clone, request.assistant_id).unwrap();
    let assistant_prompt_origin = &assistant_detail.prompts[0].prompt;
    let assistant_prompt_result = template_engine
        .parse(&assistant_prompt_origin, &template_context)
        .await;
    println!("assistant_prompt_result: {}", assistant_prompt_result);

    if assistant_detail.model.is_empty() {
        return Err(AppError::NoModelFound);
    }

    let _need_generate_title = request.conversation_id.is_empty();
    let request_prompt_result = template_engine
        .parse(&request.prompt, &template_context)
        .await;

    let app_handle_clone = app_handle.clone();
    let (conversation_id, _new_message_id, request_prompt_result_with_context, init_message_list) =
        initialize_conversation(
            &app_handle_clone,
            &request,
            &assistant_detail,
            assistant_prompt_result,
            request_prompt_result.clone(),
            override_prompt.clone(),
        )
        .await?;

    // 总是启动流式处理，即使没有预先创建消息
    let _config_feature_map = feature_config_state.config_feature_map.lock().await.clone();
    let _request_prompt_result_with_context_clone = request_prompt_result_with_context.clone();

    let app_handle_clone = app_handle.clone();

    let cancel_token = CancellationToken::new();
    // 使用一个临时的 message_id，在流式处理中会被动态创建的消息替换
    let temp_message_id = chrono::Utc::now().timestamp_millis();
    message_token_manager
        .store_token(temp_message_id, cancel_token.clone())
        .await;

        // 在异步任务外获取模型详情（避免线程安全问题）
        let llm_db = LLMDatabase::new(&app_handle).map_err(AppError::from)?;
        let provider_id = &assistant_detail.model[0].provider_id;
        let model_code = &assistant_detail.model[0].model_code;
        let model_detail = llm_db
            .get_llm_model_detail(provider_id, model_code)
            .context("Failed to get LLM model detail")?;

        let tokens = message_token_manager.get_tokens();
        let window_clone = window.clone(); // 在移动之前克隆
        let model_id = model_detail.model.id; // 提前获取模型ID
        let model_code = model_detail.model.code.clone(); // 提前获取模型代码
        let model_configs = model_detail.configs.clone(); // 提前获取模型配置
        let provider_api_type = model_detail.provider.api_type.clone(); // 提前获取API类型
        let assistant_model_configs = assistant_detail.model_configs.clone(); // 提前获取助手模型配置
        tokio::spawn(async move {
            // 直接创建数据库连接（避免线程安全问题）
            let conversation_db = ConversationDatabase::new(&app_handle_clone).unwrap();

            // 构建聊天配置
            let client = genai_client::create_client_with_config(
                &model_configs,
                &model_code,
                &provider_api_type,
            )?;

            // 创建一个临时的 ModelDetail 用于配置合并
            let temp_model_detail = crate::db::llm_db::ModelDetail {
                model: crate::db::llm_db::LLMModel {
                    id: model_id,
                    name: model_code.clone(),
                    code: model_code.clone(),
                    llm_provider_id: 0, // 临时值
                    description: String::new(), // 临时值
                    vision_support: false, // 临时值
                    audio_support: false, // 临时值
                    video_support: false, // 临时值
                },
                provider: crate::db::llm_db::LLMProvider {
                    id: 0, // 临时值
                    name: String::new(), // 临时值
                    api_type: provider_api_type.clone(),
                    description: String::new(), // 临时值
                    is_official: false, // 临时值
                    is_enabled: true, // 临时值
                },
                configs: model_configs.clone(),
            };

            let model_config_clone = ConfigBuilder::merge_model_configs(
                assistant_model_configs,
                &temp_model_detail,
                override_model_config,
            );

            let config_map = model_config_clone
                .iter()
                .filter_map(|config| {
                    config
                        .value
                        .as_ref()
                        .map(|value| (config.name.clone(), value.clone()))
                })
                .collect::<HashMap<String, String>>();

            let stream = config_map
                .get("stream")
                .and_then(|v| v.parse().ok())
                .unwrap_or(false);

            let model_name = config_map
                .get("model")
                .cloned()
                .unwrap_or_else(|| model_code.clone());

            let chat_options = ConfigBuilder::build_chat_options(&config_map);

            let chat_config = ChatConfig {
                model_name,
                stream,
                chat_options: chat_options.with_normalize_reasoning_content(true),
                client,
            };

            println!(
                "Using model: {}, stream: {}",
                chat_config.model_name, chat_config.stream
            );

            // Convert messages to ChatMessage format
            let chat_messages = build_chat_messages(&init_message_list);
            let chat_request = ChatRequest::new(chat_messages);

            if chat_config.stream {
                // 使用 genai 流式处理
                handle_stream_chat(
                    &chat_config.client,
                    &chat_config.model_name,
                    &chat_request,
                    &chat_config.chat_options,
                    conversation_id,
                    temp_message_id,
                    &cancel_token,
                    &conversation_db,
                    &tokens,
                    &window_clone,
                    &app_handle_clone,
                    _need_generate_title,
                    request.prompt.clone(),
                    _config_feature_map.clone(),
                    None, // 普通ask_ai不需要复用generation_group_id
                    None, // 普通ask_ai不需要parent_group_id
                    model_id, // 传递模型ID
                    model_code.clone(), // 传递模型名称
                )
                .await?;
            } else {
                // Use genai non-streaming
                handle_non_stream_chat(
                    &chat_config.client,
                    &chat_config.model_name,
                    &chat_request,
                    &chat_config.chat_options,
                    conversation_id,
                    temp_message_id,
                    &cancel_token,
                    &conversation_db,
                    &tokens,
                    &window_clone,
                    &app_handle_clone,
                    _need_generate_title,
                    request.prompt.clone(),
                    _config_feature_map.clone(),
                    None, // 普通ask_ai不需要复用generation_group_id
                    None, // 普通ask_ai不需要parent_group_id
                    model_id, // 传递模型ID
                    model_code.clone(), // 传递模型名称
                )
                .await?;
            }

            Ok::<(), Error>(())
        });


    println!("================================ Ask AI End ===============================================");

    Ok(AiResponse {
        conversation_id,
        add_message_id: temp_message_id, // 使用临时 message_id 作为返回值
        request_prompt_result_with_context,
    })
}

#[tauri::command]
pub async fn cancel_ai(
    message_token_manager: State<'_, MessageTokenManager>,
    message_id: i64,
) -> Result<(), String> {
    message_token_manager.cancel_request(message_id).await;
    Ok(())
}

fn init_conversation(
    app_handle: &tauri::AppHandle,
    assistant_id: i64,
    llm_model_id: i64,
    llm_model_code: String,
    messages: &Vec<(String, String, Vec<MessageAttachment>)>,
) -> Result<(Conversation, Vec<Message>), AppError> {
    let db = ConversationDatabase::new(app_handle).map_err(AppError::from)?;
    println!("init_conversation !{:?}", assistant_id);
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
                generation_group_id: None, // 初始化消息不需要 generation_group_id
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

#[tauri::command]
pub async fn regenerate_ai(
    app_handle: tauri::AppHandle,
    message_token_manager: State<'_, MessageTokenManager>,
    window: tauri::Window,
    message_id: i64,
) -> Result<AiResponse, AppError> {
    println!("================================ Regenerate AI Start ===============================================");
    let db = ConversationDatabase::new(&app_handle).map_err(AppError::from)?;
    let message = db
        .message_repo()
        .unwrap()
        .read(message_id)?
        .ok_or(AppError::DatabaseError("未找到消息".to_string()))?;

    let conversation_id = message.conversation_id;
    let conversation = db
        .conversation_repo()
        .unwrap()
        .read(conversation_id)?
        .ok_or(AppError::DatabaseError("未找到对话".to_string()))?;
    let messages = db
        .message_repo()
        .unwrap()
        .list_by_conversation_id(conversation_id)?;

    // 根据消息类型决定处理逻辑
    let (filtered_messages, _parent_message_id) = if message.message_type == "user" {
        // 用户消息重发：包含当前用户消息和之前的所有消息，新生成的assistant消息没有parent（新一轮对话）
        let filtered_messages: Vec<(Message, Option<MessageAttachment>)> = messages
            .into_iter()
            .filter(|m| m.0.id <= message_id)  // 包含当前消息
            .collect();
        (filtered_messages, None) // 用户消息重发时，新的AI回复没有parent_id
    } else {
        // AI消息重新生成：仅保留在待重新生成消息之前的历史消息，新消息以被重发的原消息为parent
        let filtered_messages: Vec<(Message, Option<MessageAttachment>)> = messages
            .into_iter()
            .filter(|m| m.0.id < message_id)
            .collect();
        (filtered_messages, Some(message_id)) // 使用被重发消息的ID作为parent_id表示这是它的一个版本
    };

    // 计算每个父消息最新的子消息（parent_id -> latest child）
    let mut latest_children: HashMap<i64, (Message, Option<MessageAttachment>)> = HashMap::new();
    let mut child_ids: HashSet<i64> = HashSet::new();

    for (msg, attach) in filtered_messages.iter() {
        if let Some(parent_id) = msg.parent_id {
            child_ids.insert(msg.id);
            latest_children
                .entry(parent_id)
                .and_modify(|e| {
                    if msg.id > e.0.id {
                        *e = (msg.clone(), attach.clone());
                    }
                })
                .or_insert((msg.clone(), attach.clone()));
        }
    }

    // 构建最终的消息列表：
    //    - 对于没有子消息的根消息(包括 system / user / assistant)，直接保留
    //    - 对于有子消息的根消息，仅保留最新的子消息
    let mut init_message_list: Vec<(String, String, Vec<MessageAttachment>)> = Vec::new();

    for (msg, attach) in filtered_messages.into_iter() {
        if child_ids.contains(&msg.id) {
            // 根消息，有子消息，后续处理
            continue;
        }

        // 使用最新的子消息（如果存在）替换当前消息
        let (final_msg, final_attach_opt) = latest_children
            .get(&msg.id)
            .cloned()
            .unwrap_or((msg, attach));

        let attachments_vec = final_attach_opt.map(|a| vec![a]).unwrap_or_else(Vec::new);

        init_message_list.push((final_msg.message_type, final_msg.content, attachments_vec));
    }

    println!("init_message_list (regenerate): {:?}", init_message_list);

    // 获取助手信息（在构建消息列表之后，以确保对话已确定）
    let assistant_id = conversation.assistant_id.unwrap();
    let assistant_detail = get_assistant(app_handle.clone(), assistant_id).unwrap();

    if assistant_detail.model.is_empty() {
        return Err(AppError::NoModelFound);
    }

    // 确定要使用的generation_group_id和parent_group_id
    let (regenerate_generation_group_id, regenerate_parent_group_id) = if message.message_type == "user" {
        // 用户消息重发：为新的AI回复生成全新的group_id
        // 查找该user message后面第一条非user、非system的消息，用它的generation_group_id作为parent_group_id
        let mut parent_group_id: Option<String> = None;
        
        // 获取对话中的所有消息，按ID排序
        let all_messages = db
            .message_repo()
            .unwrap()
            .list_by_conversation_id(conversation_id)?;
        
        // 找到当前user message在列表中的位置
        if let Some(message_index) = all_messages.iter().position(|(msg, _)| msg.id == message_id) {
            // 查找该user message后面第一条非user、非system的消息
            for (next_msg, _) in all_messages.iter().skip(message_index + 1) {
                if next_msg.message_type != "user" && 
                   next_msg.message_type != "system" && 
                   next_msg.generation_group_id.is_some() {
                    parent_group_id = next_msg.generation_group_id.clone();
                    println!("Found parent group ID for user message regenerate: {:?}", parent_group_id);
                    break;
                }
            }
        }
        
        (Some(uuid::Uuid::new_v4().to_string()), parent_group_id)
    } else {
        // AI消息重发：生成新的group_id，并将原消息的group_id作为parent_group_id
        let original_group_id = message.generation_group_id.clone();
        (Some(uuid::Uuid::new_v4().to_string()), original_group_id)
    };

    // 使用临时ID用于取消令牌管理，实际消息将在流式处理中动态创建
    let temp_message_id = chrono::Utc::now().timestamp_millis();
    
    let cancel_token = CancellationToken::new();
    message_token_manager
        .store_token(temp_message_id, cancel_token.clone())
        .await;

    // 在异步任务外获取模型详情（避免线程安全问题）
    let llm_db = LLMDatabase::new(&app_handle).map_err(AppError::from)?;
    let provider_id = &assistant_detail.model[0].provider_id;
    let model_code = &assistant_detail.model[0].model_code;
    let model_detail = llm_db
        .get_llm_model_detail(provider_id, model_code)
        .context("Failed to get LLM model detail")?;

    let tokens = message_token_manager.get_tokens();
    let window_clone = window.clone(); // 在移动之前克隆
    let app_handle_clone = app_handle.clone(); // 添加这行
    let regenerate_model_id = model_detail.model.id; // 提前获取模型ID
    let regenerate_model_code = model_detail.model.code.clone(); // 提前获取模型代码
    let regenerate_model_configs = model_detail.configs.clone(); // 提前获取模型配置
    let regenerate_provider_api_type = model_detail.provider.api_type.clone(); // 提前获取API类型
    let regenerate_assistant_model_configs = assistant_detail.model_configs.clone(); // 提前获取助手模型配置
    tokio::spawn(async move {
        // 直接创建数据库连接（避免线程安全问题）
        let conversation_db = ConversationDatabase::new(&app_handle_clone).unwrap();

        // 构建聊天配置
        let client = genai_client::create_client_with_config(
            &regenerate_model_configs,
            &regenerate_model_code,
            &regenerate_provider_api_type,
        )?;

        // 创建一个临时的 ModelDetail 用于配置合并
        let temp_model_detail = crate::db::llm_db::ModelDetail {
            model: crate::db::llm_db::LLMModel {
                id: regenerate_model_id,
                name: regenerate_model_code.clone(),
                code: regenerate_model_code.clone(),
                llm_provider_id: 0, // 临时值
                description: String::new(), // 临时值
                vision_support: false, // 临时值
                audio_support: false, // 临时值
                video_support: false, // 临时值
            },
            provider: crate::db::llm_db::LLMProvider {
                id: 0, // 临时值
                name: String::new(), // 临时值
                api_type: regenerate_provider_api_type.clone(),
                description: String::new(), // 临时值
                is_official: false, // 临时值
                is_enabled: true, // 临时值
            },
            configs: regenerate_model_configs.clone(),
        };

        let model_config_clone = ConfigBuilder::merge_model_configs(
            regenerate_assistant_model_configs,
            &temp_model_detail,
            None, // regenerate 不使用覆盖配置
        );

        let config_map = model_config_clone
            .iter()
            .filter_map(|config| {
                config
                    .value
                    .as_ref()
                    .map(|value| (config.name.clone(), value.clone()))
            })
            .collect::<HashMap<String, String>>();

        let stream = config_map
            .get("stream")
            .and_then(|v| v.parse().ok())
            .unwrap_or(false);

        let model_name = config_map
            .get("model")
            .cloned()
            .unwrap_or_else(|| regenerate_model_code.clone());

        let chat_options = ConfigBuilder::build_chat_options(&config_map);

        let chat_config = ChatConfig {
            model_name,
            stream,
            chat_options,
            client,
        };

        // Convert messages to ChatMessage format
        let chat_messages = build_chat_messages(&init_message_list);
        let chat_request = ChatRequest::new(chat_messages);

        if chat_config.stream {
            // 使用 genai 流式处理
            handle_stream_chat(
                &chat_config.client,
                &chat_config.model_name,
                &chat_request,
                &chat_config.chat_options,
                conversation_id,
                temp_message_id,
                &cancel_token,
                &conversation_db,
                &tokens,
                &window_clone,
                &app_handle_clone,
                false, // regenerate 不需要生成标题
                String::new(), // regenerate 不需要用户提示
                HashMap::new(), // regenerate 不需要配置
                regenerate_generation_group_id.clone(), // 传递generation_group_id用于复用
                regenerate_parent_group_id.clone(), // 传递parent_group_id设置版本关系
                regenerate_model_id, // 传递模型ID
                regenerate_model_code.clone(), // 传递模型名称
            )
            .await?;
        } else {
            // Use genai non-streaming
            handle_non_stream_chat(
                &chat_config.client,
                &chat_config.model_name,
                &chat_request,
                &chat_config.chat_options,
                conversation_id,
                temp_message_id,
                &cancel_token,
                &conversation_db,
                &tokens,
                &window_clone,
                &app_handle_clone,
                false, // regenerate 不需要生成标题
                String::new(), // regenerate 不需要用户提示
                HashMap::new(), // regenerate 不需要配置
                regenerate_generation_group_id.clone(), // 传递generation_group_id用于复用
                regenerate_parent_group_id.clone(), // 传递parent_group_id设置版本关系
                regenerate_model_id, // 传递模型ID
                regenerate_model_code.clone(), // 传递模型名称
            )
            .await?;
        }

        Ok::<(), Error>(())
    });

    println!("================================ Regenerate AI End ===============================================");

    Ok(AiResponse {
        conversation_id,
        add_message_id: temp_message_id,
        request_prompt_result_with_context: String::new(),
    })
}

fn add_message(
    app_handle: &tauri::AppHandle,
    parent_id: Option<i64>,
    conversation_id: i64,
    message_type: String,
    content: String,
    llm_model_id: Option<i64>,
    llm_model_name: Option<String>,
    start_time: Option<chrono::DateTime<chrono::Utc>>,
    finish_time: Option<chrono::DateTime<chrono::Utc>>,
    token_count: i32,
    generation_group_id: Option<String>,
    parent_group_id: Option<String>,
) -> Result<Message, AppError> {
    let db = ConversationDatabase::new(app_handle).map_err(AppError::from)?;
    let message = db
        .message_repo()
        .unwrap()
        .create(&Message {
            id: 0,
            parent_id,
            conversation_id,
            message_type,
            content,
            llm_model_id,
            llm_model_name,
            start_time,
            finish_time,
            created_time: chrono::Utc::now(),
            token_count,
            generation_group_id,
            parent_group_id,
        })
        .map_err(AppError::from)?;
    Ok(message.clone())
}

async fn initialize_conversation(
    app_handle: &tauri::AppHandle,
    request: &AiRequest,
    assistant_detail: &AssistantDetail,
    assistant_prompt_result: String,
    request_prompt_result: String,
    override_prompt: Option<String>,
) -> Result<
    (
        i64,
        Option<i64>,
        String,
        Vec<(String, String, Vec<MessageAttachment>)>,
    ),
    AppError,
> {
    let db = ConversationDatabase::new(app_handle).map_err(AppError::from)?;

    let (conversation_id, add_message_id, request_prompt_result_with_context, init_message_list) =
        if request.conversation_id.is_empty() {
            let message_attachment_list = db
                .attachment_repo()
                .unwrap()
                .list_by_id(&request.attachment_list.clone().unwrap_or(vec![]))?;
            // 新对话逻辑
            let text_attachments: Vec<String> = message_attachment_list
                .iter()
                .filter(|a| matches!(a.attachment_type, AttachmentType::Text))
                .filter_map(|a| {
                    Some(format!(
                        r#"<fileattachment name="{}">{}</fileattachment>"#,
                        a.attachment_url.clone().unwrap(),
                        a.attachment_content.clone().unwrap().as_str()
                    ))
                })
                .collect();
            let context = text_attachments.join("\n");
            let request_prompt_result_with_context =
                format!("{}\n{}", request_prompt_result, context);
            let init_message_list = vec![
                (
                    String::from("system"),
                    override_prompt.unwrap_or(assistant_prompt_result),
                    vec![],
                ),
                (
                    String::from("user"),
                    request_prompt_result_with_context.clone(),
                    message_attachment_list,
                ),
            ];
            println!("initialize_conversation {:?}", request.assistant_id);
            println!(
                "initialize_conversation init_message_list {:?}",
                init_message_list
            );
            let (conversation, _) = init_conversation(
                app_handle,
                request.assistant_id,
                assistant_detail.model[0].id,
                assistant_detail.model[0].model_code.clone(),
                &init_message_list,
            )?;
            (
                conversation.id,
                None, // 不预先创建空的assistant消息，让流式处理动态创建
                request_prompt_result_with_context,
                init_message_list,
            )
        } else {
            // 已存在对话逻辑
            let conversation_id = request.conversation_id.parse::<i64>()?;
            let all_messages = db
                .message_repo()
                .unwrap()
                .list_by_conversation_id(conversation_id)?;

            // 创建一个 HashMap 来存储每个消息的最新子消息
            let mut latest_children: HashMap<i64, (Message, Option<MessageAttachment>)> =
                HashMap::new();
            // 创建一个 HashSet 来存储所有作为子消息的消息 ID
            let mut child_ids: HashSet<i64> = HashSet::new();

            // 遍历所有消息，更新最新子消息和子消息 ID 集合
            for (message, attachment) in all_messages.iter() {
                if let Some(parent_id) = message.parent_id {
                    child_ids.insert(message.id);
                    latest_children
                        .entry(parent_id)
                        .and_modify(|e| *e = (message.clone(), attachment.clone()))
                        .or_insert((message.clone(), attachment.clone()));
                }
            }

            // 构建最终的消息列表
            let message_list: Vec<(String, String, Vec<MessageAttachment>)> = all_messages
                .into_iter()
                .filter(|(message, _)| !child_ids.contains(&message.id))
                .map(|(message, attachment)| {
                    let (final_message, final_attachment) = latest_children
                        .get(&message.id)
                        .map(|child| child.clone())
                        .unwrap_or((message, attachment));

                    (
                        final_message.message_type,
                        final_message.content, // 使用修改后的 content
                        final_attachment.map(|a| vec![a]).unwrap_or_else(Vec::new),
                    )
                })
                .collect();

            // 获取到消息的附件列表
            let message_attachment_list = db
                .attachment_repo()
                .unwrap()
                .list_by_id(&request.attachment_list.clone().unwrap_or(vec![]))?;
            // 过滤出文本附件
            let text_attachments: Vec<String> = message_attachment_list
                .iter()
                .filter(|a| matches!(a.attachment_type, AttachmentType::Text))
                .filter_map(|a| {
                    Some(format!(
                        r#"<fileattachment name="{}">{}</fileattachment>"#,
                        a.attachment_url.clone().unwrap(),
                        a.attachment_content.clone().unwrap().as_str()
                    ))
                })
                .collect();
            let context = text_attachments.join("\n");

            let request_prompt_result_with_context =
                format!("{}\n{}", request_prompt_result, context);
            // 添加用户消息
            let _ = add_message(
                app_handle,
                None,
                conversation_id,
                "user".to_string(),
                request_prompt_result_with_context.clone(),
                Some(assistant_detail.model[0].id),
                Some(assistant_detail.model[0].model_code.clone()),
                None,
                None,
                0,
                None, // 用户消息不需要 generation_group_id
                None, // 用户消息不需要 parent_group_id
            )?;
            let mut updated_message_list = message_list;
            updated_message_list.push((
                String::from("user"),
                request_prompt_result_with_context.clone(),
                message_attachment_list,
            ));

            (
                conversation_id,
                None, // 不预先创建空的assistant消息，让流式处理动态创建
                request_prompt_result_with_context,
                updated_message_list,
            )
        };
    Ok((
        conversation_id,
        add_message_id,
        request_prompt_result_with_context,
        init_message_list,
    ))
}

async fn generate_title(
    app_handle: &tauri::AppHandle,
    conversation_id: i64,
    user_prompt: String,
    content: String,
    config_feature_map: HashMap<String, HashMap<String, FeatureConfig>>,
    window: tauri::Window,
) -> Result<(), AppError> {
    // TODO 要检查下是否配置了对应的
    let feature_config = config_feature_map.get("conversation_summary");
    if let Some(config) = feature_config {
        // model_id, prompt, summary_length
        let provider_id = config
            .get("provider_id")
            .ok_or(AppError::NoConfigError("provider_id".to_string()))?
            .value
            .parse::<i64>()?;
        let model_code = config
            .get("model_code")
            .ok_or(AppError::NoConfigError("model_code".to_string()))?
            .value
            .clone();
        let prompt = config.get("prompt").unwrap().value.clone();
        let summary_length = config
            .get("summary_length")
            .unwrap()
            .value
            .clone()
            .parse::<i32>()
            .unwrap();
        let mut context = String::new();

        if summary_length == -1 {
            context.push_str(
                format!(
                    "# user\n {} \n\n#assistant\n {} \n\n请总结上述对话为标题，不需要包含标点符号",
                    user_prompt, content
                )
                .as_str(),
            );
        } else {
            let unsize_summary_length: usize = summary_length.try_into().unwrap();
            if user_prompt.len() > unsize_summary_length {
                context.push_str(
                    format!(
                        "# user\n {} \n\n请总结上述对话为标题，不需要包含标点符号",
                        user_prompt
                            .chars()
                            .take(unsize_summary_length)
                            .collect::<String>()
                    )
                    .as_str(),
                );
            } else {
                let assistant_summary_length = unsize_summary_length - user_prompt.len();
                if content.len() > assistant_summary_length {
                    context.push_str(format!("# user\n {} \n\n#assistant\n {} \n\n请总结上述对话为标题，不需要包含标点符号", user_prompt, content.chars().take(assistant_summary_length).collect::<String>()).as_str());
                } else {
                    context.push_str(format!("# user\n {} \n\n#assistant\n {} \n\n请总结上述对话为标题，不需要包含标点符号", user_prompt, content).as_str());
                }
            }
        }

        // 直接创建数据库连接
        let llm_db = LLMDatabase::new(app_handle).map_err(AppError::from)?;
        let model_detail = llm_db
            .get_llm_model_detail(&provider_id, &model_code)
            .unwrap();

        // Create genai client with custom config
        let client = genai_client::create_client_with_config(
            &model_detail.configs,
            &model_detail.model.code,
            &model_detail.provider.api_type,
        )?;

        // Convert messages to ChatMessage format
        let chat_messages = vec![ChatMessage::system(&prompt), ChatMessage::user(&context)];
        let chat_request = ChatRequest::new(chat_messages);

        // Use model code as model name
        let model_name = &model_detail.model.code;

        // 添加重试逻辑
        let mut attempts = 0;
        let response = loop {
            match client
                .exec_chat(model_name, chat_request.clone(), None)
                .await
            {
                Ok(chat_response) => {
                    break Ok(chat_response.first_text().unwrap_or("").to_string())
                }
                Err(e) => {
                    attempts += 1;
                    if attempts > MAX_RETRY_ATTEMPTS {
                        eprintln!("Title generation failed after {} attempts: {}", attempts, e);
                        break Err(e.to_string());
                    }
                    eprintln!("Title generation attempt {} failed: {}, retrying...", attempts, e);
                    sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                }
            }
        };
        match response {
            Err(e) => {
                println!("Chat error: {}", e);
                let _ = window.emit(ERROR_NOTIFICATION_EVENT, "生成对话标题失败，请检查配置");
            }
            Ok(response_text) => {
                println!("Chat content: {}", response_text.clone());

                let conversation_db =
                    ConversationDatabase::new(app_handle).map_err(AppError::from)?;
                let _ = conversation_db
                    .conversation_repo()
                    .unwrap()
                    .update_name(&Conversation {
                        id: conversation_id,
                        name: response_text.clone(),
                        assistant_id: None,
                        created_time: chrono::Utc::now(),
                    });
                window
                    .emit(TITLE_CHANGE_EVENT, (conversation_id, response_text.clone()))
                    .map_err(|e| e.to_string())
                    .unwrap();
            }
        }
    }

    Ok(())
}

/// 重新生成对话标题
#[tauri::command]
pub async fn regenerate_conversation_title(
    app_handle: tauri::AppHandle,
    window: tauri::Window,
    feature_config_state: State<'_, FeatureConfigState>,
    conversation_id: i64,
) -> Result<(), AppError> {
    let conversation_db = ConversationDatabase::new(&app_handle).map_err(AppError::from)?;

    // 获取对话的前两条消息（通常是用户提问和AI回答）
    let messages = conversation_db
        .message_repo()
        .unwrap()
        .list_by_conversation_id(conversation_id)?;

    if messages.len() < 2 {
        return Err(AppError::InsufficientMessages);
    }

    // 获取第一条用户消息和第一条AI回答
    let user_message = messages
        .iter()
        .find(|(msg, _)| msg.message_type == "user")
        .map(|(msg, _)| msg)
        .ok_or_else(|| AppError::InsufficientMessages)?;
    let assistant_message = messages
        .iter()
        .find(|(msg, _)| msg.message_type == "assistant")
        .map(|(msg, _)| msg)
        .ok_or_else(|| AppError::InsufficientMessages)?;

    // 获取特性配置
    let config_feature_map = feature_config_state.config_feature_map.lock().await;

    // 调用内部的 generate_title 函数
    generate_title(
        &app_handle,
        conversation_id,
        user_message.content.clone(),
        assistant_message.content.clone(),
        config_feature_map.clone(),
        window,
    )
    .await?;

    Ok(())
}

/// 完成流式消息处理的统一函数
fn finish_stream_messages(
    conversation_db: &ConversationDatabase,
    reasoning_message_id: Option<i64>,
    response_message_id: Option<i64>,
    reasoning_content: &str,
    response_content: &str,
    window: &tauri::Window,
    conversation_id: i64,
) -> Result<(), Error> {
    // 如果只有 reasoning 没有 response，则结束 reasoning
    if let Some(msg_id) = reasoning_message_id {
        if response_message_id.is_none() {
            conversation_db
                .message_repo()
                .unwrap()
                .update_finish_time(msg_id)
                .unwrap();
            
            let complete_event = ConversationEvent {
                r#type: "message_update".to_string(),
                data: serde_json::to_value(MessageUpdateEvent {
                    message_id: msg_id,
                    message_type: "reasoning".to_string(),
                    content: reasoning_content.to_string(),
                    is_done: true,
                }).unwrap(),
            };
            let _ = window.emit(
                format!("conversation_event_{}", conversation_id).as_str(),
                complete_event
            );
        }
    }
    
    // 结束 response 消息
    if let Some(msg_id) = response_message_id {
        conversation_db
            .message_repo()
            .unwrap()
            .update_finish_time(msg_id)
            .unwrap();
        
        let complete_event = ConversationEvent {
            r#type: "message_update".to_string(),
            data: serde_json::to_value(MessageUpdateEvent {
                message_id: msg_id,
                message_type: "response".to_string(),
                content: response_content.to_string(),
                is_done: true,
            }).unwrap(),
        };
        let _ = window.emit(
            format!("conversation_event_{}", conversation_id).as_str(),
            complete_event
        );
    }
    
    Ok(())
}
