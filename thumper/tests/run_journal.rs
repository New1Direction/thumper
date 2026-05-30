//! Wiring proof: the NORMAL bun execution path (not just heal) emits a chained
//! korg-ledger@v1 event. Today thumper only ledger-writes on heal; this test
//! pins the requirement that a successful `bun run` lands one verifiable
//! `run.exec` event in the same journal heal writes to — so the journal is the
//! single auditable sink for ALL of thumper's cognition, normal path included.

use serde_json::Value;
use serial_test::serial;
use thumper_cli::bun::{spawn_bun, BunCommand, BunEventOrOutcome, BunInvocation};
use thumper_cli::ledger::chain::{verify_chain, verify_dag};

fn read(path: &std::path::Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// Drain a stream to completion, returning the final outcome's `ok`.
async fn drain(mut stream: thumper_cli::bun::BunStream) -> bool {
    let mut ok = false;
    while let Some(item) = stream.rx.recv().await {
        if let BunEventOrOutcome::Outcome(o) = item {
            ok = o.ok;
        }
    }
    ok
}

#[tokio::test]
#[serial]
async fn a_normal_bun_run_emits_one_chained_run_event() {
    // Skip gracefully if bun is not installed (CI without bun).
    if thumper_cli::bun::find_bun().is_none() {
        eprintln!("bun not found — skipping normal-path ledger test");
        return;
    }

    // Isolated project with a trivial script + isolated journal.
    let dir = std::env::temp_dir().join(format!("thumper-run-it-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("package.json"),
        r#"{"name":"t","scripts":{"hello":"echo hi"}}"#,
    )
    .unwrap();

    let path = dir.join("journal.jsonl");
    std::env::set_var("THUMPER_JOURNAL_PATH", &path);
    std::env::remove_var("KORG_LEDGER_HMAC_KEY");

    let inv = BunInvocation {
        command: BunCommand::ScriptRun {
            name: "hello".to_string(),
            args: vec![],
        },
        cwd: Some(dir.clone()),
        session_id: Some("run-it".to_string()),
        timeout: None,
    };

    let stream = spawn_bun(inv).await.expect("spawn the native bun run");
    let ok = drain(stream).await;
    assert!(ok, "echo hi should succeed");

    let events = read(&path);
    // Exactly one normal-path event: the run completion.
    assert_eq!(
        events.len(),
        1,
        "a normal run must emit exactly one ledger event, got {events:?}"
    );
    let e = &events[0];
    assert_eq!(e["tool_name"], "run.exec", "normal path, not heal.*");
    assert_eq!(e["source_agent"], "thumper");
    assert_eq!(e["success"], true);
    assert_eq!(e["args"]["operation"], "script.run");
    assert_eq!(e["prev_hash"].as_str().unwrap(), "0".repeat(64));

    // It is a sound, tamper-evident chain on its own.
    assert!(
        verify_chain(&events, None).is_empty(),
        "chain must be intact"
    );
    assert!(verify_dag(&events).is_empty(), "DAG must be well-formed");

    std::env::remove_var("THUMPER_JOURNAL_PATH");
}

#[tokio::test]
#[serial]
async fn normal_runs_extend_one_continuous_chain_across_invocations() {
    if thumper_cli::bun::find_bun().is_none() {
        eprintln!("bun not found — skipping continuity test");
        return;
    }

    let dir = std::env::temp_dir().join(format!("thumper-run-cont-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("package.json"),
        r#"{"name":"t","scripts":{"hello":"echo hi"}}"#,
    )
    .unwrap();

    let path = dir.join("journal.jsonl");
    std::env::set_var("THUMPER_JOURNAL_PATH", &path);
    std::env::remove_var("KORG_LEDGER_HMAC_KEY");

    for _ in 0..2 {
        let inv = BunInvocation {
            command: BunCommand::ScriptRun {
                name: "hello".to_string(),
                args: vec![],
            },
            cwd: Some(dir.clone()),
            session_id: None,
            timeout: None,
        };
        let stream = spawn_bun(inv).await.expect("spawn run");
        drain(stream).await;
    }

    let events = read(&path);
    assert_eq!(events.len(), 2, "two runs → two chained events");
    let seqs: Vec<u64> = events
        .iter()
        .map(|e| e["seq_id"].as_u64().unwrap())
        .collect();
    assert_eq!(seqs, vec![1, 2], "seq_ids continue, not reset per run");
    assert_eq!(events[1]["prev_hash"], events[0]["entry_hash"]);
    assert!(verify_chain(&events, None).is_empty());
    assert!(verify_dag(&events).is_empty());

    std::env::remove_var("THUMPER_JOURNAL_PATH");
}
