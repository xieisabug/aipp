use crate::api::ai::chat::extract_assistant_from_message;
use crate::db::assistant_db::Assistant;

/// 创建测试用的助手列表
fn create_test_assistants() -> Vec<Assistant> {
    vec![
        Assistant {
            id: 1,
            name: "gpt4".to_string(),
            description: Some("GPT-4 assistant".to_string()),
            assistant_type: Some(0),
            is_addition: false,
            created_time: chrono::Utc::now().to_rfc3339(),
        },
        Assistant {
            id: 2,
            name: "claude".to_string(),
            description: Some("Claude assistant".to_string()),
            assistant_type: Some(0),
            is_addition: false,
            created_time: chrono::Utc::now().to_rfc3339(),
        },
        Assistant {
            id: 3,
            name: "gemini-pro".to_string(),
            description: Some("Gemini Pro assistant".to_string()),
            assistant_type: Some(0),
            is_addition: false,
            created_time: chrono::Utc::now().to_rfc3339(),
        },
        Assistant {
            id: 4,
            name: "chat gpt".to_string(),
            description: Some("ChatGPT assistant with space".to_string()),
            assistant_type: Some(0),
            is_addition: false,
            created_time: chrono::Utc::now().to_rfc3339(),
        },
        Assistant {
            id: 5,
            name: "中文名称".to_string(),
            description: Some("ChatGPT assistant with space".to_string()),
            assistant_type: Some(0),
            is_addition: false,
            created_time: chrono::Utc::now().to_rfc3339(),
        },
        Assistant {
            id: 6,
            name: "emoji👿".to_string(),
            description: Some("ChatGPT assistant with space".to_string()),
            assistant_type: Some(0),
            is_addition: false,
            created_time: chrono::Utc::now().to_rfc3339(),
        },
    ]
}

#[tokio::test]
async fn test_extract_assistant_from_message_with_existing_assistant() {
    let assistants = create_test_assistants();
    let prompt = "@gpt4 Hello, how are you?";
    let default_assistant_id = 999;

    let result = extract_assistant_from_message(&assistants, prompt, default_assistant_id).await;

    assert!(result.is_ok());
    let (assistant_id, cleaned_prompt) = result.unwrap();
    assert_eq!(assistant_id, 1); // gpt4 的 ID
    assert_eq!(cleaned_prompt, "Hello, how are you?");
}

#[tokio::test]
async fn test_extract_assistant_from_message_with_nonexistent_assistant() {
    let assistants = create_test_assistants();
    let prompt = "@unknown Hello, how are you?";
    let default_assistant_id = 999;

    let result = extract_assistant_from_message(&assistants, prompt, default_assistant_id).await;

    assert!(result.is_ok());
    let (assistant_id, cleaned_prompt) = result.unwrap();
    assert_eq!(assistant_id, 999); // 应该返回默认 ID
    assert_eq!(cleaned_prompt, "@unknown Hello, how are you?"); // 原始消息不变
}

#[tokio::test]
async fn test_extract_assistant_from_message_without_at_symbol() {
    let assistants = create_test_assistants();
    let prompt = "Hello, how are you?";
    let default_assistant_id = 999;

    let result = extract_assistant_from_message(&assistants, prompt, default_assistant_id).await;

    assert!(result.is_ok());
    let (assistant_id, cleaned_prompt) = result.unwrap();
    assert_eq!(assistant_id, 999); // 应该返回默认 ID
    assert_eq!(cleaned_prompt, "Hello, how are you?"); // 原始消息不变
}

#[tokio::test]
async fn test_extract_assistant_from_message_with_at_symbol_in_middle() {
    let assistants = create_test_assistants();
    let prompt = "Hello @gpt4, how are you?";
    let default_assistant_id = 999;

    let result = extract_assistant_from_message(&assistants, prompt, default_assistant_id).await;

    assert!(result.is_ok());
    let (assistant_id, cleaned_prompt) = result.unwrap();
    assert_eq!(assistant_id, 999); // 应该返回默认 ID（因为@不在开头）
    assert_eq!(cleaned_prompt, "Hello @gpt4, how are you?"); // 原始消息不变
}

