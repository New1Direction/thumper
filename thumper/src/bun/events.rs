//! Stable event types for the Bun harness NDJSON protocol.
//!
//! These types are intentionally kept close to the Python `cli_anything_bun/events.py`
//! contract so the two sides evolve together with minimal friction.
//!
//! See docs/bun-harness-design.md (when present) for the full rationale.

use serde::{Deserialize, Serialize};

/// The core event envelope emitted by the Python harness for every intermediate step.
/// Final results are emitted separately via `emit_result` (plain objects without the envelope).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BunEvent {
    /// ISO-8601 timestamp (with timezone)
    pub ts: String,

    /// Event name, e.g. "script.run.started", "package.add.process.stdout", "process.exited"
    pub event: String,

    /// Schema version for forward compatibility. Currently always "1".
    #[serde(default = "default_schema_version")]
    pub schema_version: String,

    /// Optional correlation ID for a whole user session / job
    #[serde(default)]
    pub session_id: Option<String>,

    /// Optional correlation ID for a single finite operation (package.add, script.run, etc.)
    #[serde(default)]
    pub op_id: Option<String>,

    /// Severity level
    pub level: EventLevel,

    /// Arbitrary payload. Consumers should treat unknown fields gracefully.
    #[serde(default)]
    pub data: serde_json::Value,
}

fn default_schema_version() -> String {
    "1".to_string()
}

/// Log level used by the harness (matches Python EventLevel).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl EventLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventLevel::Debug => "debug",
            EventLevel::Info => "info",
            EventLevel::Warn => "warn",
            EventLevel::Error => "error",
        }
    }
}

/// A final structured outcome emitted by the harness (via `emit_result`).
/// These lines do **not** have the `event` / `schema_version` envelope fields.
/// They always contain at least `ok` and `operation`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BunOutcome {
    /// Whether the operation succeeded from the harness's point of view
    pub ok: bool,

    /// Which high-level operation produced this result (e.g. "script.run", "package.add")
    pub operation: String,

    /// The rest of the payload (returncode, duration, packages, error, etc.)
    #[serde(flatten)]
    pub data: serde_json::Value,
}

/// Either a live event or a final outcome.
/// This is the item type you get when streaming from the harness.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BunEventOrOutcome {
    Event(BunEvent),
    Outcome(BunOutcome),
}

impl BunEventOrOutcome {
    /// Try to interpret the line as an event first, then as an outcome.
    pub fn from_json_line(line: &str) -> Result<Self, serde_json::Error> {
        // Prefer the event shape (has "event" field)
        if let Ok(ev) = serde_json::from_str::<BunEvent>(line) {
            if !ev.event.is_empty() {
                return Ok(BunEventOrOutcome::Event(ev));
            }
        }
        // Fall back to plain outcome / result shape
        let outcome: BunOutcome = serde_json::from_str(line)?;
        Ok(BunEventOrOutcome::Outcome(outcome))
    }
}
