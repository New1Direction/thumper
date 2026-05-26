//! Smart execution selector for Bun commands.
//!
//! This module is the single source of truth for deciding *how* a `BunCommand`
//! gets executed inside api-anything.
//!
//! Policy (as of Chunk 12):
//! - Prefer the pure-Rust native runner (`native::spawn_bun_native`) for
//!   speed and zero Python dependency.
//! - On **any** failure to even start the native path (binary not found,
//!   spawn error, permission issues, etc.), transparently fall back to the
//!   battle-tested (and now smart) Python `thump` harness.
//! - Never swallow real runtime failures from a successfully started native
//!   process (those are reported via `BunOutcome` with the real exit code).
//!
//! All call sites (TUI palette, `b` key, `cli::bun`, future absorb/generation
//! flows, ACP handlers) are expected to go through `spawn_bun` so they
//! automatically get the best available execution strategy.

use crate::bun::harness;
use crate::bun::harness::BunInvocation;
use crate::bun::native;

pub use crate::bun::harness::BunStream;

/// The canonical entry point for running any Bun command.
///
/// This function implements the "native first, Python fallback" policy.
/// It is intentionally a thin wrapper so the signature and semantics stay
/// stable even as we expand native capabilities in future chunks.
///
/// The returned `BunStream` owns the child process. Dropping it without
/// reading (e.g. `spawn_bun(inv).await.ok();`) leaks the process and its
/// stdio — hence the `#[must_use]` lint.
#[must_use = "await on the returned BunStream or spawn a handler task; dropping it leaks the child process"]
pub async fn spawn_bun(inv: BunInvocation) -> anyhow::Result<BunStream> {
    // Fast path: try the pure-Rust native Bun runner first.
    // We clone the invocation because the native path takes ownership and
    // we want to be able to fall back on any error.
    if let Ok(stream) = native::spawn_bun_native(inv.clone()).await {
        return Ok(stream);
    }

    // Transparent fallback to the Python harness.
    // This path is only taken on genuine discovery or spawn failure.
    // Real execution errors (non-zero exit, etc.) will still come through
    // the native path once it successfully starts.
    harness::spawn_bun(inv).await
}

/// Direct access to the Python-only implementation.
/// Useful for debugging, forcing old behaviour, or very specific fallbacks.
pub use harness::spawn_bun as spawn_bun_python;
