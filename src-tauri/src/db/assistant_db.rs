use super::get_db_path;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Assistant {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub assistant_type: Option<i64>, // 0: 普通对话助手, 1: 多模型对比助手，2: 工作流助手，3: 展示助手
    pub is_addition: bool,
    pub created_time: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssistantModel {
    pub id: i64,
    pub assistant_id: i64,
    pub provider_id: i64,
    pub model_code: String,
    pub alias: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssistantPrompt {
    pub id: i64,
    pub assistant_id: i64,
    pub prompt: String,
    pub created_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssistantModelConfig {
    pub id: i64,
    pub assistant_id: i64,
    pub assistant_model_id: i64,
    pub name: String,
    pub value: Option<String>,
    pub value_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssistantPromptParam {
    pub id: i64,
    pub assistant_id: i64,
    pub assistant_prompt_id: i64,
    pub param_name: String,
    pub param_type: Option<String>,
    pub param_value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssistantMCPConfig {
    pub id: i64,
    pub assistant_id: i64,
    pub mcp_server_id: i64,
    pub is_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssistantMCPToolConfig {
    pub id: i64,
    pub assistant_id: i64,
    pub mcp_tool_id: i64,
    pub is_enabled: bool,
    pub is_auto_run: bool,
}

pub struct AssistantDatabase {
    pub conn: Connection,
    pub mcp_conn: Connection,
}

impl AssistantDatabase {
    pub fn new(app_handle: &tauri::AppHandle) -> rusqlite::Result<Self> {
        let db_path = get_db_path(app_handle, "assistant.db");
        let conn = Connection::open(db_path.unwrap())?;

        let mcp_db_path = get_db_path(app_handle, "mcp.db");
        let mcp_conn = Connection::open(mcp_db_path.unwrap())?;

        Ok(AssistantDatabase { conn, mcp_conn })
    }

    pub fn create_tables(&self) -> rusqlite::Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS assistant (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                assistant_type INTEGER NOT NULL DEFAULT 0,
                is_addition BOOLEAN NOT NULL DEFAULT 0,
                created_time DATETIME DEFAULT CURRENT_TIMESTAMP
            );",
            [],
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS assistant_model (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                assistant_id INTEGER NOT NULL,
                provider_id INTEGER NOT NULL,
                model_code TEXT NOT NULL,
                alias TEXT
            );",
            [],
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS assistant_prompt (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                assistant_id INTEGER,
                prompt TEXT NOT NULL,
                created_time DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (assistant_id) REFERENCES assistant(id)
            );",
            [],
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS assistant_model_config (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                assistant_id INTEGER,
                assistant_model_id INTEGER,
                name TEXT NOT NULL,
                value TEXT,
                value_type TEXT default 'float' not null,
                FOREIGN KEY (assistant_id) REFERENCES assistant(id)
            );",
            [],
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS assistant_prompt_param (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                assistant_id INTEGER,
                assistant_prompt_id INTEGER,
                param_name TEXT NOT NULL,
                param_type TEXT,
                param_value TEXT,
                FOREIGN KEY (assistant_id) REFERENCES assistant(id),
                FOREIGN KEY (assistant_prompt_id) REFERENCES assistant_prompt(id)
            );",
            [],
        )?;

        // Create assistant MCP configuration table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS assistant_mcp_config (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                assistant_id INTEGER NOT NULL,
                mcp_server_id INTEGER NOT NULL,
                is_enabled BOOLEAN NOT NULL DEFAULT 1,
                created_time DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (assistant_id) REFERENCES assistant(id) ON DELETE CASCADE,
                UNIQUE(assistant_id, mcp_server_id)
            );",
            [],
        )?;

        // Create assistant MCP tool configuration table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS assistant_mcp_tool_config (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                assistant_id INTEGER NOT NULL,
                mcp_tool_id INTEGER NOT NULL,
                is_enabled BOOLEAN NOT NULL DEFAULT 1,
                is_auto_run BOOLEAN NOT NULL DEFAULT 0,
                created_time DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (assistant_id) REFERENCES assistant(id) ON DELETE CASCADE,
                UNIQUE(assistant_id, mcp_tool_id)
            );",
            [],
        )?;

        if let Err(err) = self.init_assistant() {
            println!("init_assistant error: {:?}", err);
        }
        Ok(())
    }

    pub fn add_assistant(
        &self,
        name: &str,
        description: &str,
        assistant_type: Option<i64>,
        is_addition: bool,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO assistant (name, description, assistant_type, is_addition) VALUES (?, ?, ?, ?)",
            params![name, description, assistant_type, is_addition],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(id)
    }

    pub fn update_assistant(&self, id: i64, name: &str, description: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE assistant SET name = ?, description = ? WHERE id = ?",
            params![name, description, id],
        )?;
        Ok(())
    }

    pub fn delete_assistant(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM assistant WHERE id = ?", params![id])?;
        Ok(())
    }

    pub fn add_assistant_prompt(&self, assistant_id: i64, prompt: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO assistant_prompt (assistant_id, prompt) VALUES (?, ?)",
            params![assistant_id, prompt],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(id)
    }

    pub fn update_assistant_prompt(&self, id: i64, prompt: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE assistant_prompt SET prompt = ? WHERE id = ?",
            params![prompt, id],
        )?;
        Ok(())
    }

    pub fn delete_assistant_prompt_by_assistant_id(&self, assistant_id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM assistant_prompt WHERE assistant_id = ?",
            params![assistant_id],
        )?;
        Ok(())
    }

    pub fn add_assistant_model(
        &self,
        assistant_id: i64,
        provider_id: i64,
        model_code: &str,
        alias: &str,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO assistant_model (assistant_id, provider_id, model_code, alias) VALUES (?, ?, ?, ?)",
            params![assistant_id, provider_id, model_code, alias],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(id)
    }

    pub fn update_assistant_model(
        &self,
        id: i64,
        provider_id: i64,
        model_code: &str,
        alias: &str,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE assistant_model SET model_code = ?, provider_id = ?, alias = ? WHERE id = ?",
            params![model_code, provider_id, alias, id],
        )?;
        Ok(())
    }

    pub fn add_assistant_model_config(
        &self,
        assistant_id: i64,
        assistant_model_id: i64,
        name: &str,
        value: &str,
        value_type: &str,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO assistant_model_config (assistant_id, assistant_model_id, name, value, value_type) VALUES (?, ?, ?, ?, ?)",
            params![assistant_id, assistant_model_id, name, value, value_type],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(id)
    }

    pub fn update_assistant_model_config(&self, id: i64, name: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE assistant_model_config SET name = ?, value = ? WHERE id = ?",
            params![name, value, id],
        )?;
        Ok(())
    }

    pub fn delete_assistant_model_config_by_assistant_id(&self, assistant_id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM assistant_model_config WHERE assistant_id = ?",
            params![assistant_id],
        )?;
        Ok(())
    }

    pub fn add_assistant_prompt_param(
        &self,
        assistant_id: i64,
        assistant_prompt_id: i64,
        param_name: &str,
        param_type: &str,
        param_value: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO assistant_prompt_param (assistant_id, assistant_prompt_id, param_name, param_type, param_value) VALUES (?, ?, ?, ?, ?)",
            params![assistant_id, assistant_prompt_id, param_name, param_type, param_value],
        )?;
        Ok(())
    }

    pub fn update_assistant_prompt_param(
        &self,
        id: i64,
        param_name: &str,
        param_type: &str,
        param_value: &str,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE assistant_prompt_param SET param_name = ?, param_type = ?, param_value = ? WHERE id = ?",
            params![param_name, param_type, param_value, id],
        )?;
        Ok(())
    }

    pub fn delete_assistant_prompt_param_by_assistant_id(&self, assistant_id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM assistant_prompt_param WHERE assistant_id = ?",
            params![assistant_id],
        )?;
        Ok(())
    }

    pub fn get_assistants(&self) -> Result<Vec<Assistant>> {
        let mut stmt = self.conn.prepare("SELECT id, name, description, assistant_type, is_addition, created_time FROM assistant")?;
        let assistant_iter = stmt.query_map(params![], |row| {
            Ok(Assistant {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                assistant_type: row.get(3)?,
                is_addition: row.get(4)?,
                created_time: row.get(5)?,
            })
        })?;

        let mut assistants = Vec::new();
        for assistant in assistant_iter {
            assistants.push(assistant?);
        }
        Ok(assistants)
    }

    pub fn get_assistant(&self, assistant_id: i64) -> Result<Assistant> {
        let mut stmt = self.conn.prepare("SELECT id, name, description, assistant_type, is_addition, created_time FROM assistant WHERE id = ?")?;
        let mut assistant_iter = stmt.query_map(params![assistant_id], |row| {
            Ok(Assistant {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                assistant_type: row.get(3)?,
                is_addition: row.get(4)?,
                created_time: row.get(5)?,
            })
        })?;

        if let Some(assistant) = assistant_iter.next() {
            return Ok(assistant?);
        }

        Err(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn get_assistant_model(&self, assistant_id: i64) -> Result<Vec<AssistantModel>> {
        let mut stmt = self.conn.prepare("SELECT id, assistant_id, provider_id, model_code, alias FROM assistant_model WHERE assistant_id = ?")?;
        let assistant_model_iter = stmt.query_map(params![assistant_id], |row| {
            println!("row: {:?}", row);
            Ok(AssistantModel {
                id: row.get(0)?,
                assistant_id: row.get(1)?,
                provider_id: row.get::<_, i64>(2)?,
                model_code: row.get(3)?,
                alias: row.get(4)?,
            })
        })?;

        let mut assistant_models = Vec::new();
        for assistant_model in assistant_model_iter {
            assistant_models.push(assistant_model?);
        }
        Ok(assistant_models)
    }

    pub fn get_assistant_prompt(&self, assistant_id: i64) -> Result<Vec<AssistantPrompt>> {
        let mut stmt = self.conn.prepare("SELECT id, assistant_id, prompt, created_time FROM assistant_prompt WHERE assistant_id = ?")?;
        let assistant_prompt_iter = stmt.query_map(params![assistant_id], |row| {
            Ok(AssistantPrompt {
                id: row.get(0)?,
                assistant_id: row.get(1)?,
                prompt: row.get(2)?,
                created_time: row.get(3)?,
            })
        })?;

        let mut assistant_prompts = Vec::new();
        for assistant_prompt in assistant_prompt_iter {
            assistant_prompts.push(assistant_prompt?);
        }
        Ok(assistant_prompts)
    }

    pub fn get_assistant_model_configs(
        &self,
        assistant_id: i64,
    ) -> Result<Vec<AssistantModelConfig>> {
        let mut stmt = self.conn.prepare("SELECT id, assistant_id, assistant_model_id, name, value, value_type FROM assistant_model_config WHERE assistant_id = ?")?;
        let assistant_model_config_iter = stmt.query_map(params![assistant_id], |row| {
            Ok(AssistantModelConfig {
                id: row.get(0)?,
                assistant_id: row.get(1)?,
                assistant_model_id: row.get(2)?,
                name: row.get(3)?,
                value: row.get(4)?,
                value_type: row.get(5)?,
            })
        })?;

        let mut assistant_model_configs = Vec::new();
        for assistant_model_config in assistant_model_config_iter {
            assistant_model_configs.push(assistant_model_config?);
        }
        Ok(assistant_model_configs)
    }

    pub fn get_assistant_model_configs_with_model_id(
        &self,
        assistant_id: i64,
        assistant_model_id: i64,
    ) -> Result<Vec<AssistantModelConfig>> {
        let mut stmt = self.conn.prepare("SELECT id, assistant_id, assistant_model_id, name, value, value_type FROM assistant_model_config WHERE assistant_id = ? AND assistant_model_id = ?")?;
        let assistant_model_config_iter =
            stmt.query_map(params![assistant_id, assistant_model_id], |row| {
                Ok(AssistantModelConfig {
                    id: row.get(0)?,
                    assistant_id: row.get(1)?,
                    assistant_model_id: row.get(2)?,
                    name: row.get(3)?,
                    value: row.get(4)?,
                    value_type: row.get(5)?,
                })
            })?;

        let mut assistant_model_configs = Vec::new();
        for assistant_model_config in assistant_model_config_iter {
            assistant_model_configs.push(assistant_model_config?);
        }
        Ok(assistant_model_configs)
    }

    pub fn get_assistant_prompt_params(
        &self,
        assistant_id: i64,
    ) -> Result<Vec<AssistantPromptParam>> {
        let mut stmt = self.conn.prepare("SELECT id, assistant_id, assistant_prompt_id, param_name, param_type, param_value FROM assistant_prompt_param WHERE assistant_id = ?")?;
        let assistant_prompt_param_iter = stmt.query_map(params![assistant_id], |row| {
            Ok(AssistantPromptParam {
                id: row.get(0)?,
                assistant_id: row.get(1)?,
                assistant_prompt_id: row.get(2)?,
                param_name: row.get(3)?,
                param_type: row.get(4)?,
                param_value: row.get(5)?,
            })
        })?;

        let mut assistant_prompt_params = Vec::new();
        for assistant_prompt_param in assistant_prompt_param_iter {
            assistant_prompt_params.push(assistant_prompt_param?);
        }
        Ok(assistant_prompt_params)
    }

    pub fn init_assistant(&self) -> Result<()> {
        self.conn.execute(
            "INSERT INTO assistant (id, name, description, is_addition) VALUES (1, '快速使用助手', '快捷键呼出的快速使用助手', 0)",
            [],
        )?;
        self.add_assistant_prompt(1, "You are a helpful assistant.")?;
        self.add_assistant_model_config(1, -1, "max_tokens", "1000", "number")?;
        self.add_assistant_model_config(1, -1, "temperature", "0.75", "float")?;
        self.add_assistant_model_config(1, -1, "top_p", "1.0", "float")?;
        self.add_assistant_model_config(1, -1, "stream", "false", "boolean")?;
        Ok(())
    }

    // MCP Configuration Methods
    pub fn get_assistant_mcp_configs(&self, assistant_id: i64) -> Result<Vec<AssistantMCPConfig>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, assistant_id, mcp_server_id, is_enabled FROM assistant_mcp_config WHERE assistant_id = ?"
        )?;
        let mcp_config_iter = stmt.query_map(params![assistant_id], |row| {
            Ok(AssistantMCPConfig {
                id: row.get(0)?,
                assistant_id: row.get(1)?,
                mcp_server_id: row.get(2)?,
                is_enabled: row.get(3)?,
            })
        })?;

        let mut mcp_configs = Vec::new();
        for mcp_config in mcp_config_iter {
            mcp_configs.push(mcp_config?);
        }
        Ok(mcp_configs)
    }

    pub fn upsert_assistant_mcp_config(
        &self,
        assistant_id: i64,
        mcp_server_id: i64,
        is_enabled: bool,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO assistant_mcp_config (assistant_id, mcp_server_id, is_enabled) VALUES (?, ?, ?)",
            params![assistant_id, mcp_server_id, is_enabled],
        )?;
        Ok(())
    }

    pub fn delete_assistant_mcp_config(&self, assistant_id: i64, mcp_server_id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM assistant_mcp_config WHERE assistant_id = ? AND mcp_server_id = ?",
            params![assistant_id, mcp_server_id],
        )?;
        Ok(())
    }

    pub fn get_assistant_mcp_tool_configs(
        &self,
        assistant_id: i64,
    ) -> Result<Vec<AssistantMCPToolConfig>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, assistant_id, mcp_tool_id, is_enabled, is_auto_run FROM assistant_mcp_tool_config WHERE assistant_id = ?"
        )?;
        let mcp_tool_config_iter = stmt.query_map(params![assistant_id], |row| {
            Ok(AssistantMCPToolConfig {
                id: row.get(0)?,
                assistant_id: row.get(1)?,
                mcp_tool_id: row.get(2)?,
                is_enabled: row.get(3)?,
                is_auto_run: row.get(4)?,
            })
        })?;

        let mut mcp_tool_configs = Vec::new();
        for mcp_tool_config in mcp_tool_config_iter {
            mcp_tool_configs.push(mcp_tool_config?);
        }
        Ok(mcp_tool_configs)
    }

    pub fn upsert_assistant_mcp_tool_config(
        &self,
        assistant_id: i64,
        mcp_tool_id: i64,
        is_enabled: bool,
        is_auto_run: bool,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO assistant_mcp_tool_config (assistant_id, mcp_tool_id, is_enabled, is_auto_run) VALUES (?, ?, ?, ?)",
            params![assistant_id, mcp_tool_id, is_enabled, is_auto_run],
        )?;
        Ok(())
    }

    pub fn delete_assistant_mcp_tool_config(
        &self,
        assistant_id: i64,
        mcp_tool_id: i64,
    ) -> Result<()> {
        self.conn.execute(
            "DELETE FROM assistant_mcp_tool_config WHERE assistant_id = ? AND mcp_tool_id = ?",
            params![assistant_id, mcp_tool_id],
        )?;
        Ok(())
    }

    pub fn get_assistant_mcp_servers_with_tools(
        &self,
        assistant_id: i64,
    ) -> Result<
        Vec<(
            i64,
            String,
            bool,
            Vec<(i64, String, String, bool, bool, String)>,
        )>,
    > {
        // 使用一条 SQL 语句获取所有需要的数据，避免 N+1 查询问题
        // 注意：由于涉及两个数据库（assistant.db 和 mcp.db），我们需要分两步查询，但可以优化为批量查询

        // 1. 获取所有启用的服务器及其配置状态
        let mut server_stmt = self.mcp_conn.prepare(
            "
            SELECT s.id, s.name
            FROM mcp_server s
            WHERE s.is_enabled = 1
            ORDER BY s.name
        ",
        )?;
        let servers: Vec<(i64, String)> = server_stmt
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        if servers.is_empty() {
            return Ok(Vec::new());
        }

        // 2. 批量获取所有服务器的配置状态
        let server_ids: Vec<String> = servers.iter().map(|(id, _)| id.to_string()).collect();
        let server_ids_placeholder = vec!["?"; server_ids.len()].join(",");
        let server_config_sql = format!(
            "SELECT mcp_server_id, is_enabled FROM assistant_mcp_config 
             WHERE assistant_id = ? AND mcp_server_id IN ({})",
            server_ids_placeholder
        );

        let mut server_config_stmt = self.conn.prepare(&server_config_sql)?;
        let mut server_config_params = vec![assistant_id];
        server_config_params.extend(servers.iter().map(|(id, _)| *id));

        let server_configs: std::collections::HashMap<i64, bool> = server_config_stmt
            .query_map(rusqlite::params_from_iter(server_config_params), |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, bool>(1)?))
            })?
            .collect::<Result<std::collections::HashMap<_, _>, _>>()?;

        // 3. 获取所有工具信息（一次性获取所有服务器的工具）
        let tools_sql = format!(
            "SELECT t.id, t.server_id, t.tool_name, t.tool_description, t.parameters
             FROM mcp_server_tool t
             WHERE t.server_id IN ({}) AND t.is_enabled = 1
             ORDER BY t.server_id, t.tool_name",
            server_ids_placeholder
        );

        let mut tools_stmt = self.mcp_conn.prepare(&tools_sql)?;
        let all_tools: Vec<(i64, i64, String, String, String)> = tools_stmt
            .query_map(
                rusqlite::params_from_iter(servers.iter().map(|(id, _)| *id)),
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;

        // 4. 批量获取工具配置状态
        let mut tool_configs: std::collections::HashMap<i64, (bool, bool)> =
            std::collections::HashMap::new();
        if !all_tools.is_empty() {
            let tool_ids: Vec<String> = all_tools
                .iter()
                .map(|(id, _, _, _, _)| id.to_string())
                .collect();
            let tool_ids_placeholder = vec!["?"; tool_ids.len()].join(",");
            let tool_config_sql = format!(
                "SELECT mcp_tool_id, is_enabled, is_auto_run FROM assistant_mcp_tool_config 
                 WHERE assistant_id = ? AND mcp_tool_id IN ({})",
                tool_ids_placeholder
            );

            let mut tool_config_stmt = self.conn.prepare(&tool_config_sql)?;
            let mut tool_config_params = vec![assistant_id];
            tool_config_params.extend(all_tools.iter().map(|(id, _, _, _, _)| *id));

            tool_configs = tool_config_stmt
                .query_map(rusqlite::params_from_iter(tool_config_params), |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        (row.get::<_, bool>(1)?, row.get::<_, bool>(2)?),
                    ))
                })?
                .collect::<Result<std::collections::HashMap<_, _>, _>>()?;
        }

        // 5. 组织数据结构
        let mut result = Vec::new();
        for (server_id, server_name) in servers {
            let server_is_enabled = server_configs.get(&server_id).copied().unwrap_or(false);

            // 获取该服务器的所有工具
            let server_tools: Vec<(i64, String, String, bool, bool, String)> = all_tools
                .iter()
                .filter(|(_, sid, _, _, _)| *sid == server_id)
                .map(
                    |(tool_id, _, tool_name, tool_description, tool_parameters)| {
                        let (tool_is_enabled, tool_is_auto_run) =
                            tool_configs.get(tool_id).copied().unwrap_or((true, false)); // Default: enabled but not auto-run

                        (
                            *tool_id,
                            tool_name.clone(),
                            tool_description.clone(),
                            tool_is_enabled,
                            tool_is_auto_run,
                            tool_parameters.clone(),
                        )
                    },
                )
                .collect();

            result.push((server_id, server_name, server_is_enabled, server_tools));
        }

        Ok(result)
    }
}
