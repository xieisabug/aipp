use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEvent {
    pub r#type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAddEvent {
    pub message_id: i64,
    pub message_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageUpdateEvent {
    pub message_id: i64,
    pub message_type: String,
    pub content: String,
    pub is_done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTypeEndEvent {
    pub message_id: i64,
    pub message_type: String,
    pub duration_ms: i64,
    pub end_time: chrono::DateTime<chrono::Utc>,
}

pub const TITLE_CHANGE_EVENT: &str = "title_change";
pub const ERROR_NOTIFICATION_EVENT: &str = "conversation-window-error-notification";


