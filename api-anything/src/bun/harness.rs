//! Core subprocess adapter for driving the Python `thump` harness (formerly cli_anything_bun).
//!
//! This module knows how to:
//! - Locate a Python interpreter
//! - Spawn `python -m thump <subcommand> ...`
//! - Stream NDJSON from stdout in real time
//! - Surface the child process exit status
//!
//! It is intentionally low-level and reusable by TUI, ACP, CLI, and future
//! native Rust paths.

use crate::bun::events::{BunEvent, BunEventOrOutcome, BunOutcome};
use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

/// High-level description of a command to send to the Bun harness.
#[derive(Debug, Clone)]
pub enum BunCommand {
    ScriptRun {
        name: String,
        args: Vec<String>,
    },
    PackageAdd {
        packages: Vec<String>,
        dev: bool,
        exact: bool,
        peer: bool,
        optional: bool,
    },
    PackageInstall {
        packages: Vec<String>,
        frozen_lockfile: bool,
    },
    PackageRemove {
        packages: Vec<String>,
    },
}

impl BunCommand {
    /// Convert into CLI arguments for `cli-anything-bun`
    pub fn to_args(&self) -> Vec<String> {
        match self {
            BunCommand::ScriptRun { name, args } => {
                let mut v = vec!["script".to_string(), "run".to_string(), name.clone()];
                v.extend(args.clone());
                v
            }
            BunCommand::PackageAdd {
                packages,
                dev,
                exact,
                peer,
                optional,
            } => {
                let mut v = vec!["package".to_string(), "add".to_string()];
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
                v
            }
            BunCommand::PackageInstall {
                packages,
                frozen_lockfile,
            } => {
                let mut v = vec!["package".to_string(), "install".to_string()];
                v.extend(packages.clone());
                if *frozen_lockfile {
                    v.push("--frozen-lockfile".to_string());
                }
                v
            }
            BunCommand::PackageRemove { packages } => {
                let mut v = vec!["package".to_string(), "remove".to_string()];
                v.extend(packages.clone());
                v
            }
        }
    }
}

/// Configuration for a single invocation of the harness.
#[derive(Debug, Clone)]
pub struct BunInvocation {
    pub command: BunCommand,
    pub cwd: Option<PathBuf>,
    pub session_id: Option<String>,
    pub timeout: Option<f64>,
}

/// Result of driving the harness: the collected events + final status.
#[derive(Debug)]
pub struct BunRun {
    pub events: Vec<BunEvent>,
    pub outcomes: Vec<BunOutcome>,
    pub exit_code: Option<i32>,
    pub session_id: Option<String>,
}

/// Low-level handle for streaming live events from a running harness invocation.
pub struct BunStream {
    pub rx: mpsc::UnboundedReceiver<BunEventOrOutcome>,
    /// Wrapped in Arc<Mutex> so background waiter tasks can call wait() while
    /// the stream is still held by callers (e.g. TUI Jobs panel).
    pub child: std::sync::Arc<tokio::sync::Mutex<tokio::process::Child>>,
    pub session_id: Option<String>,
}

/// Find a Python interpreter (prefers python3, then python).
async fn find_python() -> Result<PathBuf> {
    for candidate in ["python3", "python"] {
        if let Ok(path) = which::which(candidate) {
            return Ok(path);
        }
    }
    Err(anyhow!(
        "python3 / python not found in PATH. The Bun harness requires Python 3."
    ))
}

