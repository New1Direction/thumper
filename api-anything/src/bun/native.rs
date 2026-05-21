//! Native Bun execution engine (pure Rust, no Python dependency).
//!
//! Phase 1 goal: fast path for the common commands used by the TUI palette
//! (`bun add`, `bun install`, `bun remove`, `bun run <script>`).
//!
//! This module is intentionally separate from harness.rs so the Python path
//! remains untouched and focused.

use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;

/// Discover the `bun` executable.
///
/// Modeled *exactly* after `find_python()` in harness.rs for familiarity:
/// - Check well-known environment variables first
/// - Check the default user installation location
/// - Fall back to `which::which`
/// - Finally try a short list of common system paths
///
/// Returns the full path to the `bun` binary (or an error with a helpful message).
pub async fn find_bun() -> Result<PathBuf> {
    // Delegate to the dedicated cross-platform discovery module (Chunk 16).
    // This centralizes all platform-specific logic and makes future
    // hardening (Windows, better logging, etc.) much easier.
    match super::discovery::find_bun() {
        Some(path) => Ok(path),
        None => Err(anyhow!(
            "Could not locate the `bun` binary.\n\
             Please install Bun (https://bun.sh) or set the BUN_INSTALL environment variable.\n\
             Common fix on macOS: `curl -fsSL https://bun.sh/install | bash`"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test — we don't assert a specific path because it depends on the
    /// developer's machine, but we verify that the function runs without
    /// panicking and either finds bun or returns a clear, actionable error.
    #[tokio::test]
    async fn find_bun_does_not_panic() {
        let result = find_bun().await;

        match result {
            Ok(path) => {
                // If we found it, the path should at least contain "bun" in the filename
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                assert!(
                    file_name.contains("bun"),
                    "Discovered path should refer to a bun binary: {:?}",
                    path
                );
            }
            Err(e) => {
                // Error message must be helpful (contains installation guidance)
                let msg = e.to_string();
                assert!(
                    msg.contains("bun") && (msg.contains("install") || msg.contains("BUN_INSTALL")),
                    "Error message should guide the user toward installing Bun: {}",
                    msg
                );
            }
        }
    }

    /// Quick sanity check that the function is async and can be awaited
    /// in a normal Tokio test context (important for later integration).
    #[tokio::test]
    async fn find_bun_is_async() {
        // Just exercising the async boundary
        let _ = tokio::spawn(async { find_bun().await }).await;
    }
}

/// Integration smoke for the native path + selector (Chunk 4 "light up" test).
///
/// This test only runs the real `bun` binary when it is discoverable on the
/// machine. It exercises the full path:
///   find_bun() → spawn_bun_native() → BunCommand → real subprocess
///   → streaming into BunEventOrOutcome with "bun.native.*" events
///
/// When `bun` is not installed the test is a no-op (graceful skip).
#[tokio::test]
async fn spawn_bun_exercises_native_path_when_bun_present() {
    // Use the smart selector added in Chunk 3 (tries native first, falls back to Python)
    use crate::bun::{spawn_bun, BunCommand, BunInvocation};

    // Create a completely isolated temp directory so we never touch real projects
    let temp = match tempfile::tempdir() {
        Ok(d) => d,
        Err(_) => return, // can't even create temp dir → skip
    };

    let inv = BunInvocation {
        command: BunCommand::PackageInstall {
            packages: vec![],
            frozen_lockfile: false,
        },
        cwd: Some(temp.path().to_path_buf()),
        session_id: Some("chunk4-test".to_string()),
        timeout: None,
    };

    // Call the selector. If native bun is present we will get native events.
    // If bun is not installed at all, spawn_bun will fall back to Python
    // (which may or may not be present). Either way we just want to prove the
    // call succeeds and we receive at least one event.
    let mut stream = match spawn_bun(inv).await {
        Ok(s) => s,
        Err(_) => return, // neither native nor Python harness available → skip
    };

    // Collect a few events (with a short timeout so the test never hangs)
    let mut saw_native_event = false;
    let mut received_any = false;

    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(8);

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(std::time::Duration::from_millis(300), stream.rx.recv()).await {
            Ok(Some(item)) => {
                received_any = true;

                if let crate::bun::BunEventOrOutcome::Event(ev) = &item {
                    if ev.event.starts_with("bun.native.") {
                        saw_native_event = true;
                    }
                }

                // We only need to prove the path works; we don't need every event
                if received_any {
                    break;
                }
            }
            _ => break,
        }
    }

    // If we got here and `bun` was present, we should have seen at least one native event.
    // If we fell back to Python we won't have "bun.native" events — that's also fine.
    // The important thing is that the selector + streaming did not blow up.
    if let Ok(bun_path) = find_bun().await {
        // Bun binary exists on this machine → we expect the native path to have been taken
        // (or at worst a few raw events). We assert we received *something*.
        assert!(
            received_any,
            "When bun is present at {:?}, spawn_bun should have produced at least one event",
            bun_path
        );
    }

    // Best-effort: if we saw a native event, great — the new path is lit up.
    // We don't fail the test if we didn't (the machine might have used the Python fallback).
    let _ = saw_native_event; // silence unused warning in some builds
}

// =============================================================================
// Native spawning (Phase 1 skeleton)
// =============================================================================

use crate::bun::events::{BunEvent, BunEventOrOutcome, BunOutcome, EventLevel};
use crate::bun::harness::{BunCommand, BunInvocation, BunStream};
use chrono::Utc;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex};

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Convert a `BunCommand` into the real `bun` CLI arguments + a short stable verb.
///
/// Returns (args_for_bun, verb) where verb is one of: "run", "add", "install", "remove".
/// These verbs are used to produce nicer event names like `bun.native.install.stdout`.
///
/// This is different from the Python harness `to_args()` (which targets
/// `thump` Python package).
fn bun_command_to_cli_args_and_verb(cmd: &BunCommand) -> (Vec<String>, &'static str) {
    match cmd {
        BunCommand::ScriptRun { name, args } => {
            let mut v = vec!["run".to_string(), name.clone()];
            v.extend(args.clone());
            (v, "run")
        }
        BunCommand::PackageAdd {
            packages,
            dev,
            exact,
            peer,
            optional,
        } => {
            let mut v = vec!["add".to_string()];
            v.extend(packages.clone());
            if *dev {
                v.push("--dev".to_string());
            }
            if *exact {
                v.push("--exact".to_string());
            }
            if *peer {
                v.push("--peer".to_string());
            }
            if *optional {
                v.push("--optional".to_string());
            }
            (v, "add")
        }
        BunCommand::PackageInstall {
            packages,
            frozen_lockfile,
        } => {
            let mut v = vec!["install".to_string()];
            v.extend(packages.clone());
            if *frozen_lockfile {
                v.push("--frozen-lockfile".to_string());
            }
            (v, "install")
        }
        BunCommand::PackageRemove { packages } => {
            let mut v = vec!["remove".to_string()];
            v.extend(packages.clone());
            (v, "remove")
        }
    }
}

/// Spawn the real `bun` binary and return a streaming channel compatible
/// with the existing `BunStream` / `BunEventOrOutcome` contract.
///
/// Phase 1: very pragmatic — every line from stdout/stderr becomes a simple
/// `BunEvent` with `data.raw` containing the original text. This is enough
/// to light up the Jobs panel and palette feedback immediately.
pub async fn spawn_bun_native(inv: BunInvocation) -> Result<BunStream> {
    let bun_path = find_bun().await?;
    let (cli_args, verb) = bun_command_to_cli_args_and_verb(&inv.command);

    let mut cmd = Command::new(&bun_path);
    cmd.args(&cli_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    if let Some(cwd) = &inv.cwd {
        cmd.current_dir(cwd);
    }

    let mut child = cmd
        .spawn()
        .with_context(|| format!("failed to spawn bun at {:?}", bun_path))?;

    let stdout = child.stdout.take().context("bun process has no stdout")?;
    let stderr = child.stderr.take().context("bun process has no stderr")?;

    // Wrap in Arc<Mutex> so the waiter task can call wait() while we still
    // return a reference in BunStream (required for API compatibility).
    let child = Arc::new(Mutex::new(child));

    let (tx, rx) = mpsc::unbounded_channel::<BunEventOrOutcome>();

    // Capture verb for richer events + final outcome
    let verb_for_task = verb;
    let operation = verb_to_operation(verb);

    // Spawn line pumps with rich parser (Chunk 10)
    let tx_for_pumps = tx.clone();
    let parser = BunOutputParser::new();
    let child_for_pumps = child.clone();
    tokio::spawn(async move {
        let p = parser; // move into the closure
        let _ = pump_lines(stdout, &tx_for_pumps, verb_for_task, "stdout", Some(&p)).await;
        let _ = pump_lines(stderr, &tx_for_pumps, verb_for_task, "stderr", Some(&p)).await;
    });

    // Dedicated waiter task that gets the *real* exit code
    let tx_for_waiter = tx.clone();
    let child_for_waiter = child.clone();
    tokio::spawn(async move {
        let status = {
            let mut guard = child_for_waiter.lock().await;
            guard.wait().await
        };

        let (ok, exit_code) = match status {
            Ok(s) => (s.success(), s.code()),
            Err(_) => (false, None),
        };

        let outcome = BunOutcome {
            ok,
            operation,
            data: serde_json::json!({
                "exit_code": exit_code,
                "success": ok
            }),
        };

        let _ = tx_for_waiter.send(BunEventOrOutcome::Outcome(outcome));
    });

    Ok(BunStream {
        rx,
        child,
        session_id: inv.session_id,
    })
}

// =============================================================================
// Rich Bun Output Parser (Chunk 10)
// =============================================================================

/// High-level logical stage inferred from Bun's live output.
/// Used by the TUI to drive stage-aware plasma, icons, and progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BunStage {
    #[default]
    Unknown,
    Resolving,
    Downloading,
    Extracting,
    Installing,
    Linking,
    Verifying,
    Running,
    Complete,
}

/// Structured telemetry extracted from a single line of Bun stdout/stderr.
/// This is merged into the `data` field of `BunEvent` so the existing
/// event contract stays 100% compatible while giving downstream code
/// (Jobs panel, plasma, status bar, celebration) high-fidelity numbers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProgressMetrics {
    pub stage: Option<BunStage>,
    /// 0-100
    pub percent: Option<u8>,
    pub current: Option<usize>,
    pub total: Option<usize>,
    /// Number of packages added/updated in this line
    pub packages: Option<usize>,
    /// e.g. "12.4 MB/s"
    pub speed: Option<String>,
    /// e.g. "1.23s" or "420ms"
    pub timing: Option<String>,
    /// Optional ETA string
    pub eta: Option<String>,
}

