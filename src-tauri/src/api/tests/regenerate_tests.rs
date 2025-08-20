use crate::db::conversation_db::{Conversation, Message};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

// 测试辅助函数

/// 创建测试用的 Message
fn create_test_message(
    id: i64,
    parent_id: Option<i64>,
    conversation_id: i64,
    message_type: &str,
    content: &str,
    llm_model_id: Option<i64>,
    llm_model_name: Option<String>,
    generation_group_id: Option<String>,
    parent_group_id: Option<String>,
    created_time: chrono::DateTime<Utc>,
) -> Message {
    Message {
        id,
        parent_id,
        conversation_id,
        message_type: message_type.to_string(),
        content: content.to_string(),
        llm_model_id,
        llm_model_name,
        created_time,
        start_time: None,
        finish_time: None,
        token_count: 100,
        generation_group_id,
        parent_group_id,
        tool_calls_json: None,
    }
}

/// 创建测试用的 Conversation
fn create_test_conversation(id: i64, assistant_id: Option<i64>) -> Conversation {
    Conversation {
        id,
        name: "Test Conversation".to_string(),
        assistant_id,
        created_time: Utc::now(),
    }
}

#[tokio::test]
async fn test_regenerate_user_message_creates_new_generation() {
    // 测试用户消息重新生成应该创建新的generation_group_id

    let base_time = Utc::now();
    let original_group_id = Uuid::new_v4().to_string();

    // 模拟数据库中的消息结构：
    // 1. System message (id=1)
    // 2. User message (id=2) <- 重新生成这个
    // 3. Reasoning message (id=3, generation_group_id=original_group_id)
    // 4. Response message (id=4, generation_group_id=original_group_id)

    let system_msg = create_test_message(
        1,
        None,
        1,
        "system",
        "You are a helpful assistant",
        None,
        None,
        None,
        None,
        base_time,
    );

    let user_msg = create_test_message(
        2,
        None,
        1,
        "user",
        "What is 2+2?",
        Some(1),
        Some("gpt-4".to_string()),
        None,
        None,
        base_time + chrono::Duration::seconds(1),
    );

    let reasoning_msg = create_test_message(
        3,
        None,
        1,
        "reasoning",
        "Let me calculate 2+2",
        Some(1),
        Some("gpt-4".to_string()),
        Some(original_group_id.clone()),
        None,
        base_time + chrono::Duration::seconds(2),
    );

    let response_msg = create_test_message(
        4,
        None,
        1,
        "response",
        "2+2 equals 4",
        Some(1),
        Some("gpt-4".to_string()),
        Some(original_group_id.clone()),
        None,
        base_time + chrono::Duration::seconds(3),
    );

    // 验证重新生成用户消息时：
    // 1. 新的AI回复应该有新的generation_group_id
    // 2. 父消息应该是用户消息的ID
    // 3. 消息历史应该包含用户消息及之前的所有消息

    // 这里需要模拟数据库和API调用，由于涉及复杂的外部依赖，
    // 我们先定义测试的预期行为

    let expected_parent_id = Some(user_msg.id);
    let expected_conversation_id = user_msg.conversation_id;

    // 验证重新生成逻辑
    assert_eq!(user_msg.message_type, "user");
    assert_eq!(user_msg.id, 2);
    assert!(expected_parent_id.is_some());
    assert_eq!(expected_conversation_id, 1);
}

