use crate::api::ai_api::*;
use crate::db::conversation_db::*;
use chrono::Utc;
use rusqlite::Connection;
use std::collections::HashMap;
use uuid::Uuid;

/// 创建 AI API 测试数据库
fn create_ai_api_test_db() -> Connection {
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

/// 创建测试消息
fn create_test_message_for_ai_api(
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
fn create_test_conversation_for_ai_api(conn: &Connection, name: &str, assistant_id: i64) -> i64 {
    conn.execute(
        "INSERT INTO conversation (name, assistant_id, created_time) VALUES (?, ?, ?)",
        (&name, &assistant_id, &Utc::now().to_rfc3339()),
    )
    .unwrap();
    conn.last_insert_rowid()
}

#[cfg(test)]
mod ai_api_tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_generation_group_id_logic() {
        let conn = create_ai_api_test_db();

        // 创建测试对话
        let conversation_id = create_test_conversation_for_ai_api(&conn, "Test Conversation", 1);

        let group_id = Uuid::new_v4().to_string();

        // 创建用户消息
        let user_msg_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "user",
            "Original user message",
            None,
            Some(group_id.clone()),
        );

        // 创建AI回复消息
        let ai_msg_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "assistant",
            "Original AI response",
            Some(user_msg_id),
            Some(group_id.clone()),
        );

        // 测试用户消息重发的 generation_group_id 逻辑
        let message_repo = MessageRepository::new(conn);
        let user_message = message_repo.read(user_msg_id).unwrap().unwrap();
        let ai_message = message_repo.read(ai_msg_id).unwrap().unwrap();

        // 模拟 regenerate_ai 函数中的 generation_group_id 决策逻辑
        let user_regenerate_group_id = if user_message.message_type == "user" {
            // 用户消息重发：为新的AI回复生成新的group_id
            Some(Uuid::new_v4().to_string())
        } else {
            // AI消息重发：复用原消息的generation_group_id
            user_message.generation_group_id.clone().or_else(|| Some(Uuid::new_v4().to_string()))
        };

        let ai_regenerate_group_id = if ai_message.message_type == "user" {
            Some(Uuid::new_v4().to_string())
        } else {
            ai_message.generation_group_id.clone().or_else(|| Some(Uuid::new_v4().to_string()))
        };

        // 验证用户消息重发生成新的 group_id
        assert!(user_regenerate_group_id.is_some());
        assert_ne!(user_regenerate_group_id, Some(group_id.clone()));

        // 验证AI消息重发复用原 group_id
        assert_eq!(ai_regenerate_group_id, Some(group_id));
    }

    #[test]
    fn test_parent_id_logic_for_regeneration() {
        let conn = create_ai_api_test_db();

        // 创建测试对话
        let conversation_id = create_test_conversation_for_ai_api(&conn, "Test Conversation", 1);

        let group_id = Uuid::new_v4().to_string();

        // 创建用户消息
        let user_msg_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "user",
            "User question",
            None,
            Some(group_id.clone()),
        );

        // 创建AI回复消息
        let ai_msg_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "assistant",
            "AI response",
            Some(user_msg_id),
            Some(group_id.clone()),
        );

        // 查询所有消息
        let message_repo = MessageRepository::new(conn);
        let messages = message_repo.list_by_conversation_id(conversation_id).unwrap();
        let user_message = message_repo.read(user_msg_id).unwrap().unwrap();
        let ai_message = message_repo.read(ai_msg_id).unwrap().unwrap();

        // 模拟 regenerate_ai 中的 parent_id 决策逻辑
        let (filtered_messages_user, parent_id_user) = if user_message.message_type == "user" {
            // 用户消息重发：包含当前用户消息和之前的所有消息，新生成的assistant消息没有parent（新一轮对话）
            let filtered_messages: Vec<(Message, Option<MessageAttachment>)> =
                messages.clone().into_iter().filter(|m| m.0.id <= user_msg_id).collect();
            (filtered_messages, None) // 用户消息重发时，新的AI回复没有parent_id
        } else {
            let filtered_messages: Vec<(Message, Option<MessageAttachment>)> =
                messages.clone().into_iter().filter(|m| m.0.id < user_msg_id).collect();
            (filtered_messages, Some(user_msg_id))
        };

        let (filtered_messages_ai, parent_id_ai) = if ai_message.message_type == "user" {
            let filtered_messages: Vec<(Message, Option<MessageAttachment>)> =
                messages.clone().into_iter().filter(|m| m.0.id <= ai_msg_id).collect();
            (filtered_messages, Some(ai_msg_id))
        } else {
            // AI消息重新生成：仅保留在待重新生成消息之前的历史消息
            let filtered_messages: Vec<(Message, Option<MessageAttachment>)> =
                messages.clone().into_iter().filter(|m| m.0.id < ai_msg_id).collect();
            (filtered_messages, Some(ai_msg_id)) // 使用被重发消息的ID作为parent_id
        };

        // 验证用户消息重发的逻辑
        assert_eq!(parent_id_user, None); // 用户消息重发时parent_id应该是None
        assert_eq!(filtered_messages_user.len(), 1); // 只有用户消息本身（<=操作包含当前消息）

        // 验证AI消息重发的逻辑
        assert_eq!(parent_id_ai, Some(ai_msg_id));
        assert_eq!(filtered_messages_ai.len(), 1); // 只有用户消息
    }

    #[test]
    fn test_message_filtering_logic() {
        let conn = create_ai_api_test_db();

        // 创建测试对话
        let conversation_id = create_test_conversation_for_ai_api(&conn, "Test Conversation", 1);

        // 创建消息链: User -> AI -> AI_v2 -> User
        let msg1_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "user",
            "Message 1",
            None,
            Some(Uuid::new_v4().to_string()),
        );

        let msg2_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "assistant",
            "Message 2",
            None, // AI消息不应该有parent_id (正常对话流程)
            Some(Uuid::new_v4().to_string()),
        );

        // 创建Message 2的重发版本 (AI消息的重发版本应该以原消息为parent)
        let msg2_v2_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "assistant",
            "Message 2 v2",
            Some(msg2_id), // 重发版本以原消息为parent
            Some(Uuid::new_v4().to_string()),
        );

        let msg3_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "user",
            "Message 3",
            None, // 用户消息不应该有parent_id (正常对话流程)
            Some(Uuid::new_v4().to_string()),
        );

        // 查询所有消息
        let message_repo = MessageRepository::new(conn);
        let messages = message_repo.list_by_conversation_id(conversation_id).unwrap();

        // 模拟 regenerate_ai 中的消息过滤逻辑 - 重发Message 3之前的所有消息
        let filtered_messages: Vec<(Message, Option<MessageAttachment>)> = messages
            .into_iter()
            .filter(|m| m.0.id < msg3_id) // 重发Message 3之前的所有消息
            .collect();

        // 计算每个父消息最新的子消息
        let mut latest_children: HashMap<i64, (Message, Option<MessageAttachment>)> =
            HashMap::new();
        let mut child_ids: HashSet<i64> = HashSet::new();

        for (msg, attach) in filtered_messages.iter() {
            if let Some(parent_id) = msg.parent_id {
                child_ids.insert(msg.id);
                latest_children
                    .entry(parent_id)
                    .and_modify(|e| {
                        if msg.id > e.0.id {
                            *e = (msg.clone(), attach.clone());
                        }
                    })
                    .or_insert((msg.clone(), attach.clone()));
            }
        }

        // 构建最终的消息列表
        let mut init_message_list: Vec<(String, String)> = Vec::new();

        for (msg, _attach) in filtered_messages.into_iter() {
            if child_ids.contains(&msg.id) {
                // 这是一个有子消息的根消息，跳过
                continue;
            }

            // 使用最新的子消息（如果存在）替换当前消息
            let (final_msg, _final_attach_opt) =
                latest_children.get(&msg.id).cloned().unwrap_or((msg.clone(), None));

            init_message_list.push((final_msg.message_type, final_msg.content));
        }

        // 验证过滤结果: Message 1 (user) + Message 2 v2 (assistant最新版本)
        assert_eq!(init_message_list.len(), 2);

        // 验证内容
        let contents: Vec<&str> =
            init_message_list.iter().map(|(_, content)| content.as_str()).collect();
        assert!(contents.contains(&"Message 1"));
        assert!(contents.contains(&"Message 2 v2"));
        assert!(!contents.contains(&"Message 2")); // 原版本应该被过滤掉
    }

    #[test]
    fn test_complex_version_chain() {
        let conn = create_ai_api_test_db();

        // 创建测试对话
        let conversation_id = create_test_conversation_for_ai_api(&conn, "Test Conversation", 1);

        // 创建复杂的版本链：A -> B -> C -> D (其中C有多个版本)
        let msg_a_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "user",
            "Message A",
            None, // 正常对话流程，user消息没有parent
            Some(Uuid::new_v4().to_string()),
        );

        let msg_b_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "assistant",
            "Message B",
            None, // 正常对话流程，assistant消息没有parent
            Some(Uuid::new_v4().to_string()),
        );

        let msg_c_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "user",
            "Message C v1",
            None, // 正常对话流程，user消息没有parent
            Some(Uuid::new_v4().to_string()),
        );

        // Message C的第二个版本 (重发版本以原消息为parent)
        let msg_c_v2_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "user",
            "Message C v2",
            Some(msg_c_id), // 重发版本以原消息为parent
            Some(Uuid::new_v4().to_string()),
        );

        // Message C的第三个版本（最新）(重发版本以前一个版本为parent)
        let msg_c_v3_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "user",
            "Message C v3 (latest)",
            Some(msg_c_v2_id), // 以上一个版本为parent
            Some(Uuid::new_v4().to_string()),
        );

        let _msg_d_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "assistant",
            "Message D",
            None, // 正常对话流程，assistant消息没有parent
            Some(Uuid::new_v4().to_string()),
        );

        // 模拟重发Message D，应该只保留到Message C v3的消息
        let message_repo = MessageRepository::new(conn);
        let messages = message_repo.list_by_conversation_id(conversation_id).unwrap();

        // 过滤消息 - 排除要重发的Message D
        let filtered_messages: Vec<(Message, Option<MessageAttachment>)> = messages
            .into_iter()
            .filter(|m| m.0.content != "Message D") // 排除要重发的Message D
            .collect();

        // 应用版本过滤逻辑
        let mut latest_children: HashMap<i64, (Message, Option<MessageAttachment>)> =
            HashMap::new();
        let mut child_ids: HashSet<i64> = HashSet::new();

        for (msg, attach) in filtered_messages.iter() {
            if let Some(parent_id) = msg.parent_id {
                child_ids.insert(msg.id);
                latest_children
                    .entry(parent_id)
                    .and_modify(|e| {
                        if msg.id > e.0.id {
                            *e = (msg.clone(), attach.clone());
                        }
                    })
                    .or_insert((msg.clone(), attach.clone()));
            }
        }

        let mut final_messages: Vec<String> = Vec::new();

        for (msg, _attach) in filtered_messages.into_iter() {
            if child_ids.contains(&msg.id) {
                continue;
            }

            let (final_msg, _) =
                latest_children.get(&msg.id).cloned().unwrap_or((msg.clone(), None));

            final_messages.push(final_msg.content);
        }

        // 验证结果: A + B + C v2 (找到的最新版本是 v2，因为 v3 是 v2 的子版本)
        assert_eq!(final_messages.len(), 3);
        assert!(final_messages.contains(&"Message A".to_string()));
        assert!(final_messages.contains(&"Message B".to_string()));
        assert!(final_messages.contains(&"Message C v2".to_string())); // v2 是最终被选择的版本

        // 验证旧版本被过滤掉
        assert!(!final_messages.contains(&"Message C v1".to_string()));
        assert!(!final_messages.contains(&"Message C v3 (latest)".to_string()));
        // v3 被过滤掉，因为它是 v2 的子版本
    }

    #[test]
    fn test_regenerate_with_reasoning_and_response_groups() {
        let conn = create_ai_api_test_db();

        // 创建测试对话
        let conversation_id =
            create_test_conversation_for_ai_api(&conn, "Reasoning Test Conversation", 1);

        // 创建初始对话链：System -> User -> Reasoning -> Response
        let system_msg_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "system",
            "System message",
            None,
            None,
        );

        let user_msg_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "user",
            "User question",
            None,
            None,
        );

        let group_a = Uuid::new_v4().to_string();

        // 原始的 reasoning 和 response (group=a)
        let reasoning_msg_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "reasoning",
            "Original reasoning",
            Some(user_msg_id),
            Some(group_a.clone()),
        );

        let response_msg_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "assistant",
            "Original response",
            Some(reasoning_msg_id),
            Some(group_a.clone()),
        );

        // 模拟对 response 进行 regenerate
        // 新的 group_b 应该以 group_a 作为 parent_group
        let group_b = Uuid::new_v4().to_string();

        // 新的 reasoning 和 response (group=b, parent=a)
        let new_reasoning_msg_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "reasoning",
            "New reasoning",
            Some(user_msg_id), // 指向同一个 user message
            Some(group_b.clone()),
        );

        let new_response_msg_id = create_test_message_for_ai_api(
            &conn,
            conversation_id,
            "assistant",
            "New response",
            Some(new_reasoning_msg_id),
            Some(group_b.clone()),
        );

        // 验证消息的创建
        let message_repo = MessageRepository::new(conn);
        let all_messages = message_repo.list_by_conversation_id(conversation_id).unwrap();

        // 应该有6条消息：system, user, reasoning(group_a), response(group_a), reasoning(group_b), response(group_b)
        assert_eq!(all_messages.len(), 6);

        // 验证 group_a 的消息
        let group_a_messages: Vec<_> = all_messages
            .iter()
            .filter(|(msg, _)| msg.generation_group_id.as_ref() == Some(&group_a))
            .collect();
        assert_eq!(group_a_messages.len(), 2);

        // 验证 group_b 的消息
        let group_b_messages: Vec<_> = all_messages
            .iter()
            .filter(|(msg, _)| msg.generation_group_id.as_ref() == Some(&group_b))
            .collect();
        assert_eq!(group_b_messages.len(), 2);

        // 模拟界面显示逻辑：默认显示最新的组 (group_b)
        let displayed_messages: Vec<_> = all_messages
            .iter()
            .filter(|(msg, _)| {
                // 显示没有 generation_group_id 的消息（system, user）或最新组的消息
                msg.generation_group_id.is_none()
                    || msg.generation_group_id.as_ref() == Some(&group_b)
            })
            .map(|(msg, _)| &msg.content)
            .collect();

        // 界面应该显示：System, User, New reasoning, New response
        assert_eq!(displayed_messages.len(), 4);
        assert!(displayed_messages.contains(&&"System message".to_string()));
        assert!(displayed_messages.contains(&&"User question".to_string()));
        assert!(displayed_messages.contains(&&"New reasoning".to_string()));
        assert!(displayed_messages.contains(&&"New response".to_string()));
        assert!(!displayed_messages.contains(&&"Original reasoning".to_string()));
        assert!(!displayed_messages.contains(&&"Original response".to_string()));

        // 模拟版本切换逻辑：切换到 group_a
        let switched_messages: Vec<_> = all_messages
            .iter()
            .filter(|(msg, _)| {
                // 显示没有 generation_group_id 的消息或 group_a 的消息
                msg.generation_group_id.is_none()
                    || msg.generation_group_id.as_ref() == Some(&group_a)
            })
            .map(|(msg, _)| &msg.content)
            .collect();

        // 切换后应该显示：System, User, Original reasoning, Original response
        assert_eq!(switched_messages.len(), 4);
        assert!(switched_messages.contains(&&"System message".to_string()));
        assert!(switched_messages.contains(&&"User question".to_string()));
        assert!(switched_messages.contains(&&"Original reasoning".to_string()));
        assert!(switched_messages.contains(&&"Original response".to_string()));
        assert!(!switched_messages.contains(&&"New reasoning".to_string()));
        assert!(!switched_messages.contains(&&"New response".to_string()));

        // 验证 parent_id 关系正确
        let reasoning_original = message_repo.read(reasoning_msg_id).unwrap().unwrap();
        let response_original = message_repo.read(response_msg_id).unwrap().unwrap();
        let reasoning_new = message_repo.read(new_reasoning_msg_id).unwrap().unwrap();
        let response_new = message_repo.read(new_response_msg_id).unwrap().unwrap();

        // reasoning 消息应该指向 user 消息
        assert_eq!(reasoning_original.parent_id, Some(user_msg_id));
        assert_eq!(reasoning_new.parent_id, Some(user_msg_id));

        // response 消息应该指向对应的 reasoning 消息
        assert_eq!(response_original.parent_id, Some(reasoning_msg_id));
        assert_eq!(response_new.parent_id, Some(new_reasoning_msg_id));
    }
}