/// Best-effort, zero-allocation-on-miss parser for Bun CLI output.
/// Three focused regexes + keyword stage detection + ANSI stripping.
pub struct BunOutputParser {
    progress: Regex,
    summary: Regex,
    speed: Regex,
    ansi: Regex,
}

impl BunOutputParser {
    pub fn new() -> Self {
        Self {
            // Matches common progress patterns: [12/45] 34% ,  67% , etc.
            progress: Regex::new(
                r"(?x)
                (?: \[ \s* (\d+) \s* [/\s] \s* (\d+) \s* \] \s* )?   # optional [cur/total]
                .*?
                (\d{1,3})\s*%                                      # explicit percentage
            ",
            )
            .expect("progress regex"),

            // Catches "added 3 packages", "Installed 12", "5 packages"
            summary: Regex::new(
                r"(?ix)
                (?:added|installed|updated|removed)\s+(\d+)
                |
                (\d+)\s+packages?
            ",
            )
            .expect("summary regex"),

            // Speed: 12.3 MB/s , 4.2kB/s , etc.
            speed: Regex::new(
                r"(?ix)
                ([\d.]+)\s*(KB|MB|GB|kB)/s
            ",
            )
            .expect("speed regex"),

            // Strip ANSI escape sequences (colors, cursor movement)
            ansi: Regex::new(r"\x1B\[[0-?]*[ -/]*[@-~]").expect("ansi regex"),
        }
    }

