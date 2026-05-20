# Native Bun Execution Layer v2

**Status**: Draft  
**Date**: 2026-05  
**Author**: Grok (based on prior native runner work + Chunks 10–11)  
**Related**: `docs/native-bun-runner.md`, Chunks 6–11 (native runner + rich parser + TUI delight wiring)

---

## 1. Motivation

The native Rust Bun execution engine (Phase 1, landed in Chunks 3–11) has proven highly successful:

- Significantly faster startup than the Python `cli_anything_bun` harness
- Zero Python dependency for the common interactive paths (palette, `b` key)
- Rich, structured telemetry (`BunStage`, `ProgressMetrics`, speed, package counts, timing) that powers excellent TUI delight (stage-aware plasma, metric ribbon, celebration, etc.)
- Clean fallback to Python when native is unavailable

However, a major gap remains: **generation and absorb flows still route all Bun commands through the Python harness**.

This means:
- Users doing `g` (generate) or `a` (absorb) on Bun-based projects still pay the Python tax.
- The rich telemetry and delight built in Chunks 10–11 are under-utilized during the most important "get an API from anything" workflows.
- We have two parallel execution paths for the same verbs (`add`, `install`, `run`, `remove`).

The goal of v2 is to make the native runner the **default** for *all* Bun command execution originating from the TUI, while keeping the system pragmatic and low-risk.

---

## 2. Goals (v2 Overall)

- **Native-first experience**: The TUI prefers the pure-Rust Bun runner for every Bun command it triggers.
- **Transparent to callers**: The `BunCommands` surface, `BunInvocation`, and event streaming contract remain stable.
- **Full telemetry**: All rich data produced by the native parser continues to flow (and improves) for generation/absorb jobs.
- **Graceful degradation**: Python `cli_anything_bun` remains a reliable fallback.
- **Incremental delivery**: We can ship value in small, reviewable phases without touching the RedMicro Python generator logic.

---

## 3. Phase 1 Scope (Minimum Lovable)

**Goal**: Make the native runner the default for every Bun command that originates from the TUI (palette, explicit `b` key, background jobs, and — crucially — commands requested during absorb/generation).

### In Scope for Phase 1

- Extend the existing smart selector (`src/bun/mod.rs`) so it is the single source of truth for *all* Bun execution inside the crate.
- Wire generation/absorb flows (via `python_bridge.rs` or the absorb path) to use the native selector instead of directly calling the Python harness for Bun verbs.
- Preserve the exact same `BunCommands` / `BunInvocation` types so the generator code requires minimal (or zero) changes.
- Continue emitting the same `BunEvent` / `BunOutcome` shapes with the enriched `data` fields from the Chunk 10 parser.
- Keep the existing event contract (`bun.native.<verb>.<stream>`, `BunOutcome`, etc.) 100% intact.
- Update documentation and the Python harness (if needed) to prefer the native path when the Rust binary is available.

### Success Criteria for Phase 1

- Running `a` (absorb) or `g` (generate) on a Bun-based project uses the native runner by default.
- All existing TUI delight (plasma stages, speed in ribbon, accurate progress, celebration, etc.) works during generation/absorb.
- No regression in correctness or fallback behavior.
- `cargo test --all-features` + manual smoke of absorb + palette pass.

---

## 4. Non-Goals / Explicitly Out of Scope for Phase 1

- Replacing or re-implementing the core RedMicro Python generator logic (`absorb.py`, `api_wrapper_generator.py`, etc.). The Python bridge remains the source of truth for *what* to generate.
- Achieving 100% flag parity with the Python `cli_anything_bun` harness (we will support the verbs and options the TUI currently uses; obscure flags can stay on the Python path for now).
- Full Windows hardening and path discovery edge cases (we will document known gaps and prioritize them in a follow-up phase).
- Changing the public CLI surface (`api-anything bun ...` subcommands) in Phase 1.
- Removing the Python dependency entirely (the generator still needs it).

These items are recorded for later phases.

---

## 5. Technical Approach (High Level)

### 5.1 Single Smart Selector

We already have a smart selector in `src/bun/mod.rs`:

```rust
pub async fn spawn_bun(inv: BunInvocation) -> Result<BunStream> {
    // tries native first, falls back to Python
}
```

Phase 1 makes this selector **the only entry point** for Bun execution from anywhere in the crate (TUI palette, `cli/bun.rs`, generator flows, future ACP handlers).

### 5.2 Integration with Generation/Absorb Flows

The Python generator scripts currently shell out to `cli_anything_bun` (the installed Python package).

Options (to be decided during implementation):

- **Preferred**: Have the Rust binary expose a stable subcommand or library interface that the Python bridge can call when it needs to run Bun (`api-anything internal run-bun ...`).
- Alternative: Make the Python `cli_anything_bun` package detect the presence of the native binary and delegate to it via subprocess (similar to how some tools prefer `uv` over `pip`).
- Keep a thin compatibility shim in Python that the generator can continue to call.

The key constraint: **the generator Python code should not need to know which implementation is being used**.

### 5.3 Telemetry & Contract

- The `BunOutputParser` (Chunk 10) already produces `ProgressMetrics` + `BunStage`.
- These are serialized into the `data` field of `BunEvent`.
- Downstream code (TUI `handle_gen_update`, job tracking, plasma widgets) already knows how to consume them (or will after small wiring in Phase 1).
- No change to `BunEvent`, `BunOutcome`, or `BunEventOrOutcome`.

### 5.4 Discovery & Fallback (unchanged from v1)

The existing `find_bun()` logic (BUN_INSTALL, `~/.bun`, `which`, common paths) remains the discovery mechanism. If it fails, we fall back to the Python harness (which itself can still find Bun).

---

## 6. Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Behavioral differences between native runner and Python harness | Phase 1 will only support the verbs/options the TUI currently exercises. Any divergence will surface as a fallback to Python. |
| Python generator scripts hard-depend on `cli_anything_bun` internals | Keep a thin delegation path; do not remove the Python package in Phase 1. |
| Windows support gaps | Explicitly document that Phase 1 targets Unix-first; Windows work is tracked separately. |
| Performance regression in generation flows | The native path is expected to be faster; we will measure and fall back if needed. |
| Event contract drift | All rich data is additive inside the existing `data: Value` field. No breaking changes. |

---

## 7. Future Phases (Rough Sketch)

- **Phase 2**: Expand verb coverage and flag support; reduce reliance on Python fallback for generation.
- **Phase 3**: Windows first-class support + better error surfacing.
- **Phase 4** (stretch): Pure-Rust path for a larger subset of the generator logic itself (if RedMicro ever grows a native emitter).
- **Ongoing**: Keep improving the parser (`BunOutputParser`) as Bun adds new output formats.

---

## 8. Open Questions

1. Should the selector live in `src/bun/` and be re-exported, or should we expose a higher-level "execution service"?
2. How should the Python side discover and invoke the native binary during generation (subprocess to the main binary, a separate small binary, or library)?
3. Do we want to persist "native preferred" as a user setting, or keep it fully automatic?
4. What is the migration story for users who have the old Python harness but not `bun` installed?

These will be resolved during the design review and first implementation chunk.

---

## 9. Next Steps

1. Review and align on this document (scope, approach, risks).
2. Break Phase 1 into small, reviewable chunks (following the same rhythm as the original native runner).
3. Start with the selector unification + a narrow integration point for absorb/generation.
4. Wire any additional telemetry fields that become newly visible during generation flows.

---

This document deliberately stays narrow and pragmatic. The objective is to deliver a noticeably better "native by default" experience for users of the TUI without taking on the much larger task of replacing the RedMicro generator.

Ready for review.