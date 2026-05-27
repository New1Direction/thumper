//! Embedded SQLite database module for Thumper.
//! Manages registry tools, speculative executions, and DAG caching.

use anyhow::{anyhow, Result};
use rusqlite::{params, Connection};
use std::cell::RefCell;
use std::path::{Path, PathBuf};

thread_local! {
    static TEST_DB_PATH: RefCell<Option<PathBuf>> = RefCell::new(None);
}

#[cfg(test)]
pub fn set_test_db_path(path: PathBuf) {
    TEST_DB_PATH.with(|p| {
        *p.borrow_mut() = Some(path);
    });
}

fn db_path() -> PathBuf {
    TEST_DB_PATH.with(|p| {
        if let Some(ref path) = *p.borrow() {
            path.clone()
        } else {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            home.join(".api-anything").join("registry.db")
        }
    })
}

fn legacy_json_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".api-anything").join("registry.json")
}

/// Initialize the database and run schema setup.
/// If a legacy `registry.json` exists, migrate its contents.
pub fn init_db() -> Result<()> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let conn = Connection::open(&path)?;

    // Create registry table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS registry (
            name TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            output_dir TEXT NOT NULL,
            artifacts TEXT NOT NULL,
            absorbed INTEGER NOT NULL,
            last_generated TEXT NOT NULL
        )",
        [],
    )?;

    // Create executions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS executions (
            id TEXT PRIMARY KEY,
            command TEXT NOT NULL,
            status TEXT NOT NULL,
            confidence REAL NOT NULL,
            risk TEXT NOT NULL,
            severity TEXT NOT NULL,
            blast_radius TEXT NOT NULL,
            certainty REAL NOT NULL,
            remediation_confidence REAL NOT NULL,
            start_time TEXT NOT NULL,
            end_time TEXT,
            logs TEXT NOT NULL,
            dag_json TEXT NOT NULL,
            replay_of TEXT,
            merkle_root TEXT,
            signature TEXT
        )",
        [],
    )?;

    // Gracefully run migrations for existing databases
    let _ = conn.execute("ALTER TABLE executions ADD COLUMN merkle_root TEXT", []);
    let _ = conn.execute("ALTER TABLE executions ADD COLUMN signature TEXT", []);

    // Create chain_cache table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS chain_cache (
            hash TEXT PRIMARY KEY,
            result TEXT NOT NULL,
            created_at TEXT NOT NULL
        )",
        [],
    )?;

    // Run migration if legacy file exists
    let legacy = legacy_json_path();
    if legacy.exists() {
        if let Ok(content) = std::fs::read_to_string(&legacy) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(tools) = data.get("tools").and_then(|t| t.as_array()) {
                    for tool in tools {
                        let name = tool
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or_default();
                        let kind = tool
                            .get("kind")
                            .and_then(|k| k.as_str())
                            .unwrap_or_default();
                        let output_dir = tool
                            .get("output_dir")
                            .and_then(|o| o.as_str())
                            .unwrap_or_default();

                        let mut artifacts = Vec::new();
                        if let Some(arr) = tool.get("artifacts").and_then(|a| a.as_array()) {
                            for val in arr {
                                if let Some(s) = val.as_str() {
                                    artifacts.push(s.to_string());
                                }
                            }
                        }
                        let artifacts_json =
                            serde_json::to_string(&artifacts).unwrap_or_else(|_| "[]".to_string());

                        let absorbed = tool
                            .get("absorbed")
                            .and_then(|a| a.as_bool())
                            .unwrap_or(false);
                        let last_gen = tool
                            .get("last_generated")
                            .and_then(|l| l.as_str())
                            .unwrap_or_default();

                        if !name.is_empty() {
                            let _ = conn.execute(
                                "INSERT OR REPLACE INTO registry (name, kind, output_dir, artifacts, absorbed, last_generated)
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                                params![
                                    name,
                                    kind,
                                    output_dir,
                                    artifacts_json,
                                    if absorbed { 1 } else { 0 },
                                    last_gen
                                ],
                            );
                        }
                    }
                }
            }
        }
        // Rename legacy file to avoid migrating again
        let migrated_path = legacy.with_extension("json.migrated");
        let _ = std::fs::rename(&legacy, &migrated_path);
    }

    Ok(())
}

