//! HealLedger — a chained korg-ledger@v1 journal writer for thumper's heal loop.
//!
//! Each self-healing session opens a session, emits an event per error caught /
//! repair attempt / exit, and appends each as one canonical JSON line to a JSONL
//! journal. Because it uses the vendored `chain` canonicalization + GENESIS, the
//! resulting journal is verifiable byte-for-byte by `korgex verify` and
//! korg-registry's `verify_chain` — a recovery session becomes a non-repudiable,
//! replayable forensic trail that a *different* tool can cryptographically audit.

use super::chain;
use serde_json::{json, Map, Value};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

/// thumper's source-agent identity in the ledger.
pub const SOURCE_AGENT: &str = "thumper";

/// Resolve the journal path: `THUMPER_JOURNAL_PATH` → `KORG_JOURNAL_PATH`
/// (drop-in korgex compat) → `$HOME/.api-anything/journal.jsonl`.
pub fn default_journal_path() -> PathBuf {
    if let Ok(p) = std::env::var("THUMPER_JOURNAL_PATH") {
        return PathBuf::from(p);
    }
    if let Ok(p) = std::env::var("KORG_JOURNAL_PATH") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".api-anything")
        .join("journal.jsonl")
}

fn ledger_hmac_key() -> Option<Vec<u8>> {
    std::env::var("KORG_LEDGER_HMAC_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .map(|s| s.into_bytes())
}

/// Recover (next_seq, chain_head) from an existing journal; (1, GENESIS) if absent.
fn recover_state(path: &Path) -> (u64, String) {
    let mut max_seq = 0u64;
    let mut last_hash = chain::GENESIS_HASH.to_string();
    if let Ok(content) = std::fs::read_to_string(path) {
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                if let Some(s) = v.get("seq_id").and_then(|x| x.as_u64()) {
                    max_seq = max_seq.max(s);
                }
                if let Some(h) = v.get("entry_hash").and_then(|x| x.as_str()) {
                    last_hash = h.to_string();
                }
            }
        }
    }
    (max_seq + 1, last_hash)
}

/// One heal session = one prev_hash chain, monotonic seq_ids from 1.
pub struct HealLedger {
    path: PathBuf,
    key: Option<Vec<u8>>,
    next_seq: u64,
    prev_hash: String,
    /// When false, append() is a no-op (the heal hot path can opt out).
    enabled: bool,
}

impl HealLedger {
    /// Open a session writing to the default journal (honours env overrides).
    pub fn open() -> Self {
        Self::open_at(default_journal_path())
    }

    pub fn open_at(path: PathBuf) -> Self {
        // Recover the chain head from an existing journal so every heal session
        // continues ONE chain (the whole file stays verifiable) instead of
        // resetting to GENESIS per call. heal runs on failure, not every command,
        // so the O(n) read is acceptable; rotation is a future concern.
        let (next_seq, prev_hash) = recover_state(&path);
        Self {
            path,
            key: ledger_hmac_key(),
            next_seq,
            prev_hash,
            enabled: true,
        }
    }

    /// A disabled session: every method is a no-op. For hot paths that opt out
    /// or contexts without a writable journal.
    pub fn disabled() -> Self {
        let mut s = Self::open_at(PathBuf::new());
        s.enabled = false;
        s
    }

    /// Append a chained event and return its seq_id (0 if disabled).
    /// `args`/`result` must contain only strings/ints/bools/objects (no floats —
    /// v1 canonicalization scope) so the chain stays cross-impl verifiable.
    pub fn append(
        &mut self,
        tool_name: &str,
        args: Value,
        result: Value,
        success: bool,
        duration_ms: u64,
        triggered_by: Option<u64>,
    ) -> u64 {
        if !self.enabled {
            return 0;
        }
        let seq = self.next_seq;
        let mut ev = Map::new();
        ev.insert("schema_version".into(), json!("1.0"));
        ev.insert("seq_id".into(), json!(seq));
        ev.insert("source_agent".into(), json!(SOURCE_AGENT));
        ev.insert("tool_name".into(), json!(tool_name));
        ev.insert("args".into(), args);
        ev.insert("result".into(), result);
        ev.insert("success".into(), json!(success));
        ev.insert("duration_ms".into(), json!(duration_ms));
        if let Some(tb) = triggered_by {
            ev.insert("triggered_by".into(), json!(tb));
        }
        ev.insert("prev_hash".into(), json!(self.prev_hash));
        let value = Value::Object(ev);
        let entry_hash = chain::chain_hash(&value, self.key.as_deref());
        let mut value = value;
        value
            .as_object_mut()
            .unwrap()
            .insert("entry_hash".into(), json!(entry_hash));

        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(f, "{}", serde_json::to_string(&value).unwrap_or_default());
        }

        self.prev_hash = entry_hash;
        self.next_seq += 1;
        seq
    }
}

#[cfg(test)]
mod tests {
    use super::super::chain::{verify_chain, verify_dag};
    use super::*;
    use serde_json::json;

    fn read(path: &PathBuf) -> Vec<Value> {
        std::fs::read_to_string(path)
            .unwrap()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    #[test]
    fn a_session_produces_an_intact_verifiable_chain() {
        let dir = std::env::temp_dir().join(format!("thumper-ledger-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("journal.jsonl");
        let mut led = HealLedger::open_at(path.clone());

        let e = led.append(
            "heal.error",
            json!({"command": "bun run x"}),
            json!({}),
            false,
            0,
            None,
        );
        let r = led.append(
            "heal.repair",
            json!({"error_type": "semicolon", "file": "a.ts"}),
            json!({"strategy": "insert-semicolon"}),
            true,
            3,
            Some(e),
        );
        led.append(
            "heal.exit",
            json!({"command": "bun run x"}),
            json!({"healed": true, "patches": 1}),
            true,
            12,
            Some(r),
        );

        let events = read(&path);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0]["source_agent"], "thumper");
        assert_eq!(events[0]["prev_hash"], chain::GENESIS_HASH);
        assert_eq!(events[1]["prev_hash"], events[0]["entry_hash"]);
        assert!(
            verify_chain(&events, None).is_empty(),
            "fresh session must be intact"
        );
        assert!(
            verify_dag(&events).is_empty(),
            "causal DAG must be well-formed"
        );
    }

    #[test]
    fn tampering_a_persisted_event_is_detected() {
        let dir =
            std::env::temp_dir().join(format!("thumper-ledger-tamper-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("journal.jsonl");
        let mut led = HealLedger::open_at(path.clone());
        let e = led.append(
            "heal.error",
            json!({"command": "x"}),
            json!({}),
            false,
            0,
            None,
        );
        led.append(
            "heal.exit",
            json!({"command": "x"}),
            json!({"healed": true}),
            true,
            5,
            Some(e),
        );

        let mut events = read(&path);
        events[0]["args"] = json!({"command": "EVIL"}); // edit without recomputing
        let errors = verify_chain(&events, None);
        assert!(!errors.is_empty(), "edit must be detected");
        assert!(errors.iter().any(|e| e.contains("seq 1")), "{errors:?}");
    }

    #[test]
    fn disabled_session_is_a_noop() {
        let mut led = HealLedger::disabled();
        assert_eq!(
            led.append("heal.error", json!({}), json!({}), false, 0, None),
            0
        );
    }
}
