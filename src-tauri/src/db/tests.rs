use super::*;
use crate::db::conversation_db::*;
use chrono::Utc;
use rusqlite::Connection;
use uuid::Uuid;

/// 创建内存测试数据库并初始化表结构
fn create_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();

    // 禁用外键约束检查，简化测试
    conn.execute("PRAGMA foreign_keys = OFF", []).unwrap();

    // 创建对话表
    conn.execute(
        "CREATE TABLE conversation (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            assistant_id INTEGER,
            created_time TEXT NOT NULL
        )",
        [],
    )
    .unwrap();

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
            generation_group_id TEXT,
            parent_group_id TEXT
        )",
        [],
    )
    .unwrap();

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
        )",
        [],
    )
    .unwrap();

    conn
}

/// 创建测试用的对话数据
fn create_test_conversation(repo: &ConversationRepository) -> Conversation {
    let conversation = Conversation {
        id: 0,
        name: "Test Conversation".to_string(),
        assistant_id: Some(1),
        created_time: Utc::now(),
    };
    repo.create(&conversation).unwrap()
}

/// 创建测试用的消息数据
fn create_test_message(
    conversation_id: i64,
    message_type: &str,
    content: &str,
    parent_id: Option<i64>,
    generation_group_id: Option<String>,
) -> Message {
    Message {
        id: 0,
        parent_id,
        conversation_id,
        message_type: message_type.to_string(),
        content: content.to_string(),
        llm_model_id: Some(1),
        llm_model_name: Some("test-model".to_string()),
        created_time: Utc::now(),
        start_time: None,
        finish_time: None,
        token_count: 100,
        generation_group_id,
        parent_group_id: None,
        tool_calls_json: None,
    }
}

/// 创建共享的测试数据库连接，包含对话和消息表
fn create_shared_test_db() -> (Connection, ConversationRepository, MessageRepository, Conversation)
{
    let conn = create_test_db();
    let conv_repo = ConversationRepository::new(Connection::open_in_memory().unwrap());
    let msg_repo = MessageRepository::new(Connection::open_in_memory().unwrap());

    // 创建一个共享的测试数据库用于对话和消息
    let shared_conn = create_test_db();
    let shared_conv_repo = ConversationRepository::new(Connection::open_in_memory().unwrap());

    // 在同一个连接中创建对话
    shared_conn
        .execute(
            "INSERT INTO conversation (name, assistant_id, created_time) VALUES (?, ?, ?)",
            (&"Test Conversation", &Some(1i64), &Utc::now().to_rfc3339()),
        )
        .unwrap();
    let conversation_id = shared_conn.last_insert_rowid();

    let conversation = Conversation {
        id: conversation_id,
        name: "Test Conversation".to_string(),
        assistant_id: Some(1),
        created_time: Utc::now(),
    };

    let shared_msg_repo = MessageRepository::new(shared_conn);

    (conn, conv_repo, shared_msg_repo, conversation)
}

#[cfg(test)]
mod conversation_repository_tests {
    use super::*;

    #[test]
    fn test_conversation_crud() {
        let conn = create_test_db();
        let repo = ConversationRepository::new(conn);

        // Test create
        let conversation = create_test_conversation(&repo);
        assert!(conversation.id > 0);
        assert_eq!(conversation.name, "Test Conversation");

        // Test read
        let read_conversation = repo.read(conversation.id).unwrap().unwrap();
        assert_eq!(read_conversation.id, conversation.id);
        assert_eq!(read_conversation.name, "Test Conversation");

        // Test update
        let mut updated_conversation = read_conversation.clone();
        updated_conversation.name = "Updated Conversation".to_string();
        repo.update(&updated_conversation).unwrap();

        let updated_read = repo.read(conversation.id).unwrap().unwrap();
        assert_eq!(updated_read.name, "Updated Conversation");

        // Test delete
        repo.delete(conversation.id).unwrap();
        let deleted_read = repo.read(conversation.id).unwrap();
        assert!(deleted_read.is_none());
    }
}

