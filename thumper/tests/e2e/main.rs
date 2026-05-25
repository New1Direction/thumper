//! End-to-end tests for the `api-anything` binary (TUI + CLI/JSON/streaming).
//!
//! These tests launch the real compiled binary (debug or release) via subprocess
//! and, for TUI scenarios, drive it inside an isolated tmux session.
//!
//! # Running
//!
//! ```sh
//! cargo test --test e2e
//! cargo test --test e2e -- --nocapture
//! ```
//!
//! - TUI tests are skipped automatically when tmux is not installed.
//! - `test_generate_json` and `test_streaming` are skipped when the RedMicro
//!   generator tree is not discoverable (set REDMICRO_ROOT to force).
//!
//! This harness is modeled directly on agent-of-empires/tests/e2e/harness.rs.

mod harness;
