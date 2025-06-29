use crate::{
    api::llm_api::LlmModel,
    db::{
        assistant_db::AssistantModelConfig, conversation_db::MessageAttachment,
        llm_db::LLMProviderConfig,
    },
};
use anyhow::{bail, Result};
use futures::{future::BoxFuture, StreamExt};
use reqwest::{header::AUTHORIZATION, Client};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::ModelProvider;
use rig::client::completion::CompletionClient;
use rig::{completion::Chat, providers::ollama as rig_ollama, streaming::StreamingChat};

#[derive(Serialize, Deserialize, Debug)]
struct ModelsResponse {
    models: Vec<Model>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Model {
    name: String,
    model: String,
    modified_at: String,
    size: i64,
    digest: String,
    details: Details,
}

#[derive(Serialize, Deserialize, Debug)]
struct Details {
    parent_model: String,
    format: String,
    family: String,
    families: Vec<String>,
    parameter_size: String,
    quantization_level: String,
}

pub struct OllamaProvider {
    llm_provider_config: Vec<LLMProviderConfig>,
    client: Client,
}

impl ModelProvider for OllamaProvider {
    fn new(llm_provider_config: Vec<LLMProviderConfig>) -> Self {
        OllamaProvider {
            llm_provider_config,
            client: Client::new(),
        }
    }

    fn chat(
        &self,
        _message_id: i64,
        messages: Vec<(String, String, Vec<MessageAttachment>)>,
        model_config: Vec<AssistantModelConfig>,
        cancel_token: CancellationToken,
    ) -> BoxFuture<'static, Result<String>> {
        let config = self.llm_provider_config.clone();

        Box::pin(async move {
            let config_map: HashMap<String, String> =
                config.into_iter().map(|c| (c.name, c.value)).collect();

            let default_endpoint = &"http://localhost:11434".to_string();
            let endpoint = config_map
                .get("endpoint")
                .unwrap_or(default_endpoint)
                .trim_end_matches('/');

            // 创建 Ollama client
            let client = rig_ollama::Client::from_url(endpoint);

            // 取得模型名稱與參數
            let model_conf = model_config
                .iter()
                .filter_map(|c| c.value.as_ref().map(|v| (c.name.clone(), v.clone())))
                .collect::<HashMap<String, String>>();

            let model_name = model_conf
                .get("model")
                .cloned()
                .unwrap_or_else(|| "llama2".to_string());

            let temperature: f64 = model_conf
                .get("temperature")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.75);

            let max_tokens: Option<u64> = model_conf.get("max_tokens").and_then(|v| v.parse().ok());

            let top_p: Option<f64> = model_conf.get("top_p").and_then(|v| v.parse().ok());

            let mut agent_builder = client.agent(model_name.as_str()).temperature(temperature);
            if let Some(mt) = max_tokens {
                agent_builder = agent_builder.max_tokens(mt);
            }
            if let Some(tp) = top_p {
                agent_builder = agent_builder.additional_params(serde_json::json!({"top_p": tp}));
            }

            // 若第一則是 system, 當作 preamble
            if let Some((role, content, _)) = messages.first() {
                if role == "system" {
                    agent_builder = agent_builder.preamble(content);
                }
            }

            let agent = agent_builder.build();

            // 構造 Rig chat 歷史: 除最後一則外其餘作為 history
            let mut history: Vec<rig::completion::Message> = Vec::new();
            if !messages.is_empty() {
                for (idx, (role, content, _)) in messages.iter().enumerate() {
                    if idx == messages.len() - 1 {
                        break; // skip last -> prompt
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
                r = resp_fut => r.map_err(|e| anyhow::anyhow!(e)),
                _ = cancel_token.cancelled() => bail!("Request cancelled"),
            }?;

            Ok(response)
        })
    }

