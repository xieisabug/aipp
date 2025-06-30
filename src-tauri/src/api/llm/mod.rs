use anyhow::Result;
use std::sync::Arc;

use anthropic::AnthropicProvider;
use cohere::CohereProvider;
use deepseek::DeepSeekProvider;
use futures::future::BoxFuture;
use ollama::OllamaProvider;
use openai::OpenAIProvider;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::db::{
    assistant_db::AssistantModelConfig,
    conversation_db::MessageAttachment,
    llm_db::{LLMProvider, LLMProviderConfig},
};

use super::llm_api::LlmModel;

mod anthropic;
mod cohere;
mod deepseek;
mod ollama;
mod openai;

pub trait ModelProvider: Send + Sync {
    fn new(llm_provider_config: Vec<LLMProviderConfig>) -> Self
    where
        Self: Sized;

    fn chat(
        &self,
        message_id: i64,
        messages: Vec<(String, String, Vec<MessageAttachment>)>,
        model_config: Vec<AssistantModelConfig>,
        cancel_token: CancellationToken,
    ) -> BoxFuture<'static, Result<String>>;

    fn chat_stream(
        &self,
        message_id: i64,
        messages: Vec<(String, String, Vec<MessageAttachment>)>,
        model_config: Vec<AssistantModelConfig>,
        tx: mpsc::Sender<(i64, String, bool)>,
        cancel_token: CancellationToken,
    ) -> BoxFuture<'static, Result<()>>;

    fn models(&self) -> BoxFuture<'static, Result<Vec<LlmModel>>>;
}

pub fn get_provider(
    provider: LLMProvider,
    llm_provider_config: Vec<LLMProviderConfig>,
) -> Arc<dyn ModelProvider> {
    match provider.api_type.as_str() {
        "ollama" => Arc::new(OllamaProvider::new(llm_provider_config)), // 传入适当的配置
        "openai_api" => Arc::new(OpenAIProvider::new(llm_provider_config)), // 传入适当的配置
        "anthropic" => Arc::new(AnthropicProvider::new(llm_provider_config)), // 传入适当的配置
        "cohere" => Arc::new(CohereProvider::new(llm_provider_config)), // 传入适当的配置
        "deepseek" => Arc::new(DeepSeekProvider::new(llm_provider_config)), // DeepSeek 配置
        // 其他提供商...
        _ => panic!(
            "Unknown provider: {} and type: {}",
            provider.name, provider.api_type
        ),
    }
}
