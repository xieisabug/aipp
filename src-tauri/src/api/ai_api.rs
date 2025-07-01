use crate::api::assistant_api::get_assistant;
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
use std::time::Duration;
use tauri::Emitter;
use tauri::State;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use genai::chat::{ChatMessage, ChatOptions, ChatRequest};
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};
use genai::adapter::AdapterKind;
use futures::StreamExt;

use super::assistant_api::AssistantDetail;

// 事件名称常量
const MESSAGE_FINISH_EVENT: &str = "Tea::Event::MessageFinish";
const TITLE_CHANGE_EVENT: &str = "title_change";
const ERROR_NOTIFICATION_EVENT: &str = "conversation-window-error-notification";

// 默认端点映射
const DEFAULT_ENDPOINTS: &[(AdapterKind, &str)] = &[
    (AdapterKind::OpenAI, "https://api.openai.com/v1"),
    (AdapterKind::Anthropic, "https://api.anthropic.com"),
    (AdapterKind::Cohere, "https://api.cohere.ai/v1"),
    (AdapterKind::Gemini, "https://generativelanguage.googleapis.com/v1beta"),
    (AdapterKind::Groq, "https://api.groq.com/openai/v1"),
    (AdapterKind::Xai, "https://api.x.ai/v1"),
    (AdapterKind::DeepSeek, "https://api.deepseek.com/"),
    (AdapterKind::Ollama, "http://localhost:11434/api"),
];

/// 构建 ChatOptions 从配置映射
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

/// 将消息列表转换为 ChatMessage 格式
fn build_chat_messages(init_message_list: &[(String, String, Vec<MessageAttachment>)]) -> Vec<ChatMessage> {
    let mut chat_messages = Vec::new();
    for (role, content, _attachments) in init_message_list {
        match role.as_str() {
            "system" => chat_messages.push(ChatMessage::system(content)),
            "user" => chat_messages.push(ChatMessage::user(content)),
            "assistant" => chat_messages.push(ChatMessage::assistant(content)),
            _ => {}
        }
    }
    chat_messages
}

/// 合并模型配置
fn merge_model_configs(
    base_configs: Vec<AssistantModelConfig>,
    model_detail: &crate::db::llm_db::ModelDetail,
    override_configs: Option<HashMap<String, serde_json::Value>>,
) -> Vec<AssistantModelConfig> {
    let mut model_config_clone = base_configs;
    
    // 添加模型配置
    model_config_clone.push(AssistantModelConfig {
        id: 0,
        assistant_id: model_config_clone.first().map(|c| c.assistant_id).unwrap_or(0),
        assistant_model_id: model_detail.model.id,
        name: "model".to_string(),
        value: Some(model_detail.model.code.clone()),
        value_type: "string".to_string(),
    });

    // 应用覆盖配置
    if let Some(override_configs) = override_configs {
        for (key, value) in override_configs {
            let value_type = match &value {
                serde_json::Value::String(_) => "string",
                serde_json::Value::Number(_) => "number",
                serde_json::Value::Bool(_) => "boolean",
                serde_json::Value::Array(_) => "array",
                serde_json::Value::Object(_) => "object",
                serde_json::Value::Null => "null",
            }
            .to_string();

            let value_str = value.to_string();

            if let Some(existing_config) = model_config_clone.iter_mut().find(|c| c.name == key) {
                existing_config.value = Some(value_str);
                existing_config.value_type = value_type;
            } else {
                model_config_clone.push(AssistantModelConfig {
                    id: 0,
                    assistant_id: model_config_clone.first().map(|c| c.assistant_id).unwrap_or(0),
                    assistant_model_id: model_detail.model.id,
                    name: key,
                    value: Some(value_str),
                    value_type,
                });
            }
        }
    }

    model_config_clone
}

/// 清理消息令牌
async fn cleanup_token(tokens: &Arc<tokio::sync::Mutex<HashMap<i64, CancellationToken>>>, message_id: i64) {
    let mut map = tokens.lock().await;
    map.remove(&message_id);
}

