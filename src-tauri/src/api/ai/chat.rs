use crate::api::ai::config::{calculate_retry_delay, MAX_RETRY_ATTEMPTS};
use crate::api::ai::events::{
    ConversationEvent, MessageAddEvent, MessageUpdateEvent, ERROR_NOTIFICATION_EVENT,
};
use crate::db::conversation_db::{ConversationDatabase, Message, Repository};
use crate::db::system_db::FeatureConfig;

use futures::StreamExt;
use genai::chat::{ChatOptions, ChatRequest};
use genai::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Emitter;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

// 将错误信息转换为用户友好的中文提示
fn get_user_friendly_error_message<E: std::fmt::Display>(error: &E) -> String {
    let error_str = error.to_string().to_lowercase();

    if error_str.contains("network")
        || error_str.contains("connection")
        || error_str.contains("timeout")
    {
        "网络连接异常，请检查网络设置".to_string()
    } else if error_str.contains("unauthorized") || error_str.contains("401") {
        "身份认证失败，请检查API密钥".to_string()
    } else if error_str.contains("forbidden") || error_str.contains("403") {
        "访问被拒绝，请检查API权限".to_string()
    } else if error_str.contains("not found") || error_str.contains("404") {
        "请求的服务不存在，请检查配置".to_string()
    } else if error_str.contains("rate limit") || error_str.contains("429") {
        "请求过于频繁，请稍后重试".to_string()
    } else if error_str.contains("quota") || error_str.contains("exceeded") {
        "API配额已用完，请检查账户状态".to_string()
    } else if error_str.contains("server")
        || error_str.contains("500")
        || error_str.contains("502")
        || error_str.contains("503")
    {
        "服务器暂时不可用，请稍后重试".to_string()
    } else if error_str.contains("json") || error_str.contains("parse") {
        "响应数据格式异常".to_string()
    } else {
        "请求失败，请稍后重试".to_string()
    }
}

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
    let mut main_attempts = 0;
    let app_handle_clone = app_handle.clone();

    // 外层重试循环，处理整个流式会话
    loop {
        main_attempts += 1;
        println!(
            "[[stream_chat_attempt]]: {}/{}",
            main_attempts, MAX_RETRY_ATTEMPTS
        );

        let stream_result = attempt_stream_chat(
            client,
            model_name,
            chat_request,
            chat_options,
            conversation_id,
            cancel_token,
            conversation_db,
            tokens,
            window,
            &app_handle_clone,
            need_generate_title,
            user_prompt.clone(),
            config_feature_map.clone(),
            generation_group_id_override.clone(),
            parent_group_id_override.clone(),
            llm_model_id,
            llm_model_name.clone(),
        )
        .await;

        match stream_result {
            Ok(_) => {
                println!("[[stream_chat_completed_attempt]]: {}", main_attempts);
                return Ok(());
            }
            Err(e) => {
                println!(
                    "[[stream_chat_failed_attempt]]: {} [[error]]: {}",
                    main_attempts, e
                );

                if main_attempts >= MAX_RETRY_ATTEMPTS {
                    // 最终失败，清理资源并返回错误
                    super::conversation::cleanup_token(tokens, conversation_id).await;
                    let final_error =
                        format!("AI请求失败: {}", get_user_friendly_error_message(&e));
                    eprintln!(
                        "[[final_stream_error]]: 流式聊天在{}次尝试后失败: {}",
                        main_attempts, e
                    );

                    // 发送错误通知
                    let _ = window.emit(
                        ERROR_NOTIFICATION_EVENT,
                        get_user_friendly_error_message(&e),
                    );

                    // 创建错误消息
                    create_error_message(
                        conversation_db,
                        conversation_id,
                        llm_model_id,
                        llm_model_name.clone(),
                        &final_error,
                        generation_group_id_override.clone(),
                        parent_group_id_override.clone(),
                        window,
                    )
                    .await;

                    return Err(anyhow::anyhow!("{}", final_error));
                }

                let delay = calculate_retry_delay(main_attempts);
                println!("[[retrying_stream_delay_ms]]: {}", delay);
                sleep(Duration::from_millis(delay)).await;
            }
        }
    }
}

