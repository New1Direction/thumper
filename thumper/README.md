# Thumper (thump)

**The high-performance local execution and recovery substrate for Korgex.**

Thumper is the execution and recovery layer of the **Korgex** agent runtime. It is not intended for standalone public use; its public interface is Korgex's agent API. It maintains rolling pre-warmed sandbox workspace pools, mounts toolchains, symlinks warm package caches, manages persistent background LSP connections, and performs sub-second local compile-error healing.

## Refactoring Achievements (May 2026)

We recently completed a comprehensive audit and refactoring pass on **Thumper** (`thump`). The goal was to eliminate technical debt while preserving the project’s strengths: blazing-fast Rust TUI, beautiful Braille plasma progress bar, robust Python semantic harness, and rock-solid TMUX-driven E2E tests.

### What was accomplished

- **TUI Monolith Eliminated**  
  The 118 KB `src/tui/app.rs` (≈2,000 lines) was fully modularized into clean, focused modules:  
  - [src/tui/state.rs](file:///Users/clubpenguin/Documents/API/api-anything/src/tui/state.rs) – app state, `Page` enum, caches  
  - [src/tui/handlers.rs](file:///Users/clubpenguin/Documents/API/api-anything/src/tui/handlers.rs) – keyboard events and actions  
  - [src/tui/widgets/views.rs](file:///Users/clubpenguin/Documents/API/api-anything/src/tui/widgets/views.rs) – page rendering (Home, Palette, Absorb, etc.)  
  - [src/tui/mod.rs](file:///Users/clubpenguin/Documents/API/api-anything/src/tui/mod.rs) – public `App` wrapper with clean event/render API  
  The core `app.rs` acts as a tiny compatibility layer. Future UI changes are now fast and safe.

- **Circular Ancestry Delegation Fixed**  
  Replaced 230+ lines of fragile platform-specific parent-process scanning (ctypes, `/proc`, `NtQueryInformationProcess`, `ps -o ppid`) with a single, reliable `THUMP_PARENT_ACTIVE=1` environment variable. No more risk of infinite spawn loops.

- **Hardcoded Paths Removed**  
  All absolute paths (`/Users/clubpenguin/.../redmicro`) were replaced with a dynamic resolver:  
  - Honors `REDMICRO_ROOT` env var (perfect for CI/contributors)  
  - Falls back to `dirs::home_dir()` candidates  
  - Propagates correctly through isolated E2E test environments  
  Tests now run cleanly on any machine.

- **Build & CLI Cleanup**  
  - Consolidated four duplicate binaries (`bunny`, `thumper`, `thump-cli`, `thump`) into a single official `thump` target in `Cargo.toml`.  
  - Removed dead `argv[0]` alias logic and associated compiler warnings.  
  - Deleted the unfinished `serve` command and its feature flag entirely (CLI is now honest and minimal).

- **Test Suite Hardened**  
  - Fixed macOS `HOME` pollution in palette smoke tests (temporary dir isolation).  
  - Updated E2E harness to always target the live `thump` binary.  
  - Native emitter templates fixed (Serialize derives + brace escaping).  
  - Full suite now passes reliably: 5 unit tests + 6 TMUX-driven E2E integration tests.

### Result

Thumper is now **faster to build, easier to maintain, and more reliable** for both local development and distribution. The beautiful parts (Ratatui TUI polish, Braille plasma bar, NDJSON event streaming, Python stdlib harness) remain untouched and continue to shine.

---

## Status

Production-grade after deep native execution + TUI delight waves.

**What makes Thumper special today:**
- Real native Rust Bun runner (discovery, streaming parser with speed/percent/stage, true exit codes)
- Living plasma visualization that understands what Bun is actually doing
- Automatic, zero-config promotion to the fast path when running under `thump`

See `AGENTS.md`, the design docs under `docs/`, and the E2E tests for the quality bar.

## Installation (future)

```bash
cargo install thumper-cli
# or Homebrew / release binary (the binary is called `thump`)
```

## Usage

```bash
# Full-screen TUI (default)
thump

# Headless generate (primary agent path)
thump generate bettercap --from cli --lang python --json

# ACP / IDE mode (for Zed, Neovim, etc.)
thump agent stdio

# Generate with native Rust emitter (no Python needed)
thump generate nmap --lang rust --output ./nmap-api
cd ./nmap-api && cargo run

# Full absorption (harness + API + tests)
thump generate bettercap --absorb

# Launch the interactive Thumper Flight Deck onboarding experience
# (beautiful two-phase demo: Grok-style ignition → full ratatui cockpit
#  with live plasma + self-healing drill)
thump --demo
```

## Features

- **Native Bun Execution (default)** — `bun install`, `bun run`, `bun add` etc. are automatically routed through the blazing-fast Rust engine (`thump internal run-bun`) when launched under Thumper. Transparent Python `thump` package fallback when needed.
- **Stage-Aware Plasma & Telemetry** — the Jobs panel features a living braille fractal plasma bar that reacts to Bun stage (Resolving / Installing / Running), velocity, package count, and performance score. Real exit codes and structured metrics from the native parser.
- **Micro-Toast Celebration System** — non-intrusive top-right notifications with 2.8s fade, sparkle, and success bunny 🐰 on high-score native runs.
- **Automatic Parent Ancestry Promotion** — the Python `thump` harness intelligently detects when it is running under Thumper and promotes to the native runner with zero configuration.
- **Full-screen ratatui TUI** — fuzzy registry, live Jobs, command palette with history + live preview, absorb mode, doctor.
- **Headless & Agent-Native** — `--json`, streaming NDJSON, full ACP (`agent stdio`) for Zed / Neovim / Cursor.
- **Multi-language Emitters & Absorption** — generate production FastAPI / axum / Go servers or fully absorb a tool into harness + API + tests.
- **Rich Config & History** — persisted palette history and beautiful theming.

## Flight Deck (onboarding demo)

`thump --demo` launches a self-contained, high-signal interactive experience:

- **StreamIgnition**: Grok-style inline narrative boot with the Thumper bunny.
- **Hermes Cockpit**:
  - Station 01: Accelerated Plasma Showcase — the real `render_braille_plasma_bar` running at 5-6× speed with stage-aware Catppuccin 24-bit color.
  - Station 02: Self-Healing Drill — live diagnostic error card, `[I]` recovery trigger, plasma personality shift from angry red pulse → calm healing teal, ending in a success bunny celebration.

This is the best way to quickly experience the living plasma and predictive self-healing systems that power the real TUI.

More stations (Ancestry visualization, free-play Sandbox) are planned.

## Development

```bash
cargo run --features tui          # full-screen Thumper TUI with live plasma
cargo run -- generate --help
cargo run -- --demo               # Flight Deck onboarding experience (highly recommended)
./scripts/demo.sh                 # quick end-to-end flows
cargo test --test e2e             # native Bun + emitter checks
```

## License

MIT or Apache-2.0 (matching the broader ecosystem).

---
Thumper — stomping feet, streaming telemetry, and pure delight.  
"Tomorrow’s users will be agents — and they’ll have a beautiful TUI to watch it happen."