/// 获取默认端点
fn get_default_endpoint(adapter_kind: AdapterKind) -> &'static str {
    DEFAULT_ENDPOINTS
        .iter()
        .find(|(kind, _)| *kind == adapter_kind)
        .map(|(_, endpoint)| *endpoint)
        .unwrap_or("https://api.openai.com/v1")
}

/// 根据模型名称推断 AdapterKind
fn infer_adapter_kind(model_name: &str, api_type: &str) -> AdapterKind {
    match api_type.to_lowercase().as_str() {
        "openai_api" | "openai" => AdapterKind::OpenAI,
        "anthropic" => AdapterKind::Anthropic,
        "cohere" => AdapterKind::Cohere,
        "gemini" => AdapterKind::Gemini,
        "deepseek" => AdapterKind::DeepSeek, // DeepSeek 使用 OpenAI 兼容 API
        "ollama" => AdapterKind::Ollama,
        "xai" => AdapterKind::Xai,
        _ => {
            // 根据模型名称推断
            if model_name.starts_with("gpt") || model_name.starts_with("o1") || model_name.starts_with("o3") || model_name.starts_with("o4") {
                AdapterKind::OpenAI
            } else if model_name.starts_with("claude") {
                AdapterKind::Anthropic
            } else if model_name.starts_with("command") {
                AdapterKind::Cohere
            } else if model_name.starts_with("gemini") {
                AdapterKind::Gemini
            } else if model_name.starts_with("grok") {
                AdapterKind::Xai
            } else if model_name.starts_with("deepseek") {
                AdapterKind::OpenAI // DeepSeek 使用 OpenAI 兼容 API
            } else {
                AdapterKind::Ollama // 默认使用 Ollama
            }
        }
    }
}

