# Native Bun Runner — Design Sketch (Phase 1)

**Status**: Proposed — for review before implementation  
**Date**: 2026-05  
**Owner**: Grok (following palette rhythm)  
**Goal**: Give the TUI + CLI a fast, zero-Python path for executing real `bun` commands while keeping full backward compatibility with the existing Python semantic harness.

---

## 1. Motivation & Scope

Current state (palette + `b` key + Jobs panel):
- All Bun execution goes through `cli::bun::run(...)` → `bun::harness::spawn_bun(...)`
- `spawn_bun` (legacy path) shells out to `python -m thump` (the smart proxy that prefers native)
- This works beautifully but adds Python + harness startup cost and a hard dependency.

**This iteration (Phase 1)** targets the **common fast path** used by the command palette and context-aware `b` key:
- `bun add`, `bun install`, `bun remove`
- `bun run <script>`

Higher-level semantic features (rich script discovery, certain safety heuristics, advanced package metadata) can stay on the Python path for now.

**Non-goals for Phase 1**:
- Full feature parity with the Python harness
- Replacing the Python harness entirely
- Handling every exotic flag the Python layer supports

---

## 2. High-Level Architecture

```
BunCommands (from palette / CLI / ACP)
          │
          ▼
   cli::bun::run()
          │
          ├─► Native path (new)  ──► spawn_native_bun(...) ──► real `bun` binary
          │                                         │
          │                                         ▼
          │                                   NDJSON / line parser
          │                                         │
          │                                         ▼
          │                                   BunEvent / BunOutcome
          │
          └─► Python path (thump proxy) ──► spawn_bun(...) ──► python -m thump (auto-promotes to native)
```

Selection strategy (runtime, no compile-time flag required initially):
1. Try native first (if `bun` found in PATH / standard locations).
2. On any hard failure or unknown command shape, fall back to Python harness.
3. Expose `--no-native-bun` / `BUN_NATIVE=0` escape hatch for debugging.

This gives us the "just works" story while the native path matures.

---

## 3. Binary Discovery

New helper (modeled exactly on `find_python`):

```rust
async fn find_bun_binary() -> Result<PathBuf> {
    // 1. BUN_INSTALL / ~/.bun/bin / /usr/local/bin / PATH
    // 2. which::which("bun")
    // 3. Common macOS/Linux/Windows locations
}
```

Store the discovered path in the `BunInvocation` or a small `BunRuntime` struct.

---

## 4. Subprocess & Streaming Model

Use the same Tokio primitives already in `harness.rs`:

- `tokio::process::Command`
- Capture `stdout` + `stderr` (both are important — Bun mixes progress on stderr)
- Stream line-by-line with `AsyncBufReadExt`
- Feed into the **exact same** `mpsc::UnboundedReceiver<BunEventOrOutcome>` channel that `spawn_bun_job` already consumes.

This means **zero changes** to:
- `cli/bun.rs::spawn_bun_job`
- TUI Jobs panel rendering
- Palette execution path
- ACP tool call handlers

---

## 5. Output Parsing Strategy (The Hard Part)

Bun does **not** emit the same NDJSON envelope as our Python harness.

**Phase 1 approach (pragmatic & robust)**:

- Run `bun <subcommand> ...` with `--json` where supported (`bun run` has limited JSON, `bun install`/`add` are mostly textual).
- For progress: treat every line as a `BunEvent` with:
  - `event: "bun.<verb>.stdout"` or `"bun.<verb>.stderr"`
  - `data.raw = line`
  - `level = Info | Warn | Error` (heuristic on content)
- On exit, synthesize a final `BunOutcome`:
  - `ok = status.success()`
  - `operation = "script.run" | "package.add" | ...`
  - `data.returncode`, `data.duration` (if we measure it)

Later iterations can:
- Parse Bun's structured JSON output more deeply
- Detect "Saved lockfile", "Resolving packages", "Checked" etc. and emit richer typed events matching the Python contract.

The key invariant: anything that flows through `BunEventOrOutcome` must continue to produce valid `GenUpdate` messages for the Jobs panel.

---

## 6. Command Mapping

| `BunCommands` variant       | Native `bun` invocation          | Notes                              |
|-----------------------------|----------------------------------|------------------------------------|
| `Package::Add`              | `bun add [pkgs] [--dev ...]`     | Excellent native support           |
| `Package::Install`          | `bun install [--frozen-lockfile]`| Core strength of Bun               |
| `Package::Remove`           | `bun remove [pkgs]`              | Straightforward                    |
| `Script::Run`               | `bun run <name> [args]`          | Most common palette use case       |

Unknown or future variants fall back to Python.

---

## 7. Error Model & Fallback

- Any spawn failure for the native `bun` binary → immediate fallback to Python (with a debug log).
- Non-zero exit from `bun` → emit `GenUpdate::Error` (same as today) + final `BunOutcome { ok: false }`.
- Malformed output lines → still forward as raw events (current behavior in `spawn_bun`).

This keeps the TUI resilient even while the native parser is being hardened.

---

## 8. File / Module Layout

Proposed:

```
src/bun/
    mod.rs
    events.rs          (unchanged)
    harness.rs         (keep existing Python path)
    native.rs          (NEW — discovery + spawn_native + line→event translator)
```

Or, for minimal diff, put the native logic inside `harness.rs` behind a `spawn_bun_native` function and keep the public `spawn_bun` as the selector.

Recommendation: start with `src/bun/native.rs` for clarity, then consider merging later.

Public surface added:
- `pub async fn spawn_bun_native(inv: BunInvocation) -> Result<BunStream>`
- `pub async fn find_bun() -> Result<PathBuf>`

---

## 9. Feature Flag & Rollout

No new Cargo feature required in Phase 1 (runtime auto-detect is friendlier).

Future (Phase 2+):
- `native-bun` feature that removes the Python dependency at compile time for minimal containers.
- `python-bun` feature (default off when native is mature).

For now: pure runtime selection + `BUN_PREFER_PYTHON=1` env var as an escape hatch.

---

## 10. Risks & Open Questions

- **Output parsing quality** — Bun's human output is excellent but not machine-stable. How much heuristic magic is acceptable in v1?
- **Interactive prompts** — `bun add` can ask questions. The Python harness has some handling; native path must at least not hang.
- **Windows support** — `bun` on Windows is first-class, but path discovery differs.
- **Performance wins** — We should measure wall time (Python startup vs native) on first real jobs.
- **Script discovery** — The palette's Tab completion for scripts still relies on reading `package.json` directly (already done in `load_scripts_from_package_json`). Native runner does not need to duplicate that.

---

## 11. Proposed Implementation Order (after doc approval)

1. Add `find_bun()` + basic discovery tests.
2. Implement `spawn_bun_native()` that can run a simple `bun --version` and stream raw lines as events.
3. Wire a selector inside `cli::bun::run()` (or a new thin `spawn_bun_any`).
4. Test end-to-end via the palette smoke test + real `bun run` / `bun add` in a temp project.
5. Document the fallback behavior and new env vars.

---

**Ready for review.**

Please reply with:
- "Approved — implement" (or with specific changes)
- Any scope adjustments
- Preference on module layout (`native.rs` vs inside `harness.rs`)

Once we have the green light, we will execute with the same incremental, reviewable rhythm used for the command palette.