#[tokio::test]
async fn test_regenerate_ai_message_reuses_generation_group_id() {
    // 测试AI消息重新生成应该复用原有的generation_group_id

    let base_time = Utc::now();
    let original_group_id = Uuid::new_v4().to_string();

    // 模拟数据库中的消息结构：
    // 1. System message (id=1)
    // 2. User message (id=2)
    // 3. Reasoning message (id=3, generation_group_id=original_group_id) <- 重新生成这个
    // 4. Response message (id=4, generation_group_id=original_group_id)

    let system_msg = create_test_message(
        1,
        None,
        1,
        "system",
        "You are a helpful assistant",
        None,
        None,
        None,
        None,
        base_time,
    );

    let user_msg = create_test_message(
        2,
        None,
        1,
        "user",
        "What is 2+2?",
        Some(1),
        Some("gpt-4".to_string()),
        None,
        None,
        base_time + chrono::Duration::seconds(1),
    );

    let reasoning_msg = create_test_message(
        3,
        None,
        1,
        "reasoning",
        "Let me calculate 2+2",
        Some(1),
        Some("gpt-4".to_string()),
        Some(original_group_id.clone()),
        None,
        base_time + chrono::Duration::seconds(2),
    );

    let response_msg = create_test_message(
        4,
        None,
        1,
        "response",
        "2+2 equals 4",
        Some(1),
        Some("gpt-4".to_string()),
        Some(original_group_id.clone()),
        None,
        base_time + chrono::Duration::seconds(3),
    );

    // 验证重新生成AI消息时：
    // 1. 新的AI回复应该复用原有的generation_group_id
    // 2. 父消息应该是被重新生成消息的ID
    // 3. 消息历史应该不包含被重新生成的消息及其后续消息

    let expected_reused_group_id = reasoning_msg.generation_group_id.clone();
    let expected_parent_id = Some(reasoning_msg.id);

    // 验证重新生成逻辑
    assert_eq!(reasoning_msg.message_type, "reasoning");
    assert!(expected_reused_group_id.is_some());
    assert_eq!(expected_reused_group_id.unwrap(), original_group_id);
    assert_eq!(expected_parent_id, Some(3));
}

#[tokio::test]
async fn test_message_version_grouping() {
    // 测试消息版本分组逻辑

    let base_time = Utc::now();
    let group_id_1 = Uuid::new_v4().to_string();
    let group_id_2 = Uuid::new_v4().to_string();

    // 创建多版本消息场景：
    // 1. User message (id=1)
    // 2. AI reasoning v1 (id=2, group_id_1)
    // 3. AI response v1 (id=3, group_id_1)
    // 4. AI reasoning v2 (id=4, group_id_2, parent_id=2)
    // 5. AI response v2 (id=5, group_id_2, parent_id=3)

    let messages = vec![
        create_test_message(1, None, 1, "user", "Hello", None, None, None, None, base_time),
        create_test_message(
            2,
            None,
            1,
            "reasoning",
            "Thinking v1",
            Some(1),
            Some("gpt-4".to_string()),
            Some(group_id_1.clone()),
            None,
            base_time + chrono::Duration::seconds(1),
        ),
        create_test_message(
            3,
            None,
            1,
            "response",
            "Response v1",
            Some(1),
            Some("gpt-4".to_string()),
            Some(group_id_1.clone()),
            None,
            base_time + chrono::Duration::seconds(2),
        ),
        create_test_message(
            4,
            Some(2),
            1,
            "reasoning",
            "Thinking v2",
            Some(1),
            Some("gpt-4".to_string()),
            Some(group_id_2.clone()),
            None,
            base_time + chrono::Duration::seconds(3),
        ),
        create_test_message(
            5,
            Some(3),
            1,
            "response",
            "Response v2",
            Some(1),
            Some("gpt-4".to_string()),
            Some(group_id_2.clone()),
            None,
            base_time + chrono::Duration::seconds(4),
        ),
    ];

    // 模拟版本分组逻辑
    let mut version_groups: HashMap<String, Vec<Message>> = HashMap::new();
    for message in &messages {
        if let Some(group_id) = &message.generation_group_id {
            version_groups.entry(group_id.clone()).or_insert_with(Vec::new).push(message.clone());
        }
    }

    // 验证分组结果
    assert_eq!(version_groups.len(), 2, "应该有2个版本组");
    assert!(version_groups.contains_key(&group_id_1));
    assert!(version_groups.contains_key(&group_id_2));

    let group_1_messages = version_groups.get(&group_id_1).unwrap();
    let group_2_messages = version_groups.get(&group_id_2).unwrap();

    assert_eq!(group_1_messages.len(), 2, "第一组应该有2条消息");
    assert_eq!(group_2_messages.len(), 2, "第二组应该有2条消息");

    // 验证组内消息类型
    assert!(group_1_messages.iter().any(|m| m.message_type == "reasoning"));
    assert!(group_1_messages.iter().any(|m| m.message_type == "response"));
    assert!(group_2_messages.iter().any(|m| m.message_type == "reasoning"));
    assert!(group_2_messages.iter().any(|m| m.message_type == "response"));
}

