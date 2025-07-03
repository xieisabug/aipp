use rusqlite::{params, Connection};

use super::get_db_path;

#[derive(Debug)]
pub struct LLMProvider {
    pub id: i64,
    pub name: String,
    pub api_type: String,
    pub description: String,
    pub is_official: bool,
    pub is_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct LLMProviderConfig {
    pub id: i64,
    pub name: String,
    pub llm_provider_id: i64,
    pub value: String,
    pub append_location: String,
    pub is_addition: bool,
}

#[derive(Debug)]
pub struct LLMModel {
    pub id: i64,
    pub name: String,
    pub llm_provider_id: i64,
    pub code: String,
    pub description: String,
    pub vision_support: bool,
    pub audio_support: bool,
    pub video_support: bool,
}

#[derive(Debug)]
pub struct ModelDetail {
    pub model: LLMModel,
    pub provider: LLMProvider,
    pub configs: Vec<LLMProviderConfig>,
}

pub struct LLMDatabase {
    pub conn: Connection,
}

impl LLMDatabase {
    pub fn new(app_handle: &tauri::AppHandle) -> rusqlite::Result<Self> {
        let db_path = get_db_path(app_handle, "llm.db");
        let conn = Connection::open(db_path.unwrap())?;
        Ok(LLMDatabase { conn })
    }

    pub fn create_tables(&self) -> rusqlite::Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS llm_provider (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    api_type TEXT NOT NULL,
                    description TEXT,
                    is_official BOOLEAN NOT NULL DEFAULT 0,
                    is_enabled BOOLEAN NOT NULL DEFAULT 0,
                    created_time DATETIME DEFAULT CURRENT_TIMESTAMP
                );",
            [],
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS llm_model (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    llm_provider_id INTEGER NOT NULL,
                    code TEXT NOT NULL,
                    description TEXT,
                    vision_support BOOLEAN NOT NULL DEFAULT 0,
                    audio_support BOOLEAN NOT NULL DEFAULT 0,
                    video_support BOOLEAN NOT NULL DEFAULT 0,
                    created_time DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (llm_provider_id) REFERENCES llm_provider(id)
                );",
            [],
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS llm_provider_config (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    llm_provider_id INTEGER NOT NULL,
                    value TEXT,
                    append_location TEXT DEFAULT 'header',
                    is_addition BOOLEAN NOT NULL DEFAULT 0,
                    created_time DATETIME DEFAULT CURRENT_TIMESTAMP
                );",
            [],
        )?;

