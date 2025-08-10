use crate::api::ai::config::{MAX_RETRY_ATTEMPTS, RETRY_DELAY_MS};
use crate::api::ai::events::{ConversationEvent, MessageAddEvent, MessageUpdateEvent, ERROR_NOTIFICATION_EVENT};
use crate::db::conversation_db::{ConversationDatabase, Message, Repository};
use crate::db::system_db::FeatureConfig;
use anyhow::Error;
use futures::StreamExt;
use genai::chat::{ChatOptions, ChatRequest};
use genai::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tauri::Emitter;
use tokio_util::sync::CancellationToken;

pub async fn handle_stream_chat(
    client: &Client,
    model_name: &str,
    chat_request: &ChatRequest,
    chat_options: &ChatOptions,
    conversation_id: i64,
    cancel_token: &CancellationToken,
    conversation_db: &ConversationDatabase,
    tokens: &Arc<tokio::sync::Mutex<HashMap<i64, CancellationToken>>>,
    window: &tauri::Window,
    app_handle: &tauri::AppHandle,
    need_generate_title: bool,
    user_prompt: String,
    config_feature_map: HashMap<String, HashMap<String, FeatureConfig>>,
    generation_group_id_override: Option<String>,
    parent_group_id_override: Option<String>,
    llm_model_id: i64,
    llm_model_name: String,
) -> Result<(), anyhow::Error> {
    let mut attempts: u32 = 0;
    let app_handle_clone = app_handle.clone();
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
                eprintln!(
                    "Chat stream attempt {} failed: {}, retrying...",
                    attempts, e
                );
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

            let generation_group_id =
                generation_group_id_override.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            let is_regeneration = parent_group_id_override.is_some();
            let mut group_merge_event_emitted = false;

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
                                    ChatStreamEvent::Start => {}
                                    ChatStreamEvent::Chunk(chunk) => {
                                        if current_output_type == Some("reasoning".to_string()) {
                                            if let (Some(msg_id), Some(start_time)) = (reasoning_message_id, reasoning_start_time) {
                                                if let Err(e) = super::conversation::handle_message_type_end(
                                                    msg_id,
                                                    "reasoning",
                                                    &reasoning_content,
                                                    start_time,
                                                    &conversation_db,
                                                    &window,
                                                    conversation_id,
                                                    &app_handle_clone,
                                                ).await {
                                                    eprintln!("Failed to handle reasoning type end: {}", e);
                                                }
                                            }
                                        }

                                        if current_output_type != Some("response".to_string()) {
                                            current_output_type = Some("response".to_string());
                                        }

                                        response_content.push_str(&chunk.content);

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

                                            let add_event = ConversationEvent {
                                                r#type: "message_add".to_string(),
                                                data: serde_json::to_value(MessageAddEvent {
                                                    message_id: new_message.id,
                                                    message_type: "response".to_string(),
                                                }).unwrap(),
                                            };
                                            let _ = window.emit(
                                                format!("conversation_event_{}", conversation_id).as_str(),
                                                add_event
                                            );

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
                                        if current_output_type != Some("reasoning".to_string()) {
                                            current_output_type = Some("reasoning".to_string());
                                        }

                                        reasoning_content.push_str(&reasoning_chunk.content);

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
                                                    generation_group_id: None,
                                                    parent_group_id: None,
                                                })
                                                .unwrap();
                                            reasoning_message_id = Some(new_message.id);

                                            let add_event = ConversationEvent {
                                                r#type: "message_add".to_string(),
                                                data: serde_json::to_value(MessageAddEvent {
                                                    message_id: new_message.id,
                                                    message_type: "reasoning".to_string(),
                                                }).unwrap(),
                                            };
                                            let _ = window.emit(
                                                format!("conversation_event_{}", conversation_id).as_str(),
                                                add_event
                                            );
                                        }

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
                                    ChatStreamEvent::End(_) => {
                                        super::conversation::finish_stream_messages(
                                            &conversation_db,
                                            reasoning_message_id,
                                            response_message_id,
                                            &reasoning_content,
                                            &response_content,
                                            &window,
                                            conversation_id,
                                        )?;

                                        if need_generate_title && !response_content.is_empty() {
                                            let app_handle_clone = app_handle.clone();
                                            let user_prompt_clone = user_prompt.clone();
                                            let content_clone = response_content.clone();
                                            let config_feature_map_clone = config_feature_map.clone();
                                            let window_clone = window.clone();

                                            tokio::spawn(async move {
                                                if let Err(e) = crate::api::ai::title::generate_title(
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

                                        return Ok(());
                                    }
                                    // rust-genai 版本无 Error 变体，错误通过 Some(Err(_)) 分支处理
                                    _ => {}
                                }
                            }
                            Some(Err(e)) => {
                                eprintln!("Stream error: {}", e);
                                break;
                            }
                            None => break,
                        }
                    }
                    _ = cancel_token.cancelled() => {
                        super::conversation::cleanup_token(tokens, conversation_id).await;
                        return Err(anyhow::anyhow!("Request cancelled"));
                    }
                }
            }

            super::conversation::cleanup_token(tokens, conversation_id).await;
            Ok(())
        }
        Err(e) => {
            super::conversation::cleanup_token(tokens, conversation_id).await;
            let err_msg = format!("Chat error: {}", e);
            let now = chrono::Utc::now();
            let _ = window.emit(
                ERROR_NOTIFICATION_EVENT,
                format!("Chat request error: {}", e),
            );

            let generation_group_id =
                generation_group_id_override.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            let error_message = conversation_db
                .message_repo()
                .unwrap()
                .create(&Message {
                    id: 0,
                    parent_id: None,
                    conversation_id,
                    message_type: "error".to_string(),
                    content: err_msg.clone(),
                    llm_model_id: Some(llm_model_id),
                    llm_model_name: Some(llm_model_name.clone()),
                    created_time: now,
                    start_time: Some(now),
                    finish_time: Some(now),
                    token_count: 0,
                    generation_group_id: Some(generation_group_id.clone()),
                    parent_group_id: parent_group_id_override.clone(),
                })
                .unwrap();

            let error_event = ConversationEvent {
                r#type: "message_add".to_string(),
                data: serde_json::to_value(MessageAddEvent {
                    message_id: error_message.id,
                    message_type: "error".to_string(),
                })
                .unwrap(),
            };
            let _ = window.emit(
                format!("conversation_event_{}", conversation_id).as_str(),
                error_event,
            );

            let update_event = ConversationEvent {
                r#type: "message_update".to_string(),
                data: serde_json::to_value(MessageUpdateEvent {
                    message_id: error_message.id,
                    message_type: "error".to_string(),
                    content: err_msg.clone(),
                    is_done: true,
                })
                .unwrap(),
            };
            let _ = window.emit(
                format!("conversation_event_{}", conversation_id).as_str(),
                update_event,
            );

            eprintln!("Chat stream error: {}", e);
            Err(anyhow::anyhow!("Chat stream error: {}", e))
        }
    }
}

