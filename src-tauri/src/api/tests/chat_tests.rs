use crate::api::ai::chat::{extract_assistant_from_message, parse_assistant_mentions, ParseOptions, PositionRestriction};
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

// ========== 新的高级功能测试 ==========

#[tokio::test]
async fn test_parse_assistant_mentions_multiple_mentions() {
    let assistants = create_test_assistants();
    let content = "Hello @gpt4, please ask @claude about this, and @gemini-pro can help too.";
    
    let options = ParseOptions {
        first_only: false,
        position_restriction: PositionRestriction::Anywhere,
        remove_mentions: false,
        case_sensitive: true,
        require_word_boundary: true,
    };

    let result = parse_assistant_mentions(&assistants, content, &options).unwrap();
    
    assert_eq!(result.mentions.len(), 3);
    assert_eq!(result.mentions[0].assistant_id, 1); // gpt4
    assert_eq!(result.mentions[1].assistant_id, 2); // claude  
    assert_eq!(result.mentions[2].assistant_id, 3); // gemini-pro
    assert_eq!(result.original_content, content);
    assert_eq!(result.primary_assistant_id, Some(1));
}

#[tokio::test]
async fn test_parse_assistant_mentions_first_only_anywhere() {
    let assistants = create_test_assistants();
    let content = "Hello @gpt4, please ask @claude about this.";
    
    let options = ParseOptions {
        first_only: true,
        position_restriction: PositionRestriction::Anywhere,
        remove_mentions: false,
        case_sensitive: true,
        require_word_boundary: true,
    };

    let result = parse_assistant_mentions(&assistants, content, &options).unwrap();
    
    assert_eq!(result.mentions.len(), 1);
    assert_eq!(result.mentions[0].assistant_id, 1); // gpt4
    assert_eq!(result.primary_assistant_id, Some(1));
}

#[tokio::test]
async fn test_parse_assistant_mentions_word_boundary() {
    let assistants = create_test_assistants();
    let content = "Hello@gpt4 and @claude are here."; // 第一个@前面没空格，第二个有
    
    let options = ParseOptions {
        first_only: false,
        position_restriction: PositionRestriction::WordBoundary,
        remove_mentions: false,
        case_sensitive: true,
        require_word_boundary: true,
    };

    let result = parse_assistant_mentions(&assistants, content, &options).unwrap();
    
    // 应该只匹配第二个@claude，因为第一个@gpt4前面不是单词边界
    assert_eq!(result.mentions.len(), 1);
    assert_eq!(result.mentions[0].assistant_id, 2); // claude
    assert_eq!(result.primary_assistant_id, Some(2));
}

#[tokio::test]
async fn test_parse_assistant_mentions_remove_mentions() {
    let assistants = create_test_assistants();
    let content = "Hello @gpt4, can you help with @claude integration?";
    
    let options = ParseOptions {
        first_only: false,
        position_restriction: PositionRestriction::Anywhere,
        remove_mentions: true,
        case_sensitive: true,
        require_word_boundary: true,
    };

    let result = parse_assistant_mentions(&assistants, content, &options).unwrap();
    
    assert_eq!(result.mentions.len(), 2);
    assert_eq!(result.cleaned_content, "Hello can you help with integration?");
    assert_eq!(result.original_content, content);
}

#[tokio::test]  
async fn test_parse_assistant_mentions_case_insensitive() {
    let assistants = create_test_assistants();
    let content = "@GPT4 Hello world"; // 大写GPT4
    
    let options = ParseOptions {
        first_only: true,
        position_restriction: PositionRestriction::StartOnly,
        remove_mentions: true,
        case_sensitive: false, // 不区分大小写
        require_word_boundary: true,
    };

    let result = parse_assistant_mentions(&assistants, content, &options).unwrap();
    
    assert_eq!(result.mentions.len(), 1);
    assert_eq!(result.mentions[0].assistant_id, 1); // 应该匹配到gpt4
    assert_eq!(result.cleaned_content, "Hello world");
}