// 单次流式聊天尝试
async fn attempt_stream_chat(
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
    // 尝试建立流式连接
    println!("[[establishing_stream_connection_model]]: {}", model_name);
    let chat_stream_response = match client
        .exec_chat_stream(model_name, chat_request.clone(), Some(chat_options))
        .await
    {
        Ok(response) => {
            println!("[[stream_connection_established]]: true");
            response
        }
        Err(e) => {
            // 打印详细的连接错误信息
            eprintln!("=== Stream Connection Error ===");
            eprintln!("[[stream_connection_failed]]: {:?}", e);
            eprintln!("[[error_details]]: {}", e);
            eprintln!("[[error_debug]]: {:#?}", e);

            // 打印完整的错误链
            eprintln!("[[error_chain]]:");
            let mut current_error: Option<&dyn std::error::Error> = Some(&e);
            let mut i = 0;
            while let Some(error) = current_error {
                eprintln!("  [[error_{}]]: {}", i, error);
                eprintln!("    [[error_type]]: {}", std::any::type_name_of_val(error));

                // 特别检查是否是 reqwest 错误
                if let Some(reqwest_error) = error.downcast_ref::<reqwest::Error>() {
                    eprintln!("    [[reqwest_error_details]]:");
                    if let Some(status) = reqwest_error.status() {
                        eprintln!("      [[status_code]]: {}", status);
                        eprintln!("      [[is_client_error]]: {}", status.is_client_error());
                        eprintln!("      [[is_server_error]]: {}", status.is_server_error());
                    }
                    if let Some(url) = reqwest_error.url() {
                        eprintln!("      [[request_url]]: {}", url);
                    }
                    eprintln!("      [[is_timeout]]: {}", reqwest_error.is_timeout());
                    eprintln!("      [[is_connect]]: {}", reqwest_error.is_connect());
                    eprintln!("      [[is_request]]: {}", reqwest_error.is_request());
                    eprintln!("      [[is_body]]: {}", reqwest_error.is_body());
                    eprintln!("      [[is_decode]]: {}", reqwest_error.is_decode());
                }

                current_error = error.source();
                i += 1;
            }
            eprintln!("=== End Stream Connection Error Details ===");

            return Err(anyhow::anyhow!(
                "Failed to establish stream connection: {}",
                e
            ));
        }
    };

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
                                            &app_handle,
                                            false, // allow MCP detection
                                        ).await {
                                            eprintln!("[[reasoning_type_end_failed]]: {}", e);
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
                            ChatStreamEvent::ToolCallChunk(tool_call) => {
                                println!("[[tool_call]]: {:#?}\n", tool_call);
                            }
                            ChatStreamEvent::End(_) => {
                                // 按当前输出类型收尾，确保 response 触发 MCP 检测与事件
                                match current_output_type.as_deref() {
                                    Some("reasoning") => {
                                        if let (Some(msg_id), Some(start_time)) = (reasoning_message_id, reasoning_start_time) {
                                            if let Err(e) = super::conversation::handle_message_type_end(
                                                msg_id,
                                                "reasoning",
                                                &reasoning_content,
                                                start_time,
                                                &conversation_db,
                                                &window,
                                                conversation_id,
                                                &app_handle,
                                                false, // allow MCP detection
                                            ).await {
                                                eprintln!("[[reasoning_type_end_failed]]: {}", e);
                                            }
                                        }
                                    }
                                    Some("response") => {
                                        if let (Some(msg_id), Some(start_time)) = (response_message_id, response_start_time) {
                                            if let Err(e) = super::conversation::handle_message_type_end(
                                                msg_id,
                                                "response",
                                                &response_content,
                                                start_time,
                                                &conversation_db,
                                                &window,
                                                conversation_id,
                                                &app_handle,
                                                false, // allow MCP detection
                                            ).await {
                                                eprintln!("[[response_type_end_failed]]: {}", e);
                                            }
                                        } else {
                                            // 兜底：如果缺少 start_time 或 msg_id，至少完成事件更新
                                            super::conversation::finish_stream_messages(
                                                &conversation_db,
                                                reasoning_message_id,
                                                response_message_id,
                                                &reasoning_content,
                                                &response_content,
                                                &window,
                                                conversation_id,
                                            )?;
                                        }
                                    }
                                    _ => {
                                        // 无活跃类型时，走统一收尾
                                        super::conversation::finish_stream_messages(
                                            &conversation_db,
                                            reasoning_message_id,
                                            response_message_id,
                                            &reasoning_content,
                                            &response_content,
                                            &window,
                                            conversation_id,
                                        )?;
                                    }
                                }

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
                                            eprintln!("[[title_generation_failed]]: {}", e);
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
                        // 打印详细的流处理错误信息
                        eprintln!("=== Stream Processing Error ===");
                        eprintln!("[[error_type]]: {:?}", e);
                        eprintln!("[[error_details]]: {}", e);
                        eprintln!("[[error_debug]]: {:#?}", e);

                        // 打印完整的错误链，特别关注 reqwest 错误
                        eprintln!("[[error_chain]]:");
                        let mut current_error: Option<&dyn std::error::Error> = Some(&e);
                        let mut i = 0;
                        while let Some(error) = current_error {
                            eprintln!("  [[error_{}]]: {}", i, error);
                            eprintln!("    [[error_type]]: {}", std::any::type_name_of_val(error));

                            // 特别检查是否是 reqwest 错误并提取详细信息
                            if let Some(reqwest_error) = error.downcast_ref::<reqwest::Error>() {
                                eprintln!("    [[reqwest_error_details]]:");
                                if let Some(status) = reqwest_error.status() {
                                    eprintln!("      [[status_code]]: {}", status);
                                    eprintln!("      [[is_client_error]]: {}", status.is_client_error());
                                    eprintln!("      [[is_server_error]]: {}", status.is_server_error());
                                }
                                if let Some(url) = reqwest_error.url() {
                                    eprintln!("      [[request_url]]: {}", url);
                                }
                                eprintln!("      [[is_timeout]]: {}", reqwest_error.is_timeout());
                                eprintln!("      [[is_connect]]: {}", reqwest_error.is_connect());
                                eprintln!("      [[is_request]]: {}", reqwest_error.is_request());
                                eprintln!("      [[is_body]]: {}", reqwest_error.is_body());
                                eprintln!("      [[is_decode]]: {}", reqwest_error.is_decode());
                            }

                            current_error = error.source();
                            i += 1;
                        }
                        eprintln!("=== End Stream Error Details ===");

                        // 流处理中的错误，返回错误以便上层重试
                        super::conversation::cleanup_token(tokens, conversation_id).await;
                        return Err(anyhow::anyhow!("Stream processing failed: {}", e));
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

// 辅助函数：创建错误消息
async fn create_error_message(
    conversation_db: &ConversationDatabase,
    conversation_id: i64,
    llm_model_id: i64,
    llm_model_name: String,
    error_msg: &str,
    generation_group_id_override: Option<String>,
    parent_group_id_override: Option<String>,
    window: &tauri::Window,
) {
    let now = chrono::Utc::now();
    let generation_group_id =
        generation_group_id_override.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    if let Ok(error_message) = conversation_db.message_repo().unwrap().create(&Message {
        id: 0,
        parent_id: None,
        conversation_id,
        message_type: "error".to_string(),
        content: error_msg.to_string(),
        llm_model_id: Some(llm_model_id),
        llm_model_name: Some(llm_model_name),
        created_time: now,
        start_time: Some(now),
        finish_time: Some(now),
        token_count: 0,
        generation_group_id: Some(generation_group_id),
        parent_group_id: parent_group_id_override,
    }) {
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
                content: error_msg.to_string(),
                is_done: true,
            })
            .unwrap(),
        };
        let _ = window.emit(
            format!("conversation_event_{}", conversation_id).as_str(),
            update_event,
        );
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
                attempts += 1;

                println!("[[non_stream_chat_attempt]]: {}/{}", attempts, MAX_RETRY_ATTEMPTS);

                match client.exec_chat(model_name, chat_request.clone(), Some(chat_options)).await {
                    Ok(response) => {
                        println!("[[non_stream_chat_succeeded_attempt]]: {}", attempts);
                        break Ok(response);
                    },
                    Err(e) => {
                        // 打印详细的非流式聊天错误信息
                        eprintln!("=== Non-Stream Chat Error (attempt {}/{}) ===", attempts, MAX_RETRY_ATTEMPTS);
                        eprintln!("[[error_details]]: {}", e);
                        eprintln!("[[error_debug]]: {:#?}", e);

                        // 打印完整的错误链，特别关注 reqwest 错误
                        eprintln!("[[error_chain]]:");
                        let mut current_error: Option<&dyn std::error::Error> = Some(&e);
                        let mut i = 0;
                        while let Some(error) = current_error {
                            eprintln!("  [[error_{}]]: {}", i, error);
                            eprintln!("    [[error_type]]: {}", std::any::type_name_of_val(error));

                            // 特别检查是否是 reqwest 错误并提取详细信息
                            if let Some(reqwest_error) = error.downcast_ref::<reqwest::Error>() {
                                eprintln!("    [[reqwest_error_details]]:");
                                if let Some(status) = reqwest_error.status() {
                                    eprintln!("      [[status_code]]: {}", status);
                                    eprintln!("      [[is_client_error]]: {}", status.is_client_error());
                                    eprintln!("      [[is_server_error]]: {}", status.is_server_error());
                                }
                                if let Some(url) = reqwest_error.url() {
                                    eprintln!("      [[request_url]]: {}", url);
                                }
                                eprintln!("      [[is_timeout]]: {}", reqwest_error.is_timeout());
                                eprintln!("      [[is_connect]]: {}", reqwest_error.is_connect());
                                eprintln!("      [[is_request]]: {}", reqwest_error.is_request());
                                eprintln!("      [[is_body]]: {}", reqwest_error.is_body());
                                eprintln!("      [[is_decode]]: {}", reqwest_error.is_decode());
                            }

                            current_error = error.source();
                            i += 1;
                        }
                        eprintln!("=== End Non-Stream Error Details ===");

                        if attempts >= MAX_RETRY_ATTEMPTS {
                            let final_error = format!("AI请求失败: {}", get_user_friendly_error_message(&e));
                            eprintln!("[[final_non_stream_error]]: Non-stream chat failed after {} attempts: {}", attempts, e);

                            // 发送错误通知给前端
                            let _ = window.emit(
                                ERROR_NOTIFICATION_EVENT,
                                get_user_friendly_error_message(&e),
                            );

                            break Err(anyhow::anyhow!("{}", final_error));
                        }

                        let delay = calculate_retry_delay(attempts);
                        println!("[[retrying_delay_ms]]: {}", delay);
                        sleep(Duration::from_millis(delay)).await;
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
                        eprintln!("[[title_generation_failed]]: {}", e);
                    }
                });
            }

            Ok(())
        }
        Err(e) => {
            super::conversation::cleanup_token(tokens, conversation_id).await;
            let user_friendly_error = get_user_friendly_error_message(&e);
            let err_msg = format!("AI请求失败: {}", user_friendly_error);
            let now = chrono::Utc::now();
            let _ = window.emit(ERROR_NOTIFICATION_EVENT, user_friendly_error);

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

            eprintln!("[[chat_error]]: {}", e);
            Err(anyhow::anyhow!("Chat error: {}", e))
        }
    }
}
