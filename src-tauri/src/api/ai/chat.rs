use crate::api::ai::config::{calculate_retry_delay, MAX_RETRY_ATTEMPTS};
use crate::api::ai::events::{
    ConversationEvent, MessageAddEvent, MessageUpdateEvent, ERROR_NOTIFICATION_EVENT,
};
use crate::db::conversation_db::{ConversationDatabase, Message, Repository};
use crate::db::system_db::FeatureConfig;

use futures::StreamExt;
use genai::chat::ChatStreamEvent;
use genai::chat::{ChatOptions, ChatRequest, ToolCall};
use genai::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

// 尝试获取HTTP错误的响应体（改进版，支持POST请求）
async fn try_fetch_error_body_advanced(
    url: &str,
    status: reqwest::StatusCode,
    is_chat_api: bool,
) -> Option<String> {
    if !status.is_client_error() && !status.is_server_error() {
        return None;
    }

    println!("[[attempting_to_fetch_error_body_from_url]]: {}", url);

    // 创建一个简单的客户端来获取错误信息
    let client = reqwest::Client::new();

    if is_chat_api && url.contains("/chat/completions") {
        // 方法1: 发送一个故意错误的请求来获取错误响应
        let invalid_payload = serde_json::json!({
            "model": "invalid-model-name-that-does-not-exist",
            "messages": []
        });

        println!("[[trying_invalid_payload_method]]");
        match client.post(url).json(&invalid_payload).send().await {
            Ok(response) => {
                println!("[[error_response_status]]: {}", response.status());
                if response.status().is_client_error() || response.status().is_server_error() {
                    match response.text().await {
                        Ok(body) => {
                            println!("[[error_response_body]]: {}", body);
                            return Some(body);
                        }
                        Err(e) => {
                            println!("[[failed_to_read_error_body]]: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("[[invalid_payload_request_failed]]: {}", e);
            }
        }

        // 方法2: 发送空的POST请求
        println!("[[trying_empty_post_method]]");
        match client
            .post(url)
            .header("Content-Type", "application/json")
            .body("{}")
            .send()
            .await
        {
            Ok(response) => {
                println!("[[empty_post_response_status]]: {}", response.status());
                if response.status().is_client_error() || response.status().is_server_error() {
                    match response.text().await {
                        Ok(body) => {
                            println!("[[empty_post_error_body]]: {}", body);
                            return Some(body);
                        }
                        Err(e) => {
                            println!("[[failed_to_read_empty_post_body]]: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("[[empty_post_request_failed]]: {}", e);
            }
        }

        // 方法3: 尝试用HEAD请求来获取一些信息
        println!("[[trying_head_request_method]]");
        match client.head(url).send().await {
            Ok(response) => {
                println!("[[head_response_status]]: {}", response.status());
                println!("[[head_response_headers]]: {:?}", response.headers());
                // HEAD请求通常不会有响应体，但可能有有用的头信息
            }
            Err(e) => {
                println!("[[head_request_failed]]: {}", e);

                // 尝试从错误消息中提取有用信息
                let error_msg = e.to_string();
                if error_msg.contains("{") && error_msg.contains("}") {
                    // 尝试从错误消息中提取JSON
                    if let Some(start) = error_msg.find("{") {
                        if let Some(end) = error_msg.rfind("}") {
                            let json_part = &error_msg[start..=end];
                            println!("[[extracted_json_from_head_error]]: {}", json_part);
                            return Some(json_part.to_string());
                        }
                    }
                }
            }
        }
    } else {
        // 对于其他API，使用GET请求
        println!("[[trying_get_request_method]]");
        match client.get(url).send().await {
            Ok(response) => {
                if response.status().is_client_error() || response.status().is_server_error() {
                    match response.text().await {
                        Ok(body) => {
                            println!("[[get_error_response_body]]: {}", body);
                            return Some(body);
                        }
                        Err(e) => {
                            println!("[[failed_to_read_get_error_body]]: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("[[get_request_failed]]: {}", e);
            }
        }
    }

    None
}

// 增强的错误处理函数（简化版，避免Send问题）
async fn enhanced_error_logging_v2<E: std::error::Error + 'static>(
    error: &E,
    context: &str,
) -> String {
    eprintln!("=== {} Error ===", context);
    eprintln!("[[error_type]]: {:?}", error);
    eprintln!("[[error_details]]: {}", error);
    eprintln!("[[error_debug]]: {:#?}", error);

    // 收集错误链信息，不进行异步操作
    let mut current_error: Option<&dyn std::error::Error> = Some(error);
    let mut i = 0;
    let mut error_urls = Vec::new(); // 收集URL用于后续处理

    while let Some(err) = current_error {
        eprintln!("  [[error_{}]]: {}", i, err);
        eprintln!("    [[error_type]]: {}", std::any::type_name_of_val(err));

        // 检查错误字符串中是否包含有用信息
        let error_string = err.to_string();
        eprintln!("    [[error_string]]: {}", error_string);

        // 尝试从错误字符串中提取URL和状态码
        if error_string.contains("400") && error_string.contains("https://") {
            if let Some(start) = error_string.find("https://") {
                if let Some(end) = error_string[start..].find("\"") {
                    let url = &error_string[start..start + end];
                    eprintln!("    [[extracted_url]]: {}", url);
                    error_urls.push((url.to_string(), reqwest::StatusCode::from_u16(400).unwrap()));
                }
            }
        }

        // 特别检查是否是 reqwest 错误并提取详细信息
        if let Some(reqwest_error) = err.downcast_ref::<reqwest::Error>() {
            eprintln!("    [[reqwest_error_details]]:");

            if let Some(status) = reqwest_error.status() {
                eprintln!("      [[status_code]]: {}", status);
                eprintln!("      [[is_client_error]]: {}", status.is_client_error());
                eprintln!("      [[is_server_error]]: {}", status.is_server_error());

                if let Some(url) = reqwest_error.url() {
                    let url_str = url.to_string();
                    eprintln!("      [[request_url]]: {}", url_str);

                    // 对于错误状态码，收集URL信息但不在这里执行异步操作
                    if status.is_client_error() || status.is_server_error() {
                        error_urls.push((url_str, status));
                    }
                }
            }

            eprintln!("      [[is_timeout]]: {}", reqwest_error.is_timeout());
            eprintln!("      [[is_connect]]: {}", reqwest_error.is_connect());
            eprintln!("      [[is_request]]: {}", reqwest_error.is_request());
            eprintln!("      [[is_body]]: {}", reqwest_error.is_body());
            eprintln!("      [[is_decode]]: {}", reqwest_error.is_decode());
        } else {
            // 如果不是reqwest错误，尝试其他方式解析
            eprintln!("    [[not_reqwest_error]]: attempting string parsing");

            // 检查是否是EventSource相关的错误
            if error_string.contains("EventSource") || error_string.contains("Invalid status code")
            {
                eprintln!("    [[event_source_error_detected]]: true");

                // 尝试从字符串中提取状态码
                if error_string.contains("400") {
                    eprintln!("    [[detected_status_code]]: 400");
                }

                // 尝试提取URL
                if let Some(start) = error_string.find("url: \"") {
                    let start = start + 6; // 跳过 'url: "'
                    if let Some(end) = error_string[start..].find("\"") {
                        let url = &error_string[start..start + end];
                        eprintln!("    [[extracted_url_from_string]]: {}", url);
                        if !error_urls.iter().any(|(u, _)| u == url) {
                            error_urls.push((
                                url.to_string(),
                                reqwest::StatusCode::from_u16(400).unwrap(),
                            ));
                        }
                    }
                }
            }
        }

        current_error = err.source();
        i += 1;
    }

    // 现在在循环外处理URL（如果有的话）
    for (url_str, status) in error_urls {
        eprintln!(
            "[[processing_extracted_url]]: {} with status {}",
            url_str, status
        );
        let is_chat_api = url_str.contains("/chat/completions");
        if let Some(error_body) = try_fetch_error_body_advanced(&url_str, status, is_chat_api).await
        {
            eprintln!("      [[extracted_error_body]]: {}", error_body);
        }
    }

    eprintln!("=== End {} Error Details ===", context);

    // 返回用户友好的错误信息
    get_user_friendly_error_message(error)
}

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
        .exec_chat_stream(model_name, chat_request.clone(), Some(&chat_options))
        .await
    {
        Ok(response) => {
            println!("[[stream_connection_established]]: true");
            response
        }
        Err(e) => {
            let _user_friendly_error = enhanced_error_logging_v2(&e, "Stream Connection").await;
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
    let mut captured_tool_calls: Vec<ToolCall> = Vec::new();

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
                            ChatStreamEvent::ToolCallChunk(tool_call_chunk) => {
                                println!("[[tool_call_chunk]]: {:#?}\n", tool_call_chunk);
                            }
                            ChatStreamEvent::End(end_event) => {
                                println!("[[end_event]]: {:#?}\n", end_event);
                                // Capture tool calls if they exist
                                if let Some(tool_calls) = end_event.captured_into_tool_calls() {
                                    captured_tool_calls = tool_calls;
                                    println!("[[captured_tool_calls]]: {:#?}\n", captured_tool_calls);
                                }

                                // If native tool calls were captured, persist UI hints and DB records, and optionally auto-run
                                if !captured_tool_calls.is_empty() {
                                    // Ensure we have a response message to attach UI hints
                                    if response_message_id.is_none() {
                                        // Create a minimal response message to host MCP UI hints
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
                                                content: String::new(),
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
                                            })
                                            .unwrap(),
                                        };
                                        let _ = window.emit(
                                            format!("conversation_event_{}", conversation_id).as_str(),
                                            add_event,
                                        );
                                    }

                                    // Create DB records for tool calls, then append UI hints with DB call_id
                                    for tool_call in &captured_tool_calls {
                                        // Our tool name format is "{server}__{tool}", split it safely
                                        let (server_name, tool_name) = if let Some((s, t)) = tool_call
                                            .fn_name
                                            .split_once("__")
                                        {
                                            (s.to_string(), t.to_string())
                                        } else {
                                            // Fallback: no server prefix
                                            (String::from("default"), tool_call.fn_name.clone())
                                        };

                                        // Create MCP tool call record for later manual execution or auto-run
                                        let params_str = tool_call.fn_arguments.to_string();
                                        let created_call = crate::api::mcp_execution_api::create_mcp_tool_call(
                                            app_handle.clone(),
                                            conversation_id,
                                            response_message_id, // bind to this response message
                                            server_name.clone(),
                                            tool_name.clone(),
                                            params_str.clone(),
                                        )
                                        .await;

                                        if let Ok(tool_call_record) = created_call {
                                            // 记录原生 LLM tool_call_id 到 DB 调用映射，用于 ToolResponse 匹配
                                            crate::api::mcp_execution_api::set_llm_call_id_for_db_call(
                                                tool_call_record.id,
                                                tool_call.call_id.clone(),
                                            ).await;
                                            // Append UI hint comment for frontend renderer, now with DB call_id
                                            let ui_hint = format!(
                                                "\n\n<!-- MCP_TOOL_CALL:{} -->\n",
                                                serde_json::json!({
                                                    "server_name": server_name,
                                                    "tool_name": tool_name,
                                                    "parameters": params_str,
                                                    "call_id": tool_call_record.id,
                                                })
                                            );
                                            response_content.push_str(&ui_hint);

                                            // Persist updated content to DB and emit update (not done yet)
                                            if let Some(msg_id) = response_message_id {
                                                if let Ok(Some(mut msg)) = conversation_db
                                                    .message_repo()
                                                    .unwrap()
                                                    .read(msg_id)
                                                {
                                                    msg.content = response_content.clone();
                                                    let _ = conversation_db
                                                        .message_repo()
                                                        .unwrap()
                                                        .update(&msg);

                                                    let update_event = ConversationEvent {
                                                        r#type: "message_update".to_string(),
                                                        data: serde_json::to_value(MessageUpdateEvent {
                                                            message_id: msg_id,
                                                            message_type: "response".to_string(),
                                                            content: response_content.clone(),
                                                            is_done: false,
                                                        })
                                                        .unwrap(),
                                                    };
                                                    let _ = window.emit(
                                                        format!("conversation_event_{}", conversation_id).as_str(),
                                                        update_event,
                                                    );
                                                }
                                            }

                                            // Auto-run if configured on assistant's MCP tool
                                            if let Ok(conv) = conversation_db
                                                .conversation_repo()
                                                .unwrap()
                                                .read(conversation_id)
                                            {
                                                if let Some(assistant_id) = conv.and_then(|c| c.assistant_id) {
                                                    if let Ok(servers) = crate::api::assistant_api::get_assistant_mcp_servers_with_tools(
                                                        app_handle.clone(),
                                                        assistant_id,
                                                    )
                                                    .await
                                                    {
                                                        let mut should_auto_run = false;
                                                        for s in servers.iter() {
                                                            if s.name == server_name && s.is_enabled {
                                                                if let Some(t) = s.tools.iter().find(|t| t.name == tool_name && t.is_enabled) {
                                                                    if t.is_auto_run { should_auto_run = true; }
                                                                }
                                                            }
                                                        }
                                                        if should_auto_run {
                                                            let state = app_handle.state::<crate::AppState>();
                                                            let feature_config_state = app_handle.state::<crate::FeatureConfigState>();
                                                            let message_token_manager = app_handle.state::<crate::state::message_token::MessageTokenManager>();
                                                            if let Err(e) = crate::api::mcp_execution_api::execute_mcp_tool_call(
                                                                app_handle.clone(),
                                                                state,
                                                                feature_config_state,
                                                                message_token_manager,
                                                                window.clone(),
                                                                tool_call_record.id,
                                                            )
                                                            .await
                                                            {
                                                                eprintln!(
                                                                    "Auto-execute MCP tool failed (call_id={}): {}",
                                                                    tool_call_record.id, e
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        } else if let Err(e) = created_call {
                                            eprintln!("Failed to create MCP tool call record: {}", e);
                                        }
                                    }
                                }

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

                                // Handle captured tool calls for MCP integration
                                if !captured_tool_calls.is_empty() {
                                    println!("[[processing_tool_calls_count]]: {}", captured_tool_calls.len());

                                    // Send tool call information to frontend for potential MCP execution
                                    for tool_call in &captured_tool_calls {
                                        println!("[[tool_call_to_process]]: {} with args: {}", tool_call.fn_name, tool_call.fn_arguments);

                                        // Emit tool call event for MCP handling
                                        let tool_call_event = serde_json::json!({
                                            "type": "tool_call",
                                            "data": {
                                                "conversation_id": conversation_id,
                                                "call_id": tool_call.call_id,
                                                "function_name": tool_call.fn_name,
                                                "arguments": tool_call.fn_arguments,
                                                "response_message_id": response_message_id
                                            }
                                        });

                                        let _ = window.emit(
                                            format!("conversation_event_{}", conversation_id).as_str(),
                                            tool_call_event
                                        );
                                    }

                                    // Note: In stream mode, we emit the tool calls and let the frontend handle MCP execution
                                    // The conversation continues after tool results are provided via tool_result_continue_ask_ai
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
                        }
                    }
                    Some(Err(e)) => {
                        let _user_friendly_error = enhanced_error_logging_v2(&e, "Stream Processing").await;
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

    // 非流式：强制捕获工具调用，便于将工具以 UI 注释形式插入
    let non_stream_options = chat_options.clone().with_capture_tool_calls(true);
    let chat_result = tokio::select! {
        result = async {
            let mut attempts = 0;
            loop {
                attempts += 1;

                println!("[[non_stream_chat_attempt]]: {}/{}", attempts, MAX_RETRY_ATTEMPTS);

                match client.exec_chat(model_name, chat_request.clone(), Some(&non_stream_options)).await {
                    Ok(response) => {
                        println!("[[non_stream_chat_succeeded_attempt]]: {}", attempts);
                        break Ok(response);
                    },
                    Err(e) => {
                        let user_friendly_error = enhanced_error_logging_v2(&e, &format!("Non-Stream Chat (attempt {}/{})", attempts, MAX_RETRY_ATTEMPTS)).await;

                        if attempts >= MAX_RETRY_ATTEMPTS {
                            let final_error = format!("AI请求失败: {}", user_friendly_error);
                            eprintln!("[[final_non_stream_error]]: Non-stream chat failed after {} attempts: {}", attempts, e);

                            // 发送错误通知给前端
                            let _ = window.emit(
                                ERROR_NOTIFICATION_EVENT,
                                user_friendly_error,
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
            let mut content = chat_response.first_text().unwrap_or("").to_string();

            // 非流式：捕获原生 ToolCall 并处理（创建DB、UI注释、自动执行）
            let tool_calls: Vec<ToolCall> = chat_response
                .tool_calls()
                .into_iter()
                .map(|tc| tc.clone())
                .collect();

            if !tool_calls.is_empty() {
                println!(
                    "[[non_stream_captured_tool_calls_count]]: {}",
                    tool_calls.len()
                );
                for tool_call in tool_calls.iter() {
                    let (server_name, tool_name) =
                        if let Some((s, t)) = tool_call.fn_name.split_once("__") {
                            (s.to_string(), t.to_string())
                        } else {
                            (String::from("default"), tool_call.fn_name.clone())
                        };

                    let params_str = tool_call.fn_arguments.to_string();
                    match crate::api::mcp_execution_api::create_mcp_tool_call(
                        app_handle.clone(),
                        conversation_id,
                        Some(response_message_id),
                        server_name.clone(),
                        tool_name.clone(),
                        params_str.clone(),
                    )
                    .await
                    {
                        Ok(tool_call_record) => {
                            crate::api::mcp_execution_api::set_llm_call_id_for_db_call(
                                tool_call_record.id,
                                tool_call.call_id.clone(),
                            )
                            .await;

                            let ui_hint = format!(
                                "\n\n<!-- MCP_TOOL_CALL:{} -->\n",
                                serde_json::json!({
                                    "server_name": server_name,
                                    "tool_name": tool_name,
                                    "parameters": params_str,
                                    "call_id": tool_call_record.id,
                                })
                            );
                            content.push_str(&ui_hint);

                            // 立即更新消息内容并发送事件，让用户看到工具调用界面
                            if let Ok(Some(mut msg)) = conversation_db
                                .message_repo()
                                .unwrap()
                                .read(response_message_id)
                            {
                                msg.content = content.clone();
                                let _ = conversation_db
                                    .message_repo()
                                    .unwrap()
                                    .update(&msg);

                                // 发送工具调用界面更新事件
                                let update_event = ConversationEvent {
                                    r#type: "message_update".to_string(),
                                    data: serde_json::to_value(MessageUpdateEvent {
                                        message_id: response_message_id,
                                        message_type: "response".to_string(),
                                        content: content.clone(),
                                        is_done: false, // 还未完成，因为可能有自动执行
                                    })
                                    .unwrap(),
                                };
                                let _ = window.emit(
                                    format!("conversation_event_{}", conversation_id).as_str(),
                                    update_event,
                                );
                            }

                            // 自动执行（若配置）
                            if let Ok(conv) = conversation_db
                                .conversation_repo()
                                .unwrap()
                                .read(conversation_id)
                            {
                                if let Some(assistant_id) = conv.and_then(|c| c.assistant_id) {
                                    if let Ok(servers) = crate::api::assistant_api::get_assistant_mcp_servers_with_tools(app_handle.clone(), assistant_id).await {
                                        let mut should_auto_run = false;
                                        for s in servers.iter() {
                                            if s.name == server_name && s.is_enabled {
                                                if let Some(t) = s.tools.iter().find(|t| t.name == tool_name && t.is_enabled) {
                                                    if t.is_auto_run { should_auto_run = true; }
                                                }
                                            }
                                        }
                                        if should_auto_run {
                                            let state = app_handle.state::<crate::AppState>();
                                            let feature_config_state = app_handle.state::<crate::FeatureConfigState>();
                                            let message_token_manager = app_handle.state::<crate::state::message_token::MessageTokenManager>();
                                            if let Err(e) = crate::api::mcp_execution_api::execute_mcp_tool_call(
                                                app_handle.clone(),
                                                state,
                                                feature_config_state,
                                                message_token_manager,
                                                window.clone(),
                                                tool_call_record.id,
                                            ).await {
                                                eprintln!("Auto-execute MCP tool failed (call_id={}): {}", tool_call_record.id, e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to create MCP tool call record (non-stream): {}", e)
                        }
                    }
                }
            }
            
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
