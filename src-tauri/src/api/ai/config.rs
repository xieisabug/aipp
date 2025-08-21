use crate::db::assistant_db::AssistantModelConfig;
use genai::chat::ChatOptions;
use genai::Client;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ChatConfig {
    pub model_name: String,
    pub stream: bool,
    pub chat_options: ChatOptions,
    pub client: Client,
}

pub struct ConfigBuilder;

impl ConfigBuilder {
    pub fn build_chat_options(config_map: &HashMap<String, String>) -> ChatOptions {
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

    pub fn merge_model_configs(
        base_configs: Vec<AssistantModelConfig>,
        model_detail: &crate::db::llm_db::ModelDetail,
        override_configs: Option<HashMap<String, serde_json::Value>>,
    ) -> Vec<AssistantModelConfig> {
        let mut model_config_clone = base_configs;
        model_config_clone.push(AssistantModelConfig {
            id: 0,
            assistant_id: model_detail.model.id,
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

                if let Some(existing_config) = model_config_clone.iter_mut().find(|c| c.name == key)
                {
                    existing_config.value = Some(value_str);
                    existing_config.value_type = value_type.to_string();
                } else {
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

pub const MAX_RETRY_ATTEMPTS: u32 = 3;
pub const RETRY_DELAY_BASE_MS: u64 = 2000;
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 180; // 3分钟默认超时

/// 从网络配置中获取重试次数，如果没有配置则使用默认值
pub fn get_retry_attempts_from_config(
    config_feature_map: &HashMap<String, HashMap<String, crate::db::system_db::FeatureConfig>>,
) -> u32 {
    if let Some(network_config) = config_feature_map.get("network_config") {
        if let Some(retry_config) = network_config.get("retry_attempts") {
            if let Ok(attempts) = retry_config.value.parse::<u32>() {
                return attempts;
            }
        }
    }
    MAX_RETRY_ATTEMPTS
}

/// 从网络配置中获取请求超时时间（秒），如果没有配置则使用默认值
pub fn get_request_timeout_from_config(
    config_feature_map: &HashMap<String, HashMap<String, crate::db::system_db::FeatureConfig>>,
) -> u64 {
    if let Some(network_config) = config_feature_map.get("network_config") {
        if let Some(timeout_config) = network_config.get("request_timeout") {
            if let Ok(timeout) = timeout_config.value.parse::<u64>() {
                return timeout;
            }
        }
    }
    DEFAULT_REQUEST_TIMEOUT_SECS
}

/// 从网络配置中获取网络代理URL
pub fn get_network_proxy_from_config(
    config_feature_map: &HashMap<String, HashMap<String, crate::db::system_db::FeatureConfig>>,
) -> Option<String> {
    if let Some(network_config) = config_feature_map.get("network_config") {
        if let Some(proxy_config) = network_config.get("network_proxy") {
            let proxy_url = proxy_config.value.trim();
            if !proxy_url.is_empty() {
                return Some(proxy_url.to_string());
            }
        }
    }
    None
}

/// 计算重试延迟，使用指数退避策略
pub fn calculate_retry_delay(attempt: u32) -> u64 {
    RETRY_DELAY_BASE_MS * (2_u64.pow(attempt.saturating_sub(1)))
}
