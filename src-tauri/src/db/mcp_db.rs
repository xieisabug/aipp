use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::get_db_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServer {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub transport_type: String, // stdio, sse, http
    pub command: Option<String>,
    pub environment_variables: Option<String>,
    pub url: Option<String>,
    pub timeout: Option<i32>,
    pub is_long_running: bool,
    pub is_enabled: bool,
    pub created_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerTool {
    pub id: i64,
    pub server_id: i64,
    pub tool_name: String,
    pub tool_description: Option<String>,
    pub is_enabled: bool,
    pub is_auto_run: bool,
    pub parameters: Option<String>, // JSON string of tool parameters
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MCPServerResource {
    pub id: i64,
    pub server_id: i64,
    pub resource_uri: String,
    pub resource_name: String,
    pub resource_type: String,
    pub resource_description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MCPServerPrompt {
    pub id: i64,
    pub server_id: i64,
    pub prompt_name: String,
    pub prompt_description: Option<String>,
    pub is_enabled: bool,
    pub arguments: Option<String>, // JSON string of prompt arguments
}

pub struct MCPDatabase {
    pub conn: Connection,
}

impl MCPDatabase {
    pub fn new(app_handle: &tauri::AppHandle) -> rusqlite::Result<Self> {
        let db_path = get_db_path(app_handle, "mcp.db");
        let conn = Connection::open(db_path.unwrap())?;
        Ok(MCPDatabase { conn })
    }

    pub fn create_tables(&self) -> rusqlite::Result<()> {
        // Create MCP servers table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS mcp_server (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                transport_type TEXT NOT NULL CHECK (transport_type IN ('stdio', 'sse', 'http')),
                command TEXT,
                environment_variables TEXT,
                url TEXT,
                timeout INTEGER DEFAULT 30000,
                is_long_running BOOLEAN NOT NULL DEFAULT 0,
                is_enabled BOOLEAN NOT NULL DEFAULT 1,
                created_time DATETIME DEFAULT CURRENT_TIMESTAMP
            );",
            [],
        )?;

        // Create MCP server tools table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS mcp_server_tool (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                server_id INTEGER NOT NULL,
                tool_name TEXT NOT NULL,
                tool_description TEXT,
                is_enabled BOOLEAN NOT NULL DEFAULT 1,
                is_auto_run BOOLEAN NOT NULL DEFAULT 0,
                parameters TEXT,
                created_time DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (server_id) REFERENCES mcp_server(id) ON DELETE CASCADE,
                UNIQUE(server_id, tool_name)
            );",
            [],
        )?;

        // Create MCP server resources table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS mcp_server_resource (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                server_id INTEGER NOT NULL,
                resource_uri TEXT NOT NULL,
                resource_name TEXT NOT NULL,
                resource_type TEXT NOT NULL,
                resource_description TEXT,
                created_time DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (server_id) REFERENCES mcp_server(id) ON DELETE CASCADE,
                UNIQUE(server_id, resource_uri)
            );",
            [],
        )?;

        // Create MCP server prompts table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS mcp_server_prompt (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                server_id INTEGER NOT NULL,
                prompt_name TEXT NOT NULL,
                prompt_description TEXT,
                is_enabled BOOLEAN NOT NULL DEFAULT 1,
                arguments TEXT,
                created_time DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (server_id) REFERENCES mcp_server(id) ON DELETE CASCADE,
                UNIQUE(server_id, prompt_name)
            );",
            [],
        )?;

        Ok(())
    }

    pub fn get_mcp_servers(&self) -> rusqlite::Result<Vec<MCPServer>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, transport_type, command, environment_variables, url, timeout, is_long_running, is_enabled, created_time 
             FROM mcp_server ORDER BY created_time DESC"
        )?;

        let servers = stmt.query_map([], |row| {
            Ok(MCPServer {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                transport_type: row.get(3)?,
                command: row.get(4)?,
                environment_variables: row.get(5)?,
                url: row.get(6)?,
                timeout: row.get(7)?,
                is_long_running: row.get(8)?,
                is_enabled: row.get(9)?,
                created_time: row.get(10)?,
            })
        })?;

        let mut result = Vec::new();
        for server in servers {
            result.push(server?);
        }
        Ok(result)
    }

    pub fn get_mcp_server(&self, id: i64) -> rusqlite::Result<MCPServer> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, transport_type, command, environment_variables, url, timeout, is_long_running, is_enabled, created_time 
             FROM mcp_server WHERE id = ?"
        )?;

        let server = stmt
            .query_map([id], |row| {
                Ok(MCPServer {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    transport_type: row.get(3)?,
                    command: row.get(4)?,
                    environment_variables: row.get(5)?,
                    url: row.get(6)?,
                    timeout: row.get(7)?,
                    is_long_running: row.get(8)?,
                    is_enabled: row.get(9)?,
                    created_time: row.get(10)?,
                })
            })?
            .next()
            .transpose()?;

        match server {
            Some(server) => Ok(server),
            None => Err(rusqlite::Error::QueryReturnedNoRows),
        }
    }

    pub fn update_mcp_server(
        &self,
        id: i64,
        name: &str,
        description: Option<&str>,
        transport_type: &str,
        command: Option<&str>,
        environment_variables: Option<&str>,
        url: Option<&str>,
        timeout: Option<i32>,
        is_long_running: bool,
        is_enabled: bool,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE mcp_server SET name = ?, description = ?, transport_type = ?, command = ?, environment_variables = ?, url = ?, timeout = ?, is_long_running = ?, is_enabled = ? WHERE id = ?",
            params![name, description, transport_type, command, environment_variables, url, timeout, is_long_running, is_enabled, id],
        )?;
        Ok(())
    }

    pub fn delete_mcp_server(&self, id: i64) -> rusqlite::Result<()> {
        // Cascade delete will handle tools and resources
        self.conn
            .execute("DELETE FROM mcp_server WHERE id = ?", params![id])?;
        Ok(())
    }

    pub fn toggle_mcp_server(&self, id: i64, is_enabled: bool) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE mcp_server SET is_enabled = ? WHERE id = ?",
            params![is_enabled, id],
        )?;
        Ok(())
    }

    pub fn upsert_mcp_server(
        &self,
        name: &str,
        description: Option<&str>,
        transport_type: &str,
        command: Option<&str>,
        environment_variables: Option<&str>,
        url: Option<&str>,
        timeout: Option<i32>,
        is_long_running: bool,
        is_enabled: bool,
    ) -> rusqlite::Result<i64> {
        // First try to get existing server by name
        let existing_id = self
            .conn
            .prepare("SELECT id FROM mcp_server WHERE name = ?")?
            .query_row([name], |row| row.get::<_, i64>(0))
            .optional()?;

        match existing_id {
            Some(id) => {
                // Update existing server
                self.conn.execute(
                    "UPDATE mcp_server SET description = ?, transport_type = ?, command = ?, 
                     environment_variables = ?, url = ?, timeout = ?, is_long_running = ?, is_enabled = ?
                     WHERE id = ?",
                    params![description, transport_type, command, environment_variables, url, timeout, is_long_running, is_enabled, id],
                )?;
                Ok(id)
            }
            None => {
                // Insert new server
                let mut stmt = self.conn.prepare(
                    "INSERT INTO mcp_server (name, description, transport_type, command, environment_variables, url, timeout, is_long_running, is_enabled) 
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
                )?;

                stmt.execute(params![
                    name,
                    description,
                    transport_type,
                    command,
                    environment_variables,
                    url,
                    timeout,
                    is_long_running,
                    is_enabled
                ])?;

                Ok(self.conn.last_insert_rowid())
            }
        }
    }

    pub fn get_mcp_server_tools(&self, server_id: i64) -> rusqlite::Result<Vec<MCPServerTool>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, server_id, tool_name, tool_description, is_enabled, is_auto_run, parameters 
             FROM mcp_server_tool WHERE server_id = ? ORDER BY tool_name"
        )?;

        let tools = stmt.query_map([server_id], |row| {
            Ok(MCPServerTool {
                id: row.get(0)?,
                server_id: row.get(1)?,
                tool_name: row.get(2)?,
                tool_description: row.get(3)?,
                is_enabled: row.get(4)?,
                is_auto_run: row.get(5)?,
                parameters: row.get(6)?,
            })
        })?;

        let mut result = Vec::new();
        for tool in tools {
            result.push(tool?);
        }
        Ok(result)
    }

    pub fn update_mcp_server_tool(
        &self,
        id: i64,
        is_enabled: bool,
        is_auto_run: bool,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE mcp_server_tool SET is_enabled = ?, is_auto_run = ? WHERE id = ?",
            params![is_enabled, is_auto_run, id],
        )?;
        Ok(())
    }

    pub fn upsert_mcp_server_tool(
        &self,
        server_id: i64,
        tool_name: &str,
        tool_description: Option<&str>,
        parameters: Option<&str>,
    ) -> rusqlite::Result<i64> {
        // First try to get existing tool by server_id and tool_name
        let existing_tool = self.conn.prepare(
            "SELECT id, is_enabled, is_auto_run FROM mcp_server_tool WHERE server_id = ? AND tool_name = ?"
        )?.query_row(params![server_id, tool_name], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, bool>(1)?, row.get::<_, bool>(2)?))
        }).optional()?;

        match existing_tool {
            Some((id, _, _)) => {
                // Update existing tool, preserve user settings
                self.conn.execute(
                    "UPDATE mcp_server_tool SET tool_description = ?, parameters = ? WHERE id = ?",
                    params![tool_description, parameters, id],
                )?;
                Ok(id)
            }
            None => {
                // Insert new tool with default settings
                let mut stmt = self.conn.prepare(
                    "INSERT INTO mcp_server_tool (server_id, tool_name, tool_description, is_enabled, is_auto_run, parameters) 
                     VALUES (?, ?, ?, ?, ?, ?)"
                )?;

                stmt.execute(params![
                    server_id,
                    tool_name,
                    tool_description,
                    true,  // Default enabled
                    false, // Default not auto-run
                    parameters
                ])?;

                Ok(self.conn.last_insert_rowid())
            }
        }
    }

    pub fn get_mcp_server_resources(
        &self,
        server_id: i64,
    ) -> rusqlite::Result<Vec<MCPServerResource>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, server_id, resource_uri, resource_name, resource_type, resource_description 
             FROM mcp_server_resource WHERE server_id = ? ORDER BY resource_name"
        )?;

        let resources = stmt.query_map([server_id], |row| {
            Ok(MCPServerResource {
                id: row.get(0)?,
                server_id: row.get(1)?,
                resource_uri: row.get(2)?,
                resource_name: row.get(3)?,
                resource_type: row.get(4)?,
                resource_description: row.get(5)?,
            })
        })?;

        let mut result = Vec::new();
        for resource in resources {
            result.push(resource?);
        }
        Ok(result)
    }

    pub fn upsert_mcp_server_resource(
        &self,
        server_id: i64,
        resource_uri: &str,
        resource_name: &str,
        resource_type: &str,
        resource_description: Option<&str>,
    ) -> rusqlite::Result<i64> {
        // First try to get existing resource by server_id and resource_uri
        let existing_id = self
            .conn
            .prepare("SELECT id FROM mcp_server_resource WHERE server_id = ? AND resource_uri = ?")?
            .query_row(params![server_id, resource_uri], |row| row.get::<_, i64>(0))
            .optional()?;

        match existing_id {
            Some(id) => {
                // Update existing resource
                self.conn.execute(
                    "UPDATE mcp_server_resource SET resource_name = ?, resource_type = ?, resource_description = ? WHERE id = ?",
                    params![resource_name, resource_type, resource_description, id],
                )?;
                Ok(id)
            }
            None => {
                // Insert new resource
                let mut stmt = self.conn.prepare(
                    "INSERT INTO mcp_server_resource (server_id, resource_uri, resource_name, resource_type, resource_description) 
                     VALUES (?, ?, ?, ?, ?)"
                )?;

                stmt.execute(params![
                    server_id,
                    resource_uri,
                    resource_name,
                    resource_type,
                    resource_description
                ])?;

                Ok(self.conn.last_insert_rowid())
            }
        }
    }

    pub fn get_mcp_server_prompts(&self, server_id: i64) -> rusqlite::Result<Vec<MCPServerPrompt>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, server_id, prompt_name, prompt_description, is_enabled, arguments 
             FROM mcp_server_prompt WHERE server_id = ? ORDER BY prompt_name",
        )?;

        let prompts = stmt.query_map([server_id], |row| {
            Ok(MCPServerPrompt {
                id: row.get(0)?,
                server_id: row.get(1)?,
                prompt_name: row.get(2)?,
                prompt_description: row.get(3)?,
                is_enabled: row.get(4)?,
                arguments: row.get(5)?,
            })
        })?;

        let mut result = Vec::new();
        for prompt in prompts {
            result.push(prompt?);
        }
        Ok(result)
    }

    pub fn update_mcp_server_prompt(&self, id: i64, is_enabled: bool) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE mcp_server_prompt SET is_enabled = ? WHERE id = ?",
            params![is_enabled, id],
        )?;
        Ok(())
    }

    pub fn upsert_mcp_server_prompt(
        &self,
        server_id: i64,
        prompt_name: &str,
        prompt_description: Option<&str>,
        arguments: Option<&str>,
    ) -> rusqlite::Result<i64> {
        // First try to get existing prompt by server_id and prompt_name
        let existing_prompt = self.conn.prepare(
            "SELECT id, is_enabled FROM mcp_server_prompt WHERE server_id = ? AND prompt_name = ?"
        )?.query_row(params![server_id, prompt_name], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, bool>(1)?))
        }).optional()?;

        match existing_prompt {
            Some((id, _is_enabled)) => {
                // Update existing prompt, preserve user settings
                self.conn.execute(
                    "UPDATE mcp_server_prompt SET prompt_description = ?, arguments = ? WHERE id = ?",
                    params![prompt_description, arguments, id],
                )?;
                Ok(id)
            }
            None => {
                // Insert new prompt with default settings
                let mut stmt = self.conn.prepare(
                    "INSERT INTO mcp_server_prompt (server_id, prompt_name, prompt_description, is_enabled, arguments) 
                     VALUES (?, ?, ?, ?, ?)"
                )?;

                stmt.execute(params![
                    server_id,
                    prompt_name,
                    prompt_description,
                    true, // Default enabled
                    arguments
                ])?;

                Ok(self.conn.last_insert_rowid())
            }
        }
    }
}
