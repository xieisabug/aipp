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

use futures::StreamExt;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, ContentPart};
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{adapter::AdapterKind, Client, ModelIden, ServiceTarget};

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
    (
        AdapterKind::Gemini,
        "https://generativelanguage.googleapis.com/v1beta",
    ),
    (AdapterKind::Groq, "https://api.groq.com/openai/v1"),
    (AdapterKind::Xai, "https://api.x.ai/v1"),
    (AdapterKind::DeepSeek, "https://api.deepseek.com/"),
    (AdapterKind::Ollama, "http://localhost:11434"),
];

/// AI聊天配置
#[derive(Debug, Clone)]
struct ChatConfig {
    model_name: String,
    stream: bool,
    chat_options: ChatOptions,
    client: Client,
}

/// 聊天上下文
#[derive(Debug)]
struct ChatContext {
    conversation_id: i64,
    message_id: i64,
    need_generate_title: bool,
    request_prompt_result: String,
}

/// 配置构建器
struct ConfigBuilder;

impl ConfigBuilder {
    /// 创建客户端配置
    fn create_client_with_config(
        configs: &[crate::db::llm_db::LLMProviderConfig],
        model_name: &str,
        api_type: &str,
    ) -> Result<Client, AppError> {
        let adapter_kind = Self::infer_adapter_kind(model_name, api_type);

        let mut api_key = String::new();
        let mut endpoint_opt: Option<String> = None;

        for config in configs {
            match config.name.as_str() {
                "api_key" => {
                    api_key = config.value.clone();
                }
                "endpoint" => {
                    endpoint_opt = Some(config.value.clone());
                }
                _ => {}
            }
        }

        // 克隆值以便在闭包中使用
        let api_key_clone = api_key.clone();
        let endpoint_clone = endpoint_opt.clone();

        // 使用 ServiceTargetResolver 来配置端点和认证
        let target_resolver = ServiceTargetResolver::from_resolver_fn(
            move |service_target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
                let ServiceTarget { model, .. } = service_target;

                let endpoint = if let Some(ref ep) = endpoint_clone {
                    Endpoint::from_owned(ep.trim_end_matches('/').to_string())
                } else {
                    let default_endpoint = Self::get_default_endpoint(adapter_kind);
                    Endpoint::from_static(default_endpoint)
                };

                let auth = AuthData::from_single(api_key_clone.clone());
                let model = ModelIden::new(adapter_kind, model.model_name);

                Ok(ServiceTarget {
                    endpoint,
                    auth,
                    model,
                })
            },
        );

        let client = Client::builder()
            .with_service_target_resolver(target_resolver)
            .build();