/// Locate the thump (formerly cli_anything_bun) Python package root.
/// In development we expect it next to the api-anything directory.
/// Can be overridden with `THUMP_BUN_ROOT` (preferred) or legacy `CLI_ANYTHING_BUN_ROOT`.
fn find_bun_harness_root() -> Result<PathBuf> {
    // New canonical env var
    if let Ok(p) = std::env::var("THUMP_BUN_ROOT") {
        let path = PathBuf::from(p);
        if path.join("thump").exists() || path.join("__main__.py").exists() {
            return Ok(path);
        }
    }
    // Legacy env var (BC during rename)
    if let Ok(p) = std::env::var("CLI_ANYTHING_BUN_ROOT") {
        let path = PathBuf::from(p);
        if path.join("thump").exists()
            || path.join("cli_anything_bun").exists()
            || path.join("__main__.py").exists()
        {
            return Ok(path);
        }
    }

    // Heuristic: assume we are in api-anything/ and the harness is at ../thump (new) or ../cli_anything_bun (legacy)
    let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for candidate_name in ["thump", "cli_anything_bun"] {
        let candidate = current
            .parent()
            .map(|p| p.join(candidate_name))
            .unwrap_or_else(|| PathBuf::from(format!("../{}", candidate_name)));

        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(anyhow!(
        "Could not locate thump Python package (looked for thump/ and legacy cli_anything_bun/). \
         Set THUMP_BUN_ROOT or run from the API workspace root."
    ))
}

/// Spawn the Python Bun harness and return a live streaming channel + child handle.
pub async fn spawn_bun(inv: BunInvocation) -> Result<BunStream> {
    let python = find_python().await?;
    let harness_root = find_bun_harness_root()?;

    let mut cmd = Command::new(&python);
    cmd.arg("-m")
        .arg("thump")
        .args(inv.command.to_args())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped()) // We may surface harness stderr later
        .stdin(Stdio::null());

    if let Some(cwd) = &inv.cwd {
        cmd.current_dir(cwd);
    }

    // Make the Python package importable
    let mut env = std::env::vars().collect::<std::collections::HashMap<_, _>>();
    let python_path = env.get("PYTHONPATH").cloned().unwrap_or_default();
    let new_pp = if python_path.is_empty() {
        harness_root.to_string_lossy().to_string()
    } else {
        format!("{}:{}", harness_root.display(), python_path)
    };
    env.insert("PYTHONPATH".to_string(), new_pp);
    env.insert("THUMP_PARENT_ACTIVE".to_string(), "1".to_string());
    cmd.envs(env);

    // Pass through correlation + options via CLI flags
    if let Some(sid) = &inv.session_id {
        cmd.arg("--session-id").arg(sid);
    }
    if let Some(cwd) = &inv.cwd {
        cmd.arg("--cwd").arg(cwd);
    }
    if let Some(t) = inv.timeout {
        cmd.arg("--timeout").arg(t.to_string());
    }

    let mut child = cmd.spawn().context("failed to spawn Python Bun harness")?;

    let stdout = child.stdout.take().context("missing stdout")?;
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    let (tx, rx) = mpsc::unbounded_channel();

    // Wrap child so background tasks (or future native waiters) can wait on it
    let child = std::sync::Arc::new(tokio::sync::Mutex::new(child));

    // Background task that pumps NDJSON lines into the channel
    tokio::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            match BunEventOrOutcome::from_json_line(&line) {
                Ok(item) => {
                    let _ = tx.send(item);
                }
                Err(e) => {
                    // Malformed line — still forward it as a raw error event for visibility
                    eprintln!("[bun-harness] failed to parse line: {} ({})", line, e);
                }
            }
        }
    });

    Ok(BunStream {
        rx,
        child,
        session_id: inv.session_id,
    })
}

/// Convenience: run a command to completion and collect everything.
pub async fn run_bun_command(inv: BunInvocation) -> Result<BunRun> {
    let mut stream = spawn_bun(inv.clone()).await?;
    let mut events = Vec::new();
    let mut outcomes = Vec::new();

    while let Some(item) = stream.rx.recv().await {
        match item {
            BunEventOrOutcome::Event(ev) => events.push(ev),
            BunEventOrOutcome::Outcome(out) => outcomes.push(out),
        }
    }

    let status = stream.child.lock().await.wait().await?;
    let exit_code = status.code();

    Ok(BunRun {
        events,
        outcomes,
        exit_code,
        session_id: inv.session_id,
    })
}
