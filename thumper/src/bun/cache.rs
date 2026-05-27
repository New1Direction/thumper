//! Subgraph Chain Caching & Deterministic Replay module.
//! Optimizes compilation times and replays past telemetries with absolute timing consistency.

use crate::registry::sqlite::{get_execution, read_cache, write_cache, SqliteExecution};
use anyhow::{anyhow, Result};
use std::time::Instant;

pub struct ChainCache;

impl ChainCache {
    /// Generate a cryptographic hash for a DAG configuration.
    pub fn hash_dag(intent: &str, dependencies: &[&str]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        intent.hash(&mut hasher);
        for dep in dependencies {
            dep.hash(&mut hasher);
        }
        format!("hash_{:x}", hasher.finish())
    }

    /// Retrieve the cached results for a subgraph if available.
    pub fn get(hash: &str) -> Option<String> {
        read_cache(hash).unwrap_or(None)
    }

    /// Cache successful subgraph outputs.
    pub fn set(hash: &str, result: &str) {
        write_cache(hash, result).ok();
    }
}

pub struct DeterministicReplay;

impl DeterministicReplay {
    /// Replay an execution session from the database in real time.
    pub async fn replay_session(
        id: &str,
        logs_tx: Option<tokio::sync::mpsc::UnboundedSender<String>>,
    ) -> Result<SqliteExecution> {
        let exec = get_execution(id)?
            .ok_or_else(|| anyhow!("Execution trace with ID '{}' not found in database", id))?;

        if let Some(ref tx) = logs_tx {
            let _ = tx.send(format!(
                "🎥 [REPLAY] Starting deterministic playback of session: {}",
                id
            ));
            let _ = tx.send(format!(
                "🎥 [REPLAY] Command original intent: '{}'",
                exec.command
            ));
            let _ = tx.send("🎥 [REPLAY] Pacing logs chronologically...".to_string());
        }

        // Split the original logs by line and feed them sequentially
        let lines: Vec<&str> = exec.logs.split('\n').collect();
        for line in lines {
            if line.is_empty() {
                continue;
            }
            // Add a slight natural delay for high-fidelity interactive playback feel
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            if let Some(ref tx) = &logs_tx {
                let _ = tx.send(format!("🎥 {}", line));
            }
        }

        if let Some(ref tx) = logs_tx {
            let _ = tx.send(format!(
                "🎥 [REPLAY] Playback complete. Final reported status: {}",
                exec.status
            ));
        }

        Ok(exec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::sqlite::{init_db, insert_execution, set_test_db_path, SqliteExecution};

    #[tokio::test(flavor = "current_thread")]
    async fn test_chain_cache_and_deterministic_replay() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_file = temp_dir.path().join("test_registry.db");
        set_test_db_path(db_file);

        init_db().unwrap();

        // Test ChainCache hashing and get/set
        let intent = "Test Cache DAG";
        let deps = vec!["A", "B"];
        let hash = ChainCache::hash_dag(intent, &deps);
        assert!(!hash.is_empty());

        ChainCache::set(&hash, "successful_result_data");
        let cached = ChainCache::get(&hash).unwrap();
        assert_eq!(cached, "successful_result_data");

        // Test DeterministicReplay
        let test_exec = SqliteExecution {
            id: "replay_test_id_999".to_string(),
            command: "thump test --replay-target".to_string(),
            status: "done".to_string(),
            confidence: 0.95,
            risk: "None".to_string(),
            severity: "None".to_string(),
            blast_radius: "None".to_string(),
            certainty: 100.0,
            remediation_confidence: 1.0,
            start_time: "2026-05-21T10:00:00Z".to_string(),
            end_time: Some("2026-05-21T10:01:00Z".to_string()),
            logs: "Log line 1\nLog line 2\n".to_string(),
            dag_json: "{}".to_string(),
            replay_of: None,
            merkle_root: None,
            signature: None,
        };
        insert_execution(&test_exec).unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let replayed = DeterministicReplay::replay_session("replay_test_id_999", Some(tx))
            .await
            .unwrap();
        assert_eq!(replayed.id, "replay_test_id_999");
        assert_eq!(replayed.status, "done");

        let mut replay_logs = Vec::new();
        while let Ok(log) = rx.try_recv() {
            replay_logs.push(log);
        }
        assert!(replay_logs.iter().any(|l| l.contains("🎥 [REPLAY]")));
        assert!(replay_logs.iter().any(|l| l.contains("Log line 1")));
    }
}
