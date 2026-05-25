//! Structured output helpers for headless + agent use.
//! Every command that produces data should go through here for --json consistency.

use anyhow::Result;
use serde::Serialize;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use tracing;

/// Guard so that ACP stdio server (which owns stdout for JSON-RPC wire protocol)
/// can suppress all progress/JSON emissions that would otherwise corrupt the
/// protocol stream. Set to true at the start of `agent stdio`.
static IN_ACP_MODE: AtomicBool = AtomicBool::new(false);

/// Called by the ACP server to take exclusive ownership of stdout.
pub(crate) fn set_acp_mode(enabled: bool) {
    IN_ACP_MODE.store(enabled, Ordering::SeqCst);
}

/// Print a serializable value as pretty JSON + newline to stdout.
/// Used by `--json` paths and ACP streaming adapters.
pub fn print_json<T: Serialize>(value: &T) -> Result<()> {
    if IN_ACP_MODE.load(Ordering::SeqCst) {
        // ACP owns stdout; route any diagnostic via tracing instead (caller
        // should have used tracing for progress inside ACP handlers).
        tracing::debug!(
            "(acp) suppressed print_json: {:?}",
            serde_json::to_value(value).ok()
        );
        return Ok(());
    }
    let mut stdout = io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, value)?;
    writeln!(stdout)?;
    stdout.flush()?;
    Ok(())
}

/// Streaming event for long-running operations (generate, absorb, validate).
/// Emitted as newline-delimited JSON when `--stream` or output-format=streaming-json.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum StreamEvent {
    Progress {
        stage: String,
        pct: u8,
        message: Option<String>,
    },
    Artifact {
        path: String,
        kind: String,
        size: Option<u64>,
    },
    #[allow(dead_code)]
    Thought { text: String },
    #[allow(dead_code)]
    Warning { message: String },
    End {
        status: String,
        id: Option<String>,
        duration_ms: Option<u64>,
    },
}

pub fn emit_stream(event: &StreamEvent) -> Result<()> {
    if IN_ACP_MODE.load(Ordering::SeqCst) {
        tracing::debug!("(acp) suppressed stream event: {:?}", event);
        return Ok(());
    }
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, event)?;
    writeln!(stdout)?;
    stdout.flush()?;
    Ok(())
}