#[tokio::test]
async fn test_parse_assistant_mentions_no_space_required() {
    let assistants = create_test_assistants();
    let content = "@gpt4help me with this"; // @gpt4后面直接跟字母
    
    let options = ParseOptions {
        first_only: true,
        position_restriction: PositionRestriction::StartOnly,
        remove_mentions: false,
        case_sensitive: true,
        require_word_boundary: false, // 不要求词边界
    };

    let result = parse_assistant_mentions(&assistants, content, &options).unwrap();
    
    // 这种情况下不应该匹配，因为@gpt4help不是完整的gpt4名称
    assert_eq!(result.mentions.len(), 0);
    assert_eq!(result.primary_assistant_id, None);
}

#[tokio::test]
async fn test_parse_assistant_mentions_with_space_in_name() {
    let assistants = create_test_assistants();
    let content = "Hello @chat gpt, how are you?";
    
    let options = ParseOptions {
        first_only: true,
        position_restriction: PositionRestriction::Anywhere,
        remove_mentions: true,
        case_sensitive: true,
        require_word_boundary: true,
    };

    let result = parse_assistant_mentions(&assistants, content, &options).unwrap();
    
    assert_eq!(result.mentions.len(), 1);
    assert_eq!(result.mentions[0].assistant_id, 4); // chat gpt
    assert_eq!(result.cleaned_content, "Hello how are you?");
}

#[tokio::test]
async fn test_parse_assistant_mentions_unicode_and_emoji() {
    let assistants = create_test_assistants();
    let content = "你好 @中文名称，还有 @emoji👿 assistant";
    
    let options = ParseOptions {
        first_only: false,
        position_restriction: PositionRestriction::Anywhere,
        remove_mentions: true,
        case_sensitive: true,
        require_word_boundary: true,
    };

    let result = parse_assistant_mentions(&assistants, content, &options).unwrap();
    
    assert_eq!(result.mentions.len(), 2);
    assert_eq!(result.mentions[0].assistant_id, 5); // 中文名称
    assert_eq!(result.mentions[1].assistant_id, 6); // emoji👿
    assert_eq!(result.cleaned_content, "你好 还有 assistant");
}

#[tokio::test]
async fn test_parse_assistant_mentions_no_matches() {
    let assistants = create_test_assistants();
    let content = "Hello @unknown and @nonexistent helpers";
    
    let options = ParseOptions {
        first_only: false,
        position_restriction: PositionRestriction::Anywhere,
        remove_mentions: false,
        case_sensitive: true,
        require_word_boundary: true,
    };

    let result = parse_assistant_mentions(&assistants, content, &options).unwrap();
    
    assert_eq!(result.mentions.len(), 0);
    assert_eq!(result.primary_assistant_id, None);
    assert_eq!(result.cleaned_content, content); // 没有变化
    assert_eq!(result.original_content, content);
}

#[tokio::test]
async fn test_parse_assistant_mentions_overlapping_names() {
    // 测试名称重叠的情况，比如有gpt和gpt4两个助手
    let mut assistants = create_test_assistants();
    assistants.push(Assistant {
        id: 7,
        name: "gpt".to_string(),
        description: Some("GPT assistant".to_string()),
        assistant_type: Some(0),
        is_addition: false,
        created_time: chrono::Utc::now().to_rfc3339(),
    });

    let content = "@gpt4 and @gpt should work correctly";
    
    let options = ParseOptions {
        first_only: false,
        position_restriction: PositionRestriction::Anywhere,
        remove_mentions: false,
        case_sensitive: true,
        require_word_boundary: true,
    };

    let result = parse_assistant_mentions(&assistants, content, &options).unwrap();
    
    assert_eq!(result.mentions.len(), 2);
    // 应该正确匹配更长的名称优先
    assert_eq!(result.mentions[0].assistant_id, 1); // gpt4
    assert_eq!(result.mentions[1].assistant_id, 7); // gpt
}

