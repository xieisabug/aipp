use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AiRequest {
    pub conversation_id: String,
    pub assistant_id: i64,
    pub prompt: String,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
    pub attachment_list: Option<Vec<i64>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AiResponse {
    pub conversation_id: i64,
    pub request_prompt_result_with_context: String,
}