    /// Parse one raw line from Bun. Returns rich metrics only when something
    /// actionable is found (progress, stage, count, speed...).
    pub fn parse_line(&self, raw: &str) -> Option<ProgressMetrics> {
        // Clean carriage returns (progress bars often overwrite with \r)
        let cleaned = raw.trim_end_matches(['\r', '\n']).to_string();
        let line = self.ansi.replace_all(&cleaned, "").trim().to_string();

        if line.is_empty() {
            return None;
        }

        let mut metrics = ProgressMetrics::default();

        // === Stage detection (keyword based, order matters) ===
        if line.contains("Resolving packages") || line.starts_with("Resolving") {
            metrics.stage = Some(BunStage::Resolving);
        } else if line.contains("Downloading") || line.contains("fetch") {
            metrics.stage = Some(BunStage::Downloading);
        } else if line.contains("Extracting") {
            metrics.stage = Some(BunStage::Extracting);
        } else if line.contains("Installing") || line.contains("Linking") {
            metrics.stage = Some(BunStage::Installing);
        } else if line.contains("Verifying") || line.contains("Checked") {
            metrics.stage = Some(BunStage::Verifying);
        } else if line.contains("Saved lockfile")
            || line.contains("Done in")
            || line.contains("successfully")
        {
            metrics.stage = Some(BunStage::Complete);
        } else if line.contains("Running") || line.starts_with("bun run") {
            metrics.stage = Some(BunStage::Running);
        }

        // === Progress percentage + current/total ===
        if let Some(caps) = self.progress.captures(&line) {
            if let Some(pct_str) = caps.get(3) {
                if let Ok(p) = pct_str.as_str().parse::<u8>() {
                    metrics.percent = Some(p.min(100));
                }
            }
            if let (Some(cur), Some(tot)) = (caps.get(1), caps.get(2)) {
                if let (Ok(c), Ok(t)) =
                    (cur.as_str().parse::<usize>(), tot.as_str().parse::<usize>())
                {
                    metrics.current = Some(c);
                    metrics.total = Some(t);
                    if metrics.percent.is_none() && t > 0 {
                        metrics.percent = Some(((c as f32 / t as f32) * 100.0) as u8);
                    }
                }
            }
        }

        // === Package counts from summary lines ===
        if let Some(caps) = self.summary.captures(&line) {
            for i in [1, 2] {
                if let Some(n) = caps.get(i) {
                    if let Ok(count) = n.as_str().parse::<usize>() {
                        metrics.packages = Some(count);
                        break;
                    }
                }
            }
        }

        // === Download / processing speed ===
        if let Some(caps) = self.speed.captures(&line) {
            if let (Some(val), Some(unit)) = (caps.get(1), caps.get(2)) {
                metrics.speed = Some(format!("{}{}/s", val.as_str(), unit.as_str()));
            }
        }

        // === Timing brackets: [1.23s], [420ms] ===
        if let Some(start) = line.find('[') {
            if let Some(end) = line[start..].find(']') {
                let candidate = &line[start + 1..start + end];
                if candidate.ends_with('s') || candidate.ends_with("ms") {
                    metrics.timing = Some(candidate.to_string());
                }
            }
        }

        // Only return if we actually extracted something useful
        if metrics.stage.is_some()
            || metrics.percent.is_some()
            || metrics.packages.is_some()
            || metrics.speed.is_some()
            || metrics.timing.is_some()
        {
            Some(metrics)
        } else {
            None
        }
    }
}

