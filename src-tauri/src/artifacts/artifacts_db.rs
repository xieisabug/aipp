use crate::db::get_db_path;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArtifactCollection {
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub description: String,
    pub artifact_type: String, // vue, react, html, svg, xml, markdown, mermaid
    pub code: String,
    pub tags: Option<String>, // JSON string for flexible tag storage
    pub created_time: String,
    pub last_used_time: Option<String>,
    pub use_count: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewArtifactCollection {
    pub name: String,
    pub icon: String,
    pub description: String,
    pub artifact_type: String,
    pub code: String,
    pub tags: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateArtifactCollection {
    pub id: i64,
    pub name: Option<String>,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub tags: Option<String>,
}

pub struct ArtifactsDatabase {
    pub conn: Connection,
}

impl ArtifactsDatabase {
    pub fn new(app_handle: &tauri::AppHandle) -> rusqlite::Result<Self> {
        let db_path = get_db_path(app_handle, "artifacts.db");
        let conn = Connection::open(db_path.unwrap())?;

        Ok(ArtifactsDatabase { conn })
    }

    pub fn create_tables(&self) -> rusqlite::Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS artifacts_collection (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                icon TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                artifact_type TEXT NOT NULL CHECK (artifact_type IN ('vue', 'react', 'html', 'svg', 'xml', 'markdown', 'mermaid')),
                code TEXT NOT NULL,
                tags TEXT, -- JSON string for flexible tag storage
                created_time DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_used_time DATETIME,
                use_count INTEGER NOT NULL DEFAULT 0
            );",
            [],
        )?;

        // Create index for faster searching
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_artifacts_collection_type ON artifacts_collection(artifact_type);",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_artifacts_collection_name ON artifacts_collection(name);",
            [],
        )?;

        Ok(())
    }

    /// Save a new artifact to collection
    pub fn save_artifact(&self, artifact: NewArtifactCollection) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO artifacts_collection (name, icon, description, artifact_type, code, tags) 
             VALUES (?, ?, ?, ?, ?, ?)",
        )?;

        stmt.execute(params![
            artifact.name,
            artifact.icon,
            artifact.description,
            artifact.artifact_type,
            artifact.code,
            artifact.tags
        ])?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get all artifacts with optional type filter
    pub fn get_artifacts(&self, artifact_type: Option<&str>) -> Result<Vec<ArtifactCollection>> {
        let query = if let Some(_) = artifact_type {
            "SELECT id, name, icon, description, artifact_type, code, tags, created_time, last_used_time, use_count 
             FROM artifacts_collection 
             WHERE artifact_type = ? 
             ORDER BY use_count DESC, last_used_time DESC, created_time DESC"
        } else {
            "SELECT id, name, icon, description, artifact_type, code, tags, created_time, last_used_time, use_count 
             FROM artifacts_collection 
             ORDER BY use_count DESC, last_used_time DESC, created_time DESC"
        };

        let mut stmt = self.conn.prepare(query)?;

        let row_mapper = |row: &rusqlite::Row| {
            Ok(ArtifactCollection {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                description: row.get(3)?,
                artifact_type: row.get(4)?,
                code: row.get(5)?,
                tags: row.get(6)?,
                created_time: row.get(7)?,
                last_used_time: row.get(8)?,
                use_count: row.get(9)?,
            })
        };

        let rows = if let Some(type_filter) = artifact_type {
            stmt.query_map([type_filter], row_mapper)?
        } else {
            stmt.query_map([], row_mapper)?
        };

        let mut artifacts = Vec::new();
        for artifact_result in rows {
            artifacts.push(artifact_result?);
        }

        Ok(artifacts)
    }

    /// Get artifact by ID
    pub fn get_artifact_by_id(&self, id: i64) -> Result<Option<ArtifactCollection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, icon, description, artifact_type, code, tags, created_time, last_used_time, use_count 
             FROM artifacts_collection 
             WHERE id = ?"
        )?;

        let mut rows = stmt.query_map([id], |row| {
            Ok(ArtifactCollection {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                description: row.get(3)?,
                artifact_type: row.get(4)?,
                code: row.get(5)?,
                tags: row.get(6)?,
                created_time: row.get(7)?,
                last_used_time: row.get(8)?,
                use_count: row.get(9)?,
            })
        })?;

        if let Some(artifact_result) = rows.next() {
            Ok(Some(artifact_result?))
        } else {
            Ok(None)
        }
    }

    /// Search artifacts by name, description, or tags
    pub fn search_artifacts(&self, query: &str) -> Result<Vec<ArtifactCollection>> {
        let search_pattern = format!("%{}%", query.to_lowercase());

        let mut stmt = self.conn.prepare(
            "SELECT id, name, icon, description, artifact_type, code, tags, created_time, last_used_time, use_count 
             FROM artifacts_collection 
             WHERE LOWER(name) LIKE ? OR LOWER(description) LIKE ? OR LOWER(tags) LIKE ?
             ORDER BY use_count DESC, last_used_time DESC, created_time DESC"
        )?;

        let rows = stmt.query_map([&search_pattern, &search_pattern, &search_pattern], |row| {
            Ok(ArtifactCollection {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                description: row.get(3)?,
                artifact_type: row.get(4)?,
                code: row.get(5)?,
                tags: row.get(6)?,
                created_time: row.get(7)?,
                last_used_time: row.get(8)?,
                use_count: row.get(9)?,
            })
        })?;

        let mut artifacts = Vec::new();
        for artifact_result in rows {
            artifacts.push(artifact_result?);
        }

        Ok(artifacts)
    }

    /// Update artifact metadata (name, icon, description, tags)
    pub fn update_artifact(&self, update: UpdateArtifactCollection) -> Result<()> {
        let mut query_parts = Vec::new();
        let mut params = Vec::new();

        if let Some(name) = &update.name {
            query_parts.push("name = ?");
            params.push(name.as_str());
        }
        if let Some(icon) = &update.icon {
            query_parts.push("icon = ?");
            params.push(icon.as_str());
        }
        if let Some(description) = &update.description {
            query_parts.push("description = ?");
            params.push(description.as_str());
        }
        if let Some(tags) = &update.tags {
            query_parts.push("tags = ?");
            params.push(tags.as_str());
        }

        if query_parts.is_empty() {
            return Ok(()); // Nothing to update
        }

        let query =
            format!("UPDATE artifacts_collection SET {} WHERE id = ?", query_parts.join(", "));

        let id_string = update.id.to_string();
        params.push(&id_string);

        self.conn.execute(&query, rusqlite::params_from_iter(params))?;
        Ok(())
    }

    /// Delete artifact by ID
    pub fn delete_artifact(&self, id: i64) -> Result<bool> {
        let rows_affected =
            self.conn.execute("DELETE FROM artifacts_collection WHERE id = ?", [id])?;

        Ok(rows_affected > 0)
    }

    /// Increment use count and update last used time
    pub fn increment_use_count(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE artifacts_collection 
             SET use_count = use_count + 1, last_used_time = CURRENT_TIMESTAMP 
             WHERE id = ?",
            [id],
        )?;

        Ok(())
    }

    /// Get artifacts statistics
    pub fn get_statistics(&self) -> Result<(i64, i64)> {
        let total_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM artifacts_collection", [], |row| row.get(0))?;

        let total_uses: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(use_count), 0) FROM artifacts_collection",
            [],
            |row| row.get(0),
        )?;

        Ok((total_count, total_uses))
    }
}
