use super::assistant_api::AssistantDetail;
use crate::api::ai::chat::{
    handle_non_stream_chat as ai_handle_non_stream_chat,
    handle_stream_chat as ai_handle_stream_chat,
};
use crate::api::ai::config::{ChatConfig, ConfigBuilder};
use crate::api::ai::conversation::{build_chat_messages, build_chat_messages_with_context, init_conversation};
use crate::api::ai::events::{ConversationEvent, MessageAddEvent, MessageUpdateEvent};
use crate::api::ai::mcp::{collect_mcp_info_for_assistant, format_mcp_prompt};
use crate::api::ai::title::generate_title;
use crate::api::ai::types::{AiRequest, AiResponse};
use crate::api::assistant_api::get_assistant;
use crate::api::genai_client;
use crate::db::conversation_db::{AttachmentType, Repository};
use crate::db::conversation_db::{ConversationDatabase, Message, MessageAttachment};
use crate::db::llm_db::LLMDatabase;
use crate::errors::AppError;
use crate::state::message_token::MessageTokenManager;
use crate::template_engine::TemplateEngine;
use crate::{AppState, FeatureConfigState};
use anyhow::Context;
use anyhow::Error;
use genai::chat::ChatRequest;
use std::collections::{HashMap, HashSet};
use tauri::Emitter;
use tauri::State;
use tokio_util::sync::CancellationToken;

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
        "ask_ai - [[request]]: {:#?}\n[[override_model_config]]: {:#?}\n[[override_prompt]]: {:#?}\n",
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
    println!("[[assistant_prompt_result]]: {}\n", assistant_prompt_result);

    if assistant_detail.model.is_empty() {
        return Err(AppError::NoModelFound);
    }

    // 收集 MCP 信息
    let mcp_info = collect_mcp_info_for_assistant(&app_handle, request.assistant_id).await?;
    println!(
        "[[MCP enabled_servers]]: {} [[native_toolcall]]: {}\n",
        mcp_info.enabled_servers.len(),
        mcp_info.use_native_toolcall
    );
    let is_native_toolcall = mcp_info.use_native_toolcall;

    // 注意：是否使用提供商原生 toolcall 在后续构造 ChatRequest 时再判断
    let assistant_prompt_result =
        if mcp_info.enabled_servers.len() > 0 && !mcp_info.use_native_toolcall {
            let mcp_formatted_prompt = format_mcp_prompt(assistant_prompt_result, &mcp_info).await;
            println!("[[MCP formatted_prompt]]: {}\n", mcp_formatted_prompt);
            mcp_formatted_prompt
        } else {
            assistant_prompt_result
        };

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

    // 非原生 toolcall 时，将历史中的 tool_result 在“发送给 LLM 的消息”里当作用户消息。
    // 注意：DB 与 UI 不变，仅用于请求时的上下文构造。
    let final_message_list_for_llm: Vec<(String, String, Vec<MessageAttachment>)> = if is_native_toolcall {
        init_message_list.clone()
    } else {
        init_message_list
            .iter()
            .map(|(message_type, content, attachments)| {
                if message_type == "tool_result" {
                    (String::from("user"), content.clone(), Vec::new())
                } else {
                    (message_type.clone(), content.clone(), attachments.clone())
                }
            })
            .collect()
    };

    // 总是启动流式处理，即使没有预先创建消息
    let _config_feature_map = feature_config_state.config_feature_map.lock().await.clone();
    let _request_prompt_result_with_context_clone = request_prompt_result_with_context.clone();

    let app_handle_clone = app_handle.clone();

    let cancel_token = CancellationToken::new();

    message_token_manager
        .store_token(conversation_id, cancel_token.clone())
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
    
    let task_handle = tokio::spawn(async move {
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
                llm_provider_id: 0,         // 临时值
                description: String::new(), // 临时值
                vision_support: false,      // 临时值
                audio_support: false,       // 临时值
                video_support: false,       // 临时值
            },
            provider: crate::db::llm_db::LLMProvider {
                id: 0,               // 临时值
                name: String::new(), // 临时值
                api_type: provider_api_type.clone(),
                description: String::new(), // 临时值
                is_official: false,         // 临时值
                is_enabled: true,           // 临时值
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
            "[[model_name]]: {} [[stream]]: {}\n",
            chat_config.model_name, chat_config.stream
        );

        // 将消息转换为 ChatMessage（已按是否原生 toolcall 处理过 tool_result）
        let chat_messages = build_chat_messages(&final_message_list_for_llm);
        let chat_request = ChatRequest::new(chat_messages);

        if chat_config.stream {
            // 使用 genai 流式处理
            ai_handle_stream_chat(
                &chat_config.client,
                &chat_config.model_name,
                &chat_request,
                &chat_config.chat_options,
                conversation_id,
                &cancel_token,
                &conversation_db,
                &tokens,
                &window_clone,
                &app_handle_clone,
                _need_generate_title,
                request.prompt.clone(),
                _config_feature_map.clone(),
                None,               // 普通ask_ai不需要复用generation_group_id
                None,               // 普通ask_ai不需要parent_group_id
                model_id,           // 传递模型ID
                model_code.clone(), // 传递模型名称
            )
            .await?;
        } else {
            // Use genai non-streaming
            ai_handle_non_stream_chat(
                &chat_config.client,
                &chat_config.model_name,
                &chat_request,
                &chat_config.chat_options,
                conversation_id,
                &cancel_token,
                &conversation_db,
                &tokens,
                &window_clone,
                &app_handle_clone,
                _need_generate_title,
                request.prompt.clone(),
                _config_feature_map.clone(),
                None,               // 普通ask_ai不需要复用generation_group_id
                None,               // 普通ask_ai不需要parent_group_id
                model_id,           // 传递模型ID
                model_code.clone(), // 传递模型名称
            )
            .await?;
        }

        Ok::<(), Error>(())
    });
    
    // 等待任务完成并处理错误
    if let Err(join_error) = task_handle.await {
        eprintln!("[[task_join_error]]: {}\n", join_error);
        return Err(AppError::InternalError(format!("任务执行失败: {}", join_error)));
    }

    println!("================================ Ask AI End ===============================================");

    Ok(AiResponse {
        conversation_id,
        request_prompt_result_with_context,
    })
}