// ========== 性能测试 ==========

#[tokio::test]
async fn test_parse_assistant_mentions_performance_large_text_with_mention() {
    let assistants = create_test_assistants();
    
    // 创建大段文本，在后段添加@助手
    let mut large_text = String::new();
    
    // 添加10000行文本内容，每行约100字符，总共约1MB
    for i in 0..10000 {
        large_text.push_str(&format!(
            "这是第{}行文本内容，包含一些中文字符和English words，用来测试性能表现。Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris.\n",
            i + 1
        ));
    }
    
    // 在后段添加@助手提及
    large_text.push_str("最后在这里提及 @gpt4 助手来帮助我们完成任务。");
    
    println!("测试文本长度: {} 字符", large_text.len());
    
    let options = ParseOptions {
        first_only: true,
        position_restriction: PositionRestriction::Anywhere,
        remove_mentions: false,
        case_sensitive: true,
        require_word_boundary: true,
    };

    // 开始性能测试
    let start_time = std::time::Instant::now();
    
    let result = parse_assistant_mentions(&assistants, &large_text, &options).unwrap();
    
    let elapsed = start_time.elapsed();
    
    println!("解析耗时: {:?}", elapsed);
    println!("找到的@mentions数量: {}", result.mentions.len());
    
    // 验证结果正确性
    assert_eq!(result.mentions.len(), 1);
    assert_eq!(result.mentions[0].assistant_id, 1); // gpt4
    assert_eq!(result.primary_assistant_id, Some(1));
    
    // 性能断言：在1MB文本中应该能在合理时间内完成（比如200ms内）
    assert!(elapsed.as_millis() < 200, "解析时间过长: {:?}", elapsed);
}

#[tokio::test]
async fn test_parse_assistant_mentions_performance_large_text_no_mention() {
    let assistants = create_test_assistants();
    
    // 创建大段文本，完全没有@符号
    let mut large_text = String::new();
    
    // 添加10000行文本内容，每行约100字符，总共约1MB
    for i in 0..10000 {
        large_text.push_str(&format!(
            "这是第{}行普通文本内容，包含一些中文字符和English words，但是没有任何助手提及符号。Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris.\n",
            i + 1
        ));
    }
    
    // 在最后添加一些不包含@的结尾文本
    large_text.push_str("最后的文本内容也没有任何助手提及，只是普通的文字内容。");
    
    println!("测试文本长度: {} 字符", large_text.len());
    
    let options = ParseOptions {
        first_only: false, // 搜索全部，这样会遍历整个文本
        position_restriction: PositionRestriction::Anywhere,
        remove_mentions: false,
        case_sensitive: true,
        require_word_boundary: true,
    };

    // 开始性能测试
    let start_time = std::time::Instant::now();
    
    let result = parse_assistant_mentions(&assistants, &large_text, &options).unwrap();
    
    let elapsed = start_time.elapsed();
    
    println!("解析耗时: {:?}", elapsed);
    println!("找到的@mentions数量: {}", result.mentions.len());
    
    // 验证结果正确性
    assert_eq!(result.mentions.len(), 0);
    assert_eq!(result.primary_assistant_id, None);
    
    // 性能断言：即使没有@符号，在1MB文本中也应该能在合理时间内完成（比如150ms内）
    assert!(elapsed.as_millis() < 150, "解析时间过长: {:?}", elapsed);
}

