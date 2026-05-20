# AGENTS.md — API Anything Development Rules

> This file governs all AI-assisted work on the `api-anything` crate (the native Rust "get an API from anything" TUI + CLI + ACP binary).

## Core Principles (inherited from agent-of-empires + RedMicro rigor)
1. **No dead code.** Every module, function, and type must be used. Run `cargo clippy -- -D warnings` and `cargo build` clean before committing.
2. **Explicit wiring.** No magic macros or hidden registration. Every CLI verb, TUI view, ACP handler, and generator backend must be listed in `cli/definition.rs`, `tui/home/mod.rs`, or equivalent and wired in `main.rs` or `App::new`.
3. **Artifact discipline.** When using `/implement` or any multi-round agent loop on this crate, thread the exact same `/tmp/grok-impl-*` + review files. Never invent new paths mid-loop.
4. **Real backends first.** The python bridge (`api_wrapper_generator.py`, `absorb.py`) and real external tools are the source of truth. Do not re-implement their logic in Rust until a pure-Rust path is explicitly requested and the python version is the reference.
5. **Test before expand.** Every new generator engine, ACP handler, or TUI component gets a `TEST.md` written first (plan) then tests that invoke the *installed* binary via subprocess where possible (see `tests/e2e/harness.rs`).
6. **Feature flags for size.** `tui`, `acp`, `serve`, `python-bridge` are additive. The default/minimal build must remain useful for headless agent use in containers.

## Project Layout (do not deviate without updating this file)
```
src/
  main.rs                 # entry + dispatch only (keep tiny)
  cli/
    definition.rs         # single source of truth for clap surface
    output.rs             # print_json + streaming helpers (used everywhere)
    generate.rs ...       # one file per subcommand
  tui/                    # gated behind "tui" feature
  acp/                    # gated behind "acp" feature
  generator/              # core logic + python_bridge.rs (subprocess + json)
  registry/               # models + store (serde + fs)
  config.rs logging.rs
```

## Adding a New CLI Verb
1. Add variant to `Commands` enum in `cli/definition.rs`.
2. Create `cli/<verb>.rs` with `pub fn run(...)`.
3. Wire the match arm in `main.rs` (or a `cli::dispatch`).
4. Add `--json` support via `output::print_json` if it produces structured data.
5. Update `docs/cli/reference.md` (or run `cargo xtask gen-docs` when it exists).
6. Add a one-line example to `README.md` and a test in `tests/e2e/`.

## Adding a TUI View or Dialog
- Follow the aoe pattern: `home/`, `components/`, `dialogs/`.
- Input handling returns `Option<Action>`; execution happens in `app.rs`.
- Never block the event loop. Long work goes through tokio channels or a `Poller` struct.
- Theme tokens live in `tui/styles/`. Use the warm industrial palette (amber/copper) unless user requests otherwise.

## ACP / IDE Protocol
- Use the exact `agent-client-protocol` + `-tokio` crates and versions that `agent-of-empires` pins.
- All extension methods live under `x.ai/api-anything/*`.
- Streaming must use the same `session/update` + custom event shapes that the Grok ACP docs and aoe cockpit use.
- Test stdio mode with the same harnesses aoe uses for cockpit smoke tests.

## Python Bridge & Generator
- `generator/python_bridge.rs` is the **only** place that shells out to RedMicro python.
- Always pass absolute paths, capture stdout/stderr, surface structured JSON errors.
- Fallback: if bridge fails or `--no-python`, use the minimal native `engine.rs` templates.
- Never hard-code paths to the user's RedMicro checkout; discover via `which python` + known relative locations or config.

## Testing Rules
- `tests/e2e/harness.rs` must be able to launch the debug + release binary, send keys to the TUI, capture screens, and assert on JSON output of CLI commands.
- Real tool absorption tests (point at `rustscan`, a local binary, or one of the RedMicro examples) are required for generator changes.
- Serial tests for anything that touches the terminal or global registry dir (use `#[serial]` from `serial_test`).
- `cargo test --all-features` must pass.

## Documentation & Polish
- `docs/cli/reference.md` is generated from clap — do not hand-edit.
- Every public command and ACP method must have an agent-usable example in the SKILL.md that will live under `skills/api-anything/SKILL.md` (future).
- `doctor` subcommand must detect: rustc/cargo, python3 + key RedMicro scripts, writable registry dir, template dir.

## Git / Commits
- Conventional commits or clear "feat(cli): add registry list --json".
- Never commit `Cargo.lock` changes that are only dep updates unless intentional.
- Run `cargo fmt && cargo clippy -- -D warnings` before push.

## When in Doubt
- Read the equivalent file in `agent-of-empires/src/...` first — it is the reference implementation for "lightweight but complete Rust TUI + ACP + CLI + serve".
- Escalate to the user via `ask_user_question` on architectural choices (new output lang, major registry schema change, etc.).
- For `/implement` runs on this crate itself, use effort ≥2 and include the security + tests specialists when touching generators or ACP.

This file is the single source of truth for the AI working on api-anything. Update it when layout or rules change.

---
*Initial version created during Phase 0 scaffolding. Adapted from agent-of-empires/AGENTS.md + RedMicro rigor + CLI-Anything test philosophy.*