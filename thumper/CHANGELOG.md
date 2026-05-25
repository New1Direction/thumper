# Changelog

## [0.2.0] - 2026-05

### Added
- Full-screen ratatui TUI with live Jobs panel, fuzzy search, absorb mode toggle
- Real ACP server (`agent stdio`) with generation on prompt and tool call streaming
- Native Rust (axum) and Go emitters that produce immediately buildable projects
- Full `--absorb` flow that generates CLI harness + API + tests via RedMicro
- Expanded E2E test suite (including compilation verification of native emitters)

### Changed
- Major parallel development + stabilization pass
- Much richer interactive and agent-native experience

## [0.1.0] - Initial
- Basic CLI + Python bridge + scaffolding
- Foundation for TUI, ACP, and multi-language emitters