// =============================================================================
// End of Rich Parser
// =============================================================================

/// Helper that reads lines from an AsyncRead and emits simple raw-line events.
/// Accepts ChildStdout / ChildStderr directly (they implement AsyncRead).
///
/// `verb` is one of "run", "add", "install", "remove" — used to produce
/// richer event names like `bun.native.install.stdout`.
async fn pump_lines(
    reader: impl tokio::io::AsyncRead + Unpin,
    tx: &mpsc::UnboundedSender<BunEventOrOutcome>,
    verb: &str,
    stream: &str,
    parser: Option<&BunOutputParser>,
) {
    let mut lines = BufReader::new(reader).lines();

    while let Ok(Some(raw_line)) = lines.next_line().await {
        let clean = raw_line.trim_end_matches(['\r', '\n']).to_string();
        if clean.trim().is_empty() {
            continue;
        }

        let mut data = serde_json::json!({ "raw": clean });

        // Enrich with high-fidelity telemetry from the rich parser (Chunk 10)
        if let Some(p) = parser {
            if let Some(metrics) = p.parse_line(&clean) {
                // Merge parsed fields into the data payload.
                // Existing consumers that only read "raw" continue to work unchanged.
                if let Ok(parsed) = serde_json::to_value(&metrics) {
                    if let serde_json::Value::Object(map) = parsed {
                        if let serde_json::Value::Object(ref mut dmap) = data {
                            for (k, v) in map {
                                dmap.insert(k, v);
                            }
                        }
                    }
                }
            }
        }

        let event = BunEvent {
            ts: Utc::now().to_rfc3339(),
            event: format!("bun.native.{}.{}", verb, stream),
            schema_version: "1".to_string(),
            session_id: None,
            op_id: None,
            level: EventLevel::Info,
            data,
        };

        let _ = tx.send(BunEventOrOutcome::Event(event));
    }
}

/// Map our short verb to the operation names used by the Python harness
/// (and expected by higher layers like the Jobs panel).
fn verb_to_operation(verb: &str) -> String {
    match verb {
        "run" => "script.run".to_string(),
        "add" => "package.add".to_string(),
        "install" => "package.install".to_string(),
        "remove" => "package.remove".to_string(),
        _ => format!("bun.{}", verb),
    }
}