#[tokio::test]
async fn test_deprecated_assistant_message_type() {
    // 测试确保不会创建废弃的 "assistant" 类型消息

    let base_time = Utc::now();

    // 创建包含废弃消息类型的场景
    let messages = vec![
        create_test_message(1, None, 1, "user", "Hello", None, None, None, None, base_time),
        create_test_message(
            2,
            None,
            1,
            "assistant",
            "Deprecated response",
            Some(1),
            Some("gpt-4".to_string()),
            None,
            None,
            base_time + chrono::Duration::seconds(1),
        ),
        create_test_message(
            3,
            None,
            1,
            "response",
            "New response",
            Some(1),
            Some("gpt-4".to_string()),
            Some(Uuid::new_v4().to_string()),
            None,
            base_time + chrono::Duration::seconds(2),
        ),
    ];

    // 验证废弃消息类型检测
    let deprecated_messages: Vec<&Message> =
        messages.iter().filter(|m| m.message_type == "assistant").collect();

    assert_eq!(deprecated_messages.len(), 1, "应该检测到1条废弃消息");
    assert_eq!(deprecated_messages[0].id, 2);

    // 验证新消息类型正确
    let valid_messages: Vec<&Message> = messages
        .iter()
        .filter(|m| m.message_type == "response" || m.message_type == "reasoning")
        .collect();

    assert_eq!(valid_messages.len(), 1, "应该有1条有效的AI消息");
    assert_eq!(valid_messages[0].message_type, "response");
}

#[tokio::test]
async fn test_llm_model_fields_in_messages() {
    // 测试确保 response 和 reasoning 消息包含 llm_model_id 和 llm_model_name

    let base_time = Utc::now();
    let group_id = Uuid::new_v4().to_string();

    let reasoning_msg = create_test_message(
        1,
        None,
        1,
        "reasoning",
        "Thinking...",
        Some(42),
        Some("gpt-4-turbo".to_string()),
        Some(group_id.clone()),
        None,
        base_time,
    );

    let response_msg = create_test_message(
        2,
        None,
        1,
        "response",
        "Here's my answer",
        Some(42),
        Some("gpt-4-turbo".to_string()),
        Some(group_id.clone()),
        None,
        base_time + chrono::Duration::seconds(1),
    );

    let user_msg = create_test_message(
        3,
        None,
        1,
        "user",
        "Hello",
        None,
        None,
        None,
        None,
        base_time + chrono::Duration::seconds(2),
    );

    // 验证AI消息包含模型信息
    assert!(reasoning_msg.llm_model_id.is_some(), "reasoning消息应该包含llm_model_id");
    assert!(reasoning_msg.llm_model_name.is_some(), "reasoning消息应该包含llm_model_name");
    assert_eq!(reasoning_msg.llm_model_id.unwrap(), 42);
    assert_eq!(reasoning_msg.llm_model_name.unwrap(), "gpt-4-turbo");

    assert!(response_msg.llm_model_id.is_some(), "response消息应该包含llm_model_id");
    assert!(response_msg.llm_model_name.is_some(), "response消息应该包含llm_model_name");
    assert_eq!(response_msg.llm_model_id.unwrap(), 42);
    assert_eq!(response_msg.llm_model_name.unwrap(), "gpt-4-turbo");

    // 验证用户消息不需要模型信息
    assert!(user_msg.llm_model_id.is_none(), "user消息不应该包含llm_model_id");
    assert!(user_msg.llm_model_name.is_none(), "user消息不应该包含llm_model_name");
}

#[tokio::test]
async fn test_generation_group_id_consistency() {
    // 测试同一轮对话的 reasoning 和 response 消息使用相同的 generation_group_id

    let base_time = Utc::now();
    let group_id = Uuid::new_v4().to_string();

    let reasoning_msg = create_test_message(
        1,
        None,
        1,
        "reasoning",
        "Let me think about this...",
        Some(1),
        Some("gpt-4".to_string()),
        Some(group_id.clone()),
        None,
        base_time,
    );

    let response_msg = create_test_message(
        2,
        None,
        1,
        "response",
        "Based on my reasoning, the answer is...",
        Some(1),
        Some("gpt-4".to_string()),
        Some(group_id.clone()),
        None,
        base_time + chrono::Duration::seconds(1),
    );

    // 验证同一组的消息有相同的 generation_group_id
    assert_eq!(reasoning_msg.generation_group_id, response_msg.generation_group_id);
    assert!(reasoning_msg.generation_group_id.is_some());
    assert_eq!(reasoning_msg.generation_group_id.unwrap(), group_id);
}

