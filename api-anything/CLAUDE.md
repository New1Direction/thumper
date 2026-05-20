# CLAUDE.md — API Anything (for Claude / Grok / other agents working in this tree)

This is a **Rust** project (see `AGENTS.md` for the authoritative rules).

## Quick Orientation
- Binary name: `api-anything`
- The goal: native, fast, TUI + headless + ACP implementation of "get an API/harness from anything"
- Primary reference implementation for architecture: `agent-of-empires` (sibling or nearby checkout)
- Python bridge lives in RedMicro `supporting-tools/api-harness/`

## Before You Edit
1. Read `AGENTS.md` (the real rules).
2. Run `cargo check --all-features` and `cargo clippy -- -D warnings`.
3. If touching CLI surface, also look at `src/cli/definition.rs`.
4. If adding TUI, look at how aoe does `tui/app.rs` + `home/`.

## Common Commands
```bash
cargo build --release
cargo run -- tui                 # or just `cargo run`
cargo run -- generate rustscan --json
cargo test --all-features
```

## Python Bridge
The crate shells out to the RedMicro python tools by default. Make sure the path resolution in `generator/python_bridge.rs` is correct for the current machine.

## ACP Testing
`api-anything agent stdio` speaks the real Agent Client Protocol. Use the same test clients that aoe's cockpit tests use.

## Do Not
- Invent new magic registration systems.
- Bypass the explicit match arms in main / dispatch.
- Commit large generated artifacts or lockfile noise.

Follow the "thread the exact same artifact file paths" rule when the user runs `/implement` on this crate.

See `AGENTS.md` for the full contract.