/// 从配置创建自定义 Client
fn create_client_with_config(
    configs: &[crate::db::llm_db::LLMProviderConfig], 
    model_name: &str,
    api_type: &str
) -> Result<Client, AppError> {
    let config_map: HashMap<String, String> = configs
        .iter()
        .map(|c| (c.name.clone(), c.value.clone()))
        .collect();

    let api_key = config_map
        .get("api_key")
        .ok_or_else(|| AppError::NoConfigError("api_key".to_string()))?;

    let endpoint_opt = config_map.get("endpoint").cloned();
    let adapter_kind = infer_adapter_kind(model_name, api_type);
    let api_key = api_key.clone();

    let target_resolver = ServiceTargetResolver::from_resolver_fn(
        move |service_target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
            let ServiceTarget { model, .. } = service_target;
            
            let auth = AuthData::from_single(api_key.clone());
            let model_iden = ModelIden::new(adapter_kind, model.model_name);
            
            let endpoint = if let Some(ep) = &endpoint_opt {
                Endpoint::from_owned(ep.trim_end_matches('/').to_string())
            } else {
                Endpoint::from_static(get_default_endpoint(adapter_kind))
            };

            Ok(ServiceTarget {
                endpoint,
                auth,
                model: model_iden,
            })
        },
    );

    let client = Client::builder()
        .with_service_target_resolver(target_resolver)
        .build();

    Ok(client)
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
    println!(
        "ask_ai: {:?}, override_model_config: {:?}, override_prompt: {:?}",
        request, override_model_config, override_prompt
    );
    let template_engine = TemplateEngine::new();
    let mut template_context = HashMap::new();
    let (tx, mut rx) = mpsc::channel(100);

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

    let need_generate_title = request.conversation_id.is_empty();
    let request_prompt_result = template_engine
        .parse(&request.prompt, &template_context)
        .await;

    let app_handle_clone = app_handle.clone();
    let (conversation_id, new_message_id, request_prompt_result_with_context, init_message_list) =
        initialize_conversation(
            &app_handle_clone,
            &request,
            &assistant_detail,
            assistant_prompt_result,
            request_prompt_result.clone(),
            override_prompt.clone(),
        )
        .await?;

    if new_message_id.is_some() {
        let config_feature_map = feature_config_state.config_feature_map.lock().await.clone();

        let app_handle_clone = app_handle.clone();

        let cancel_token = CancellationToken::new();
        let message_id = new_message_id.unwrap();
        message_token_manager
            .store_token(new_message_id.unwrap(), cancel_token.clone())
            .await;

        let tokens = message_token_manager.get_tokens();
        tokio::spawn(async move {
            let db = LLMDatabase::new(&app_handle_clone)
                .map_err(Error::from)
                .context("Failed to create LLMDatabase")?;
            let conversation_db = ConversationDatabase::new(&app_handle_clone).unwrap();
            let provider_id = &assistant_detail.model[0].provider_id;
            let model_code = &assistant_detail.model[0].model_code;
            let model_detail = db
                .get_llm_model_detail(provider_id, model_code)
                .context("Failed to get LLM model detail")?;
            println!("model detail : {:#?}", model_detail);

            // Create genai client with custom config
            let client = create_client_with_config(
                &model_detail.configs, 
                &model_detail.model.code, 
                &model_detail.provider.api_type
            )?;

            // Prepare model configurations
            let model_config_clone = merge_model_configs(
                assistant_detail.model_configs.clone(),
                &model_detail,
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

            // Extract model name from config
            let model_name = config_map
                .get("model")
                .cloned()
                .unwrap_or_else(|| model_detail.model.code.clone());

            // Convert messages to ChatMessage format and build chat options
            let chat_messages = build_chat_messages(&init_message_list);
            let chat_request = ChatRequest::new(chat_messages);
            let chat_options = build_chat_options(&config_map);

            println!("Using model: {}, stream: {}", model_name, stream);

            if stream {
                // 使用 genai 流式处理
                match client.exec_chat_stream(&model_name, chat_request, Some(&chat_options)).await {
                    Ok(chat_stream_response) => {
                        let mut chat_stream = chat_stream_response.stream;
                        let mut full_content = String::new();
                        
                        loop {
                            tokio::select! {
                                stream_result = chat_stream.next() => {
                                    match stream_result {
                                        Some(Ok(stream_event)) => {
                                            use genai::chat::ChatStreamEvent;
                                            match stream_event {
                                                ChatStreamEvent::Start => {
                                                    // 流开始，更新开始时间
                                                    conversation_db
                                                        .message_repo()
                                                        .unwrap()
                                                        .update_start_time(message_id)
                                                        .unwrap();
                                                }
                                                ChatStreamEvent::Chunk(chunk) => {
                                                    // 接收到内容块
                                                    full_content.push_str(&chunk.content);
                                                    if let Err(e) = tx.send((message_id, full_content.clone(), false)).await {
                                                        eprintln!("Failed to send chunk: {}", e);
                                                        break;
                                                    }
                                                }
                                                ChatStreamEvent::ReasoningChunk(reasoning_chunk) => {
                                                    // 推理内容块 (如 o1 模型的推理过程)
                                                    // 这里我们也加入到内容中，但可以根据需要单独处理
                                                    full_content.push_str(&reasoning_chunk.content);
                                                    if let Err(e) = tx.send((message_id, full_content.clone(), false)).await {
                                                        eprintln!("Failed to send reasoning chunk: {}", e);
                                                        break;
                                                    }
                                                }
                                                ChatStreamEvent::ToolCallChunk(_tool_chunk) => {
                                                    // 工具调用块，暂时忽略
                                                    // 可以根据需要处理工具调用
                                                }
                                                ChatStreamEvent::End(stream_end) => {
                                                    // 流结束
                                                    if let Some(captured_text) = stream_end.captured_first_text() {
                                                        full_content = captured_text.to_string();
                                                    }
                                                    
                                                    conversation_db
                                                        .message_repo()
                                                        .unwrap()
                                                        .update_finish_time(message_id)
                                                        .unwrap();
                                                    
                                                    tx.send((message_id, full_content.clone(), true)).await.unwrap();
                                                    break;
                                                }
                                            }
                                        },
                                        Some(Err(e)) => {
                                            eprintln!("Stream error: {}", e);
                                            cleanup_token(&tokens, message_id).await;
                                            let err_msg = format!("Chat stream error: {}", e);
                                            tx.send((message_id, err_msg, true)).await.unwrap();
                                            break;
                                        },
                                        None => {
                                            // 流意外结束
                                            conversation_db
                                                .message_repo()
                                                .unwrap()
                                                .update_finish_time(message_id)
                                                .unwrap();
                                            tx.send((message_id, full_content.clone(), true)).await.unwrap();
                                            break;
                                        }
                                    }
                                }
                                _ = cancel_token.cancelled() => {
                                    println!("Chat stream cancelled");
                                    break;
                                }
                            }
                        }
                    },
                    Err(e) => {
                        cleanup_token(&tokens, message_id).await;
                        let err_msg = format!("Chat stream error: {}", e);
                        tx.send((message_id, err_msg, true)).await.unwrap();
                        eprintln!("Chat stream error: {}", e);
                    }
                }
            } else {
                conversation_db
                    .message_repo()
                    .unwrap()
                    .update_start_time(message_id)
                    .unwrap();

                // Use genai non-streaming
                let chat_result = tokio::select! {
                    result = client.exec_chat(&model_name, chat_request, Some(&chat_options)) => result,
                    _ = cancel_token.cancelled() => {
                        cleanup_token(&tokens, message_id).await;
                        return Err(anyhow::anyhow!("Request cancelled"));
                    }
                };

                match chat_result {
                    Ok(chat_response) => {
                        let content = chat_response.first_text().unwrap_or("").to_string();
                        println!("Chat content: {}", content.clone());

                        conversation_db
                            .message_repo()
                            .unwrap()
                            .update_finish_time(message_id)
                            .unwrap();
                        tx.send((message_id, content.clone(), true)).await.unwrap();
                        // Ensure tx is closed after sending the message
                        drop(tx);
                    }
                    Err(e) => {
                        cleanup_token(&tokens, message_id).await;
                        let err_msg = format!("Chat error: {}", e);
                        tx.send((message_id, err_msg, true)).await.unwrap();
                        eprintln!("Chat error: {}", e);
                    }
                }
            }

            Ok::<(), Error>(())
        });

        let app_handle_clone = app_handle.clone();
        let tokens = message_token_manager.get_tokens();
        let window_clone = window.clone();
        tokio::spawn(async move {
            loop {
                match timeout(Duration::from_secs(600), rx.recv()).await {
                    Ok(Some((id, content, done))) => {
                        println!("Received data: id={}, content={}", id, content);
                        window_clone
                            .emit(format!("message_{}", id).as_str(), content.clone())
                            .map_err(|e| e.to_string())
                            .unwrap();

                        if done {
                            let conversation_db = ConversationDatabase::new(&app_handle_clone)
                                .map_err(|e: rusqlite::Error| e.to_string())
                                .unwrap();

                            let mut message = conversation_db
                                .message_repo()
                                .unwrap()
                                .read(new_message_id.unwrap())
                                .unwrap()
                                .unwrap();
                            message.content = content.clone().to_string();
                            conversation_db
                                .message_repo()
                                .unwrap()
                                .update(&message)
                                .unwrap();

                            println!("Message finish: id={}", id);
                            window_clone
                                .emit(
                                    format!("message_{}", id).as_str(),
                                    MESSAGE_FINISH_EVENT,
                                )
                                .map_err(|e| e.to_string())
                                .unwrap();
                            if need_generate_title {
                                generate_title(
                                    &app_handle_clone,
                                    conversation_id,
                                    request_prompt_result.clone(),
                                    content.clone().to_string(),
                                    config_feature_map.clone(),
                                    window_clone.clone(),
                                )
                                .await
                                .map_err(|e| e.to_string())
                                .unwrap();
                            }
                            cleanup_token(&tokens, message_id).await;
                        }
                    }
                    Ok(None) => {
                        cleanup_token(&tokens, message_id).await;
                        println!("Channel closed");
                        break;
                    }
                    Err(err) => {
                        cleanup_token(&tokens, message_id).await;
                        println!("Timeout waiting for data from channel: {:?}", err);
                        break;
                    }
                }
            }
        });
    }

    Ok(AiResponse {
        conversation_id,
        add_message_id: new_message_id.unwrap(),
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

    let parent_ids: HashSet<i64> = messages.iter().filter_map(|m| m.0.parent_id).collect();
    println!("parent_ids: {:?}", parent_ids);

    let parent_max_child: HashMap<i64, i64> = messages
        .iter()
        .filter(|m| {
            if let Some(pid) = m.0.parent_id {
                parent_ids.contains(&pid)
            } else {
                false
            }
        })
        .fold(HashMap::new(), |mut acc, m| {
            if let Some(parent_id) = m.0.parent_id {
                let msg_id = m.0.id;
                let entry = acc.entry(parent_id).or_insert(msg_id);
                if msg_id > *entry {
                    *entry = msg_id;
                }
            }
            acc
        });
    println!("parent_max_child: {:?}", parent_max_child);

    let max_child_ids: HashSet<i64> = parent_max_child.values().cloned().collect();
    println!("max_child_ids: {:?}", max_child_ids);

    let assistant_id = conversation.assistant_id.unwrap();
    let assistant_detail = get_assistant(app_handle.clone(), assistant_id).unwrap();

    if assistant_detail.model.is_empty() {
        return Err(AppError::NoModelFound);
    }

    let init_message_list = messages
        .into_iter()
        .filter_map(|m: (Message, Option<MessageAttachment>)| {
            if m.0.id >= message_id {
                return None::<(String, String, Vec<MessageAttachment>)>;
            }

            if parent_ids.contains(&m.0.id) {
                // 这是一个父消息，保留它
                Some((m.0.message_type, m.0.content, vec![]))
            } else if max_child_ids.contains(&m.0.id) {
                // 这是一个子消息，并且是最大 id 的子消息，保留它
                Some((m.0.message_type, m.0.content, vec![]))
            } else {
                // 其他情况，过滤掉
                None
            }
        })
        .collect::<Vec<_>>();
    println!("init_message_list: {:?}", init_message_list);

    let (tx, mut rx) = mpsc::channel(100);

    let app_handle_clone = app_handle.clone();
    let new_message = add_message(
        &app_handle_clone,
        Some(message_id),
        conversation_id,
        "assistant".to_string(),
        String::new(),
        Some(assistant_detail.model[0].id),
        Some(assistant_detail.model[0].model_code.clone()),
        None,
        None,
        0,
    )?;
    let new_message_id = new_message.id;

    let cancel_token = CancellationToken::new();
    message_token_manager
        .store_token(new_message_id, cancel_token.clone())
        .await;

    let tokens = message_token_manager.get_tokens();
    tokio::spawn(async move {
        let db = LLMDatabase::new(&app_handle_clone)
            .map_err(Error::from)
            .context("Failed to create LLMDatabase")?;
        let conversation_db = ConversationDatabase::new(&app_handle_clone).unwrap();
        let provider_id = &assistant_detail.model[0].provider_id;
        let model_code = &assistant_detail.model[0].model_code;
        let model_detail = db
            .get_llm_model_detail(provider_id, model_code)
            .context("Failed to get LLM model detail")?;

        // Create genai client with custom config
        let client = create_client_with_config(
            &model_detail.configs, 
            &model_detail.model.code, 
            &model_detail.provider.api_type
        )?;

        let model_config_clone = merge_model_configs(
            assistant_detail.model_configs.clone(),
            &model_detail,
            None,
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

        // Extract model name from config
        let model_name = config_map
            .get("model")
            .cloned()
            .unwrap_or_else(|| model_detail.model.code.clone());

        // Convert messages to ChatMessage format and build chat options
        let chat_messages = build_chat_messages(&init_message_list);
        let chat_request = ChatRequest::new(chat_messages);
        let chat_options = build_chat_options(&config_map);

        if stream {
            // 使用 genai 流式处理
            match client.exec_chat_stream(&model_name, chat_request, Some(&chat_options)).await {
                Ok(chat_stream_response) => {
                    let mut chat_stream = chat_stream_response.stream;
                    let mut full_content = String::new();
                    
                    loop {
                        tokio::select! {
                            stream_result = chat_stream.next() => {
                                match stream_result {
                                    Some(Ok(stream_event)) => {
                                        use genai::chat::ChatStreamEvent;
                                        match stream_event {
                                            ChatStreamEvent::Start => {
                                                // 流开始，更新开始时间
                                                conversation_db
                                                    .message_repo()
                                                    .unwrap()
                                                    .update_start_time(new_message_id)
                                                    .unwrap();
                                            }
                                            ChatStreamEvent::Chunk(chunk) => {
                                                // 接收到内容块
                                                full_content.push_str(&chunk.content);
                                                if let Err(e) = tx.send((new_message_id, full_content.clone(), false)).await {
                                                    eprintln!("Failed to send chunk: {}", e);
                                                    break;
                                                }
                                            }
                                            ChatStreamEvent::ReasoningChunk(reasoning_chunk) => {
                                                // 推理内容块 (如 o1 模型的推理过程)
                                                full_content.push_str(&reasoning_chunk.content);
                                                if let Err(e) = tx.send((new_message_id, full_content.clone(), false)).await {
                                                    eprintln!("Failed to send reasoning chunk: {}", e);
                                                    break;
                                                }
                                            }
                                            ChatStreamEvent::ToolCallChunk(_tool_chunk) => {
                                                // 工具调用块，暂时忽略
                                            }
                                            ChatStreamEvent::End(stream_end) => {
                                                // 流结束
                                                if let Some(captured_text) = stream_end.captured_first_text() {
                                                    full_content = captured_text.to_string();
                                                }
                                                
                                                conversation_db
                                                    .message_repo()
                                                    .unwrap()
                                                    .update_finish_time(new_message_id)
                                                    .unwrap();
                                                
                                                tx.send((new_message_id, full_content.clone(), true)).await.unwrap();
                                                break;
                                            }
                                        }
                                    },
                                    Some(Err(e)) => {
                                        eprintln!("Stream error: {}", e);
                                        cleanup_token(&tokens, new_message_id).await;
                                        let err_msg = format!("Chat stream error: {}", e);
                                        tx.send((new_message_id, err_msg, true)).await.unwrap();
                                        break;
                                    },
                                    None => {
                                        // 流意外结束
                                        conversation_db
                                            .message_repo()
                                            .unwrap()
                                            .update_finish_time(new_message_id)
                                            .unwrap();
                                        tx.send((new_message_id, full_content.clone(), true)).await.unwrap();
                                        break;
                                    }
                                }
                            }
                            _ = cancel_token.cancelled() => {
                                println!("Chat stream cancelled");
                                break;
                            }
                        }
                    }
                },
                Err(e) => {
                    cleanup_token(&tokens, new_message_id).await;
                    let err_msg = format!("Chat stream error: {}", e);
                    tx.send((new_message_id, err_msg, true)).await.unwrap();
                    eprintln!("Chat stream error: {}", e);
                }
            }
        } else {
            conversation_db
                .message_repo()
                .unwrap()
                .update_start_time(new_message_id)
                .unwrap();

            // Use genai non-streaming
            let chat_result = tokio::select! {
                result = client.exec_chat(&model_name, chat_request, Some(&chat_options)) => result,
                _ = cancel_token.cancelled() => {
                    cleanup_token(&tokens, new_message_id).await;
                    return Err(anyhow::anyhow!("Request cancelled"));
                }
            };

            match chat_result {
                Ok(chat_response) => {
                    let content = chat_response.first_text().unwrap_or("").to_string();

                    conversation_db
                        .message_repo()
                        .unwrap()
                        .update_finish_time(new_message_id)
                        .unwrap();
                    tx.send((new_message_id, content.clone(), true))
                        .await
                        .unwrap();
                    // Ensure tx is closed after sending the message
                    drop(tx);
                }
                Err(e) => {
                    cleanup_token(&tokens, new_message_id).await;
                    let err_msg = format!("Chat error: {}", e);
                    tx.send((new_message_id, err_msg, true)).await.unwrap();
                    eprintln!("Chat error: {}", e);
                }
            }
        }

        Ok::<(), Error>(())
    });

    let app_handle_clone = app_handle.clone();
    let tokens = message_token_manager.get_tokens();
    let window_clone = window.clone();
    tokio::spawn(async move {
        loop {
            match timeout(Duration::from_secs(600), rx.recv()).await {
                Ok(Some((id, content, done))) => {
                    println!("Received data: id={}, content={}", id, content);
                    window_clone
                        .emit(format!("message_{}", id).as_str(), content.clone())
                        .map_err(|e| e.to_string())
                        .unwrap();

                    if done {
                        let conversation_db = ConversationDatabase::new(&app_handle_clone)
                            .map_err(|e: rusqlite::Error| e.to_string())
                            .unwrap();

                        let mut message = conversation_db
                            .message_repo()
                            .unwrap()
                            .read(new_message_id)
                            .unwrap()
                            .unwrap();
                        message.content = content.clone().to_string();
                        conversation_db
                            .message_repo()
                            .unwrap()
                            .update(&message)
                            .unwrap();

                        println!("Message finish: id={}", id);
                        window_clone
                            .emit(
                                format!("message_{}", id).as_str(),
                                "Tea::Event::MessageFinish",
                            )
                            .map_err(|e| e.to_string())
                            .unwrap();

                        cleanup_token(&tokens, new_message_id).await;
                        break;
                    }
                }
                Ok(None) => {
                    cleanup_token(&tokens, new_message_id).await;
                    println!("Channel closed");
                    break;
                }
                Err(err) => {
                    cleanup_token(&tokens, new_message_id).await;
                    println!("Timeout waiting for data from channel: {:?}", err);
                    break;
                }
            }
        }
    });

    Ok(AiResponse {
        conversation_id,
        add_message_id: new_message_id,
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
    let db = get_conversation_db(app_handle)?;

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
            let add_message = add_message(
                app_handle,
                None,
                conversation.id,
                "assistant".to_string(),
                String::new(),
                Some(assistant_detail.model[0].id),
                Some(assistant_detail.model[0].model_code.clone()),
                None,
                None,
                0,
            )?;
            (
                conversation.id,
                Some(add_message.id),
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
            )?;
            let mut updated_message_list = message_list;
            updated_message_list.push((
                String::from("user"),
                request_prompt_result_with_context.clone(),
                message_attachment_list,
            ));

            let add_assistant_message = add_message(
                app_handle,
                None,
                conversation_id,
                "assistant".to_string(),
                String::new(),
                Some(assistant_detail.model[0].id),
                Some(assistant_detail.model[0].model_code.clone()),
                None,
                None,
                0,
            )?;
            (
                conversation_id,
                Some(add_assistant_message.id),
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

        let db = get_llm_db(app_handle)?;
        let model_detail = db.get_llm_model_detail(&provider_id, &model_code).unwrap();

        // Create genai client with custom config
        let client = create_client_with_config(
            &model_detail.configs, 
            &model_detail.model.code, 
            &model_detail.provider.api_type
        )?;

        // Convert messages to ChatMessage format
        let chat_messages = vec![ChatMessage::system(&prompt), ChatMessage::user(&context)];
        let chat_request = ChatRequest::new(chat_messages);

        // Use model code as model name
        let model_name = &model_detail.model.code;

        let response = client
            .exec_chat(model_name, chat_request, None)
            .await
            .map(|chat_response| chat_response.first_text().unwrap_or("").to_string())
            .map_err(|e| e.to_string());
        match response {
            Err(e) => {
                println!("Chat error: {}", e);
                let _ = window.emit(
                    ERROR_NOTIFICATION_EVENT,
                    "生成对话标题失败，请检查配置",
                );
            }
            Ok(response_text) => {
                println!("Chat content: {}", response_text.clone());

                let conversation_db = get_conversation_db(app_handle)?;
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

fn get_conversation_db(app_handle: &tauri::AppHandle) -> Result<ConversationDatabase, AppError> {
    ConversationDatabase::new(app_handle).map_err(AppError::from)
}

fn get_llm_db(app_handle: &tauri::AppHandle) -> Result<LLMDatabase, AppError> {
    LLMDatabase::new(app_handle).map_err(AppError::from)
}
