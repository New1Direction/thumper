//! Bun execution layer.
//!
//! This module is the central hub for everything related to running Bun
//! commands from within Thumper (thump).
//!
//! It re-exports the stable public surface (`BunCommand`, `BunInvocation`,
//! events, etc.) and provides the canonical smart entry point `spawn_bun`
//! that all higher layers should use.

pub mod discovery;
pub mod events;
pub mod execution;
pub mod harness;
pub mod native;
pub mod parent;

pub use discovery::find_bun;
pub use events::{BunEvent, BunEventOrOutcome, BunOutcome, EventLevel};
pub use execution::{spawn_bun, spawn_bun_python};
pub use harness::{run_bun_command, BunCommand, BunInvocation, BunStream};
pub use parent::{get_ancestry_diagnostics, get_ancestry_report, is_launched_by_thumper};