        Ok(client)
    }

    /// 推断适配器类型
    fn infer_adapter_kind(model_name: &str, api_type: &str) -> AdapterKind {
        match api_type.to_lowercase().as_str() {
            "openai" => AdapterKind::OpenAI,
            "anthropic" => AdapterKind::Anthropic,
            "cohere" => AdapterKind::Cohere,
            "gemini" => AdapterKind::Gemini,
            "groq" => AdapterKind::Groq,
            "xai" => AdapterKind::Xai,
            "deepseek" => AdapterKind::DeepSeek,
            "ollama" => AdapterKind::Ollama,
            _ => {
                // 根据模型名称推断
                let model_lower = model_name.to_lowercase();
                if model_lower.contains("gpt") || model_lower.contains("o1") {
                    AdapterKind::OpenAI
                } else if model_lower.contains("claude") {
                    AdapterKind::Anthropic
                } else if model_lower.contains("gemini") {
                    AdapterKind::Gemini
                } else if model_lower.contains("llama") || model_lower.contains("qwen") {
                    AdapterKind::Ollama
                } else {
                    AdapterKind::OpenAI // 默认
                }
            }
        }
    }

    /// 获取默认端点
    fn get_default_endpoint(adapter_kind: AdapterKind) -> &'static str {
        DEFAULT_ENDPOINTS
            .iter()
            .find(|(kind, _)| *kind == adapter_kind)
            .map(|(_, endpoint)| *endpoint)
            .unwrap_or("https://api.openai.com/v1")
    }

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
fn build_chat_messages(
    init_message_list: &[(String, String, Vec<MessageAttachment>)],
) -> Vec<ChatMessage> {
    let mut chat_messages = Vec::new();
    
    for (role, content, attachments) in init_message_list {
        // 如果没有附件，使用简单的文本消息
        if attachments.is_empty() {
            match role.as_str() {
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
                    if let Some(url) = &attachment.attachment_url {
                        // 推断图像的媒体类型
                        let media_type = infer_media_type_from_url(url);
                        content_parts.push(ContentPart::from_image_url(&media_type, url));
                    } else if let Some(content) = &attachment.attachment_content {
                        // 如果没有URL但有内容（可能是base64），作为文本处理
                        content_parts.push(ContentPart::from_text(&format!(
                            "\n\n[图像附件内容]\n{}", content
                        )));
                    }
                },
                crate::db::conversation_db::AttachmentType::Text => {
                    // 文本附件
                    if let Some(attachment_content) = &attachment.attachment_content {
                        let file_name = attachment.attachment_url.as_deref().unwrap_or("未知文件");
                        content_parts.push(ContentPart::from_text(&format!(
                            "\n\n[文本附件: {}]\n{}", file_name, attachment_content
                        )));
                    }
                },
                crate::db::conversation_db::AttachmentType::PDF |
                crate::db::conversation_db::AttachmentType::Word |
                crate::db::conversation_db::AttachmentType::PowerPoint |
                crate::db::conversation_db::AttachmentType::Excel => {
                    // 其他文档类型，作为文本内容处理
                    if let Some(attachment_content) = &attachment.attachment_content {
                        let file_name = attachment.attachment_url.as_deref().unwrap_or("未知文档");
                        let file_type = match attachment.attachment_type {
                            crate::db::conversation_db::AttachmentType::PDF => "PDF文档",
                            crate::db::conversation_db::AttachmentType::Word => "Word文档",
                            crate::db::conversation_db::AttachmentType::PowerPoint => "PowerPoint文档",
                            crate::db::conversation_db::AttachmentType::Excel => "Excel文档",
                            _ => "文档",
                        };
                        content_parts.push(ContentPart::from_text(&format!(
                            "\n\n[{}: {}]\n{}", file_type, file_name, attachment_content
                        )));
                    }
                },
            }
        }

        // 创建包含多个内容部分的消息
        match role.as_str() {
            "system" => {
                // 系统消息通常不支持多媒体内容，将所有内容合并为文本
                let combined_text = content_parts.iter()
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
            },
            "user" => {
                chat_messages.push(ChatMessage::user(content_parts));
            },
            "assistant" => {
                // 助手消息也通常是纯文本，将内容合并
                let combined_text = content_parts.iter()
                    .map(|_| content.clone()) // 临时处理
                    .collect::<Vec<_>>()
                    .join("");
                chat_messages.push(ChatMessage::assistant(&combined_text));
            },
            _ => {}
        }
    }
    
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

/// 清理消息令牌
async fn cleanup_token(
    tokens: &Arc<tokio::sync::Mutex<HashMap<i64, CancellationToken>>>,
    message_id: i64,
) {
    let mut map = tokens.lock().await;
    map.remove(&message_id);
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
    chat_request: ChatRequest,
    chat_options: &ChatOptions,
    message_id: i64,
    tx: &mpsc::Sender<(i64, String, bool)>,
    cancel_token: &CancellationToken,
    conversation_db: &ConversationDatabase,
    tokens: &Arc<tokio::sync::Mutex<HashMap<i64, CancellationToken>>>,
) -> Result<(), anyhow::Error> {
    match client
        .exec_chat_stream(model_name, chat_request, Some(chat_options))
        .await
    {
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
                                        conversation_db
                                            .message_repo()
                                            .unwrap()
                                            .update_start_time(message_id)
                                            .unwrap();
                                    }
                                    ChatStreamEvent::Chunk(chunk) => {
                                        full_content.push_str(&chunk.content);
                                                                            if let Err(e) = tx.send((message_id, full_content.clone(), false)).await {
                                        eprintln!("Failed to send chunk: {}", e);
                                        return Ok(());
                                    }
                                    }
                                    ChatStreamEvent::ReasoningChunk(reasoning_chunk) => {
                                        full_content.push_str(&reasoning_chunk.content);
                                                                            if let Err(e) = tx.send((message_id, full_content.clone(), false)).await {
                                        eprintln!("Failed to send reasoning chunk: {}", e);
                                        return Ok(());
                                    }
                                    }
                                    ChatStreamEvent::ToolCallChunk(_tool_chunk) => {
                                        // 工具调用块，暂时忽略
                                    }
                                    ChatStreamEvent::End(stream_end) => {
                                        if let Some(captured_text) = stream_end.captured_first_text() {
                                            full_content = captured_text.to_string();
                                        }

                                        conversation_db
                                            .message_repo()
                                            .unwrap()
                                            .update_finish_time(message_id)
                                            .unwrap();

                                        tx.send((message_id, full_content.clone(), true)).await.unwrap();
                                        return Ok(());
                                    }
                                }
                            },
                            Some(Err(e)) => {
                                eprintln!("Stream error: {}", e);
                                cleanup_token(tokens, message_id).await;
                                let err_msg = format!("Chat stream error: {}", e);
                                tx.send((message_id, err_msg, true)).await.unwrap();
                                return Err(anyhow::anyhow!("Stream error: {}", e));
                            },
                            None => {
                                conversation_db
                                    .message_repo()
                                    .unwrap()
                                    .update_finish_time(message_id)
                                    .unwrap();
                                tx.send((message_id, full_content.clone(), true)).await.unwrap();
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
            cleanup_token(tokens, message_id).await;
            let err_msg = format!("Chat stream error: {}", e);
            tx.send((message_id, err_msg, true)).await.unwrap();
            eprintln!("Chat stream error: {}", e);
            return Err(anyhow::anyhow!("Chat stream error: {}", e));
        }
    }
}

