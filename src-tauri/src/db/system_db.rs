use rusqlite::{params, Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};

use super::get_db_path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeatureConfig {
    pub id: Option<i64>,
    pub feature_code: String,
    pub key: String,
    pub value: String,
    pub data_type: String,
    pub description: Option<String>,
}

pub struct SystemDatabase {
    pub conn: Connection,
}

impl SystemDatabase {
    pub fn new(app_handle: &tauri::AppHandle) -> Result<Self> {
        let db_path = get_db_path(app_handle, "system.db");
        let conn = Connection::open(db_path.unwrap())?;
        Ok(SystemDatabase { conn })
    }

    pub fn create_tables(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS system_config (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    key TEXT NOT NULL UNIQUE,
                    value TEXT NOT NULL,
                    created_time DATETIME DEFAULT CURRENT_TIMESTAMP
                );",
            [],
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS feature_config (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                feature_code TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT,
                data_type TEXT,
                description TEXT,
                UNIQUE(feature_code, key)
            )",
            [],
        )?;
        Ok(())
    }

    pub fn add_system_config(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO system_config (key, value) VALUES (?, ?)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_config(&self, key: &str) -> Result<String> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM system_config WHERE key = ?")?;
        let mut rows = stmt.query_map(params![key], |row| Ok(row.get(0)?))?;

        if let Some(row) = rows.next() {
            let value = row?; // Handle potential error
            Ok(value)
        } else {
            Ok(String::new())
        }
    }

    pub fn update_system_config(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE system_config SET value = ? WHERE key = ?",
            params![value, key],
        )?;
        Ok(())
    }

    pub fn delete_system_config(&self, key: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM system_config WHERE key = ?", params![key])?;
        Ok(())
    }

    pub fn add_feature_config(&self, config: &FeatureConfig) -> Result<()> {
        self.conn.execute(
            "INSERT INTO feature_config (feature_code, key, value, data_type, description)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                config.feature_code,
                config.key,
                config.value,
                config.data_type,
                config.description
            ],
        )?;
        Ok(())
    }

    pub fn update_feature_config(&self, config: &FeatureConfig) -> Result<()> {
        self.conn.execute(
            "UPDATE feature_config SET value = ?1, data_type = ?2, description = ?3
             WHERE feature_code = ?4 AND key = ?5",
            params![
                config.value,
                config.data_type,
                config.description,
                config.feature_code,
                config.key
            ],
        )?;
        Ok(())
    }

    pub fn delete_feature_config_by_feature_code(&self, feature_code: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM feature_config WHERE feature_code = ?1",
            params![feature_code],
        )?;
        Ok(())
    }

    pub fn get_feature_config(&self, feature_code: &str, key: &str) -> Result<Option<FeatureConfig>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, feature_code, key, value, data_type, description
             FROM feature_config WHERE feature_code = ?1 AND key = ?2",
        )?;
        let config = stmt
            .query_row(params![feature_code, key], |row| {
                Ok(FeatureConfig {
                    id: Some(row.get(0)?),
                    feature_code: row.get(1)?,
                    key: row.get(2)?,
                    value: row.get(3)?,
                    data_type: row.get(4)?,
                    description: row.get(5)?,
                })
            })
            .optional()?;
        Ok(config)
    }

    // 查询特定模块的所有配置
    fn get_feature_config_by_module(&self, feature_code: &str) -> Result<Vec<FeatureConfig>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, feature_code, key, value, data_type, description
             FROM feature_config WHERE feature_code = ?1",
        )?;
        let configs = stmt
            .query_map(params![feature_code], |row| {
                Ok(FeatureConfig {
                    id: Some(row.get(0)?),
                    feature_code: row.get(1)?,
                    key: row.get(2)?,
                    value: row.get(3)?,
                    data_type: row.get(4)?,
                    description: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(configs)
    }

    // 查询特定模块的所有配置
    pub fn get_all_feature_config(&self) -> Result<Vec<FeatureConfig>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, feature_code, key, value, data_type, description
             FROM feature_config",
        )?;
        let configs = stmt
            .query_map(params![], |row| {
                Ok(FeatureConfig {
                    id: Some(row.get(0)?),
                    feature_code: row.get(1)?,
                    key: row.get(2)?,
                    value: row.get(3)?,
                    data_type: row.get(4)?,
                    description: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(configs)
    }

    pub fn init_feature_config(&self) -> rusqlite::Result<()> {
        self.add_feature_config(&FeatureConfig {
            id: None,
            feature_code: "conversation_summary".to_string(),
            key: "summary_length".to_string(),
            value: "100".to_string(),
            data_type: "string".to_string(),
            description: Some("对话总结使用长度".to_string()),
        })?;
        self.add_feature_config(&FeatureConfig {
            id: None,
            feature_code: "conversation_summary".to_string(),
            key: "prompt".to_string(),
            value: "请根据提供的大模型问答对话,总结一个简洁明了的标题。标题要求:
- 字数在5-15个字左右，必须是中文，不要包含标点符号
- 准确概括对话的核心主题，尽量贴近用户的提问
- 不要透露任何私人信息
- 用祈使句或陈述句"
                .to_string(),
            data_type: "string".to_string(),
            description: Some("对话总结使用长度".to_string()),
        })?;
        Ok(())
    }
}
