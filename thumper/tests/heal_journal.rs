//! Wiring proof: a real heal session emits an intact, verifiable korg-ledger@v1
//! chain. This is the "ledger_write is now real" test — thumper's self-healing
//! produces a non-repudiable, replayable forensic trail a different tool can
//! cryptographically audit.

use serde_json::Value;
use serial_test::serial;
use thumper_cli::bun::recovery::heal_node;
use thumper_cli::ledger::chain::{verify_chain, verify_dag};

fn read(path: &std::path::Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[tokio::test]
#[serial]
async fn heal_session_writes_an_intact_verifiable_chain() {
    let dir = std::env::temp_dir().join(format!("thumper-heal-it-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("journal.jsonl");
    std::env::set_var("THUMPER_JOURNAL_PATH", &path);
    std::env::remove_var("KORG_LEDGER_HMAC_KEY");

    // No stderr/worktree → the fallback heal path runs and returns Ok(true).
    let healed = heal_node("bun run app.ts", None).await.unwrap();
    assert!(healed);

    let events = read(&path);
    assert_eq!(
        events.len(),
        3,
        "expected heal.error + heal.repair + heal.exit, got {events:?}"
    );
    assert_eq!(events[0]["tool_name"], "heal.error");
    assert_eq!(events[0]["source_agent"], "thumper");
    assert_eq!(events[0]["prev_hash"].as_str().unwrap(), "0".repeat(64));
    assert_eq!(events[1]["tool_name"], "heal.repair");
    assert_eq!(events[1]["triggered_by"], events[0]["seq_id"]); // repair ← error
    assert_eq!(events[2]["tool_name"], "heal.exit");
    assert_eq!(events[2]["result"]["healed"], true);
    assert_eq!(events[2]["triggered_by"], events[1]["seq_id"]); // exit ← last (repair)

    // the trail is a sound, tamper-evident chain
    assert!(
        verify_chain(&events, None).is_empty(),
        "chain must be intact"
    );
    assert!(
        verify_dag(&events).is_empty(),
        "causal DAG must be well-formed"
    );

    std::env::remove_var("THUMPER_JOURNAL_PATH");
}

#[tokio::test]
#[serial]
async fn two_heal_sessions_extend_one_continuous_chain() {
    let dir = std::env::temp_dir().join(format!("thumper-heal-cont-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("journal.jsonl");
    std::env::set_var("THUMPER_JOURNAL_PATH", &path);
    std::env::remove_var("KORG_LEDGER_HMAC_KEY");

    heal_node("cmd one", None).await.unwrap();
    heal_node("cmd two", None).await.unwrap();

    let events = read(&path);
    assert_eq!(
        events.len(),
        6,
        "two sessions × 3 events → 6 chained events"
    );
    // seq_ids continue (1..=6), not reset per session, and the whole file verifies
    let seqs: Vec<u64> = events
        .iter()
        .map(|e| e["seq_id"].as_u64().unwrap())
        .collect();
    assert_eq!(seqs, vec![1, 2, 3, 4, 5, 6]);
    assert!(
        verify_chain(&events, None).is_empty(),
        "continuous chain must verify"
    );
    assert!(verify_dag(&events).is_empty());

    std::env::remove_var("THUMPER_JOURNAL_PATH");
}
