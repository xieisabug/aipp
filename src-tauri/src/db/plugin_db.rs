use super::get_db_path;
use chrono::prelude::*;
use rusqlite::{params, Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Plugin {
    pub plugin_id: i64,
    pub name: String,
    pub version: String,
    pub folder_name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginStatus {
    pub status_id: i64,
    pub plugin_id: i64,
    pub is_active: bool,
    pub last_run: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginConfiguration {
    pub config_id: i64,
    pub plugin_id: i64,
    pub config_key: String,
    pub config_value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginData {
    pub data_id: i64,
    pub plugin_id: i64,
    pub session_id: String,
    pub data_key: String,
    pub data_value: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct PluginDatabase {
    pub conn: Connection,
}

impl PluginDatabase {
    pub fn new(app_handle: &tauri::AppHandle) -> rusqlite::Result<Self> {
        let db_path = get_db_path(app_handle, "plugin.db");
        let conn = Connection::open(db_path.unwrap())?;
        Ok(PluginDatabase { conn })
    }

    pub fn create_tables(&self) -> rusqlite::Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS Plugins (
                plugin_id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                folder_name TEXT NOT NULL,
                description TEXT,
                author TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS PluginStatus (
                status_id INTEGER PRIMARY KEY AUTOINCREMENT,
                plugin_id INTEGER,
                is_active INTEGER DEFAULT 1,
                last_run TIMESTAMP,
                FOREIGN KEY (plugin_id) REFERENCES Plugins(plugin_id)
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS PluginConfigurations (
                config_id INTEGER PRIMARY KEY AUTOINCREMENT,
                plugin_id INTEGER,
                config_key TEXT NOT NULL,
                config_value TEXT,
                FOREIGN KEY (plugin_id) REFERENCES Plugins(plugin_id)
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS PluginData (
                data_id INTEGER PRIMARY KEY AUTOINCREMENT,
                plugin_id INTEGER,
                session_id TEXT NOT NULL,
                data_key TEXT NOT NULL,
                data_value TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (plugin_id) REFERENCES Plugins(plugin_id)
            )",
            [],
        )?;

        Ok(())
    }

    // Plugin CRUD
    pub fn add_plugin(
        &self,
        name: &str,
        version: &str,
        folder_name: &str,
        description: Option<&str>,
        author: Option<&str>,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO Plugins (name, version, folder_name, description, author) VALUES (?, ?, ?, ?, ?)",
            params![name, version, folder_name, description, author],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_plugins(&self) -> Result<Vec<Plugin>> {
        let mut stmt = self.conn.prepare(
            "SELECT plugin_id, name, version, folder_name, description, author, created_at, updated_at FROM Plugins ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Plugin {
                plugin_id: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                folder_name: row.get(3)?,
                description: row.get(4)?,
                author: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_plugin(&self, plugin_id: i64) -> Result<Option<Plugin>> {
        self.conn
            .query_row(
                "SELECT plugin_id, name, version, folder_name, description, author, created_at, updated_at FROM Plugins WHERE plugin_id = ?",
                [plugin_id],
                |row| {
                    Ok(Plugin {
                        plugin_id: row.get(0)?,
                        name: row.get(1)?,
                        version: row.get(2)?,
                        folder_name: row.get(3)?,
                        description: row.get(4)?,
                        author: row.get(5)?,
                        created_at: row.get(6)?,
                        updated_at: row.get(7)?,
                    })
                },
            )
            .optional()
    }

    pub fn update_plugin(&self, plugin: &Plugin) -> Result<()> {
        self.conn.execute(
            "UPDATE Plugins SET name = ?, version = ?, folder_name = ?, description = ?, author = ?, updated_at = ? WHERE plugin_id = ?",
            params![
                plugin.name,
                plugin.version,
                plugin.folder_name,
                plugin.description,
                plugin.author,
                plugin.updated_at,
                plugin.plugin_id
            ],
        )?;
        Ok(())
    }

    pub fn delete_plugin(&self, plugin_id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM Plugins WHERE plugin_id = ?", [plugin_id])?;
        Ok(())
    }

    // PluginStatus helpers
    pub fn get_plugin_status(&self, plugin_id: i64) -> Result<Option<PluginStatus>> {
        self.conn
            .query_row(
                "SELECT status_id, plugin_id, is_active, last_run FROM PluginStatus WHERE plugin_id = ?",
                [plugin_id],
                |row| {
                    Ok(PluginStatus {
                        status_id: row.get(0)?,
                        plugin_id: row.get(1)?,
                        is_active: row.get::<_, i64>(2)? != 0,
                        last_run: row.get(3)?,
                    })
                },
            )
            .optional()
    }

    pub fn upsert_plugin_status(
        &self,
        plugin_id: i64,
        is_active: bool,
        last_run: Option<DateTime<Utc>>,
    ) -> Result<i64> {
        if let Some(existing) = self.get_plugin_status(plugin_id)? {
            self.conn.execute(
                "UPDATE PluginStatus SET is_active = ?, last_run = ? WHERE status_id = ?",
                params![is_active as i64, last_run, existing.status_id],
            )?;
            Ok(existing.status_id)
        } else {
            self.conn.execute(
                "INSERT INTO PluginStatus (plugin_id, is_active, last_run) VALUES (?, ?, ?)",
                params![plugin_id, is_active as i64, last_run],
            )?;
            Ok(self.conn.last_insert_rowid())
        }
    }

    pub fn delete_plugin_status(&self, status_id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM PluginStatus WHERE status_id = ?", [status_id])?;
        Ok(())
    }

    // Configurations
    pub fn get_plugin_configurations(&self, plugin_id: i64) -> Result<Vec<PluginConfiguration>> {
        let mut stmt = self.conn.prepare(
            "SELECT config_id, plugin_id, config_key, config_value FROM PluginConfigurations WHERE plugin_id = ?",
        )?;
        let rows = stmt.query_map([plugin_id], |row| {
            Ok(PluginConfiguration {
                config_id: row.get(0)?,
                plugin_id: row.get(1)?,
                config_key: row.get(2)?,
                config_value: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn set_plugin_configuration(
        &self,
        plugin_id: i64,
        config_key: &str,
        config_value: Option<&str>,
    ) -> Result<i64> {
        // try update first
        let existing_id: Option<i64> = self
            .conn
            .prepare("SELECT config_id FROM PluginConfigurations WHERE plugin_id = ? AND config_key = ?")?
            .query_row(params![plugin_id, config_key], |row| row.get(0))
            .optional()?;

        match existing_id {
            Some(id) => {
                self.conn.execute(
                    "UPDATE PluginConfigurations SET config_value = ? WHERE config_id = ?",
                    params![config_value, id],
                )?;
                Ok(id)
            }
            None => {
                self.conn.execute(
                    "INSERT INTO PluginConfigurations (plugin_id, config_key, config_value) VALUES (?, ?, ?)",
                    params![plugin_id, config_key, config_value],
                )?;
                Ok(self.conn.last_insert_rowid())
            }
        }
    }

    pub fn delete_plugin_configuration(&self, config_id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM PluginConfigurations WHERE config_id = ?", [config_id])?;
        Ok(())
    }

    // Data
    pub fn get_plugin_data_by_session(
        &self,
        plugin_id: i64,
        session_id: &str,
    ) -> Result<Vec<PluginData>> {
        let mut stmt = self.conn.prepare(
            "SELECT data_id, plugin_id, session_id, data_key, data_value, created_at, updated_at FROM PluginData WHERE plugin_id = ?1 AND session_id = ?2",
        )?;
        let rows = stmt.query_map(params![plugin_id, session_id], |row| {
            Ok(PluginData {
                data_id: row.get(0)?,
                plugin_id: row.get(1)?,
                session_id: row.get(2)?,
                data_key: row.get(3)?,
                data_value: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn add_plugin_data(&self, data: &PluginData) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO PluginData (plugin_id, session_id, data_key, data_value, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
            params![
                data.plugin_id,
                data.session_id,
                data.data_key,
                data.data_value,
                data.created_at,
                data.updated_at
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_plugin_data(&self, data_id: i64, data_value: Option<&str>, updated_at: DateTime<Utc>) -> Result<()> {
        self.conn.execute(
            "UPDATE PluginData SET data_value = ?, updated_at = ? WHERE data_id = ?",
            params![data_value, updated_at, data_id],
        )?;
        Ok(())
    }

    pub fn delete_plugin_data(&self, data_id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM PluginData WHERE data_id = ?", [data_id])?;
        Ok(())
    }
}
