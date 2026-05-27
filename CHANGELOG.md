# Changelog

All notable changes to thumper are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

For binary-specific notes that aren't user-facing release content, see
[`thumper/CHANGELOG.md`](thumper/CHANGELOG.md).

## [Unreleased]

### Added
- Repo-root README pointing visitors to `thumper/` (Rust binary) and `thump/` (Python harness).

### Changed
- Repo renamed from `api-anything` to `thumper` to match Korgex subsystem positioning.
- `cargo fmt` cleanup across the workspace.
- TUI startup tagline cycling slowed to 2–3 phrases over the 8s intro.
- Centered header jiggle stabilized; branding renamed to THUMPER throughout; run loop optimized to prevent terminal flashing.
- Stale `redmicro/api-anything` GitHub URLs in `thump/README.md` replaced with the canonical `New1Direction/thumper` URL.
- Broken `file://` paths in `thumper/README.md` replaced with relative links.
- `.gitignore` extended for `antigravity-awesome-skills/` (external project cloned for reference).

### Fixed
- 2 thumper Highs closed (from the 2026-05-25 ecosystem audit).
- 2 thumper Mediums closed (from the 2026-05-25 ecosystem audit).
- Multiple Lows closed (from the 2026-05-25 ecosystem audit).
- Ratatui TUI modularized into focused submodules (`state.rs`, `handlers.rs`, `widgets/views.rs`, `mod.rs`); 118 KB monolith eliminated.
- Hardcoded absolute paths (`/Users/clubpenguin/.../redmicro`) replaced with a dynamic resolver that honors `REDMICRO_ROOT` and falls back to `dirs::home_dir()` candidates.
- 230+ lines of fragile platform-specific parent-process scanning (ctypes, `/proc`, `NtQueryInformationProcess`, `ps -o ppid`) replaced with a single `THUMP_PARENT_ACTIVE=1` env var — no more risk of infinite spawn loops.
- Four duplicate binaries (`bunny`, `thumper`, `thump-cli`, `thump`) consolidated into a single official `thump` target.

## [0.1.0] — 2026-05-20

### Added
- **Initial release.** Native execution layer v2 + delight layer.
- Rust `thump` binary with full-screen ratatui TUI, Braille plasma progress bar, native Bun runner (streaming parser with speed / percent / stage, real exit codes), stage-aware telemetry.
- Python `thump` harness as the semantic fallback — structured event-driven layer over the Bun runtime emitting a stable NDJSON event envelope.
- Automatic Parent Ancestry Promotion: the Python harness detects when it's running under thumper and promotes to the native runner with zero configuration.
- Multi-language emitters: generate immediately-buildable FastAPI / axum / Go API projects from a single tool description.
- Full `--absorb` flow: generate CLI harness + API + tests in one shot.
- ACP server (`thump agent stdio`) for Zed / Neovim / Cursor integration.
- Headless mode (`--json`, streaming NDJSON) for agent consumption.
- Stream-Ignition + Hermes Cockpit onboarding demo (`thump --demo`).
- 50 tests passing (22 lib + 22 bin + 6 e2e TMUX-driven integration).

[Unreleased]: https://github.com/New1Direction/thumper/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/New1Direction/thumper/releases/tag/v0.1.0
