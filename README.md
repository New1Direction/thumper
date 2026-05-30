# thumper

**The local execution and recovery substrate for autonomous AI coding agents.**

Thumper keeps pre-warmed sandbox workspace pools, mounts toolchains, symlinks warm package caches, holds persistent background LSP connections, and performs sub-second local compile-error healing. It's the layer that sits under [korgex](https://github.com/New1Direction/korgex) and turns "the model edited a file" into "the model edited a file, the workspace ran the tests, and the result was streamed back, all in under a second."

This repo ships two complementary pieces:

| Subdir | Language | Role |
|---|---|---|
| [`thumper/`](thumper/) | Rust | The native `thump` binary — ratatui TUI, Braille plasma progress bar, native Bun runner, NDJSON event stream, ACP/stdio agent mode |
| [`thump/`](thump/) | Python | The semantic harness — a structured event-emitting layer over the Bun runtime, used when the native path isn't available |
| [`tests/`](tests/) | Python | Cross-cutting tests for the Python harness |

The Rust side is the fast path; the Python harness is the semantic fallback. Under `thump`, the Python `thump` package automatically promotes to the native runner via `THUMP_PARENT_ACTIVE=1` — zero configuration.

---

## Quickstart

### Native binary (Rust)

```bash
cargo install thumper-cli        # installs the `thump` binary
thump --help

# Full-screen TUI:
thump

# Onboarding demo (best first run):
thump --demo
```

<details><summary>…or build from source</summary>

```bash
git clone https://github.com/New1Direction/thumper.git
cd thumper/thumper
cargo build --release
./target/release/thump --help
```
</details>

### Python harness only

```bash
pip install thump
python -m thump --help
```

---

## What's in the box

- **Native Rust runner** — `bun install`, `bun run`, `bun add` etc. routed through a streaming parser with real exit codes
- **Stage-aware plasma TUI** — Braille fractal progress that reacts to Bun stage (resolving / installing / running), velocity, and performance score
- **Self-healing drill** — interactive error card → `[I]` recovery trigger → plasma personality shift → success
- **Headless mode** — `--json`, NDJSON event stream, full ACP for Zed / Neovim / Cursor
- **Multi-language emitters** — generate FastAPI / axum / Go servers, or absorb a tool into harness + API + tests

For deep details on each piece, see [`thumper/README.md`](thumper/README.md) (Rust binary) and [`thump/README.md`](thump/README.md) (Python harness).

---

## Status

- **50 tests passing** across the workspace (22 lib + 22 bin + 6 e2e TMUX-driven integration)
- Rust + Python paths are both production-quality for local use
- Published: **`thumper-cli`** on [crates.io](https://crates.io/crates/thumper-cli) (the `thump` binary) · **`thump`** on [PyPI](https://pypi.org/project/thump/)

---

## Related projects

- [**korgex**](https://github.com/New1Direction/korgex) — the autonomous coding agent that runs on top of thumper
- [**korg**](https://github.com/New1Direction/korg) — the causally-ordered event ledger that records every agent decision; thumper events feed into korg's journal when both are deployed together
- [**korgchat**](https://github.com/New1Direction/korgchat) — the chat product built on the same ledger

---

## License

MIT or Apache-2.0, at your option.
