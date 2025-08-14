use crate::api::ai::events::TITLE_CHANGE_EVENT;
use crate::api::genai_client;
use crate::db::llm_db::LLMDatabase;
use crate::db::system_db::FeatureConfig;
use crate::errors::AppError;
use crate::db::conversation_db::{ConversationDatabase, Conversation};
use crate::utils::window_utils::send_error_to_appropriate_window;
use genai::chat::{ChatMessage, ChatRequest};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use crate::api::ai::config::{get_retry_attempts_from_config, calculate_retry_delay, get_network_proxy_from_config, get_request_timeout_from_config};
use tauri::Emitter;

pub async fn generate_title(
    app_handle: &tauri::AppHandle,
    conversation_id: i64,
    user_prompt: String,
    content: String,
    config_feature_map: HashMap<String, HashMap<String, FeatureConfig>>,
    window: tauri::Window,
) -> Result<(), AppError> {
    let feature_config = config_feature_map.get("conversation_summary");
    if let Some(config) = feature_config {
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
            if content.is_empty() {
                context.push_str(
                    format!(
                        "# user\n {} \n\n请为上述用户问题生成一个简洁的标题，不需要包含标点符号",
                        user_prompt
                    )
                    .as_str(),
                );
            } else {
                context.push_str(
                    format!(
                        "# user\n {} \n\n#assistant\n {} \n\n请总结上述对话为标题，不需要包含标点符号",
                        user_prompt, content
                    )
                    .as_str(),
                );
            }
        } else {
            let unsize_summary_length: usize = summary_length.try_into().unwrap();
            if content.is_empty() {
                if user_prompt.len() > unsize_summary_length {
                    context.push_str(
                        format!(
                            "# user\n {} \n\n请为上述用户问题生成一个简洁的标题，不需要包含标点符号",
                            user_prompt
                                .chars()
                                .take(unsize_summary_length)
                                .collect::<String>()
                        )
                        .as_str(),
                    );
                } else {
                    context.push_str(
                        format!(
                            "# user\n {} \n\n请为上述用户问题生成一个简洁的标题，不需要包含标点符号",
                            user_prompt
                        )
                        .as_str(),
                    );
                }
            } else {
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
        }

        let llm_db = LLMDatabase::new(app_handle).map_err(AppError::from)?;
        let model_detail = llm_db
            .get_llm_model_detail(&provider_id, &model_code)
            .unwrap();

        // 从配置中获取网络代理和超时设置
        let network_proxy = get_network_proxy_from_config(&config_feature_map);
        let request_timeout = get_request_timeout_from_config(&config_feature_map);
        
        // 检查供应商是否启用了代理（标题生成通常不需要代理，设为false）
        let proxy_enabled = false;
        
        let client = genai_client::create_client_with_config(
            &model_detail.configs,
            &model_detail.model.code,
            &model_detail.provider.api_type,
            network_proxy.as_deref(),
            proxy_enabled,
            Some(request_timeout),
        )?;

        let chat_messages = vec![ChatMessage::system(&prompt), ChatMessage::user(&context)];
        let chat_request = ChatRequest::new(chat_messages);
        let model_name = &model_detail.model.code;

        // 从配置中获取最大重试次数
        let max_retry_attempts = get_retry_attempts_from_config(&config_feature_map);

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
                    if attempts >= max_retry_attempts {
                        eprintln!("Title generation failed after {} attempts: {}", attempts, e);
                        break Err(e.to_string());
                    }
                    eprintln!(
                        "Title generation attempt {} failed: {}, retrying...",
                        attempts, e
                    );
                    let delay = calculate_retry_delay(attempts);
                    sleep(Duration::from_millis(delay)).await;
                }
            }
        };
        match response {
            Err(e) => {
                println!("Chat error: {}", e);
                send_error_to_appropriate_window(&window, "生成对话标题失败，请检查配置");
            }
            Ok(response_text) => {
                let conversation_db = ConversationDatabase::new(app_handle).map_err(AppError::from)?;
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


