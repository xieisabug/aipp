use crate::db::conversation_db::*;
use chrono::Utc;
use rusqlite::Connection;
use std::collections::HashMap;
use uuid::Uuid;

/// 创建测试用数据库
fn create_conversation_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    
    // 禁用外键约束检查
    conn.execute("PRAGMA foreign_keys = OFF", []).unwrap();
    
    // 创建对话表
    conn.execute(
        "CREATE TABLE conversation (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            assistant_id INTEGER,
            created_time TEXT NOT NULL
        )", []
    ).unwrap();
    
    // 创建消息表
    conn.execute(
        "CREATE TABLE message (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            parent_id INTEGER,
            conversation_id INTEGER NOT NULL,
            message_type TEXT NOT NULL,
            content TEXT NOT NULL,
            llm_model_id INTEGER,
            llm_model_name TEXT,
            created_time TEXT NOT NULL,
            start_time TEXT,
            finish_time TEXT,
            token_count INTEGER DEFAULT 0,
            generation_group_id TEXT
        )", []
    ).unwrap();
    
    // 创建消息附件表
    conn.execute(
        "CREATE TABLE message_attachment (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message_id INTEGER NOT NULL,
            attachment_type INTEGER NOT NULL,
            attachment_url TEXT,
            attachment_content TEXT,
            attachment_hash TEXT,
            use_vector BOOLEAN DEFAULT 0,
            token_count INTEGER
        )", []
    ).unwrap();
    
    conn
}

/// 创建测试消息
fn create_test_message_for_api(
    conn: &Connection,
    conversation_id: i64,
    message_type: &str,
    content: &str,
    parent_id: Option<i64>,
    generation_group_id: Option<String>,
) -> i64 {
    conn.execute(
        "INSERT INTO message (conversation_id, message_type, content, llm_model_id, llm_model_name, created_time, token_count, parent_id, generation_group_id) 
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        (
            &conversation_id,
            &message_type,
            &content,
            &Some(1i64),
            &Some("test-model"),
            &Utc::now().to_rfc3339(),
            &100i32,
            &parent_id,
            &generation_group_id,
        )
    ).unwrap();
    conn.last_insert_rowid()
}

/// 创建测试对话
fn create_test_conversation_for_api(conn: &Connection, name: &str, assistant_id: i64) -> i64 {
    conn.execute(
        "INSERT INTO conversation (name, assistant_id, created_time) VALUES (?, ?, ?)",
        (&name, &assistant_id, &Utc::now().to_rfc3339())
    ).unwrap();
    conn.last_insert_rowid()
}

#[tokio::test]
async fn test_version_management_logic() {
    let conn = create_conversation_test_db();
    
    // 创建测试对话
    let conversation_id = create_test_conversation_for_api(&conn, "Test Conversation", 1);
    
    let group_id = Uuid::new_v4().to_string();
    
    // 创建原始用户消息
    let user_msg_id = create_test_message_for_api(
        &conn,
        conversation_id,
        "user",
        "Original user message",
        None,
        Some(group_id.clone())
    );
    
    // 创建原始 AI 回复消息
    let ai_msg_id = create_test_message_for_api(
        &conn,
        conversation_id,
        "assistant",
        "Original AI response",
        Some(user_msg_id),
        Some(group_id.clone())
    );
    
    // 创建 AI 消息重发版本1
    let ai_msg_v2_id = create_test_message_for_api(
        &conn,
        conversation_id,
        "assistant",
        "Regenerated AI response v1",
        Some(ai_msg_id),
        Some(group_id.clone())
    );
    
    // 创建 AI 消息重发版本2（最新版本）
    let _ai_msg_v3_id = create_test_message_for_api(
        &conn,
        conversation_id,
        "assistant",
        "Regenerated AI response v2 (latest)",
        Some(ai_msg_v2_id),
        Some(group_id.clone())
    );
    
    // 查询数据库获取所有消息
    let message_repo = MessageRepository::new(conn);
    let messages = message_repo
        .list_by_conversation_id(conversation_id)
        .unwrap();
    
    // 模拟 get_conversation_with_messages 的版本管理逻辑
    let mut message_details: Vec<MessageDetail> = Vec::new();
    let mut attachment_map: HashMap<i64, Vec<MessageAttachment>> = HashMap::new();
    
    for (message, attachment) in messages.clone() {
        if let Some(attachment) = attachment {
            attachment_map
                .entry(message.id)
                .or_default()
                .push(attachment);
        }
    }
    
    let message_map: HashMap<i64, Message> = messages
        .clone()
        .into_iter()
        .map(|(message, _)| (message.id, message))
        .collect();
    
    for (message_id, message) in message_map {
        let attachment_list = attachment_map.get(&message_id).cloned().unwrap_or_default();
        message_details.push(MessageDetail {
            id: message.id,
            conversation_id: message.conversation_id,
            message_type: message.message_type,
            content: message.content,
            llm_model_id: message.llm_model_id,
            created_time: message.created_time,
            start_time: message.start_time,
            finish_time: message.finish_time,
            token_count: message.token_count,
            generation_group_id: message.generation_group_id,
            attachment_list,
            regenerate: Vec::new(),
            parent_id: message.parent_id,
        });
    }
    
    // 构建 regenerate 关系
    let regenerate_map: HashMap<i64, Vec<MessageDetail>> = message_details
        .iter()
        .filter(|m| m.parent_id.is_some())
        .map(|m| (m.parent_id.unwrap(), m.clone()))
        .fold(HashMap::new(), |mut acc, (parent_id, message)| {
            acc.entry(parent_id).or_default().push(message);
            acc
        });
    
    for message in &mut message_details {
        if let Some(regenerated) = regenerate_map.get(&message.id) {
            let mut sorted_regenerated = regenerated.clone();
            sorted_regenerated.sort_by_key(|m| m.created_time);
            message.regenerate = sorted_regenerated;
        }
    }
    
    // 测试版本过滤逻辑
    let mut latest_children: HashMap<i64, MessageDetail> = HashMap::new();
    let mut child_message_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();
    
    for message in &message_details {
        if let Some(parent_id) = message.parent_id {
            child_message_ids.insert(message.id);
            latest_children
                .entry(parent_id)
                .and_modify(|existing| {
                    if message.created_time > existing.created_time {
                        *existing = message.clone();
                    }
                })
                .or_insert(message.clone());
        }
    }
    
    let mut final_messages: Vec<MessageDetail> = Vec::new();
    for message in message_details {
        if child_message_ids.contains(&message.id) {
            continue;
        }
        
        if let Some(latest_child) = latest_children.get(&message.id) {
            final_messages.push(latest_child.clone());
        } else {
            final_messages.push(message);
        }
    }
    
    final_messages.sort_by_key(|m| m.created_time);
    
    // 验证结果：应该只有用户消息和最新的AI回复
    assert_eq!(final_messages.len(), 2);
    
    // 第一条应该是用户消息
    assert_eq!(final_messages[0].message_type, "user");
    assert_eq!(final_messages[0].content, "Original user message");
    
    // 第二条应该是最新的AI回复
    assert_eq!(final_messages[1].message_type, "assistant");
    assert_eq!(final_messages[1].content, "Regenerated AI response v2 (latest)");
    
    // 验证 generation_group_id 相同
    assert_eq!(final_messages[0].generation_group_id, Some(group_id.clone()));
    assert_eq!(final_messages[1].generation_group_id, Some(group_id));
}