#[tauri::command]
pub async fn tool_result_continue_ask_ai(
    app_handle: tauri::AppHandle,
    _state: State<'_, AppState>,
    _feature_config_state: State<'_, FeatureConfigState>,
    message_token_manager: State<'_, MessageTokenManager>,
    window: tauri::Window,
    conversation_id: String,
    assistant_id: i64,
    tool_call_id: String,
    tool_result: String,
) -> Result<AiResponse, AppError> {
    println!("================================ Tool Result Continue AI Start ===============================================");
    println!(
        "[[conversation_id]]: {}\n[[assistant_id]]: {}\n[[tool_call_id]]: {}\n[[tool_result]]: {}\n",
        conversation_id, assistant_id, tool_call_id, tool_result
    );

    let conversation_id_i64 = conversation_id.parse::<i64>()?;
    let db = ConversationDatabase::new(&app_handle).map_err(AppError::from)?;
    
    // Get conversation details (validate exists)
    let _conversation = db
        .conversation_repo()
        .unwrap()
        .read(conversation_id_i64)
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::DatabaseError("对话未找到".to_string()))?;

    // Get assistant details  
    let assistant_detail = get_assistant(app_handle.clone(), assistant_id).unwrap();
    if assistant_detail.model.is_empty() {
        return Err(AppError::NoModelFound);
    }

    // Create tool_result message in database
    let tool_result_content = format!(
        "Tool execution completed:\n\nTool Call ID: {}\nResult:\n{}",
        tool_call_id,
        tool_result
    );

    let _tool_result_message = add_message(
        &app_handle,
        None,
        conversation_id_i64,
        "tool_result".to_string(),
        tool_result_content,
        Some(assistant_detail.model[0].id),
        Some(assistant_detail.model[0].model_code.clone()),
        Some(chrono::Utc::now()),
        Some(chrono::Utc::now()),
        0,
        None,
        None,
    )?;

    // Get all existing messages
    let all_messages = db
        .message_repo()
        .unwrap()
        .list_by_conversation_id(conversation_id_i64)?;

    // Build message list with latest children (same logic as ask_ai)
    let mut latest_children: HashMap<i64, (Message, Option<MessageAttachment>)> = HashMap::new();
    let mut child_ids: HashSet<i64> = HashSet::new();

    for (message, attachment) in all_messages.iter() {
        if let Some(parent_id) = message.parent_id {
            child_ids.insert(message.id);
            latest_children
                .entry(parent_id)
                .and_modify(|e| *e = (message.clone(), attachment.clone()))
                .or_insert((message.clone(), attachment.clone()));
        }
    }

    // Build final message list including the new tool_result message
    let init_message_list: Vec<(String, String, Vec<MessageAttachment>)> = all_messages
        .into_iter()
        .filter(|(message, _)| !child_ids.contains(&message.id))
        .map(|(message, attachment)| {
            let (final_message, final_attachment) = latest_children
                .get(&message.id)
                .map(|child| child.clone())
                .unwrap_or((message, attachment));

            (
                final_message.message_type,
                final_message.content,
                final_attachment.map(|a| vec![a]).unwrap_or_else(Vec::new),
            )
        })
        .collect();

    println!("[[init_message_list (tool_result_continue)]]: {:#?}\n", init_message_list);

    // 收集 MCP 信息
    let mcp_info = collect_mcp_info_for_assistant(&app_handle, assistant_id).await?;
    println!(
        "[[MCP enabled_servers]]: {} [[native_toolcall]]: {}\n",
        mcp_info.enabled_servers.len(),
        mcp_info.use_native_toolcall
    );
    let is_native_toolcall = mcp_info.use_native_toolcall;

    let cancel_token = CancellationToken::new();
    message_token_manager
        .store_token(conversation_id_i64, cancel_token.clone())
        .await;

    // Get model details (same as ask_ai)
    let llm_db = LLMDatabase::new(&app_handle).map_err(AppError::from)?;
    let provider_id = &assistant_detail.model[0].provider_id;
    let model_code = &assistant_detail.model[0].model_code;
    let model_detail = llm_db
        .get_llm_model_detail(provider_id, model_code)
        .context("Failed to get LLM model detail")?;

    let tokens = message_token_manager.get_tokens();
    let window_clone = window.clone();
    let model_id = model_detail.model.id;
    let model_code = model_detail.model.code.clone();
    let model_configs = model_detail.configs.clone();
    let provider_api_type = model_detail.provider.api_type.clone();
    let assistant_model_configs = assistant_detail.model_configs.clone();
    
    let task_handle = tokio::spawn(async move {
        let conversation_db = ConversationDatabase::new(&app_handle).unwrap();

        // Build chat configuration (same as ask_ai)
        let client = genai_client::create_client_with_config(
            &model_configs,
            &model_code,
            &provider_api_type,
        )?;

        let temp_model_detail = crate::db::llm_db::ModelDetail {
            model: crate::db::llm_db::LLMModel {
                id: model_id,
                name: model_code.clone(),
                code: model_code.clone(),
                llm_provider_id: 0,
                description: String::new(),
                vision_support: false,
                audio_support: false,
                video_support: false,
            },
            provider: crate::db::llm_db::LLMProvider {
                id: 0,
                name: String::new(),
                api_type: provider_api_type.clone(),
                description: String::new(),
                is_official: false,
                is_enabled: true,
            },
            configs: model_configs.clone(),
        };

        let model_config_clone = ConfigBuilder::merge_model_configs(
            assistant_model_configs,
            &temp_model_detail,
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
            "[[model_name]]: {} [[stream]]: {}\n",
            chat_config.model_name, chat_config.stream
        );

        // 根据是否为原生 toolcall 选择不同的消息组织策略：
        // - 原生：将 "tool_result" 转为 ToolResponse（含 tool_call_id）
        // - 非原生：把所有 "tool_result" 在内存里映射成 "user" 文本消息，避免向提供商发送 ToolResponse 导致 4xx/5xx
        let chat_request = if is_native_toolcall {
            let chat_messages = build_chat_messages_with_context(&init_message_list, Some(tool_call_id.clone()));
            println!("[[chat_messages (tool_result_continue native)]]: {:#?}\n", chat_messages);
            ChatRequest::new(chat_messages)
        } else {
            let transformed_list: Vec<(String, String, Vec<MessageAttachment>)> = init_message_list
                .iter()
                .map(|(message_type, content, attachments)| {
                    if message_type == "tool_result" {
                        // 将工具结果作为用户侧输入提供给模型（仅在请求中使用，不更改 DB 与 UI）
                        (String::from("user"), content.clone(), Vec::new())
                    } else {
                        (message_type.clone(), content.clone(), attachments.clone())
                    }
                })
                .collect();

            let chat_messages = build_chat_messages(&transformed_list);
            println!("[[chat_messages (tool_result_continue non_native)]]: {:#?}\n", chat_messages);
            ChatRequest::new(chat_messages)
        };

        if chat_config.stream {
            ai_handle_stream_chat(
                &chat_config.client,
                &chat_config.model_name,
                &chat_request,
                &chat_config.chat_options,
                conversation_id_i64,
                &cancel_token,
                &conversation_db,
                &tokens,
                &window_clone,
                &app_handle,
                false,  // no title generation needed
                String::new(), // no user prompt
                HashMap::new(), // no feature config needed
                None,   // no generation_group_id reuse
                None,   // no parent_group_id
                model_id,
                model_code.clone(),
            )
            .await?;
        } else {
            ai_handle_non_stream_chat(
                &chat_config.client,
                &chat_config.model_name,
                &chat_request,
                &chat_config.chat_options,
                conversation_id_i64,
                &cancel_token,
                &conversation_db,
                &tokens,
                &window_clone,
                &app_handle,
                false,  // no title generation needed
                String::new(), // no user prompt
                HashMap::new(), // no feature config needed
                None,   // no generation_group_id reuse
                None,   // no parent_group_id
                model_id,
                model_code.clone(),
            )
            .await?;
        }

        Ok::<(), Error>(())
    });
    
    // 等待任务完成并处理错误
    if let Err(join_error) = task_handle.await {
        eprintln!("[[tool_continue_task_join_error]]: {}\n", join_error);
        return Err(AppError::InternalError(format!("工具继续任务执行失败: {}", join_error)));
    }

    println!("================================ Tool Result Continue AI End ===============================================");

    Ok(AiResponse {
        conversation_id: conversation_id_i64,
        request_prompt_result_with_context: format!("Tool result: {}", tool_result),
    })
}