/// 处理非流式聊天
async fn handle_non_stream_chat(
    client: &Client,
    model_name: &str,
    chat_request: ChatRequest,
    chat_options: &ChatOptions,
    message_id: i64,
    tx: &mpsc::Sender<(i64, String, bool)>,
    cancel_token: &CancellationToken,
    conversation_db: &ConversationDatabase,
    tokens: &Arc<tokio::sync::Mutex<HashMap<i64, CancellationToken>>>,
) -> Result<(), anyhow::Error> {
    conversation_db
        .message_repo()
        .unwrap()
        .update_start_time(message_id)
        .unwrap();

    let chat_result = tokio::select! {
        result = client.exec_chat(model_name, chat_request, Some(chat_options)) => result,
        _ = cancel_token.cancelled() => {
            cleanup_token(tokens, message_id).await;
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
            Ok(())
        }
        Err(e) => {
            cleanup_token(tokens, message_id).await;
            let err_msg = format!("Chat error: {}", e);
            tx.send((message_id, err_msg, true)).await.unwrap();
            eprintln!("Chat error: {}", e);
            Err(anyhow::anyhow!("Chat error: {}", e))
        }
    }
}

/// 处理聊天响应消息
async fn handle_chat_response(
    app_handle: tauri::AppHandle,
    window: tauri::Window,
    chat_context: ChatContext,
    config_feature_map: HashMap<String, HashMap<String, FeatureConfig>>,
    tokens: Arc<tokio::sync::Mutex<HashMap<i64, CancellationToken>>>,
    mut rx: mpsc::Receiver<(i64, String, bool)>,
) {
    loop {
        match timeout(Duration::from_secs(600), rx.recv()).await {
            Ok(Some((id, content, done))) => {
                println!("Received data: id={}, content={}", id, content);
                window
                    .emit(format!("message_{}", id).as_str(), content.clone())
                    .map_err(|e| e.to_string())
                    .unwrap();

                if done {
                    let conversation_db = ConversationDatabase::new(&app_handle)
                        .map_err(|e: rusqlite::Error| e.to_string())
                        .unwrap();

                    let mut message = conversation_db
                        .message_repo()
                        .unwrap()
                        .read(chat_context.message_id)
                        .unwrap()
                        .unwrap();
                    message.content = content.clone().to_string();
                    conversation_db
                        .message_repo()
                        .unwrap()
                        .update(&message)
                        .unwrap();

                    println!("Message finish: id={}", id);
                    window
                        .emit(format!("message_{}", id).as_str(), MESSAGE_FINISH_EVENT)
                        .map_err(|e| e.to_string())
                        .unwrap();

                    if chat_context.need_generate_title {
                        generate_title(
                            &app_handle,
                            chat_context.conversation_id,
                            chat_context.request_prompt_result.clone(),
                            content.clone().to_string(),
                            config_feature_map.clone(),
                            window.clone(),
                        )
                        .await
                        .map_err(|e| e.to_string())
                        .unwrap();
                    }
                    cleanup_token(&tokens, chat_context.message_id).await;
                    break;
                }
            }
            Ok(None) => {
                cleanup_token(&tokens, chat_context.message_id).await;
                println!("Channel closed");
                break;
            }
            Err(err) => {
                cleanup_token(&tokens, chat_context.message_id).await;
                println!("Timeout waiting for data from channel: {:?}", err);
                break;
            }
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
    println!(
        "ask_ai: {:?}, override_model_config: {:?}, override_prompt: {:?}",
        request, override_model_config, override_prompt
    );
    let template_engine = TemplateEngine::new();
    let mut template_context = HashMap::new();
    let (tx, rx) = mpsc::channel(100);

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
        let request_prompt_result_with_context_clone = request_prompt_result_with_context.clone();

        let app_handle_clone = app_handle.clone();

        let cancel_token = CancellationToken::new();
        let message_id = new_message_id.unwrap();
        message_token_manager
            .store_token(new_message_id.unwrap(), cancel_token.clone())
            .await;

        // 在异步任务外获取模型详情（避免线程安全问题）
        let llm_db = LLMDatabase::new(&app_handle).map_err(AppError::from)?;
        let provider_id = &assistant_detail.model[0].provider_id;
        let model_code = &assistant_detail.model[0].model_code;
        let model_detail = llm_db
            .get_llm_model_detail(provider_id, model_code)
            .context("Failed to get LLM model detail")?;

        let tokens = message_token_manager.get_tokens();
        tokio::spawn(async move {
            // 直接创建数据库连接（避免线程安全问题）
            let conversation_db = ConversationDatabase::new(&app_handle_clone).unwrap();

            // 构建聊天配置
            let client = ConfigBuilder::create_client_with_config(
                &model_detail.configs,
                &model_detail.model.code,
                &model_detail.provider.api_type,
            )?;

            let model_config_clone = ConfigBuilder::merge_model_configs(
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

            let model_name = config_map
                .get("model")
                .cloned()
                .unwrap_or_else(|| model_detail.model.code.clone());

            let chat_options = ConfigBuilder::build_chat_options(&config_map);

            let chat_config = ChatConfig {
                model_name,
                stream,
                chat_options,
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
                    chat_request,
                    &chat_config.chat_options,
                    message_id,
                    &tx,
                    &cancel_token,
                    &conversation_db,
                    &tokens,
                )
                .await?;
            } else {
                // Use genai non-streaming
                handle_non_stream_chat(
                    &chat_config.client,
                    &chat_config.model_name,
                    chat_request,
                    &chat_config.chat_options,
                    message_id,
                    &tx,
                    &cancel_token,
                    &conversation_db,
                    &tokens,
                )
                .await?;
            }

            Ok::<(), Error>(())
        });

        let app_handle_clone = app_handle.clone();
        let tokens = message_token_manager.get_tokens();
        let window_clone = window.clone();

        // 创建 ChatContext
        let chat_context = ChatContext {
            conversation_id,
            message_id: new_message_id.unwrap(),
            need_generate_title,
            request_prompt_result: request_prompt_result_with_context_clone,
        };

        tokio::spawn(async move {
            handle_chat_response(
                app_handle_clone,
                window_clone,
                chat_context,
                config_feature_map.clone(),
                tokens,
                rx,
            )
            .await;
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

    let (tx, rx) = mpsc::channel(100);

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

    // 在异步任务外获取模型详情（避免线程安全问题）
    let llm_db = LLMDatabase::new(&app_handle).map_err(AppError::from)?;
    let provider_id = &assistant_detail.model[0].provider_id;
    let model_code = &assistant_detail.model[0].model_code;
    let model_detail = llm_db
        .get_llm_model_detail(provider_id, model_code)
        .context("Failed to get LLM model detail")?;

    let tokens = message_token_manager.get_tokens();
    tokio::spawn(async move {
        // 直接创建数据库连接（避免线程安全问题）
        let conversation_db = ConversationDatabase::new(&app_handle_clone).unwrap();

        // 构建聊天配置
        let client = ConfigBuilder::create_client_with_config(
            &model_detail.configs,
            &model_detail.model.code,
            &model_detail.provider.api_type,
        )?;

        let model_config_clone = ConfigBuilder::merge_model_configs(
            assistant_detail.model_configs.clone(),
            &model_detail,
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
            .unwrap_or_else(|| model_detail.model.code.clone());

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
                chat_request,
                &chat_config.chat_options,
                new_message_id,
                &tx,
                &cancel_token,
                &conversation_db,
                &tokens,
            )
            .await?;
        } else {
            // Use genai non-streaming
            handle_non_stream_chat(
                &chat_config.client,
                &chat_config.model_name,
                chat_request,
                &chat_config.chat_options,
                new_message_id,
                &tx,
                &cancel_token,
                &conversation_db,
                &tokens,
            )
            .await?;
        }

        Ok::<(), Error>(())
    });

    let app_handle_clone = app_handle.clone();
    let tokens = message_token_manager.get_tokens();
    let window_clone = window.clone();

    // 创建 ChatContext
    let chat_context = ChatContext {
        conversation_id,
        message_id: new_message_id,
        need_generate_title: false,
        request_prompt_result: String::new(),
    };

    tokio::spawn(async move {
        handle_chat_response(
            app_handle_clone,
            window_clone,
            chat_context,
            HashMap::new(),
            tokens,
            rx,
        )
        .await;
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

        // 直接创建数据库连接
        let llm_db = LLMDatabase::new(app_handle).map_err(AppError::from)?;
        let model_detail = llm_db
            .get_llm_model_detail(&provider_id, &model_code)
            .unwrap();

        // Create genai client with custom config
        let client = ConfigBuilder::create_client_with_config(
            &model_detail.configs,
            &model_detail.model.code,
            &model_detail.provider.api_type,
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