#[cfg(test)]
mod message_repository_tests {
    use super::*;

    #[test]
    fn test_message_crud() {
        let (_, _, msg_repo, conversation) = create_shared_test_db();

        // Test create message
        let message = create_test_message(
            conversation.id,
            "user",
            "Test message",
            None,
            Some(Uuid::new_v4().to_string()),
        );
        let created_message = msg_repo.create(&message).unwrap();
        assert!(created_message.id > 0);
        assert_eq!(created_message.content, "Test message");

        // Test read message
        let read_message = msg_repo.read(created_message.id).unwrap().unwrap();
        assert_eq!(read_message.id, created_message.id);
        assert_eq!(read_message.content, "Test message");

        // Test update message
        let mut updated_message = read_message.clone();
        updated_message.content = "Updated message".to_string();
        msg_repo.update(&updated_message).unwrap();

        let updated_read = msg_repo.read(created_message.id).unwrap().unwrap();
        assert_eq!(updated_read.content, "Updated message");

        // Test delete message
        msg_repo.delete(created_message.id).unwrap();
        let deleted_read = msg_repo.read(created_message.id).unwrap();
        assert!(deleted_read.is_none());
    }

    #[test]
    fn test_list_messages_by_conversation_id() {
        let (_, _, msg_repo, conversation) = create_shared_test_db();

        // 创建多条消息
        let messages = vec![
            create_test_message(
                conversation.id,
                "user",
                "Message 1",
                None,
                Some(Uuid::new_v4().to_string()),
            ),
            create_test_message(
                conversation.id,
                "assistant",
                "Message 2",
                None,
                Some(Uuid::new_v4().to_string()),
            ),
            create_test_message(
                conversation.id,
                "user",
                "Message 3",
                None,
                Some(Uuid::new_v4().to_string()),
            ),
        ];

        for message in &messages {
            msg_repo.create(message).unwrap();
        }

        // 查询对话的所有消息
        let retrieved_messages = msg_repo.list_by_conversation_id(conversation.id).unwrap();
        assert_eq!(retrieved_messages.len(), 3);

        // 验证消息内容
        let contents: Vec<String> =
            retrieved_messages.iter().map(|(msg, _)| msg.content.clone()).collect();
        assert!(contents.contains(&"Message 1".to_string()));
        assert!(contents.contains(&"Message 2".to_string()));
        assert!(contents.contains(&"Message 3".to_string()));
    }
}

#[cfg(test)]
mod version_management_tests {
    use super::*;

    #[test]
    fn test_generation_group_id_management() {
        let (_, _, msg_repo, conversation) = create_shared_test_db();

        let group_id = Uuid::new_v4().to_string();

        // 创建用户消息
        let user_message = create_test_message(
            conversation.id,
            "user",
            "User question",
            None,
            Some(group_id.clone()),
        );
        let created_user_msg = msg_repo.create(&user_message).unwrap();

        // 创建AI回复消息（使用相同的generation_group_id）
        let ai_message = create_test_message(
            conversation.id,
            "assistant",
            "AI response",
            Some(created_user_msg.id),
            Some(group_id.clone()),
        );
        let created_ai_msg = msg_repo.create(&ai_message).unwrap();

        // 验证generation_group_id相同
        assert_eq!(created_user_msg.generation_group_id, Some(group_id.clone()));
        assert_eq!(created_ai_msg.generation_group_id, Some(group_id.clone()));

        // 验证parent_id关系
        assert_eq!(created_ai_msg.parent_id, Some(created_user_msg.id));
    }