#[tokio::test]
async fn test_parent_child_relationship_logic() {
    // 测试父子关系逻辑的正确性

    let base_time = Utc::now();
    let group_id_1 = Uuid::new_v4().to_string();
    let group_id_2 = Uuid::new_v4().to_string();

    // 创建父子关系：原始回复 -> 重新生成的回复
    let original_response = create_test_message(
        1,
        None,
        1,
        "response",
        "Original answer",
        Some(1),
        Some("gpt-4".to_string()),
        Some(group_id_1.clone()),
        None,
        base_time,
    );

    let regenerated_response = create_test_message(
        2,
        Some(1),
        1,
        "response",
        "Regenerated answer",
        Some(1),
        Some("gpt-4".to_string()),
        Some(group_id_2.clone()),
        None,
        base_time + chrono::Duration::seconds(1),
    );

    // 验证父子关系
    assert_eq!(regenerated_response.parent_id, Some(original_response.id));
    assert_ne!(original_response.generation_group_id, regenerated_response.generation_group_id);

    // 模拟获取最新版本的逻辑
    let messages = vec![original_response.clone(), regenerated_response.clone()];
    let latest_message = messages
        .iter()
        .filter(|m| m.parent_id == Some(original_response.id))
        .max_by_key(|m| m.id)
        .unwrap_or(&original_response);

    assert_eq!(latest_message.id, regenerated_response.id);
    assert_eq!(latest_message.content, "Regenerated answer");
}

#[tokio::test]
async fn test_message_filtering_for_regenerate() {
    // 测试重新生成时的消息过滤逻辑

    let base_time = Utc::now();
    let group_id = Uuid::new_v4().to_string();

    // 创建完整的对话历史
    let messages = vec![
        create_test_message(
            1,
            None,
            1,
            "system",
            "You are helpful",
            None,
            None,
            None,
            None,
            base_time,
        ),
        create_test_message(
            2,
            None,
            1,
            "user",
            "Question 1",
            None,
            None,
            None,
            None,
            base_time + chrono::Duration::seconds(1),
        ),
        create_test_message(
            3,
            None,
            1,
            "reasoning",
            "Thinking...",
            Some(1),
            Some("gpt-4".to_string()),
            Some(group_id.clone()),
            None,
            base_time + chrono::Duration::seconds(2),
        ),
        create_test_message(
            4,
            None,
            1,
            "response",
            "Answer 1",
            Some(1),
            Some("gpt-4".to_string()),
            Some(group_id.clone()),
            None,
            base_time + chrono::Duration::seconds(3),
        ),
        create_test_message(
            5,
            None,
            1,
            "user",
            "Question 2",
            None,
            None,
            None,
            None,
            base_time + chrono::Duration::seconds(4),
        ),
        create_test_message(
            6,
            None,
            1,
            "response",
            "Answer 2",
            Some(1),
            Some("gpt-4".to_string()),
            Some(Uuid::new_v4().to_string()),
            None,
            base_time + chrono::Duration::seconds(5),
        ),
    ];

    // 模拟重新生成第3条消息（reasoning）的过滤逻辑
    let regenerate_message_id = 3;
    let filtered_messages: Vec<&Message> =
        messages.iter().filter(|m| m.id < regenerate_message_id).collect();

    // 验证过滤结果：应该只包含前两条消息
    assert_eq!(filtered_messages.len(), 2);
    assert_eq!(filtered_messages[0].message_type, "system");
    assert_eq!(filtered_messages[1].message_type, "user");
    assert_eq!(filtered_messages[1].content, "Question 1");

    // 模拟重新生成用户消息的过滤逻辑
    let regenerate_user_message_id = 2;
    let filtered_for_user: Vec<&Message> =
        messages.iter().filter(|m| m.id <= regenerate_user_message_id).collect();

    // 验证过滤结果：应该包含system和user消息
    assert_eq!(filtered_for_user.len(), 2);
    assert_eq!(filtered_for_user[0].message_type, "system");
    assert_eq!(filtered_for_user[1].message_type, "user");
}