#[tokio::test]
async fn test_parent_child_relationship_building() {
    let conn = create_conversation_test_db();
    
    // 创建测试对话
    let conversation_id = create_test_conversation_for_api(&conn, "Test Conversation", 1);
    
    // 创建消息链：A -> B -> C -> D
    let msg_a_id = create_test_message_for_api(
        &conn,
        conversation_id,
        "user",
        "Message A",
        None,
        Some(Uuid::new_v4().to_string())
    );
    
    let msg_b_id = create_test_message_for_api(
        &conn,
        conversation_id,
        "assistant",
        "Message B",
        Some(msg_a_id),
        Some(Uuid::new_v4().to_string())
    );
    
    let msg_c_id = create_test_message_for_api(
        &conn,
        conversation_id,
        "user",
        "Message C",
        Some(msg_b_id),
        Some(Uuid::new_v4().to_string())
    );
    
    let _msg_d_id = create_test_message_for_api(
        &conn,
        conversation_id,
        "assistant",
        "Message D",
        Some(msg_c_id),
        Some(Uuid::new_v4().to_string())
    );
    
    // 查询消息
    let message_repo = MessageRepository::new(conn);
    let messages = message_repo
        .list_by_conversation_id(conversation_id)
        .unwrap();
    
    // 验证 parent_id 关系
    let message_map: HashMap<i64, Message> = messages
        .into_iter()
        .map(|(message, _)| (message.id, message))
        .collect();
    
    let msg_a = message_map.get(&msg_a_id).unwrap();
    let msg_b = message_map.get(&msg_b_id).unwrap();
    let msg_c = message_map.get(&msg_c_id).unwrap();
    
    assert_eq!(msg_a.parent_id, None);
    assert_eq!(msg_b.parent_id, Some(msg_a_id));
    assert_eq!(msg_c.parent_id, Some(msg_b_id));
}

#[tokio::test]
async fn test_multiple_generation_groups() {
    let conn = create_conversation_test_db();
    
    // 创建测试对话
    let conversation_id = create_test_conversation_for_api(&conn, "Test Conversation", 1);
    
    let group_1 = Uuid::new_v4().to_string();
    let group_2 = Uuid::new_v4().to_string();
    
    // 第一个生成组：用户消息 + AI回复
    let user_msg_1_id = create_test_message_for_api(
        &conn,
        conversation_id,
        "user",
        "User message 1",
        None,
        Some(group_1.clone())
    );
    
    let ai_msg_1_id = create_test_message_for_api(
        &conn,
        conversation_id,
        "assistant",
        "AI response 1",
        Some(user_msg_1_id),
        Some(group_1.clone())
    );
    
    // 第二个生成组：用户消息 + AI回复
    let user_msg_2_id = create_test_message_for_api(
        &conn,
        conversation_id,
        "user",
        "User message 2",
        Some(ai_msg_1_id), // 延续对话
        Some(group_2.clone())
    );
    
    let _ai_msg_2_id = create_test_message_for_api(
        &conn,
        conversation_id,
        "assistant",
        "AI response 2",
        Some(user_msg_2_id),
        Some(group_2.clone())
    );
    
    // 查询消息
    let message_repo = MessageRepository::new(conn);
    let messages = message_repo
        .list_by_conversation_id(conversation_id)
        .unwrap();
    
    // 验证不同 generation_group_id 的消息被正确创建
    let mut group_1_count = 0;
    let mut group_2_count = 0;
    
    for (message, _) in messages {
        match message.generation_group_id.as_deref() {
            Some(g) if g == group_1 => group_1_count += 1,
            Some(g) if g == group_2 => group_2_count += 1,
            _ => {}
        }
    }
    
    assert_eq!(group_1_count, 2); // 用户消息1 + AI回复1
    assert_eq!(group_2_count, 2); // 用户消息2 + AI回复2
}