    #[test]
    fn test_parent_child_relationships() {
        let (_, _, msg_repo, conversation) = create_shared_test_db();

        // 创建消息链：用户消息 -> AI回复 -> 用户回复 -> AI回复
        let user_msg1 = create_test_message(
            conversation.id,
            "user",
            "First user message",
            None,
            Some(Uuid::new_v4().to_string()),
        );
        let created_user_msg1 = msg_repo.create(&user_msg1).unwrap();

        let ai_msg1 = create_test_message(
            conversation.id,
            "assistant",
            "First AI response",
            Some(created_user_msg1.id),
            Some(Uuid::new_v4().to_string()),
        );
        let created_ai_msg1 = msg_repo.create(&ai_msg1).unwrap();

        let user_msg2 = create_test_message(
            conversation.id,
            "user",
            "Second user message",
            Some(created_ai_msg1.id),
            Some(Uuid::new_v4().to_string()),
        );
        let created_user_msg2 = msg_repo.create(&user_msg2).unwrap();

        let ai_msg2 = create_test_message(
            conversation.id,
            "assistant",
            "Second AI response",
            Some(created_user_msg2.id),
            Some(Uuid::new_v4().to_string()),
        );
        let created_ai_msg2 = msg_repo.create(&ai_msg2).unwrap();

        // 验证parent_id关系链
        assert_eq!(created_user_msg1.parent_id, None);
        assert_eq!(created_ai_msg1.parent_id, Some(created_user_msg1.id));
        assert_eq!(created_user_msg2.parent_id, Some(created_ai_msg1.id));
        assert_eq!(created_ai_msg2.parent_id, Some(created_user_msg2.id));

        // 查询所有消息
        let all_messages = msg_repo.list_by_conversation_id(conversation.id).unwrap();
        assert_eq!(all_messages.len(), 4);
    }

    #[test]
    fn test_message_regeneration_scenarios() {
        let (_, _, msg_repo, conversation) = create_shared_test_db();

        let original_group_id = Uuid::new_v4().to_string();

        // 创建原始用户消息
        let original_user_msg = create_test_message(
            conversation.id,
            "user",
            "Original user message",
            None,
            Some(original_group_id.clone()),
        );
        let created_original_user = msg_repo.create(&original_user_msg).unwrap();

        // 创建原始AI回复
        let original_ai_msg = create_test_message(
            conversation.id,
            "assistant",
            "Original AI response",
            Some(created_original_user.id),
            Some(original_group_id.clone()),
        );
        let created_original_ai = msg_repo.create(&original_ai_msg).unwrap();

        // 模拟AI消息重发（应该使用相同的generation_group_id）
        let regenerated_ai_msg = create_test_message(
            conversation.id,
            "assistant",
            "Regenerated AI response",
            Some(created_original_ai.id),    // parent_id指向被重发的消息
            Some(original_group_id.clone()), // 使用相同的generation_group_id
        );
        let created_regenerated_ai = msg_repo.create(&regenerated_ai_msg).unwrap();

        // 验证重发逻辑
        assert_eq!(created_regenerated_ai.generation_group_id, Some(original_group_id.clone()));
        assert_eq!(created_regenerated_ai.parent_id, Some(created_original_ai.id));

        // 模拟用户消息重发（应该创建新的generation_group_id）
        let new_group_id = Uuid::new_v4().to_string();
        let regenerated_user_msg = create_test_message(
            conversation.id,
            "user",
            "Regenerated user message",
            Some(created_original_user.id), // parent_id指向被重发的消息
            Some(new_group_id.clone()),     // 新的generation_group_id
        );
        let created_regenerated_user = msg_repo.create(&regenerated_user_msg).unwrap();

        // 验证用户重发逻辑
        assert_eq!(created_regenerated_user.generation_group_id, Some(new_group_id));
        assert_eq!(created_regenerated_user.parent_id, Some(created_original_user.id));
        assert_ne!(created_regenerated_user.generation_group_id, Some(original_group_id));

        // 查询所有消息
        let all_messages = msg_repo.list_by_conversation_id(conversation.id).unwrap();
        assert_eq!(all_messages.len(), 4);
    }
}