    fn chat_stream(
        &self,
        message_id: i64,
        messages: Vec<(String, String, Vec<MessageAttachment>)>,
        model_config: Vec<AssistantModelConfig>,
        tx: mpsc::Sender<(i64, String, bool)>,
        cancel_token: CancellationToken,
    ) -> BoxFuture<'static, Result<()>> {
        let config = self.llm_provider_config.clone();

        Box::pin(async move {
            let config_map: HashMap<String, String> =
                config.into_iter().map(|c| (c.name, c.value)).collect();

            let default_endpoint = &"http://localhost:11434".to_string();
            let endpoint = config_map
                .get("endpoint")
                .unwrap_or(default_endpoint)
                .trim_end_matches('/');

            // 创建 Ollama client
            let client = rig_ollama::Client::from_url(endpoint);

            // 取得模型名稱與參數
            let model_conf = model_config
                .iter()
                .filter_map(|c| c.value.as_ref().map(|v| (c.name.clone(), v.clone())))
                .collect::<HashMap<String, String>>();

            let model_name = model_conf
                .get("model")
                .cloned()
                .unwrap_or_else(|| "llama2".to_string());

            let temperature: f64 = model_conf
                .get("temperature")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.75);

            let max_tokens: Option<u64> = model_conf.get("max_tokens").and_then(|v| v.parse().ok());

            let top_p: Option<f64> = model_conf.get("top_p").and_then(|v| v.parse().ok());

            let mut agent_builder = client.agent(model_name.as_str()).temperature(temperature);
            if let Some(mt) = max_tokens {
                agent_builder = agent_builder.max_tokens(mt);
            }
            if let Some(tp) = top_p {
                agent_builder = agent_builder.additional_params(serde_json::json!({"top_p": tp}));
            }

            // 若第一則是 system, 當作 preamble
            if let Some((role, content, _)) = messages.first() {
                if role == "system" {
                    agent_builder = agent_builder.preamble(content);
                }
            }

            let agent = agent_builder.build();

            // 構造歷史與 prompt
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
                s = agent.stream_chat(&prompt_content, history) => s.map_err(|e| anyhow::anyhow!(e))?,
                _ = cancel_token.cancelled() => bail!("Request cancelled"),
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
                                // stream ended
                                break;
                            }
                        }
                    }
                    _ = cancel_token.cancelled() => {
                        break;
                    }
                }
            }

            // 結束後發送完成事件
            tx.send((message_id, full_text.clone(), true)).await?;
            Ok(())
        })
    }

    fn models(&self) -> BoxFuture<'static, Result<Vec<LlmModel>>> {
        let config = self.llm_provider_config.clone();
        let client = self.client.clone();

        Box::pin(async move {
            let mut result = Vec::new();

            let config_map: HashMap<String, String> =
                config.into_iter().map(|c| (c.name, c.value)).collect();

            let default_endpoint = &"http://localhost:11434".to_string();
            let endpoint = config_map
                .get("endpoint")
                .unwrap_or(default_endpoint)
                .trim_end_matches('/');
            let url = format!("{}/api/tags", endpoint);
            let api_key = config_map.get("api_key").unwrap_or(&"".to_string()).clone();

            let response = client
                .get(&url)
                .header(AUTHORIZATION, &format!("Bearer {}", api_key))
                .send()
                .await?;

            let models_response: ModelsResponse = response.json().await?;

            for model in models_response.models {
                let llm_model = LlmModel {
                    id: 0, // You need to set this according to your needs
                    name: model.name,
                    llm_provider_id: 10, // You need to set this according to your needs
                    code: model.model,
                    description: format!(
                        "Family: {}, Parameter Size: {}, Quantization Level: {}",
                        model.details.family,
                        model.details.parameter_size,
                        model.details.quantization_level
                    ),
                    vision_support: false, // Set this according to your needs
                    audio_support: false,  // Set this according to your needs
                    video_support: false,  // Set this according to your needs
                };
                result.push(llm_model);
            }

            Ok(result)
        })
    }
}
