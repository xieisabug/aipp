use super::ModelProvider;
use crate::{
    api::llm_api::LlmModel,
    db::{conversation_db::MessageAttachment, llm_db::LLMProviderConfig},
};
use anyhow::Result;
use futures::StreamExt;
use rig::client::completion::CompletionClient;
use rig::completion::Chat;
use rig::providers::anthropic::ClientBuilder;
use rig::streaming::StreamingChat;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;

#[derive(Serialize, Deserialize, Debug)]
struct ModelsResponse {
    models: Vec<Model>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Model {
    name: String,
    description: String,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AnthropicUsage {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AnthropicContentBlock {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AnthropicTextDelta {
    #[serde(rename = "type")]
    pub delta_type: Option<String>,
    pub text: Option<String>,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Option<AnthropicUsage>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AnthropicMessage {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: Option<String>,
    pub content: Option<Vec<AnthropicContentBlock>>,
    pub model: Option<String>,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Option<AnthropicUsage>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub struct AnthropicChatCompletionChunk {
    #[serde(rename = "type")]
    pub event_type: String,
    pub index: Option<usize>,
    pub delta: Option<AnthropicTextDelta>,
    pub message: Option<AnthropicMessage>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AnthropicErrorMessage {
    #[serde(rename = "type")]
    pub error_type: String,
    pub error: AnthropicErrorDetails,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AnthropicErrorDetails {
    pub details: Option<serde_json::Value>,
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

#[derive(Debug)]
pub enum ToolChoice {
    Auto,
    Any,
    Tool(String),
}

impl Serialize for ToolChoice {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ToolChoice::Auto => {
                serde::Serialize::serialize(&serde_json::json!({"type": "auto"}), serializer)
            }
            ToolChoice::Any => {
                serde::Serialize::serialize(&serde_json::json!({"type": "any"}), serializer)
            }
            ToolChoice::Tool(name) => serde::Serialize::serialize(
                &serde_json::json!({"type": "tool", "name": name}),
                serializer,
            ),
        }
    }
}

pub struct AnthropicProvider {
    llm_provider_config: Vec<LLMProviderConfig>,
}

impl ModelProvider for AnthropicProvider {
    fn new(llm_provider_config: Vec<crate::db::llm_db::LLMProviderConfig>) -> Self
    where
        Self: Sized,
    {
        AnthropicProvider {
            llm_provider_config,
        }
    }

    fn chat(
        &self,
        _message_id: i64,
        messages: Vec<(String, String, Vec<MessageAttachment>)>,
        model_config: Vec<crate::db::assistant_db::AssistantModelConfig>,
        cancel_token: CancellationToken,
    ) -> futures::future::BoxFuture<'static, Result<String>> {
        let config = self.llm_provider_config.clone();

        Box::pin(async move {
            let config_map: HashMap<String, String> =
                config.into_iter().map(|c| (c.name, c.value)).collect();

            let api_key = config_map
                .get("api_key")
                .ok_or_else(|| anyhow::anyhow!("Missing api_key in provider config"))?;

            let endpoint_opt = config_map.get("endpoint");
            let client = if let Some(ep) = endpoint_opt {
                ClientBuilder::new(api_key)
                    .base_url(ep.trim_end_matches('/'))
                    .anthropic_version("2023-06-01")
                    .build()
            } else {
                ClientBuilder::new(api_key)
                    .anthropic_version("2023-06-01")
                    .build()
            };

            let model_conf = model_config
                .iter()
                .filter_map(|c| c.value.as_ref().map(|v| (c.name.clone(), v.clone())))
                .collect::<HashMap<String, String>>();

            let model_name = model_conf
                .get("model")
                .cloned()
                .unwrap_or_else(|| "claude-3-sonnet-20240229".to_string());

            let temperature: f64 = model_conf
                .get("temperature")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.75);

            let max_tokens: u64 = model_conf
                .get("max_tokens")
                .and_then(|v| v.parse().ok())
                .unwrap_or(2000);

            let top_p: Option<f64> = model_conf.get("top_p").and_then(|v| v.parse().ok());

            let mut agent_builder = client
                .agent(model_name.as_str())
                .temperature(temperature)
                .max_tokens(max_tokens);
            if let Some(tp) = top_p {
                agent_builder = agent_builder.additional_params(serde_json::json!({"top_p": tp}));
            }

            if let Some((role, content, _)) = messages.first() {
                if role == "system" {
                    agent_builder = agent_builder.preamble(content);
                }
            }

            let agent = agent_builder.build();

            let mut history: Vec<rig::completion::Message> = Vec::new();
            if !messages.is_empty() {
                for (idx, (role, content, _)) in messages.iter().enumerate() {
                    if idx == messages.len() - 1 {
                        break;
                    }
                    match role.as_str() {
                        "user" => history.push(rig::completion::Message::user(content.clone())),
                        "assistant" => {
                            history.push(rig::completion::Message::assistant(content.clone()))
                        }
                        _ => {}
                    }
                }
            }

            let prompt_content = messages
                .last()
                .map(|(_, content, _)| content.clone())
                .unwrap_or_default();

            let resp_fut = agent.chat(&prompt_content, history);

            let response = tokio::select! {
                r = resp_fut => r.map_err(|e| anyhow::anyhow!("{:?}", e)),
                _ = cancel_token.cancelled() => anyhow::bail!("Request cancelled"),
            }?;

            Ok(response)
        })
    }

    fn chat_stream(
        &self,
        message_id: i64,
        messages: Vec<(String, String, Vec<MessageAttachment>)>,
        model_config: Vec<crate::db::assistant_db::AssistantModelConfig>,
        tx: tokio::sync::mpsc::Sender<(i64, String, bool)>,
        cancel_token: CancellationToken,
    ) -> futures::future::BoxFuture<'static, Result<()>> {
        let config = self.llm_provider_config.clone();

        Box::pin(async move {
            let config_map: HashMap<String, String> =
                config.into_iter().map(|c| (c.name, c.value)).collect();

            let api_key = config_map
                .get("api_key")
                .ok_or_else(|| anyhow::anyhow!("Missing api_key in provider config"))?;

            let endpoint_opt = config_map.get("endpoint");
            let client = if let Some(ep) = endpoint_opt {
                ClientBuilder::new(api_key)
                    .base_url(ep.trim_end_matches('/'))
                    .anthropic_version("2023-06-01")
                    .build()
            } else {
                ClientBuilder::new(api_key)
                    .anthropic_version("2023-06-01")
                    .build()
            };

            let model_conf = model_config
                .iter()
                .filter_map(|c| c.value.as_ref().map(|v| (c.name.clone(), v.clone())))
                .collect::<HashMap<String, String>>();

            let model_name = model_conf
                .get("model")
                .cloned()
                .unwrap_or_else(|| "claude-3-sonnet-20240229".to_string());

            let temperature: f64 = model_conf
                .get("temperature")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.75);

            let max_tokens: u64 = model_conf
                .get("max_tokens")
                .and_then(|v| v.parse().ok())
                .unwrap_or(2000);

            let top_p: Option<f64> = model_conf.get("top_p").and_then(|v| v.parse().ok());

            let mut agent_builder = client
                .agent(model_name.as_str())
                .temperature(temperature)
                .max_tokens(max_tokens);
            if let Some(tp) = top_p {
                agent_builder = agent_builder.additional_params(serde_json::json!({"top_p": tp}));
            }

            if let Some((role, content, _)) = messages.first() {
                if role == "system" {
                    agent_builder = agent_builder.preamble(content);
                }
            }

            let agent = agent_builder.build();

            let mut history: Vec<rig::completion::Message> = Vec::new();
            if !messages.is_empty() {
                for (idx, (role, content, _)) in messages.iter().enumerate() {
                    if idx == messages.len() - 1 {
                        break;
                    }
                    match role.as_str() {
                        "user" => history.push(rig::completion::Message::user(content.clone())),
                        "assistant" => {
                            history.push(rig::completion::Message::assistant(content.clone()))
                        }
                        _ => {}
                    }
                }
            }

            let prompt_content = messages
                .last()
                .map(|(_, content, _)| content.clone())
                .unwrap_or_default();

            let mut stream = tokio::select! {
                s = agent.stream_chat(&prompt_content, history) => s.map_err(|e| anyhow::anyhow!("{:?}", e))?,
                _ = cancel_token.cancelled() => anyhow::bail!("Request cancelled"),
            };

            let mut full_text = String::new();

            loop {
                tokio::select! {
                    maybe_chunk = stream.next() => {
                        match maybe_chunk {
                            Some(Ok(chunk)) => {
                                if let rig::completion::AssistantContent::Text(text) = chunk {
                                    full_text.push_str(text.text.as_str());
                                }
                                tx.send((message_id, full_text.clone(), false)).await?;
                            },
                            Some(Err(e)) => {
                                eprintln!("stream chunk error: {:?}", e);
                                break;
                            },
                            None => {
                                break;
                            }
                        }
                    }
                    _ = cancel_token.cancelled() => {
                        break;
                    }
                }
            }

            tx.send((message_id, full_text.clone(), true)).await?;
            Ok(())
        })
    }

    fn models(&self) -> futures::future::BoxFuture<'static, Result<Vec<LlmModel>>> {
        let mut result = Vec::new();

        let models = vec![
            (
                "Claude 3 Opus",
                "claude-3-opus-20240229",
                "Powerful model for highly complex tasks",
            ),
            (
                "Claude 3.5 Sonnet",
                "claude-3-5-sonnet-20240620",
                "Most intelligent model",
            ),
            (
                "Claude 3 Sonnet",
                "claude-3-sonnet-20240229",
                "Balance of intelligence and speed",
            ),
            (
                "Claude 3 Haiku",
                "claude-3-haiku-20240307",
                "Fastest and most compact model for near-instant responsiveness",
            ),
        ];

        for model in models {
            let llm_model = LlmModel {
                id: 0, // You need to set this according to your needs
                name: model.0.to_string(),
                llm_provider_id: 2, // Assuming Anthropic is provider_id 2
                code: model.1.to_string(),
                description: model.2.to_string(),
                vision_support: true, // Set this according to your needs
                audio_support: false, // Set this according to your needs
                video_support: false, // Set this according to your needs
            };
            result.push(llm_model);
        }

        Box::pin(async move { Ok(result) })
    }
}