#[tokio::test]
async fn test_parse_assistant_mentions_performance_multiple_mentions_in_large_text() {
    let assistants = create_test_assistants();
    
    // 创建包含多个@提及的大段文本
    let mut large_text = String::new();
    
    // 前段添加一些@提及
    large_text.push_str("开始的时候我想请教 @claude 一些问题，");
    
    // 中间添加大量文本
    for i in 0..5000 {
        large_text.push_str(&format!(
            "这是第{}行文本内容，包含一些中文字符和English words。Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation.\n",
            i + 1
        ));
    }
    
    // 中段添加@提及
    large_text.push_str("中间部分我需要 @gpt4 来帮助分析，");
    
    // 继续添加大量文本
    for i in 5000..10000 {
        large_text.push_str(&format!(
            "这是第{}行文本内容，包含一些中文字符和English words。Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation.\n",
            i + 1
        ));
    }
    
    // 后段添加@提及
    large_text.push_str("最后请 @gemini-pro 来总结一下整个内容。");
    
    println!("测试文本长度: {} 字符", large_text.len());
    
    let options = ParseOptions {
        first_only: false, // 查找所有@提及
        position_restriction: PositionRestriction::Anywhere,
        remove_mentions: false,
        case_sensitive: true,
        require_word_boundary: true,
    };

    // 开始性能测试
    let start_time = std::time::Instant::now();
    
    let result = parse_assistant_mentions(&assistants, &large_text, &options).unwrap();
    
    let elapsed = start_time.elapsed();
    
    println!("解析耗时: {:?}", elapsed);
    println!("找到的@mentions数量: {}", result.mentions.len());
    
    // 验证结果正确性
    assert_eq!(result.mentions.len(), 3);
    assert_eq!(result.mentions[0].assistant_id, 2); // claude
    assert_eq!(result.mentions[1].assistant_id, 1); // gpt4
    assert_eq!(result.mentions[2].assistant_id, 3); // gemini-pro
    assert_eq!(result.primary_assistant_id, Some(2));
    
    // 性能断言：在包含多个@提及的1MB文本中应该能在合理时间内完成（比如200ms内）
    assert!(elapsed.as_millis() < 200, "解析时间过长: {:?}", elapsed);
}

#[tokio::test]  
async fn test_parse_assistant_mentions_performance_with_remove_mentions() {
    let assistants = create_test_assistants();
    
    // 创建包含多个@提及的大段文本，测试移除@提及的性能
    let mut large_text = String::new();
    
    large_text.push_str("请 @gpt4 帮我，");
    
    // 添加大量文本
    for i in 0..8000 {
        large_text.push_str(&format!(
            "第{}段内容：这里有很多文字内容需要处理和分析，包含中文和English mixed content。",
            i + 1
        ));
    }
    
    large_text.push_str("还需要 @claude 协助，");
    
    for i in 8000..10000 {
        large_text.push_str(&format!(
            "第{}段内容：继续添加更多的文本内容来测试性能表现。",
            i + 1
        ));
    }
    
    large_text.push_str("最后请 @中文名称 来总结。");
    
    println!("测试文本长度: {} 字符", large_text.len());
    
    let options = ParseOptions {
        first_only: false,
        position_restriction: PositionRestriction::Anywhere,
        remove_mentions: true, // 测试移除@提及的性能
        case_sensitive: true,
        require_word_boundary: true,
    };

    // 开始性能测试
    let start_time = std::time::Instant::now();
    
    let result = parse_assistant_mentions(&assistants, &large_text, &options).unwrap();
    
    let elapsed = start_time.elapsed();
    
    println!("解析并移除@mentions耗时: {:?}", elapsed);
    println!("找到的@mentions数量: {}", result.mentions.len());
    println!("清理后文本长度: {}", result.cleaned_content.len());
    
    // 验证结果正确性
    assert_eq!(result.mentions.len(), 3);
    assert!(result.cleaned_content.len() > 0);
    assert!(result.cleaned_content.len() < large_text.len()); // 应该比原文本短
    
    // 验证@提及被正确移除
    assert!(!result.cleaned_content.contains("@gpt4"));
    assert!(!result.cleaned_content.contains("@claude"));
    assert!(!result.cleaned_content.contains("@中文名称"));
    
    // 性能断言：移除@提及操作也应该在合理时间内完成（比如250ms内）
    assert!(elapsed.as_millis() < 250, "解析和清理时间过长: {:?}", elapsed);
}
