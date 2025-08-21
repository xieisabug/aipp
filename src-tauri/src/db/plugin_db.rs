use super::get_db_path;
use crate::errors::AppError;
use chrono::prelude::*;
use rusqlite::{Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

pub trait Repository<T> {
    fn create(&self, item: &T) -> Result<T>;
    fn read(&self, id: i64) -> Result<Option<T>>;
    fn update(&self, item: &T) -> Result<()>;
    fn delete(&self, id: i64) -> Result<()>;
}

pub struct PluginRepository {
    conn: Connection,
}

impl PluginRepository {
    pub fn new(conn: Connection) -> Self {
        PluginRepository { conn }
    }

    pub fn list(&self) -> Result<Vec<Plugin>> {
        let mut stmt = self.conn.prepare(
            "SELECT plugin_id, name, version, folder_name, description, author, created_at, updated_at
             FROM Plugins
             ORDER BY created_at DESC",
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
}

impl Repository<Plugin> for PluginRepository {
    fn create(&self, plugin: &Plugin) -> Result<Plugin> {
        self.conn.execute(
            "INSERT INTO Plugins (name, version, folder_name, description, author, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (
                &plugin.name,
                &plugin.version,
                &plugin.folder_name,
                &plugin.description,
                &plugin.author,
                &plugin.created_at,
                &plugin.updated_at,
            ),
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(Plugin {
            plugin_id: id,
            name: plugin.name.clone(),
            version: plugin.version.clone(),
            folder_name: plugin.folder_name.clone(),
            description: plugin.description.clone(),
            author: plugin.author.clone(),
            created_at: plugin.created_at,
            updated_at: plugin.updated_at,
        })
    }

    fn read(&self, id: i64) -> Result<Option<Plugin>> {
        self.conn
            .query_row(
                "SELECT plugin_id, name, version, folder_name, description, author, created_at, updated_at
                 FROM Plugins WHERE plugin_id = ?",
                &[&id],
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

    fn update(&self, plugin: &Plugin) -> Result<()> {
        self.conn.execute(
            "UPDATE Plugins SET name = ?1, version = ?2, folder_name = ?3, description = ?4, author = ?5, updated_at = ?6
             WHERE plugin_id = ?7",
            (
                &plugin.name,
                &plugin.version,
                &plugin.folder_name,
                &plugin.description,
                &plugin.author,
                &plugin.updated_at,
                &plugin.plugin_id,
            ),
        )?;
        Ok(())
    }

    fn delete(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM Plugins WHERE plugin_id = ?", &[&id])?;
        Ok(())
    }
}

pub struct PluginStatusRepository {
    conn: Connection,
}

impl PluginStatusRepository {
    pub fn new(conn: Connection) -> Self {
        PluginStatusRepository { conn }
    }

    pub fn get_status_by_plugin_id(&self, plugin_id: i64) -> Result<Option<PluginStatus>> {
        self.conn
            .query_row(
                "SELECT status_id, plugin_id, is_active, last_run
                 FROM PluginStatus WHERE plugin_id = ?",
                &[&plugin_id],
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
}

impl Repository<PluginStatus> for PluginStatusRepository {
    fn create(&self, status: &PluginStatus) -> Result<PluginStatus> {
        self.conn.execute(
            "INSERT INTO PluginStatus (plugin_id, is_active, last_run)
             VALUES (?1, ?2, ?3)",
            (&status.plugin_id, &(status.is_active as i64), &status.last_run),
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(PluginStatus {
            status_id: id,
            plugin_id: status.plugin_id,
            is_active: status.is_active,
            last_run: status.last_run,
        })
    }

    fn read(&self, id: i64) -> Result<Option<PluginStatus>> {
        self.conn
            .query_row(
                "SELECT status_id, plugin_id, is_active, last_run
                 FROM PluginStatus WHERE status_id = ?",
                &[&id],
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

    fn update(&self, status: &PluginStatus) -> Result<()> {
        self.conn.execute(
            "UPDATE PluginStatus SET is_active = ?1, last_run = ?2
             WHERE status_id = ?3",
            (&(status.is_active as i64), &status.last_run, &status.status_id),
        )?;
        Ok(())
    }

    fn delete(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM PluginStatus WHERE status_id = ?", &[&id])?;
        Ok(())
    }
}

pub struct PluginConfigurationRepository {
    conn: Connection,
}

impl PluginConfigurationRepository {
    pub fn new(conn: Connection) -> Self {
        PluginConfigurationRepository { conn }
    }

    pub fn get_configurations_by_plugin_id(
        &self,
        plugin_id: i64,
    ) -> Result<Vec<PluginConfiguration>> {
        let mut stmt = self.conn.prepare(
            "SELECT config_id, plugin_id, config_key, config_value
             FROM PluginConfigurations
             WHERE plugin_id = ?",
        )?;
        let rows = stmt.query_map(&[&plugin_id], |row| {
            Ok(PluginConfiguration {
                config_id: row.get(0)?,
                plugin_id: row.get(1)?,
                config_key: row.get(2)?,
                config_value: row.get(3)?,
            })
        })?;
        rows.collect()
    }
}

impl Repository<PluginConfiguration> for PluginConfigurationRepository {
    fn create(&self, config: &PluginConfiguration) -> Result<PluginConfiguration> {
        self.conn.execute(
            "INSERT INTO PluginConfigurations (plugin_id, config_key, config_value)
             VALUES (?1, ?2, ?3)",
            (&config.plugin_id, &config.config_key, &config.config_value),
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(PluginConfiguration {
            config_id: id,
            plugin_id: config.plugin_id,
            config_key: config.config_key.clone(),
            config_value: config.config_value.clone(),
        })
    }

    fn read(&self, id: i64) -> Result<Option<PluginConfiguration>> {
        self.conn
            .query_row(
                "SELECT config_id, plugin_id, config_key, config_value
                 FROM PluginConfigurations WHERE config_id = ?",
                &[&id],
                |row| {
                    Ok(PluginConfiguration {
                        config_id: row.get(0)?,
                        plugin_id: row.get(1)?,
                        config_key: row.get(2)?,
                        config_value: row.get(3)?,
                    })
                },
            )
            .optional()
    }

    fn update(&self, config: &PluginConfiguration) -> Result<()> {
        self.conn.execute(
            "UPDATE PluginConfigurations SET config_value = ?1
             WHERE config_id = ?2",
            (&config.config_value, &config.config_id),
        )?;
        Ok(())
    }

    fn delete(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM PluginConfigurations WHERE config_id = ?", &[&id])?;
        Ok(())
    }
}

pub struct PluginDataRepository {
    conn: Connection,
}

impl PluginDataRepository {
    pub fn new(conn: Connection) -> Self {
        PluginDataRepository { conn }
    }

    pub fn get_data_by_plugin_and_session(
        &self,
        plugin_id: i64,
        session_id: &str,
    ) -> Result<Vec<PluginData>> {
        let mut stmt = self.conn.prepare(
            "SELECT data_id, plugin_id, session_id, data_key, data_value, created_at, updated_at
             FROM PluginData
             WHERE plugin_id = ?1 AND session_id = ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![plugin_id, session_id], |row| {
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
}

impl Repository<PluginData> for PluginDataRepository {
    fn create(&self, data: &PluginData) -> Result<PluginData> {
        self.conn.execute(
            "INSERT INTO PluginData (plugin_id, session_id, data_key, data_value, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                &data.plugin_id,
                &data.session_id,
                &data.data_key,
                &data.data_value,
                &data.created_at,
                &data.updated_at,
            ),
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(PluginData {
            data_id: id,
            plugin_id: data.plugin_id,
            session_id: data.session_id.clone(),
            data_key: data.data_key.clone(),
            data_value: data.data_value.clone(),
            created_at: data.created_at,
            updated_at: data.updated_at,
        })
    }

    fn read(&self, id: i64) -> Result<Option<PluginData>> {
        self.conn
            .query_row(
                "SELECT data_id, plugin_id, session_id, data_key, data_value, created_at, updated_at
                 FROM PluginData WHERE data_id = ?",
                &[&id],
                |row| {
                    Ok(PluginData {
                        data_id: row.get(0)?,
                        plugin_id: row.get(1)?,
                        session_id: row.get(2)?,
                        data_key: row.get(3)?,
                        data_value: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .optional()
    }

    fn update(&self, data: &PluginData) -> Result<()> {
        self.conn.execute(
            "UPDATE PluginData SET data_value = ?1, updated_at = ?2
             WHERE data_id = ?3",
            (&data.data_value, &data.updated_at, &data.data_id),
        )?;
        Ok(())
    }

    fn delete(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM PluginData WHERE data_id = ?", &[&id])?;
        Ok(())
    }
}

pub struct PluginDatabase {
    db_path: PathBuf,
}

impl PluginDatabase {
    pub fn new(app_handle: &tauri::AppHandle) -> rusqlite::Result<Self> {
        let db_path = get_db_path(app_handle, "plugin.db");

        Ok(PluginDatabase { db_path: db_path.unwrap() })
    }

    pub fn plugin_repo(&self) -> Result<PluginRepository, AppError> {
        let conn = Connection::open(self.db_path.clone()).map_err(AppError::from)?;
        Ok(PluginRepository::new(conn))
    }

    pub fn plugin_status_repo(&self) -> Result<PluginStatusRepository, AppError> {
        let conn = Connection::open(self.db_path.clone()).map_err(AppError::from)?;
        Ok(PluginStatusRepository::new(conn))
    }

    pub fn plugin_config_repo(&self) -> Result<PluginConfigurationRepository, AppError> {
        let conn = Connection::open(self.db_path.clone()).map_err(AppError::from)?;
        Ok(PluginConfigurationRepository::new(conn))
    }

    pub fn plugin_data_repo(&self) -> Result<PluginDataRepository, AppError> {
        let conn = Connection::open(self.db_path.clone()).map_err(AppError::from)?;
        Ok(PluginDataRepository::new(conn))
    }

    pub fn create_tables(&self) -> rusqlite::Result<()> {
        let conn = Connection::open(self.db_path.clone()).unwrap();

        conn.execute(
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

        conn.execute(
            "CREATE TABLE IF NOT EXISTS PluginStatus (
                status_id INTEGER PRIMARY KEY AUTOINCREMENT,
                plugin_id INTEGER,
                is_active INTEGER DEFAULT 1,
                last_run TIMESTAMP,
                FOREIGN KEY (plugin_id) REFERENCES Plugins(plugin_id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS PluginConfigurations (
                config_id INTEGER PRIMARY KEY AUTOINCREMENT,
                plugin_id INTEGER,
                config_key TEXT NOT NULL,
                config_value TEXT,
                FOREIGN KEY (plugin_id) REFERENCES Plugins(plugin_id)
            )",
            [],
        )?;

        conn.execute(
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
}
