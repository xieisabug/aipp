use std::path::PathBuf;

use chrono::prelude::*;
use rusqlite::{Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::utils::db_utils::{get_datetime_from_row, get_required_datetime_from_row};

use super::get_db_path;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum AttachmentType {
    Image = 1,
    Text = 2,
    PDF = 3,
    Word = 4,
    PowerPoint = 5,
    Excel = 6,
}

impl TryFrom<i64> for AttachmentType {
    type Error = rusqlite::Error;

    fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
        match value {
            1 => Ok(AttachmentType::Image),
            2 => Ok(AttachmentType::Text),
            3 => Ok(AttachmentType::PDF),
            4 => Ok(AttachmentType::Word),
            5 => Ok(AttachmentType::PowerPoint),
            6 => Ok(AttachmentType::Excel),
            _ => Err(rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Integer,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Invalid attachment type: {}", value),
                )),
            )),
        }
    }
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Conversation {
    pub id: i64,
    pub name: String,
    pub assistant_id: Option<i64>,
    pub created_time: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub conversation_id: i64,
    pub message_type: String,
    pub content: String,
    pub llm_model_id: Option<i64>,
    pub llm_model_name: Option<String>,
    pub created_time: DateTime<Utc>,
    pub start_time: Option<DateTime<Utc>>,
    pub finish_time: Option<DateTime<Utc>>,
    pub token_count: i32,
    pub generation_group_id: Option<String>,
    pub parent_group_id: Option<String>,
    pub tool_calls_json: Option<String>, // 保存原始 tool_calls JSON
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageDetail {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub conversation_id: i64,
    pub message_type: String,
    pub content: String,
    pub llm_model_id: Option<i64>,
    pub created_time: DateTime<Utc>,
    pub start_time: Option<DateTime<Utc>>,
    pub finish_time: Option<DateTime<Utc>>,
    pub token_count: i32,
    pub generation_group_id: Option<String>,
    pub parent_group_id: Option<String>,
    pub tool_calls_json: Option<String>,
    pub attachment_list: Vec<MessageAttachment>,
    pub regenerate: Vec<MessageDetail>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageAttachment {
    pub id: i64,
    pub message_id: i64,
    pub attachment_type: AttachmentType,
    pub attachment_url: Option<String>,
    pub attachment_content: Option<String>,
    pub attachment_hash: Option<String>,
    pub use_vector: bool,
    pub token_count: Option<i32>,
}

pub trait Repository<T> {
    fn create(&self, item: &T) -> Result<T>;
    fn read(&self, id: i64) -> Result<Option<T>>;
    fn update(&self, item: &T) -> Result<()>;
    fn delete(&self, id: i64) -> Result<()>;
}

pub struct ConversationRepository {
    conn: Connection,
}

impl ConversationRepository {
    pub fn new(conn: Connection) -> Self {
        ConversationRepository { conn }
    }

    pub fn list(&self, page: u32, per_page: u32) -> Result<Vec<Conversation>> {
        let offset = (page - 1) * per_page;
        let mut stmt = self.conn.prepare(
            "SELECT id, name, assistant_id, created_time
             FROM conversation
             ORDER BY created_time DESC
             LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt.query_map(&[&per_page, &offset], |row| {
            Ok(Conversation {
                id: row.get(0)?,
                name: row.get(1)?,
                assistant_id: row.get(2)?,
                created_time: get_required_datetime_from_row(row, 3, "created_time")?,
            })
        })?;
        rows.collect()
    }

    pub fn update_assistant_id(
        &self,
        origin_assistant_id: i64,
        assistant_id: Option<i64>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE conversation SET assistant_id = ?1 WHERE assistant_id = ?2",
            (&assistant_id, &origin_assistant_id),
        )?;
        Ok(())
    }

    pub fn update_name(&self, conversation: &Conversation) -> Result<()> {
        self.conn.execute(
            "UPDATE conversation SET name = ?1 WHERE id = ?2",
            (&conversation.name, &conversation.id),
        )?;
        Ok(())
    }
}

impl Repository<Conversation> for ConversationRepository {
    fn create(&self, conversation: &Conversation) -> Result<Conversation> {
        self.conn.execute(
            "INSERT INTO conversation (name, assistant_id, created_time) VALUES (?1, ?2, ?3)",
            (&conversation.name, &conversation.assistant_id, &conversation.created_time),
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(Conversation {
            id,
            name: conversation.name.clone(),
            assistant_id: conversation.assistant_id,
            created_time: conversation.created_time,
        })
    }

    fn read(&self, id: i64) -> Result<Option<Conversation>> {
        self.conn
            .query_row(
                "SELECT id, name, assistant_id, created_time FROM conversation WHERE id = ?",
                &[&id],
                |row| {
                    Ok(Conversation {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        assistant_id: row.get(2)?,
                        created_time: get_required_datetime_from_row(row, 3, "created_time")?,
                    })
                },
            )
            .optional()
    }

    fn update(&self, conversation: &Conversation) -> Result<()> {
        self.conn.execute(
            "UPDATE conversation SET name = ?1, assistant_id = ?2 WHERE id = ?3",
            (&conversation.name, &conversation.assistant_id, &conversation.id),
        )?;
        Ok(())
    }

    fn delete(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM conversation WHERE id = ?", &[&id])?;
        Ok(())
    }
}

pub struct MessageRepository {
    conn: Connection,
}

impl MessageRepository {
    pub fn new(conn: Connection) -> Self {
        MessageRepository { conn }
    }

    pub fn list_by_conversation_id(
        &self,
        conversation_id: i64,
    ) -> Result<Vec<(Message, Option<MessageAttachment>)>> {
        let mut stmt = self.conn.prepare("SELECT message.id, message.parent_id, message.conversation_id, message.message_type, message.content, message.llm_model_id, message.llm_model_name, message.created_time, message.start_time, message.finish_time, message.token_count, message.generation_group_id, message.parent_group_id, message.tool_calls_json, ma.attachment_type, ma.attachment_url, ma.attachment_content, ma.use_vector as attachment_use_vector, ma.token_count as attachment_token_count
                                          FROM message
                                          LEFT JOIN message_attachment ma on message.id = ma.message_id
                                          WHERE conversation_id = ?1")?;
        let rows = stmt.query_map(&[&conversation_id], |row| {
            let attachment_type_int: Option<i64> = row.get(14).ok();
            let attachment_type = attachment_type_int.map(AttachmentType::try_from).transpose()?;
            let message = Message {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                conversation_id: row.get(2)?,
                message_type: row.get(3)?,
                content: row.get(4)?,
                llm_model_id: row.get(5)?,
                llm_model_name: row.get(6)?,
                created_time: get_required_datetime_from_row(row, 7, "created_time")?,
                start_time: get_datetime_from_row(row, 8)?,
                finish_time: get_datetime_from_row(row, 9)?,
                token_count: row.get(10)?,
                generation_group_id: row.get(11)?,
                parent_group_id: row.get(12)?,
                tool_calls_json: row.get(13)?,
            };
            let attachment = if attachment_type.is_some() {
                Some(MessageAttachment {
                    id: 0,
                    message_id: row.get(0)?,
                    attachment_type: attachment_type.unwrap(),
                    attachment_url: row.get(15)?,
                    attachment_content: row.get(16)?,
                    attachment_hash: None,
                    use_vector: row.get(17)?,
                    token_count: row.get(18)?,
                })
            } else {
                None
            };
            Ok((message, attachment))
        })?;
        rows.collect()
    }

    pub fn update_finish_time(&self, id: i64) -> Result<()> {
        self.conn
            .execute("UPDATE message SET finish_time = CURRENT_TIMESTAMP WHERE id = ?1", [&id])?;
        Ok(())
    }

    /// 更新消息内容
    pub fn update_content(&self, id: i64, content: &str) -> Result<()> {
        self.conn.execute("UPDATE message SET content = ?1 WHERE id = ?2", (content, id))?;
        Ok(())
    }
}

impl Repository<Message> for MessageRepository {
    fn create(&self, message: &Message) -> Result<Message> {
        self.conn.execute(
            "INSERT INTO message (parent_id, conversation_id, message_type, content, llm_model_id, llm_model_name, created_time, start_time, finish_time, token_count, generation_group_id, parent_group_id, tool_calls_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            (
                &message.parent_id,
                &message.conversation_id,
                &message.message_type,
                &message.content,
                &message.llm_model_id,
                &message.llm_model_name,
                &message.created_time,
                &message.start_time,
                &message.finish_time,
                &message.token_count,
                &message.generation_group_id,
                &message.parent_group_id,
                &message.tool_calls_json,
            ),
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(Message {
            id,
            parent_id: message.parent_id,
            conversation_id: message.conversation_id,
            message_type: message.message_type.clone(),
            content: message.content.clone(),
            llm_model_id: message.llm_model_id,
            llm_model_name: message.llm_model_name.clone(),
            created_time: message.created_time,
            start_time: message.start_time,
            finish_time: message.finish_time,
            token_count: message.token_count,
            generation_group_id: message.generation_group_id.clone(),
            parent_group_id: message.parent_group_id.clone(),
            tool_calls_json: message.tool_calls_json.clone(),
        })
    }

    fn read(&self, id: i64) -> Result<Option<Message>> {
        self.conn
            .query_row("SELECT id, parent_id, conversation_id, message_type, content, llm_model_id, llm_model_name, created_time, start_time, finish_time, token_count, generation_group_id, parent_group_id, tool_calls_json FROM message WHERE id = ?", &[&id], |row| {
                Ok(Message {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    conversation_id: row.get(2)?,
                    message_type: row.get(3)?,
                    content: row.get(4)?,
                    llm_model_id: row.get(5)?,
                    llm_model_name: row.get(6)?,
                    created_time: get_required_datetime_from_row(row, 7, "created_time")?,
                    start_time: get_datetime_from_row(row, 8)?,
                    finish_time: get_datetime_from_row(row, 9)?,
                    token_count: row.get(10)?,
                    generation_group_id: row.get(11)?,
                    parent_group_id: row.get(12)?,
                    tool_calls_json: row.get(13)?,
                })
            })
            .optional()
    }

    fn update(&self, message: &Message) -> Result<()> {
        self.conn.execute(
            "UPDATE message SET conversation_id = ?1, message_type = ?2, content = ?3, llm_model_id = ?4, llm_model_name = ?5, token_count = ?6, tool_calls_json = ?7 WHERE id = ?8",
            (
                &message.conversation_id,
                &message.message_type,
                &message.content,
                &message.llm_model_id,
                &message.llm_model_name,
                &message.token_count,
                &message.tool_calls_json,
                &message.id,
            ),
        )?;
        Ok(())
    }

    fn delete(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM message WHERE id = ?", &[&id])?;
        Ok(())
    }
}

pub struct MessageAttachmentRepository {
    conn: Connection,
}

impl MessageAttachmentRepository {
    pub fn new(conn: Connection) -> Self {
        MessageAttachmentRepository { conn }
    }

    pub fn list_by_id(&self, id_list: &Vec<i64>) -> Result<Vec<MessageAttachment>> {
        let id_list_str: Vec<String> = id_list.iter().map(|id| id.to_string()).collect();
        let id_list_str = id_list_str.join(",");
        let query = format!("SELECT * FROM message_attachment WHERE id IN ({})", id_list_str);
        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt.query_map([], |row| {
            let attachment_type_int: i64 = row.get(2)?;
            let attachment_type = AttachmentType::try_from(attachment_type_int)?;
            Ok(MessageAttachment {
                id: row.get(0)?,
                message_id: row.get(1)?,
                attachment_type,
                attachment_url: row.get(3)?,
                attachment_content: row.get(4)?,
                attachment_hash: None,
                use_vector: row.get(5)?,
                token_count: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn read_by_attachment_hash(
        &self,
        attachment_hash: &str,
    ) -> Result<Option<MessageAttachment>> {
        self.conn
            .query_row("SELECT id, message_id, attachment_type, attachment_url, attachment_content, use_vector, token_count FROM message_attachment WHERE attachment_hash = ?", &[&attachment_hash], |row| {
                let attachment_type_int: i64 = row.get(2)?;
                let attachment_type = AttachmentType::try_from(attachment_type_int)?;
                Ok(MessageAttachment {
                    id: row.get(0)?,
                    message_id: row.get(1)?,
                    attachment_type,
                    attachment_url: row.get(3)?,
                    attachment_content: row.get(4)?,
                    attachment_hash: None,
                    use_vector: row.get(5)?,
                    token_count: row.get(6)?,
                })
            })
            .optional()
    }
}

impl Repository<MessageAttachment> for MessageAttachmentRepository {
    fn create(&self, attachment: &MessageAttachment) -> Result<MessageAttachment> {
        self.conn.execute(
            "INSERT INTO message_attachment (message_id, attachment_type, attachment_url, attachment_content, attachment_hash, use_vector, token_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (&attachment.message_id, &(attachment.attachment_type as i64), &attachment.attachment_url, &attachment.attachment_content, &attachment.attachment_hash, &attachment.use_vector, &attachment.token_count),
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(MessageAttachment {
            id,
            message_id: attachment.message_id,
            attachment_type: attachment.attachment_type,
            attachment_url: attachment.attachment_url.clone(),
            attachment_content: attachment.attachment_content.clone(),
            attachment_hash: None,
            use_vector: attachment.use_vector,
            token_count: attachment.token_count,
        })
    }

    fn read(&self, id: i64) -> Result<Option<MessageAttachment>> {
        self.conn
            .query_row("SELECT * FROM message_attachment WHERE id = ?", &[&id], |row| {
                let attachment_type_int: i64 = row.get(2)?;
                let attachment_type = AttachmentType::try_from(attachment_type_int)?;
                Ok(MessageAttachment {
                    id: row.get(0)?,
                    message_id: row.get(1)?,
                    attachment_type,
                    attachment_url: row.get(3)?,
                    attachment_content: row.get(4)?,
                    attachment_hash: None,
                    use_vector: row.get(5)?,
                    token_count: row.get(6)?,
                })
            })
            .optional()
    }

    fn update(&self, attachment: &MessageAttachment) -> Result<()> {
        self.conn.execute(
            "UPDATE message_attachment SET message_id = ?1 WHERE id = ?2",
            (&attachment.message_id, &attachment.id),
        )?;
        Ok(())
    }

    fn delete(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM message_attachment WHERE id = ?", &[&id])?;
        Ok(())
    }
}

pub struct ConversationDatabase {
    db_path: PathBuf,
}

impl ConversationDatabase {
    pub fn new(app_handle: &tauri::AppHandle) -> rusqlite::Result<Self> {
        let db_path = get_db_path(app_handle, "conversation.db");

        Ok(ConversationDatabase { db_path: db_path.unwrap() })
    }

    pub fn get_connection(&self) -> rusqlite::Result<Connection> {
        Connection::open(&self.db_path)
    }

    pub fn conversation_repo(&self) -> Result<ConversationRepository, AppError> {
        let conn = Connection::open(self.db_path.clone()).map_err(AppError::from)?;
        Ok(ConversationRepository::new(conn))
    }

    pub fn message_repo(&self) -> Result<MessageRepository, AppError> {
        let conn = Connection::open(self.db_path.clone()).map_err(AppError::from)?;
        Ok(MessageRepository::new(conn))
    }

    pub fn attachment_repo(&self) -> Result<MessageAttachmentRepository, AppError> {
        let conn = Connection::open(self.db_path.clone()).map_err(AppError::from)?;
        Ok(MessageAttachmentRepository::new(conn))
    }

    pub fn create_tables(&self) -> rusqlite::Result<()> {
        let conn = Connection::open(self.db_path.clone()).unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS conversation (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                assistant_id INTEGER,
                created_time DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS message (
                id              INTEGER
                primary key autoincrement,
                conversation_id INTEGER not null,
                message_type    TEXT    not null,
                content         TEXT    not null,
                llm_model_id    INTEGER,
                created_time    DATETIME default CURRENT_TIMESTAMP,
                token_count     INTEGER,
                parent_id       integer,
                start_time      DATETIME,
                finish_time     DATETIME,
                llm_model_name  TEXT,
                generation_group_id TEXT,
                parent_group_id TEXT,
                tool_calls_json TEXT
            )",
            [],
        )?;

        // 添加迁移逻辑：如果parent_group_id或tool_calls_json列不存在，则添加它们
        let mut stmt = conn.prepare("PRAGMA table_info(message)")?;
        let column_info: Vec<String> = stmt
            .query_map([], |row| {
                let column_name: String = row.get(1)?;
                Ok(column_name)
            })?
            .collect::<Result<Vec<String>, _>>()?;

        if !column_info.contains(&"parent_group_id".to_string()) {
            conn.execute("ALTER TABLE message ADD COLUMN parent_group_id TEXT", [])?;
        }
        if !column_info.contains(&"tool_calls_json".to_string()) {
            conn.execute("ALTER TABLE message ADD COLUMN tool_calls_json TEXT", [])?;
        }

        conn.execute(
            "CREATE TABLE IF NOT EXISTS message_attachment (
                id                 INTEGER
                primary key autoincrement,
                message_id         INTEGER,
                attachment_type    INTEGER           not null,
                attachment_url     TEXT,
                attachment_hash    TEXT,
                attachment_content TEXT,
                use_vector         BOOLEAN default 0 not null,
                token_count        INTEGER
            )",
            [],
        )?;

        Ok(())
    }
}