        if let Err(err) = self.init_llm_provider() {
            println!("init_llm_provider error: {:?}", err);
        }
        Ok(())
    }

    pub fn add_llm_provider(
        &self,
        name: &str,
        api_type: &str,
        description: &str,
        is_official: bool,
        is_enabled: bool,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO llm_provider (name, api_type, description, is_official, is_enabled) VALUES (?, ?, ?, ?, ?)",
            params![name, api_type, description, is_official, is_enabled],
        )?;
        Ok(())
    }

    pub fn get_llm_providers(
        &self,
    ) -> rusqlite::Result<Vec<(i64, String, String, String, bool, bool)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, api_type, description, is_official, is_enabled FROM llm_provider",
        )?;
        let llm_providers = stmt.query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })?;

        let mut result = Vec::new();
        for llm_provider in llm_providers {
            result.push(llm_provider?);
        }
        Ok(result)
    }

    pub fn get_llm_provider(&self, id: i64) -> rusqlite::Result<LLMProvider> {
        let mut stmt = self.conn.prepare("SELECT id, name, api_type, description, is_official, is_enabled FROM llm_provider WHERE id = ?")?;
        let provider = stmt
            .query_map([id], |row| {
                Ok(LLMProvider {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    api_type: row.get(2)?,
                    description: row.get(3)?,
                    is_official: row.get(4)?,
                    is_enabled: row.get(5)?,
                })
            })?
            .next()
            .transpose()?;

        match provider {
            Some(provider) => Ok(provider),
            None => Err(rusqlite::Error::QueryReturnedNoRows),
        }
    }

    pub fn update_llm_provider(
        &self,
        id: i64,
        name: &str,
        api_type: &str,
        description: &str,
        is_enabled: bool,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE llm_provider SET name = ?, api_type = ?, description = ?, is_enabled = ? WHERE id = ?",
            params![name, api_type, description, is_enabled, id],
        )?;
        Ok(())
    }

    pub fn delete_llm_provider(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "DELETE FROM llm_provider_config WHERE llm_provider_id = ?",
            params![id],
        )?;
        self.conn.execute(
            "DELETE FROM llm_model WHERE llm_provider_id = ?",
            params![id],
        )?;
        self.conn
            .execute("DELETE FROM llm_provider WHERE id = ?", params![id])?;
        Ok(())
    }

    pub fn get_llm_provider_config(
        &self,
        llm_provider_id: i64,
    ) -> rusqlite::Result<Vec<LLMProviderConfig>> {
        let mut stmt = self.conn.prepare("SELECT id, name, llm_provider_id, value, append_location, is_addition FROM llm_provider_config WHERE llm_provider_id = ?")?;
        let configs = stmt.query_map([llm_provider_id], |row| {
            Ok(LLMProviderConfig {
                id: row.get(0)?,
                name: row.get(1)?,
                llm_provider_id: row.get(2)?,
                value: row.get(3)?,
                append_location: row.get(4)?,
                is_addition: row.get(5)?,
            })
        })?;

        let mut result = Vec::new();
        for config in configs {
            result.push(config?);
        }
        Ok(result)
    }

    pub fn update_llm_provider_config(
        &self,
        llm_provider_id: i64,
        name: &str,
        value: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO llm_provider_config (id, name, llm_provider_id, value) VALUES ((SELECT id FROM llm_provider_config WHERE llm_provider_id = ? AND name = ?), ?, ?, ?)",
            params![llm_provider_id, name, name, llm_provider_id, value],
        )?;
        Ok(())
    }

    pub fn add_llm_model(
        &self,
        name: &str,
        llm_provider_id: i64,
        code: &str,
        description: &str,
        vision_support: bool,
        audio_support: bool,
        video_support: bool,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO llm_model (name, llm_provider_id, code, description, vision_support, audio_support, video_support) VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![name, llm_provider_id, code, description, vision_support, audio_support, video_support],
        )?;
        Ok(())
    }

    pub fn get_all_llm_models(
        &self,
    ) -> rusqlite::Result<Vec<(i64, String, i64, String, String, bool, bool, bool)>> {
        let mut stmt = self.conn.prepare("SELECT id, name, llm_provider_id, code, description, vision_support, audio_support, video_support FROM llm_model")?;
        let llm_models = stmt.query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        })?;

        let mut result = Vec::new();
        for llm_model in llm_models {
            result.push(llm_model?);
        }
        Ok(result)
    }

    pub fn get_llm_models(
        &self,
        provider_id: String,
    ) -> rusqlite::Result<Vec<(i64, String, i64, String, String, bool, bool, bool)>> {
        let mut stmt = self.conn.prepare("SELECT id, name, llm_provider_id, code, description, vision_support, audio_support, video_support FROM llm_model WHERE llm_provider_id = ?")?;
        let llm_models = stmt.query_map([provider_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        })?;

        let mut result = Vec::new();
        for llm_model in llm_models {
            result.push(llm_model?);
        }
        Ok(result)
    }

    pub fn get_llm_model_detail(
        &self,
        provider_id: &i64,
        model_code: &String,
    ) -> rusqlite::Result<ModelDetail> {
        let mut stmt = self.conn.prepare("SELECT id, name, llm_provider_id, code, description, vision_support, audio_support, video_support FROM llm_model WHERE llm_provider_id = ? AND code = ?")?;
        let model = stmt
            .query_map([&provider_id.to_string(), model_code], |row| {
                Ok(LLMModel {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    llm_provider_id: row.get(2)?,
                    code: row.get(3)?,
                    description: row.get(4)?,
                    vision_support: row.get(5)?,
                    audio_support: row.get(6)?,
                    video_support: row.get(7)?,
                })
            })?
            .next()
            .transpose()?;

        let model = match model {
            Some(model) => model,
            None => return Err(rusqlite::Error::QueryReturnedNoRows),
        };

        let provider_id = model.llm_provider_id;
        let provider = self.get_llm_provider(provider_id)?;
        let configs = self.get_llm_provider_config(provider_id)?;

        Ok(ModelDetail {
            model,
            provider,
            configs,
        })
    }

    pub fn get_llm_model_detail_by_id(&self, id: &i64) -> rusqlite::Result<ModelDetail> {
        let mut stmt = self.conn.prepare("SELECT id, name, llm_provider_id, code, description, vision_support, audio_support, video_support FROM llm_model WHERE id = ?")?;
        let model = stmt
            .query_map([id], |row| {
                Ok(LLMModel {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    llm_provider_id: row.get(2)?,
                    code: row.get(3)?,
                    description: row.get(4)?,
                    vision_support: row.get(5)?,
                    audio_support: row.get(6)?,
                    video_support: row.get(7)?,
                })
            })?
            .next()
            .transpose()?;

        let model = match model {
            Some(model) => model,
            None => return Err(rusqlite::Error::QueryReturnedNoRows),
        };

        let provider_id = model.llm_provider_id;
        let provider = self.get_llm_provider(provider_id)?;
        let configs = self.get_llm_provider_config(provider_id)?;

        Ok(ModelDetail {
            model,
            provider,
            configs,
        })
    }

    pub fn delete_llm_model(&self, provider_id: i64, code: String) -> rusqlite::Result<()> {
        self.conn.execute(
            "DELETE FROM llm_model WHERE llm_provider_id = ? AND code = ?",
            params![provider_id, code],
        )?;
        Ok(())
    }

    pub fn delete_llm_model_by_provider(&self, provider_id: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "DELETE FROM llm_model WHERE llm_provider_id = ?",
            params![provider_id],
        )?;
        Ok(())
    }

    pub fn get_models_for_select(&self) -> Result<Vec<(String, String, i64, i64)>, String> {
        let mut stmt = match self.conn.prepare(
            "
            SELECT
                (p.name || ' / ' || m.name) AS name,
                m.code,
                m.id,
                m.llm_provider_id
            FROM
                llm_model m
            JOIN
                llm_provider p ON m.llm_provider_id = p.id
            WHERE p.is_enabled = 1
        ",
        ) {
            Ok(stmt) => stmt,
            Err(e) => return Err(e.to_string()), // Convert rusqlite::Error to String
        };

        let models = match stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        }) {
            Ok(models) => models,
            Err(e) => return Err(e.to_string()), // Convert rusqlite::Error to String
        };

        let mut result = Vec::new();
        for model in models {
            match model {
                Ok(model) => result.push(model),
                Err(e) => return Err(e.to_string()), // Convert rusqlite::Error to String
            }
        }
        Ok(result)
    }

    pub fn init_llm_provider(&self) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO llm_provider (id, name, api_type, description, is_official) VALUES (1, 'OpenAI', 'openai_api', 'OpenAI API', 1)",
            [],
        )?;
        self.conn.execute(
            "INSERT INTO llm_provider (id, name, api_type, description, is_official) VALUES (10, 'Ollama', 'ollama', 'Ollama API', 1)",
            [],
        )?;
        self.conn.execute(
            "INSERT INTO llm_provider (id, name, api_type, description, is_official) VALUES (20, 'Anthropic', 'anthropic', 'Anthropic API', 1);",
            [],
        )?;
        self.conn.execute(
            "INSERT INTO llm_provider (id, name, api_type, description, is_official) VALUES (30, 'DeepSeek', 'deepseek', 'DeepSeek API', 1);",
            [],
        )?;

        Ok(())
    }

    pub fn debug(&self) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO llm_model (id, name, llm_provider_id, code, description, vision_support, audio_support, video_support) VALUES (9999, 'yi:9b-v1.5', 10, 'yi:9b-v1.5', 'yi:9b-v1.5', 0, 0, 0)",
            [],
        )?;

        Ok(())
    }
}
