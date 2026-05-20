# Multi-Platform Hardening for the Native Bun Execution Layer

**Status**: Draft  
**Date**: 2026-05  
**Related**: `docs/native-bun-execution-layer-v2.md`, Chunks 12–15

---

## 1. Motivation

The native Bun execution layer (v1 + v2) has reached a major milestone: the Rust runner + rich telemetry + Python shim with automatic parent-process detection now makes the native path the default experience for both interactive TUI use and absorb/generation flows.

However, the current implementation has a clear and significant limitation:

**It is heavily Unix-centric.**

- Rust `find_bun()` has a Unix-only fallback list and relies on `dirs::home_dir()` + `which`.
- Python `_is_running_under_api_anything()` uses `/proc` heavily and only falls back to `ps`.
- Several assumptions around paths, process tree walking, and environment variables are Linux/macOS-first.

This means the "native is the default" promise currently works best on developer machines running Linux or macOS. Windows users (and some constrained environments) still fall back to the Python harness more often than necessary.

Making the system robust across **Linux, macOS, and Windows** is now the highest-leverage technical improvement we can make.

---

## 2. Current State & Platform Gaps

| Area                        | Current State                          | Linux | macOS | Windows | Gap |
|----------------------------|----------------------------------------|-------|-------|---------|-----|
| Bun binary discovery       | `find_bun()` + `which` + BUN_INSTALL   | Good  | Good  | Partial | Weak Windows paths (Scoop, Winget, manual installs) |
| User home detection        | `dirs::home_dir()`                     | Good  | Good  | Good    | Minor (roaming vs local profile) |
| Parent process detection   | `/proc` + `ps` fallback (Python)       | Good  | Good  | None    | **Major** – no Windows support |
| Common installation paths  | Hard-coded Unix list                   | Good  | Good  | None    | No Windows locations |
| Environment variable handling | Works                                 | Good  | Good  | Good    | None |
| Event streaming            | Works cross-platform                   | Good  | Good  | Good    | None |

---

## 3. Goals

- Make Bun discovery reliable on Windows, macOS, and Linux without requiring users to set `BUN_INSTALL`.
- Make parent-process detection (the key to auto-enabling the native path during absorb/generation) work on Windows.
- Keep the Python shim (`cli_anything_bun/process.py`) and Rust selector (`src/bun/execution.rs` + `native.rs`) clean and maintainable.
- Preserve the existing "prefer native, transparent fallback" contract.
- Add platform-specific tests so regressions are caught early.

---

## 4. Proposed Phased Approach

### Phase 1: Bun Binary Discovery (Highest immediate value)
- Improve `find_bun()` in Rust to handle common Windows installation methods:
  - `%USERPROFILE%\.bun\bin\bun.exe`
  - Scoop (`~/scoop/apps/bun/current/bun.exe`)
  - Winget / Chocolatey locations
  - `BUN_INSTALL` already works — just needs better Windows path handling
- Add equivalent logic (or reuse via subprocess call) for the Python shim when needed.
- Expand the "common locations" list with platform guards.

### Phase 2: Cross-Platform Parent Process Detection
- Replace/improve `_is_running_under_api_anything()` in Python with proper Windows support:
  - Use `wmic process` or `Get-CimInstance Win32_Process` (via `subprocess` or `psutil` if we accept a small dep).
  - macOS already mostly works via `ps`.
- Consider a small, optional dependency (`psutil`) behind a feature flag or keep it pure-stdlib + `ps`/`wmic`.
- Add caching for the ancestry walk (it only needs to run once per process).

### Phase 3: Integration, Testing & Polish
- Add integration tests that simulate different platform environments (via mocking).
- Ensure the TUI delight layer and internal command (`internal run-bun`) behave correctly on all platforms.
- Update documentation and error messages to be platform-aware.
- Decide on long-term strategy for process tree walking (pure Python vs small helper binary vs `psutil`).

---

## 5. Technical Considerations

### Rust Side (`src/bun/native.rs`)
- Use `#[cfg(target_os = "...")]` guards.
- Leverage `dirs` crate more (it already handles Windows roaming/local profile nuances).
- Consider adding `winreg` or simple registry checks only if common installers store Bun there (usually not needed).

### Python Side (`cli_anything_bun/process.py`)
- Keep the shim dependency-free if possible.
- Windows process walking via `wmic` or PowerShell is acceptable for a thin compatibility layer.
- Example Windows command:
  ```powershell
  wmic process where (processid=%PID%) get parentprocessid,commandline
  ```

### Graceful Degradation
- If parent detection fails on any platform, the system must still work (user can always set the env var manually).
- Never let platform detection errors cause Bun commands to fail.

---

## 6. Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Windows process tree walking is fragile | Use multiple methods (`wmic` + PowerShell) and fall back silently |
| Adding `psutil` increases complexity | Prefer pure-stdlib + subprocess first. Only consider `psutil` if maintenance cost becomes high |
| Different Bun installers on Windows create many paths | Prioritize the official `~/.bun` layout + `BUN_INSTALL` + `which` (which already works well on Windows) |
| Performance of ancestry walking | Cache the result for the lifetime of the process |

---

## 7. Open Questions

1. Should we take a small dependency on `psutil` for the Python shim to make parent detection reliable across platforms, or stay with `subprocess + ps/wmic`?
2. Do we want to expose a `--detect-platform` or internal diagnostic command to help debug discovery issues?
3. How aggressively should we support esoteric Windows install methods (Scoop, Winget, manual zip, etc.) vs relying on `BUN_INSTALL` + PATH?
4. Should the Rust `find_bun()` and Python shim share discovery logic (e.g., via a small JSON output from the Rust binary)?

---

## 8. Proposed First Chunk (Chunk 16)

**Title**: Cross-Platform Bun Binary Discovery

Focus:
- Harden `find_bun()` in Rust for Windows common paths.
- Add equivalent (or delegated) logic for the Python side when the env var is set.
- Add basic platform-specific unit tests.
- Update error messages to be more helpful on Windows.

This is a low-risk, high-confidence slice that immediately improves the experience for Windows users while the more complex parent-detection work happens in parallel or next.

---

This document is intentionally narrow. The goal is to turn the current "works great on developer Unix machines" experience into a production-grade, cross-platform foundation.

Ready for review and alignment on scope before we break it into the first implementation chunk.