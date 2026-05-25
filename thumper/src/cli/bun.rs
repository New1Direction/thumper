//! CLI handler for the `bun` subcommand family.
//!
//! Wires the Bun execution layer to both:
//! - Headless CLI usage (`thump bun script run dev`) — any alias works
//! - TUI Jobs panel (via optional GenUpdate channel)
//!
//! All execution goes through the smart selector in `bun::execution` which
//! prefers the native Rust runner and falls back to the Python harness only
//! on real discovery/spawn failure.

use crate::bun::BunEventOrOutcome;
use crate::bun::{spawn_bun, BunCommand, BunInvocation};

use anyhow::Result;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::task;

/// The update type already used by the TUI Jobs system and python_bridge.
/// We reuse it here so Bun jobs appear in the exact same panel.
pub use crate::tui::GenUpdate; // re-export for convenience in main

/// Run a Bun command from the CLI (headless or TUI-aware).
///
/// If `progress_tx` is Some, live events are forwarded as `GenUpdate` so the
/// TUI Jobs panel lights up. Otherwise we print a compact NDJSON or summary.
pub async fn run(
    bun_cmd: crate::cli::definition::BunCommands,
    cwd: Option<PathBuf>,
    session_id: Option<String>,
    progress_tx: Option<mpsc::UnboundedSender<GenUpdate>>,
) -> Result<()> {
    let command = match bun_cmd {
        crate::cli::definition::BunCommands::Script { command } => match command {
            crate::cli::definition::BunScriptCommands::Run { name, args } => {
                BunCommand::ScriptRun { name, args }
            }
        },
        crate::cli::definition::BunCommands::Package { command } => match command {
            crate::cli::definition::BunPackageCommands::Add {
                packages,
                dev,
                exact,
                peer,
                optional,
            } => BunCommand::PackageAdd {
                packages,
                dev,
                exact,
                peer,
                optional,
            },
            crate::cli::definition::BunPackageCommands::Install {
                packages,
                frozen_lockfile,
            } => BunCommand::PackageInstall {
                packages,
                frozen_lockfile,
            },
            crate::cli::definition::BunPackageCommands::Remove { packages } => {
                BunCommand::PackageRemove { packages }
            }
        },
    };

    let tool_label = match &command {
        BunCommand::ScriptRun { name, .. } => format!("bun:script:{}", name),
        BunCommand::PackageAdd { packages, .. } => {
            format!("bun:package:add:{}", packages.join("+"))
        }
        BunCommand::PackageInstall { packages, .. } => {
            if packages.is_empty() {
                "bun:package:install".to_string()
            } else {
                format!("bun:package:install:{}", packages.join("+"))
            }
        }
        BunCommand::PackageRemove { packages } => {
            format!("bun:package:remove:{}", packages.join("+"))
        }
    };

    let inv = BunInvocation {
        command,
        cwd,
        session_id: session_id.clone(),
        timeout: None,
    };

    if let Some(tx) = progress_tx {
        // TUI mode: stream events into the Jobs panel
        spawn_bun_job(tool_label, inv, tx).await
    } else {
        // Headless CLI mode
        run_headless(inv, &tool_label).await
    }
}

