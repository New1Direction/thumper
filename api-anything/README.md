# api-anything

**Get an API (and a harness) from anything** — native Rust binary + full-screen TUI, headless JSON/streaming, and first-class ACP/IDE Protocol support.

This is the high-performance, agent-native, IDE-integratable successor to the Python `cli-anything` + RedMicro `api-harness` system.

## Status

Mature enough for real use after a large parallel development push.

Core capabilities delivered:
- Full-screen ratatui TUI with live generation jobs, fuzzy search, and absorb mode
- Headless + streaming JSON
- Real ACP server (`agent stdio`) for IDE integration
- Full absorption flow (`--absorb`) producing CLI harness + API + tests
- Native emitters for Rust (axum) and Go that produce immediately buildable projects

See `AGENTS.md` and the E2E tests for current quality bar.

## Installation (future)

```bash
cargo install api-anything
# or Homebrew / release binary
```

## Usage

```bash
# Full-screen TUI (default)
api-anything

# Headless generate (primary agent path)
api-anything generate bettercap --from cli --lang python --json

# ACP / IDE mode (for Zed, Neovim, etc.)
api-anything agent stdio

# Generate with native Rust emitter (no Python needed)
api-anything generate nmap --lang rust --output ./nmap-api
cd ./nmap-api && cargo run

# Full absorption (harness + API + tests)
api-anything generate bettercap --absorb

# Daemon + HTTP API
api-anything serve --port 2481
```

## Features

- **Full-screen ratatui TUI** — registry browser, generate wizard, live streaming output, command palette
- **Headless** — every command supports `--json` and `streaming-json`
- **ACP / IDE Protocol** — `agent stdio` makes it a first-class "API expert" inside Zed, Neovim, etc.
- **Serve mode** — axum HTTP + WS for remote orchestrators and web dashboards (feature-gated)
- **Multi-language emitters** — Python FastAPI, Rust axum, Go, ... (via templates + engine)
- **Real backend** — prefers the battle-tested RedMicro python generators; pure-Rust fast path available

## Development

```bash
cargo run --features tui          # full-screen dashboard
cargo run -- generate --help
./scripts/demo.sh                 # quick end-to-end flows
cargo test --test e2e             # including native emitter compilation checks
```

## License

To be decided (likely MIT or Apache-2.0, matching the org).

---
Built with the same rigor as agent-of-empires and the RedMicro agent ecosystem. "Tomorrow’s users will be agents."