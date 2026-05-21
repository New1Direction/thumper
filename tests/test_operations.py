"""
Tests for the Operation abstraction (cli_anything_bun/operations.py).

These tests focus on the contract:
- Lifetime management via context manager
- Automatic op_id + correlation on every emitted event
- Easy .emit() integration with the stable envelope
- Success path → clean emit_result(ok=true)
- Error path → *.failed + emit_result(ok=false) with normalized error model
"""

import json
import sys
from io import StringIO

import pytest

from thump.operations import Operation, operation


def _capture(fn):
    """Capture all stdout produced by calling fn()."""
    old = sys.stdout
    buf = StringIO()
    sys.stdout = buf
    try:
        fn()
    finally:
        sys.stdout = old
    return buf.getvalue()


def _lines(text):
    return [json.loads(l) for l in text.strip().splitlines() if l.strip()]


def test_operation_lifetime_and_correlation():
    """started + user events + final result must all share the generated op_id."""
    events = []

    def run():
        with Operation("package.install", {"packages": ["hono"]}) as op:
            events.append(("op_id", op.op_id))
            op.emit("resolved", {"count": 7})
            op.set_result(added=1)

    output = _capture(run)
    parsed = _lines(output)

    op_id = events[0][1]

    # started
    assert parsed[0]["event"] == "package.install.started"
    assert parsed[0]["op_id"] == op_id
    assert parsed[0]["data"]["packages"] == ["hono"]

    # user event
    assert parsed[1]["event"] == "package.install.resolved"
    assert parsed[1]["op_id"] == op_id

    # final result (no envelope)
    result = parsed[-1]
    assert "event" not in result
    assert result["ok"] is True
    assert result["operation"] == "package.install"
    assert result["added"] == 1


def test_operation_error_path():
    """On exception: emit *.failed (with normalized error) + result(ok=false)."""
    output = _capture(lambda: (
        pytest.raises(ValueError, match="test failure") and
        None   # we will run the block manually
    ))

    # Manually run the failing block under capture
    def failing_block():
        with Operation("script.run", {"script": "test"}) as op:
            op.emit("executing")
            raise ValueError("test failure from operation")

    try:
        failing_block()
    except ValueError:
        pass

    # Re-capture properly
    def run_failing():
        with pytest.raises(ValueError):
            with Operation("script.run", {"script": "test"}) as op:
                op.emit("executing")
                raise ValueError("test failure from operation")

    output = _capture(run_failing)
    parsed = _lines(output)

    assert len(parsed) == 4
    started, executing, failed, result = parsed

    assert started["event"] == "script.run.started"
    assert executing["event"] == "script.run.executing"
    assert failed["event"] == "script.run.failed"
    assert failed["level"] == "error"

    err = failed["data"]
    assert err["type"] == "runtime_error"
    assert "test failure" in err["message"]
    assert err["exception_type"] == "ValueError"

    assert result["ok"] is False
    assert result["operation"] == "script.run"
    assert result["error"]["type"] == "runtime_error"


def test_convenience_context_manager_and_set_result():
    def run():
        with operation("package.remove") as op:
            op.set_result(removed=["lodash"], warnings=0)

    output = _capture(run)
    result = _lines(output)[-1]

    assert result["ok"] is True
    assert result["operation"] == "package.remove"
    assert result["removed"] == ["lodash"]


def test_op_ids_are_unique():
    op1 = Operation("x")
    op2 = Operation("x")
    assert op1.op_id != op2.op_id