/// Background task that drives the Bun harness and forwards events to the TUI Jobs channel.
/// Mirrors the pattern used in `tui/app.rs` for generation and in `python_bridge`.
async fn spawn_bun_job(
    tool: String,
    inv: BunInvocation,
    tx: mpsc::UnboundedSender<GenUpdate>,
) -> Result<()> {
    // Announce the job
    let _ = tx.send(GenUpdate::Progress {
        tool: tool.clone(),
        stage: "starting".to_string(),
        pct: 5,
        msg: format!("Starting Bun operation: {}", tool),
    });

    // Spawn the actual streaming work so we don't block the UI thread
    let tool_for_task = tool.clone();
    let tx_for_task = tx.clone();

    task::spawn(async move {
        match spawn_bun(inv).await {
            Ok(mut stream) => {
                // Rolling window: keep only the last 4 stderr lines for diagnostics
                let mut stderr_window: std::collections::VecDeque<String> =
                    std::collections::VecDeque::with_capacity(4);

                while let Some(item) = stream.rx.recv().await {
                    match item {
                        BunEventOrOutcome::Event(ev) => {
                            // Capture stderr lines into the rolling window
                            if ev.event.ends_with(".stderr") {
                                if let Some(raw) = ev.data.get("raw").and_then(|v| v.as_str()) {
                                    if stderr_window.len() == 4 {
                                        stderr_window.pop_front();
                                    }
                                    stderr_window.push_back(raw.to_string());
                                }
                            }

                            // Forward the *full* rich native event name (bun.native.<verb>.<stream>)
                            // and now also the structured telemetry from Chunk 10 parser.
                            let stage = ev.event.clone();

                            let raw = ev
                                .data
                                .get("raw")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .trim()
                                .to_string();

                            // Use real percent from the native BunOutputParser when available (Chunk 11)
                            let pct = ev
                                .data
                                .get("percent")
                                .and_then(|v| v.as_u64())
                                .map(|p| p as u8)
                                .unwrap_or_else(|| {
                                    if ev.event.contains("started") {
                                        20
                                    } else if ev.event.contains("exited") {
                                        90
                                    } else {
                                        50
                                    }
                                });

                            // Include speed in the displayed msg when present (visible in jobs + footer)
                            let msg =
                                if let Some(sp) = ev.data.get("speed").and_then(|v| v.as_str()) {
                                    format!("{} [{}]", raw, sp)
                                } else {
                                    raw
                                };

                            let _ = tx_for_task.send(GenUpdate::Progress {
                                tool: tool_for_task.clone(),
                                stage,
                                pct,
                                msg: msg.chars().take(120).collect::<String>(), // keep UI clean
                            });
                        }
                        BunEventOrOutcome::Outcome(outcome) => {
                            if outcome.ok {
                                let _ = tx_for_task.send(GenUpdate::Done {
                                    tool: tool_for_task.clone(),
                                    path: format!("Bun op succeeded (op={})", outcome.operation),
                                });
                            } else {
                                // Real exit_code from native waiter (or Python harness)
                                let code = outcome
                                    .data
                                    .get("exit_code")
                                    .and_then(|v| v.as_i64())
                                    .map(|c| format!(" (exit {})", c))
                                    .unwrap_or_default();
                                let err = format!("Bun operation failed{}", code);

                                // Drain the last 4 stderr lines into the diagnostics payload
                                let diagnostics: Vec<String> = stderr_window.drain(..).collect();

                                let _ = tx_for_task.send(GenUpdate::Error {
                                    tool: tool_for_task.clone(),
                                    err,
                                    diagnostics,
                                });
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let _ = tx_for_task.send(GenUpdate::Error {
                    tool: tool_for_task,
                    err: e.to_string(),
                    diagnostics: vec![],
                });
            }
        }
    });

    Ok(())
}

/// Headless execution: prints a compact view of the event stream.
async fn run_headless(inv: BunInvocation, label: &str) -> Result<()> {
    println!("[bun] Starting {} (session={:?})", label, inv.session_id);

    let mut stream = spawn_bun(inv).await?;

    while let Some(item) = stream.rx.recv().await {
        match item {
            BunEventOrOutcome::Event(ev) => {
                if ev.event.contains("stdout") || ev.event.contains("stderr") {
                    if let Some(raw) = ev.data.get("raw").and_then(|v| v.as_str()) {
                        print!("{}", raw);
                    }
                } else {
                    println!("[{}] {}", ev.level.as_str(), ev.event);
                }
            }
            BunEventOrOutcome::Outcome(outcome) => {
                if outcome.ok {
                    println!(
                        "\n✅ Bun operation completed successfully ({}).",
                        outcome.operation
                    );
                } else {
                    eprintln!("\n❌ Bun operation failed: {:?}", outcome.data);
                }
            }
        }
    }

    println!("[bun] Harness finished for {}", label);
    Ok(())
}

/// Simple demo entry point used by the TUI to trigger a visible Bun job.
/// Runs "package install" (harmless, produces rich live output) and streams
/// updates through the GenUpdate channel so the Jobs panel lights up in real time.
#[allow(dead_code)]
pub async fn run_bun_demo(
    progress_tx: Option<mpsc::UnboundedSender<GenUpdate>>,
) -> anyhow::Result<()> {
    let cmd = crate::cli::definition::BunCommands::Package {
        command: crate::cli::definition::BunPackageCommands::Install {
            packages: vec![],
            frozen_lockfile: false,
        },
    };
    run(cmd, None, None, progress_tx).await
}