pub async fn handle_non_stream_chat(
    client: &Client,
    model_name: &str,
    chat_request: &ChatRequest,
    chat_options: &ChatOptions,
    conversation_id: i64,
    cancel_token: &CancellationToken,
    conversation_db: &ConversationDatabase,
    tokens: &Arc<tokio::sync::Mutex<HashMap<i64, CancellationToken>>>,
    window: &tauri::Window,
    app_handle: &tauri::AppHandle,
    need_generate_title: bool,
    user_prompt: String,
    config_feature_map: HashMap<String, HashMap<String, FeatureConfig>>,
    generation_group_id_override: Option<String>,
    parent_group_id_override: Option<String>,
    llm_model_id: i64,
    llm_model_name: String,
) -> Result<(), anyhow::Error> {
    let generation_group_id = generation_group_id_override
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

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
            parent_group_id: parent_group_id_override.clone(),
        })
        .unwrap();
    let response_message_id = response_message.id;

    let add_event = ConversationEvent {
        r#type: "message_add".to_string(),
        data: serde_json::to_value(MessageAddEvent {
            message_id: response_message_id,
            message_type: "response".to_string(),
        })
        .unwrap(),
    };
    let _ = window.emit(
        format!("conversation_event_{}", conversation_id).as_str(),
        add_event,
    );

    let chat_result = tokio::select! {
        result = async {
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
            super::conversation::cleanup_token(tokens, conversation_id).await;
            return Err(anyhow::anyhow!("Request cancelled"));
        }
    };

    match chat_result {
        Ok(chat_response) => {
            let content = chat_response.first_text().unwrap_or("").to_string();
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

            let update_event = ConversationEvent {
                r#type: "message_update".to_string(),
                data: serde_json::to_value(MessageUpdateEvent {
                    message_id: response_message_id,
                    message_type: "response".to_string(),
                    content: content.clone(),
                    is_done: true,
                })
                .unwrap(),
            };
            let _ = window.emit(
                format!("conversation_event_{}", conversation_id).as_str(),
                update_event,
            );

            if need_generate_title && !content.is_empty() {
                let app_handle_clone = app_handle.clone();
                let user_prompt_clone = user_prompt.clone();
                let content_clone = content.clone();
                let config_feature_map_clone = config_feature_map.clone();
                let window_clone = window.clone();

                tokio::spawn(async move {
                    if let Err(e) = crate::api::ai::title::generate_title(
                        &app_handle_clone,
                        conversation_id,
                        user_prompt_clone,
                        content_clone,
                        config_feature_map_clone,
                        window_clone,
                    )
                    .await
                    {
                        eprintln!("Failed to generate title: {}", e);
                    }
                });
            }

            Ok(())
        }
        Err(e) => {
            super::conversation::cleanup_token(tokens, conversation_id).await;
            let err_msg = format!("Chat error: {}", e);
            let now = chrono::Utc::now();
            let _ = window.emit(
                ERROR_NOTIFICATION_EVENT,
                format!("Chat request error: {}", e),
            );

            let generation_group_id = generation_group_id_override
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            let error_message = conversation_db
                .message_repo()
                .unwrap()
                .create(&Message {
                    id: 0,
                    parent_id: None,
                    conversation_id,
                    message_type: "error".to_string(),
                    content: err_msg.clone(),
                    llm_model_id: Some(llm_model_id),
                    llm_model_name: Some(llm_model_name.clone()),
                    created_time: now,
                    start_time: Some(now),
                    finish_time: Some(now),
                    token_count: 0,
                    generation_group_id: Some(generation_group_id.clone()),
                    parent_group_id: parent_group_id_override.clone(),
                })
                .unwrap();

            let error_event = ConversationEvent {
                r#type: "message_add".to_string(),
                data: serde_json::to_value(MessageAddEvent {
                    message_id: error_message.id,
                    message_type: "error".to_string(),
                })
                .unwrap(),
            };
            let _ = window.emit(
                format!("conversation_event_{}", conversation_id).as_str(),
                error_event,
            );

            let update_event = ConversationEvent {
                r#type: "message_update".to_string(),
                data: serde_json::to_value(MessageUpdateEvent {
                    message_id: error_message.id,
                    message_type: "error".to_string(),
                    content: err_msg.clone(),
                    is_done: true,
                })
                .unwrap(),
            };
            let _ = window.emit(
                format!("conversation_event_{}", conversation_id).as_str(),
                update_event,
            );

            eprintln!("Chat error: {}", e);
            Err(anyhow::anyhow!("Chat error: {}", e))
        }
    }
}


