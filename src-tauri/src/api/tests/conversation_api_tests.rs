use crate::api::conversation_api::process_message_versions;
use crate::db::conversation_db::MessageDetail;
use chrono::Utc;
use uuid::Uuid;

/// 创建测试用的 MessageDetail
fn create_message_detail(
    id: i64,
    conversation_id: i64,
    message_type: &str,
    content: &str,
    parent_id: Option<i64>,
    generation_group_id: Option<String>,
    parent_group_id: Option<String>,
    created_time: chrono::DateTime<Utc>,
) -> MessageDetail {
    MessageDetail {
        id,
        parent_id,
        conversation_id,
        message_type: message_type.to_string(),
        content: content.to_string(),
        llm_model_id: Some(1),
        created_time,
        start_time: None,
        finish_time: None,
        token_count: 100,
        generation_group_id,
        parent_group_id,
        attachment_list: Vec::new(),
        regenerate: Vec::new(),
        tool_calls_json: None,
    }
}

#[tokio::test]
async fn test_version_management_logic() {
    let base_time = Utc::now();
    let group_id = Uuid::new_v4().to_string();

    // 创建测试消息：用户消息 -> AI回复 -> 重新生成1 -> 重新生成2（最新）
    let user_msg = create_message_detail(
        1,
        1,
        "user",
        "Original user message",
        None,
        Some(group_id.clone()),
        None,
        base_time,
    );
    let ai_msg = create_message_detail(
        2,
        1,
        "assistant",
        "Original AI response",
        None,
        Some(group_id.clone()),
        None,
        base_time + chrono::Duration::seconds(1),
    );
    let ai_msg_v2 = create_message_detail(
        3,
        1,
        "assistant",
        "Regenerated AI response v1",
        Some(2),
        Some(group_id.clone()),
        None,
        base_time + chrono::Duration::seconds(2),
    );
    let ai_msg_v3 = create_message_detail(
        4,
        1,
        "assistant",
        "Regenerated AI response v2 (latest)",
        Some(3),
        Some(group_id.clone()),
        None,
        base_time + chrono::Duration::seconds(3),
    );

    let message_details = vec![user_msg, ai_msg, ai_msg_v2, ai_msg_v3];

    // 测试核心业务逻辑
    let final_messages = process_message_versions(message_details);

    // 验证结果
    assert_eq!(final_messages.len(), 2, "Expected 2 messages but got {}", final_messages.len());
    assert_eq!(final_messages[0].message_type, "user");
    assert_eq!(final_messages[0].content, "Original user message");
    assert_eq!(final_messages[1].message_type, "assistant");
    assert_eq!(final_messages[1].content, "Regenerated AI response v2 (latest)");
    assert_eq!(final_messages[0].generation_group_id, Some(group_id.clone()));
    assert_eq!(final_messages[1].generation_group_id, Some(group_id));
}

#[tokio::test]
async fn test_empty_message_list() {
    let message_details: Vec<MessageDetail> = Vec::new();
    let final_messages = process_message_versions(message_details);
    assert!(final_messages.is_empty());
}

#[tokio::test]
async fn test_single_user_message() {
    let base_time = Utc::now();
    let user_msg = create_message_detail(
        1,
        1,
        "user",
        "Hello",
        None,
        Some(Uuid::new_v4().to_string()),
        None,
        base_time,
    );

    let message_details = vec![user_msg];
    let final_messages = process_message_versions(message_details);

    assert_eq!(final_messages.len(), 1);
    assert_eq!(final_messages[0].content, "Hello");
    assert_eq!(final_messages[0].message_type, "user");
}
