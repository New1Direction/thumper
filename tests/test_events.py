"""
Basic tests for the Bun event envelope and NDJSON emitter.

These tests validate the core contract defined in the design document:
- Stable envelope shape
- schema_version
- Correlation fields (op_id / session_id)
- Level support
- Raw output preservation
- Result vs event separation
"""

import json
import sys
from io import StringIO

import pytest

from cli_anything_bun.events import (
    BunEvent,
    EventLevel,
    SCHEMA_VERSION,
    emit,
    emit_event,
    emit_result,
    info,
    warn,
    error,
    raw_stdout,
    raw_stderr,
    event_from_json,
)


def _capture_output(fn):
    """Run fn while capturing everything written to sys.stdout. Returns the captured text."""
    old_stdout = sys.stdout
    buf = StringIO()
    sys.stdout = buf
    try:
        fn()
    finally:
        sys.stdout = old_stdout
    return buf.getvalue()


def test_bun_event_envelope_defaults():
    """Core envelope fields are present and have correct defaults."""
    event = BunEvent(event="test.something.happened")

    assert event.event == "test.something.happened"
    assert event.schema_version == SCHEMA_VERSION
    assert event.level == EventLevel.INFO
    assert event.data == {}
    assert event.session_id is None
    assert event.op_id is None
    assert event.ts  # should be an ISO timestamp


def test_to_dict_and_json_roundtrip():
    event = BunEvent(
        event="package.install.started",
        data={"packages": ["hono"]},
        level=EventLevel.INFO,
        session_id="sess-123",
        op_id="op-456",
    )

    d = event.to_dict()
    assert d["event"] == "package.install.started"
    assert d["schema_version"] == "1"
    assert d["level"] == "info"
    assert d["session_id"] == "sess-123"
    assert d["op_id"] == "op-456"
    assert d["data"] == {"packages": ["hono"]}

    # Roundtrip via JSON
    json_str = event.to_json()
    parsed = json.loads(json_str)
    assert parsed["event"] == "package.install.started"


def test_event_from_json():
    raw = '{"ts":"2026-05-20T12:00:00Z","event":"session.started","schema_version":"1","level":"info","session_id":"dev-001","data":{}}'
    event = event_from_json(raw)

    assert event.event == "session.started"
    assert event.session_id == "dev-001"
    assert event.level == EventLevel.INFO


def test_emit_writes_ndjson_line():
    """emit() should write a single compact NDJSON line and return the event."""
    def run():
        return emit(
            "package.add.started",
            {"package": "hono"},
            level=EventLevel.INFO,
            op_id="add-001",
        )

    event = run()  # run while capturing
    # We need to capture the side-effect of the emit above.
    # Simpler: just re-emit inside the capturer for the test.
    def emit_and_capture():
        emit(
            "package.add.started",
            {"package": "hono"},
            level=EventLevel.INFO,
            op_id="add-001",
        )

    output = _capture_output(emit_and_capture)
    lines = [line for line in output.strip().split("\n") if line]

    assert len(lines) == 1
    data = json.loads(lines[0])

    assert data["event"] == "package.add.started"
    assert data["schema_version"] == "1"
    assert data["level"] == "info"
    assert data["op_id"] == "add-001"
    assert data["data"]["package"] == "hono"

    # The returned object should match
    assert event.event == "package.add.started"


def test_raw_stdout_and_stderr():
    """Raw output events should be emitted with the correct structure."""
    out_event = raw_stdout("Server started on :3000", op_id="dev-001")
    err_event = raw_stderr("Warning: something", op_id="dev-001")

    assert out_event.event == "process.stdout"
    assert out_event.data["stream"] == "stdout"
    assert out_event.data["raw"] == "Server started on :3000"
    assert out_event.level == EventLevel.DEBUG

    assert err_event.event == "process.stderr"
    assert err_event.data["stream"] == "stderr"
    assert err_event.level == EventLevel.WARN


def test_emit_result_is_separate_from_events(capfd):
    """emit_result should produce clean JSON without the event envelope."""
    result = {
        "ok": True,
        "command": "package install",
        "packages_added": 2,
        "elapsed_ms": 1240,
    }

    emit_result(result)

    captured = capfd.readouterr()
    line = captured.out.strip()
    data = json.loads(line)

    # Should NOT contain the event envelope fields
    assert "event" not in data
    assert "schema_version" not in data
    assert "level" not in data
    assert data["ok"] is True
    assert data["packages_added"] == 2


def test_level_enum_values():
    assert EventLevel.DEBUG.value == "debug"
    assert EventLevel.INFO.value == "info"
    assert EventLevel.WARN.value == "warn"
    assert EventLevel.ERROR.value == "error"


def test_convenience_helpers_use_correct_level(capfd):
    info("package.resolved")
    warn("package.outdated")
    error("package.resolution.failed")

    captured = capfd.readouterr()
    lines = [l for l in captured.out.strip().split("\n") if l]

    assert len(lines) == 3
    assert json.loads(lines[0])["level"] == "info"
    assert json.loads(lines[1])["level"] == "warn"
    assert json.loads(lines[2])["level"] == "error"