/// A structure representing a single tool stored in the database registry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SqliteTool {
    pub name: String,
    pub kind: String,
    pub output_dir: String,
    pub artifacts: Vec<String>,
    pub absorbed: bool,
    pub last_generated: String,
}

/// Retrieve all tools from the SQLite registry.
pub fn load_tools() -> Result<Vec<SqliteTool>> {
    let conn = Connection::open(db_path())?;
    let mut stmt = conn.prepare(
        "SELECT name, kind, output_dir, artifacts, absorbed, last_generated FROM registry",
    )?;
    let tool_iter = stmt.query_map([], |row| {
        let artifacts_json: String = row.get(3)?;
        let artifacts: Vec<String> = serde_json::from_str(&artifacts_json).unwrap_or_default();
        let absorbed_int: i32 = row.get(4)?;
        Ok(SqliteTool {
            name: row.get(0)?,
            kind: row.get(1)?,
            output_dir: row.get(2)?,
            artifacts,
            absorbed: absorbed_int != 0,
            last_generated: row.get(5)?,
        })
    })?;

    let mut tools = Vec::new();
    for tool in tool_iter {
        tools.push(tool?);
    }
    Ok(tools)
}

/// Insert or update a tool in the SQLite registry database.
pub fn save_tool(
    name: &str,
    kind: &str,
    output_dir: &str,
    artifacts: &[String],
    absorbed: bool,
) -> Result<()> {
    let conn = Connection::open(db_path())?;
    let artifacts_json = serde_json::to_string(artifacts)?;
    let last_gen = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT OR REPLACE INTO registry (name, kind, output_dir, artifacts, absorbed, last_generated)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            name,
            kind,
            output_dir,
            artifacts_json,
            if absorbed { 1 } else { 0 },
            last_gen
        ],
    )?;

    Ok(())
}

/// Telemetry structure representing a single speculative/DAG execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SqliteExecution {
    pub id: String,
    pub command: String,
    pub status: String,
    pub confidence: f64,
    pub risk: String,
    pub severity: String,
    pub blast_radius: String,
    pub certainty: f64,
    pub remediation_confidence: f64,
    pub start_time: String,
    pub end_time: Option<String>,
    pub logs: String,
    pub dag_json: String,
    pub replay_of: Option<String>,
    pub merkle_root: Option<String>,
    pub signature: Option<String>,
}

/// Log a new execution event to the database.
pub fn insert_execution(exec: &SqliteExecution) -> Result<()> {
    let conn = Connection::open(db_path())?;
    conn.execute(
        "INSERT INTO executions (id, command, status, confidence, risk, severity, blast_radius, certainty, remediation_confidence, start_time, end_time, logs, dag_json, replay_of, merkle_root, signature)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            exec.id,
            exec.command,
            exec.status,
            exec.confidence,
            exec.risk,
            exec.severity,
            exec.blast_radius,
            exec.certainty,
            exec.remediation_confidence,
            exec.start_time,
            exec.end_time,
            exec.logs,
            exec.dag_json,
            exec.replay_of,
            exec.merkle_root,
            exec.signature,
        ],
    )?;
    Ok(())
}

/// Update execution status, logs, or metrics.
pub fn update_execution(
    id: &str,
    status: &str,
    certainty: f64,
    logs: &str,
    end_time: Option<String>,
) -> Result<()> {
    let conn = Connection::open(db_path())?;
    conn.execute(
        "UPDATE executions SET status = ?2, certainty = ?3, logs = ?4, end_time = ?5 WHERE id = ?1",
        params![id, status, certainty, logs, end_time],
    )?;
    Ok(())
}

