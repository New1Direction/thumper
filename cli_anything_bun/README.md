# cli-anything-bun

**Semantic harness for Bun** — a clean, observable, agent-native layer over the Bun JavaScript runtime.

This Python package provides a structured, event-driven interface for running Bun commands (`bun run`, `bun add`, `bun install`, `bun remove`, etc.) while emitting a stable NDJSON event stream.

It is designed to be used from:
- Command line
- TUI / IDE integrations
- AI agents (via ACP / stdio)

## Features

- **Stable event envelope** — every operation emits structured events (`*.started`, `process.stdout`, `process.exited`, etc.) with `op_id` / `session_id` correlation.
- **Raw output preservation** — stdout/stderr from Bun is never lost.
- **Operation abstraction** — clean success/error paths with normalized error model.
- **Live process streaming** — subprocess execution with real-time event emission.
- **First-class commands**:
  - `script run`
  - `package add` / `install` / `remove`
- Works great from Rust, Python, or directly on the CLI.

## Installation (development)

```bash
git clone https://github.com/YOUR_USERNAME/cli-anything-bun.git
cd cli-anything-bun
pip install -e .
```

Or run directly:

```bash
python -m cli_anything_bun --help
```

## Usage

### From Python

```python
from cli_anything_bun import script_run, package_add

result = script_run("dev", cwd="my-project")
result = package_add(["hono", "zod"], cwd="my-project", dev=True)
```

### From CLI

```bash
python -m cli_anything_bun script run dev
python -m cli_anything_bun package add hono --dev
python -m cli_anything_bun package install
```

### Streaming events

All commands emit NDJSON to stdout. Each line is either:

- A structured event (with `event`, `op_id`, `data`, etc.)
- A final result object (`{"ok": true, "operation": "..."}`)

This makes it trivial to consume from Rust, Go, another Python process, or an agent.

## Architecture

```
Bun CLI
   ↓
cli_anything_bun (Python semantic layer)
   ↓
Stable NDJSON event stream + final results
   ↓
Consumers: CLI • TUI Jobs • ACP agents • Rust adapter
```

## Related Projects

- [api-anything](https://github.com/YOUR_USERNAME/api-anything) — The Rust TUI/CLI/ACP frontend that drives this harness.

## Status

This is the Python-first implementation (Phase 1) as described in the design document.  
A native Rust implementation of the same protocol is planned for later.

## License

MIT
