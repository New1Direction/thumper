#!/usr/bin/env bash
# Quick demo script for api-anything
# Shows the main flows after the parallel development + stabilization

set -e

echo "=== api-anything Demo ==="
echo

echo "1. Native Rust emitter (produces a real compilable axum project)"
cargo run -- generate nmap --lang rust --output /tmp/nmap-rust-api
echo "   Generated project:"
ls -1 /tmp/nmap-rust-api
echo

echo "2. Native Go emitter"
cargo run -- generate rustscan --lang go --output /tmp/rustscan-go-api
echo "   Generated project:"
ls -1 /tmp/rustscan-go-api
echo

echo "3. Full absorb flow (requires RedMicro)"
if [ -d "$HOME/.grok/skills/redmicro" ] || [ -n "$REDMICRO_ROOT" ]; then
    cargo run -- generate bettercap --absorb --output /tmp/bettercap-full
    echo "   Absorb package created with harness + API + tests"
else
    echo "   (Skipping — set REDMICRO_ROOT to run full absorb demo)"
fi
echo

echo "4. ACP mode (for IDEs)"
echo "   Run: cargo run --features acp -- agent stdio"
echo "   Then send initialize + newSession + prompt via JSON-RPC"
echo

echo "=== Demo complete ==="
echo "Tip: cargo run --features tui   for the full-screen dashboard with live generation"