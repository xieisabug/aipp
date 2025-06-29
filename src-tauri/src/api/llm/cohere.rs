use std::collections::HashMap;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::{
    api::llm_api::LlmModel,
    db::{conversation_db::MessageAttachment, llm_db::LLMProviderConfig},
};

use super::ModelProvider;
use futures::StreamExt;
use rig::client::completion::CompletionClient;
use rig::{completion::Chat, providers::cohere as rig_cohere, streaming::StreamingChat};

#[derive(Serialize, Deserialize, Debug)]
struct ModelsResponse {
    models: Vec<Model>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Model {
    name: String,
    endpoints: Vec<String>,
    finetuned: bool,
    context_length: u32,
    tokenizer_url: Option<String>,
    default_endpoints: Vec<String>,
}

pub struct CohereProvider {
    llm_provider_config: Vec<LLMProviderConfig>,
}

impl ModelProvider for CohereProvider {
    fn new(llm_provider_config: Vec<crate::db::llm_db::LLMProviderConfig>) -> Self
    where
        Self: Sized,
    {
        CohereProvider {
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

            // 若有自訂 endpoint 則改用 from_url
            let endpoint_opt = config_map.get("endpoint");
            let client = if let Some(ep) = endpoint_opt {
                rig_cohere::Client::from_url(api_key, ep.trim_end_matches('/'))
            } else {
                rig_cohere::Client::new(api_key)
            };

            // 取得模型名稱與溫度
            let model_conf = model_config
                .iter()
                .filter_map(|c| c.value.as_ref().map(|v| (c.name.clone(), v.clone())))
                .collect::<HashMap<String, String>>();

            let model_name = model_conf
                .get("model")
                .cloned()
                .unwrap_or_else(|| "command".to_string());

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
                agent_builder = agent_builder.additional_params(serde_json::json!({"p": tp}));
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

            // 若有自訂 endpoint 則改用 from_url
            let endpoint_opt = config_map.get("endpoint");
            let client = if let Some(ep) = endpoint_opt {
                rig_cohere::Client::from_url(api_key, ep.trim_end_matches('/'))
            } else {
                rig_cohere::Client::new(api_key)
            };

            // 取得模型名稱與溫度
            let model_conf = model_config
                .iter()
                .filter_map(|c| c.value.as_ref().map(|v| (c.name.clone(), v.clone())))
                .collect::<HashMap<String, String>>();

            let model_name = model_conf
                .get("model")
                .cloned()
                .unwrap_or_else(|| "command".to_string());

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
                agent_builder = agent_builder.additional_params(serde_json::json!({"p": tp}));
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

    fn models(&self) -> futures::future::BoxFuture<'static, Result<Vec<LlmModel>>> {
        let config = self.llm_provider_config.clone();

        Box::pin(async move {
            let mut result = Vec::new();

            let config_map: HashMap<String, String> =
                config.into_iter().map(|c| (c.name, c.value)).collect();
            println!("config_map: {:?}", config_map);

            let default_endpoint = &"https://api.cohere.ai/v1".to_string();
            let endpoint = config_map
                .get("endpoint")
                .unwrap_or(default_endpoint)
                .trim_end_matches('/');
            let url = format!("{}/models", endpoint);
            let api_key = config_map.get("api_key").unwrap().clone();
            println!("Cohere models endpoint : {}", url);

            let client = reqwest::Client::new();
            let response = client
                .get(&url)
                .header("Accept", "application/json")
                .header("Authorization", &format!("bearer {}", api_key))
                .send()
                .await?;

            // 读取响应体为字符串
            let body = response.text().await?;
            println!("Response body: {}", body);

            // 将字符串解析为 JSON
            let models_response: ModelsResponse = serde_json::from_str(&body)?;
            println!("models_response: {:?}", models_response);

            for model in models_response.models {
                let llm_model = LlmModel {
                    id: 0, // You need to set this according to your needs
                    name: model.name.clone(),
                    llm_provider_id: 1, // You need to set this according to your needs
                    code: model.name.clone(),
                    description: format!(
                        "Model name: {}, Model context_length: {}",
                        model.name.clone(),
                        model.context_length,
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