/// Load an execution by ID.
pub fn get_execution(id: &str) -> Result<Option<SqliteExecution>> {
    let conn = Connection::open(db_path())?;
    let mut stmt = conn.prepare(
        "SELECT id, command, status, confidence, risk, severity, blast_radius, certainty, remediation_confidence, start_time, end_time, logs, dag_json, replay_of, merkle_root, signature FROM executions WHERE id = ?1"
    )?;
    let mut rows = stmt.query(params![id])?;

    if let Some(row) = rows.next()? {
        Ok(Some(SqliteExecution {
            id: row.get(0)?,
            command: row.get(1)?,
            status: row.get(2)?,
            confidence: row.get(3)?,
            risk: row.get(4)?,
            severity: row.get(5)?,
            blast_radius: row.get(6)?,
            certainty: row.get(7)?,
            remediation_confidence: row.get(8)?,
            start_time: row.get(9)?,
            end_time: row.get(10)?,
            logs: row.get(11)?,
            dag_json: row.get(12)?,
            replay_of: row.get(13)?,
            merkle_root: row.get(14)?,
            signature: row.get(15)?,
        }))
    } else {
        Ok(None)
    }
}

/// Cache subgraph results.
pub fn write_cache(hash: &str, result: &str) -> Result<()> {
    let conn = Connection::open(db_path())?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR REPLACE INTO chain_cache (hash, result, created_at) VALUES (?1, ?2, ?3)",
        params![hash, result, now],
    )?;
    Ok(())
}

/// Fetch cached subgraph results.
pub fn read_cache(hash: &str) -> Result<Option<String>> {
    let conn = Connection::open(db_path())?;
    let mut stmt = conn.prepare("SELECT result FROM chain_cache WHERE hash = ?1")?;
    let mut rows = stmt.query(params![hash])?;
    if let Some(row) = rows.next()? {
        let result: String = row.get(0)?;
        Ok(Some(result))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_registry_workflow() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_file = temp_dir.path().join("test_registry.db");
        set_test_db_path(db_file);

        // Initialize db
        init_db().unwrap();

        // 1. Tool Caching & Retrieval
        let test_tool_name = "test_tool_999";
        save_tool(
            test_tool_name,
            "test_kind",
            "/test/path",
            &["test_art".to_string()],
            true,
        )
        .unwrap();

        let tools = load_tools().unwrap();
        let found = tools.iter().find(|t| t.name == test_tool_name).unwrap();
        assert_eq!(found.kind, "test_kind");
        assert_eq!(found.output_dir, "/test/path");
        assert_eq!(found.artifacts, vec!["test_art".to_string()]);
        assert!(found.absorbed);

        // 2. Executions Insertion, Update & Query
        let exec = SqliteExecution {
            id: "exec_test_id_123".to_string(),
            command: "thump test --mock".to_string(),
            status: "running".to_string(),
            confidence: 0.99,
            risk: "None".to_string(),
            severity: "None".to_string(),
            blast_radius: "None".to_string(),
            certainty: 80.0,
            remediation_confidence: 0.9,
            start_time: "2026-05-21T00:00:00Z".to_string(),
            end_time: None,
            logs: "Starting test...\n".to_string(),
            dag_json: "{}".to_string(),
            replay_of: None,
            merkle_root: Some("root_hash_xyz".to_string()),
            signature: Some("sig_val_abc".to_string()),
        };

        insert_execution(&exec).unwrap();

        let loaded = get_execution("exec_test_id_123").unwrap().unwrap();
        assert_eq!(loaded.command, "thump test --mock");
        assert_eq!(loaded.status, "running");
        assert_eq!(loaded.merkle_root, Some("root_hash_xyz".to_string()));
        assert_eq!(loaded.signature, Some("sig_val_abc".to_string()));

        // Update the execution
        update_execution(
            "exec_test_id_123",
            "completed",
            100.0,
            "Starting test...\nSuccess!\n",
            Some("2026-05-21T00:01:00Z".to_string()),
        )
        .unwrap();

        let updated = get_execution("exec_test_id_123").unwrap().unwrap();
        assert_eq!(updated.status, "completed");
        assert_eq!(updated.certainty, 100.0);
        assert_eq!(updated.logs, "Starting test...\nSuccess!\n");
        assert_eq!(updated.end_time, Some("2026-05-21T00:01:00Z".to_string()));

        // 3. Cache read & write
        let cache_hash = "hash_abc_123";
        write_cache(cache_hash, "cached_result_data").unwrap();

        let cached_val = read_cache(cache_hash).unwrap().unwrap();
        assert_eq!(cached_val, "cached_result_data");
    }
}