#[tauri::command]
pub async fn cancel_ai(
    message_token_manager: State<'_, MessageTokenManager>,
    conversation_id: i64,
) -> Result<(), String> {
    message_token_manager.cancel_request(conversation_id).await;
    Ok(())
}

#[tauri::command]
pub async fn regenerate_ai(
    app_handle: tauri::AppHandle,
    message_token_manager: State<'_, MessageTokenManager>,
    window: tauri::Window,
    message_id: i64,
) -> Result<AiResponse, AppError> {
    println!("================================ Regenerate AI Start ===============================================");
    // TODO 没有兼容mcp
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
            .filter(|m| m.0.id <= message_id) // 包含当前消息
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

    println!("[[init_message_list (regenerate)]]: {:#?}\n", init_message_list);

    // 获取助手信息（在构建消息列表之后，以确保对话已确定）
    let assistant_id = conversation.assistant_id.unwrap();
    let assistant_detail = get_assistant(app_handle.clone(), assistant_id).unwrap();

    if assistant_detail.model.is_empty() {
        return Err(AppError::NoModelFound);
    }

    // 兼容 MCP：根据助手配置判断是否使用提供商原生 toolcall
    let mcp_info = crate::api::ai::mcp::collect_mcp_info_for_assistant(&app_handle, assistant_id).await?;
    let is_native_toolcall = mcp_info.use_native_toolcall;

    // 确定要使用的generation_group_id和parent_group_id
    let (regenerate_generation_group_id, regenerate_parent_group_id) =
        if message.message_type == "user" {
            // 用户消息重发：为新的AI回复生成全新的group_id
            // 查找该user message后面第一条非user、非system的消息，用它的generation_group_id作为parent_group_id
            let mut parent_group_id: Option<String> = None;

            // 获取对话中的所有消息，按ID排序
            let all_messages = db
                .message_repo()
                .unwrap()
                .list_by_conversation_id(conversation_id)?;

            // 找到当前user message在列表中的位置
            if let Some(message_index) = all_messages
                .iter()
                .position(|(msg, _)| msg.id == message_id)
            {
                // 查找该user message后面第一条非user、非system的消息
                for (next_msg, _) in all_messages.iter().skip(message_index + 1) {
                    if next_msg.message_type != "user"
                        && next_msg.message_type != "system"
                        && next_msg.generation_group_id.is_some()
                    {
                        parent_group_id = next_msg.generation_group_id.clone();
                        println!(
                            "[[parent_group_id for user message regenerate]]: {:?}\n",
                            parent_group_id
                        );
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

    let cancel_token = CancellationToken::new();
    message_token_manager
        .store_token(conversation_id, cancel_token.clone())
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
    let task_handle = tokio::spawn(async move {
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
                llm_provider_id: 0,         // 临时值
                description: String::new(), // 临时值
                vision_support: false,      // 临时值
                audio_support: false,       // 临时值
                video_support: false,       // 临时值
            },
            provider: crate::db::llm_db::LLMProvider {
                id: 0,               // 临时值
                name: String::new(), // 临时值
                api_type: regenerate_provider_api_type.clone(),
                description: String::new(), // 临时值
                is_official: false,         // 临时值
                is_enabled: true,           // 临时值
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

        // 将历史消息转换为 ChatMessage：
        // - 原生 toolcall：按默认逻辑（tool_result -> ToolResponse）
        // - 非原生：把所有 tool_result 映射成 "user" 文本，仅用于请求
        let final_message_list_for_llm: Vec<(String, String, Vec<MessageAttachment>)> = if is_native_toolcall {
            init_message_list.clone()
        } else {
            init_message_list
                .iter()
                .map(|(message_type, content, attachments)| {
                    if message_type == "tool_result" {
                        (String::from("user"), content.clone(), Vec::new())
                    } else {
                        (message_type.clone(), content.clone(), attachments.clone())
                    }
                })
                .collect()
        };

        let chat_messages = build_chat_messages(&final_message_list_for_llm);
        println!("[[final_chat_messages (regenerate)]]: {:#?}\n", chat_messages);
        let chat_request = ChatRequest::new(chat_messages);

        if chat_config.stream {
            // 使用 genai 流式处理
            ai_handle_stream_chat(
                &chat_config.client,
                &chat_config.model_name,
                &chat_request,
                &chat_config.chat_options,
                conversation_id,
                &cancel_token,
                &conversation_db,
                &tokens,
                &window_clone,
                &app_handle_clone,
                false,                                  // regenerate 不需要生成标题
                String::new(),                          // regenerate 不需要用户提示
                HashMap::new(),                         // regenerate 不需要配置
                regenerate_generation_group_id.clone(), // 传递generation_group_id用于复用
                regenerate_parent_group_id.clone(),     // 传递parent_group_id设置版本关系
                regenerate_model_id,                    // 传递模型ID
                regenerate_model_code.clone(),          // 传递模型名称
            )
            .await?;
        } else {
            // Use genai non-streaming
            ai_handle_non_stream_chat(
                &chat_config.client,
                &chat_config.model_name,
                &chat_request,
                &chat_config.chat_options,
                conversation_id,
                &cancel_token,
                &conversation_db,
                &tokens,
                &window_clone,
                &app_handle_clone,
                false,                                  // regenerate 不需要生成标题
                String::new(),                          // regenerate 不需要用户提示
                HashMap::new(),                         // regenerate 不需要配置
                regenerate_generation_group_id.clone(), // 传递generation_group_id用于复用
                regenerate_parent_group_id.clone(),     // 传递parent_group_id设置版本关系
                regenerate_model_id,                    // 传递模型ID
                regenerate_model_code.clone(),          // 传递模型名称
            )
            .await?;
        }

        Ok::<(), Error>(())
    });
    
    // 等待任务完成并处理错误
    if let Err(join_error) = task_handle.await {
        eprintln!("[[regenerate_task_join_error]]: {}\n", join_error);
        return Err(AppError::InternalError(format!("重新生成任务执行失败: {}", join_error)));
    }

    println!("================================ Regenerate AI End ===============================================");

    Ok(AiResponse {
        conversation_id,
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
            println!("[[initialize_conversation assistant_id]]: {}\n", request.assistant_id);
            println!(
                "[[initialize_conversation init_message_list]]: {:#?}\n",
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
            let user_message = add_message(
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

            // 发送消息添加事件
            let add_event = ConversationEvent {
                r#type: "message_add".to_string(),
                data: serde_json::to_value(MessageAddEvent {
                    message_id: user_message.id,
                    message_type: "user".to_string(),
                })
                .unwrap(),
            };

            let _ = app_handle.emit(
                format!("conversation_event_{}", conversation_id).as_str(),
                add_event,
            );

            let update_event = ConversationEvent {
                r#type: "message_update".to_string(),
                data: serde_json::to_value(MessageUpdateEvent {
                    message_id: user_message.id,
                    message_type: "user".to_string(),
                    content: request_prompt_result_with_context.clone(),
                    is_done: false,
                })
                .unwrap(),
            };
            let _ = app_handle.emit(
                format!("conversation_event_{}", conversation_id).as_str(),
                update_event,
            );

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

/// 重新生成对话标题
#[tauri::command]
pub async fn regenerate_conversation_title(
    app_handle: tauri::AppHandle,
    window: tauri::Window,
    feature_config_state: State<'_, FeatureConfigState>,
    conversation_id: i64,
) -> Result<(), AppError> {
    let conversation_db = ConversationDatabase::new(&app_handle).map_err(AppError::from)?;

    // 获取对话的消息
    let messages = conversation_db
        .message_repo()
        .unwrap()
        .list_by_conversation_id(conversation_id)?;

    if messages.is_empty() {
        return Err(AppError::InsufficientMessages);
    }

    // 获取第一条用户消息（必须有）
    let user_message = messages
        .iter()
        .find(|(msg, _)| msg.message_type == "user")
        .map(|(msg, _)| msg)
        .ok_or_else(|| AppError::InsufficientMessages)?;

    // 获取第一条AI回答（可选）
    let response_message = messages
        .iter()
        .find(|(msg, _)| msg.message_type == "response")
        .map(|(msg, _)| msg);

    // 获取特性配置
    let config_feature_map = feature_config_state.config_feature_map.lock().await;

    // 调用内部的 generate_title 函数
    let response_content = response_message
        .map(|msg| msg.content.clone())
        .unwrap_or_default(); // 如果没有回答，使用空字符串

    generate_title(
        &app_handle,
        conversation_id,
        user_message.content.clone(),
        response_content,
        config_feature_map.clone(),
        window,
    )
    .await?;

    Ok(())
}