#[tokio::test]
async fn test_extract_assistant_from_message_with_special_assistant_name() {
    let assistants = create_test_assistants();
    let prompt = "@gemini-pro Can you help me with coding?";
    let default_assistant_id = 999;

    let result = extract_assistant_from_message(&assistants, prompt, default_assistant_id).await;

    assert!(result.is_ok());
    let (assistant_id, cleaned_prompt) = result.unwrap();
    assert_eq!(assistant_id, 3); // gemini-pro 的 ID
    assert_eq!(cleaned_prompt, "Can you help me with coding?");
}

#[tokio::test]
async fn test_extract_assistant_from_message_with_only_at_symbol() {
    let assistants = create_test_assistants();
    let prompt = "@";
    let default_assistant_id = 999;

    let result = extract_assistant_from_message(&assistants, prompt, default_assistant_id).await;

    assert!(result.is_ok());
    let (assistant_id, cleaned_prompt) = result.unwrap();
    assert_eq!(assistant_id, 999); // 应该返回默认 ID
    assert_eq!(cleaned_prompt, "@"); // 原始消息不变
}

#[tokio::test]
async fn test_extract_assistant_from_message_with_at_symbol_but_no_space() {
    let assistants = create_test_assistants();
    let prompt = "@gpt4help me";
    let default_assistant_id = 999;

    let result = extract_assistant_from_message(&assistants, prompt, default_assistant_id).await;

    assert!(result.is_ok());
    let (assistant_id, cleaned_prompt) = result.unwrap();
    // 正则表达式要求 @assistant_name 后面有空格，所以这个应该不匹配
    assert_eq!(assistant_id, 999); // 应该返回默认 ID
    assert_eq!(cleaned_prompt, "@gpt4help me"); // 原始消息不变
}

#[tokio::test]
async fn test_extract_assistant_from_message_case_sensitive() {
    let assistants = create_test_assistants();
    let prompt = "@GPT4 Hello, how are you?"; // 大写的 GPT4
    let default_assistant_id = 999;

    let result = extract_assistant_from_message(&assistants, prompt, default_assistant_id).await;

    assert!(result.is_ok());
    let (assistant_id, cleaned_prompt) = result.unwrap();
    // 应该区分大小写，所以找不到 GPT4（只有 gpt4）
    assert_eq!(assistant_id, 999); // 应该返回默认 ID
    assert_eq!(cleaned_prompt, "@GPT4 Hello, how are you?"); // 原始消息不变
}

#[tokio::test]
async fn test_extract_assistant_from_message_with_space() {
    let assistants = create_test_assistants();
    let prompt = "@chat gpt Hello, how are you?";
    let default_assistant_id = 999;

    let result = extract_assistant_from_message(&assistants, prompt, default_assistant_id).await;

    assert!(result.is_ok());
    let (assistant_id, cleaned_prompt) = result.unwrap();
    assert_eq!(assistant_id, 4);
    assert_eq!(cleaned_prompt, "Hello, how are you?"); // 原始消息不变
}

#[tokio::test]
async fn test_extract_assistant_from_message_with_chinese_name() {
    let assistants = create_test_assistants();
    let prompt = "@中文名称 Hello, how are you?";
    let default_assistant_id = 999;

    let result = extract_assistant_from_message(&assistants, prompt, default_assistant_id).await;

    assert!(result.is_ok());
    let (assistant_id, cleaned_prompt) = result.unwrap();
    assert_eq!(assistant_id, 5);
    assert_eq!(cleaned_prompt, "Hello, how are you?"); // 原始消息不变
}

#[tokio::test]
async fn test_extract_assistant_from_message_with_emoji_name() {
    let assistants = create_test_assistants();
    let prompt = "@emoji👿 Hello, how are you?";
    let default_assistant_id = 999;

    let result = extract_assistant_from_message(&assistants, prompt, default_assistant_id).await;

    assert!(result.is_ok());
    let (assistant_id, cleaned_prompt) = result.unwrap();
    assert_eq!(assistant_id, 6);
    assert_eq!(cleaned_prompt, "Hello, how are you?"); // 原始消息不变
}
