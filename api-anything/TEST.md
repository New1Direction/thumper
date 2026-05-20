# TEST.md — Generator & CLI Surface

Written before heavy implementation (Phase 1/2).

## Goals
- Every `generate` path must emit the exact `GenerateResult` + `StreamEvent` contract defined in `src/cli/output.rs` and `src/cli/generate.rs`.
- The python bridge must be exercised in E2E tests when the RedMicro tree is present.
- `api-anything` binary (debug + release) must be testable via subprocess from the e2e harness.
- 100% of the clap surface documented and round-trippable via `--help` + JSON.

## Unit Tests (to be added)
- `generator::python_bridge` happy path + error mapping (mocked subprocess)
- `registry::store` round-trip (temp dir)
- `cli::output` streaming NDJSON writer

## E2E Harness (tests/e2e/harness.rs)
The harness (`TuiTestHarness`) was created in `tests/e2e/harness.rs` (plus `tests/e2e/main.rs` for the `cargo test --test e2e` binary) using the exact patterns from `agent-of-empires/tests/e2e/harness.rs`:

- Isolated `TempDir` as `$HOME` / `XDG_CONFIG_HOME`
- `run_cli(&["generate", "rustscan", "--json", "-o", ...])` for subprocess invocation (captures Output, stdout is clean JSON/NDJSON)
- tmux-backed TUI driver (`spawn_tui`, `send_keys("q")`, `capture_screen`, `wait_for`, `wait_for_exit`, `assert_screen_contains`, Drop cleanup) — chosen over raw portable-pty to match the battle-tested sibling harness and avoid new dependencies
- All TUI/terminal tests are `#[serial]` via the already-present `serial_test` crate
- Automatic skips: `require_tmux!()` for TUI tests; `redmicro_available()` helper for generate tests (no hard failure when the external RedMicro tree is absent)

Implemented tests (all live in harness.rs for a minimal foundation):
- `test_generate_json` — runs `api-anything generate rustscan --json -o <temp>` , parses the `GenerateResult` schema (id/name/status/artifacts/duration_ms/output_dir), asserts a real on-disk file exists for each reported artifact.
- `test_streaming` — runs with `--stream`, validates every stdout line is NDJSON with a `"type"` field (Progress/Artifact/End etc.), last event is `{"type":"end","status":"ok"}`.
- `test_tui_basic` (stretch goal completed) — `spawn_tui()`, waits for "API ANYTHING" + "Registry" + "q quit", sends `q`, asserts the tmux session exits cleanly and is no longer alive.

Wiring:
- `cargo test --test e2e` (or `cargo test --test e2e -- --nocapture` for screen dumps on TUI failures)
- `cargo test --test e2e -- generate` to run only the generate/stream tests.

## Integration with RedMicro
When `python-bridge` feature (or auto-detect) is active, the E2E must be able to invoke the real `api_wrapper_generator.py` and produce a working FastAPI server that passes `uvicorn --check` or import.

## Current Status (Phase 1 → Phase 2)
- [x] JSON contract for generate (stub + real)
- [x] Streaming event contract (NDJSON `StreamEvent`)
- [x] Real bridge call (exercised by E2E when RedMicro present)
- [x] E2E harness file (`tests/e2e/harness.rs` + `main.rs`)
- [x] Schema validation of output + on-disk artifact verification
- [x] `test_generate_json`, `test_streaming`, `test_tui_basic` all implemented and serial-safe

Run with:
```bash
cargo test --test e2e                 # all E2E (skips gracefully without tmux/RedMicro)
cargo test --test e2e -- --nocapture  # live screen dumps on TUI failures
cargo test --test e2e generate        # just the JSON + streaming tests
cargo run -- generate rustscan --json | jq .
```

The harness is now the official foundation for long-term quality of the CLI, generator bridge, TUI, and future ACP/serve paths. All tests follow the exact same idioms as the excellent agent-of-empires harness.