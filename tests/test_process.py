"""
Basic tests for the Process runner (cli_anything_bun/process.py).

Focus: live streaming, op_id correlation via Operation, result shape,
raw preservation, and graceful handling of success + non-zero exits.
"""

import json
import sys
from io import StringIO

from cli_anything_bun import Operation, Process, run_process


def _capture(fn):
    old = sys.stdout
    buf = StringIO()
    sys.stdout = buf
    try:
        fn()
    finally:
        sys.stdout = old
    return buf.getvalue()


def _parse(text):
    return [json.loads(l) for l in text.strip().splitlines() if l.strip()]


def test_run_process_simple_success():
    result = run_process(["echo", "hello process"])
    assert result.returncode == 0
    assert result.pid > 0
    assert result.duration >= 0
    assert "echo" in " ".join(result.cmd)


def test_process_inside_operation_streams_raw_and_correlates():
    output = _capture(lambda: (
        lambda: None  # placeholder
    ))

    def run():
        with Operation("script.run", {"script": "smoke"}) as op:
            with Process(["sh", "-c", "echo 'line one'; echo 'line two'"], op=op) as proc:
                pass
            op.set_result(returncode=proc.result.returncode)

    output = _capture(run)
    events = _parse(output)

    # We should see at least: script.run.started, process.started (namespaced), two stdout, process.exited, final result
    started = [e for e in events if e.get("event", "").endswith(".started")][0]
    op_id = started["op_id"]

    # All process.* events under the operation should carry the op_id
    stdout_events = [e for e in events if e.get("event", "").endswith("process.stdout")]
    assert len(stdout_events) >= 1
    for ev in stdout_events:
        assert ev["op_id"] == op_id
        assert "raw" in ev["data"]

    exited = [e for e in events if e.get("event", "").endswith("process.exited")][0]
    assert exited["op_id"] == op_id
    assert exited["data"]["returncode"] == 0

    final = events[-1]
    assert final["ok"] is True
    assert final["returncode"] == 0


def test_nonzero_exit_is_recorded():
    result = run_process(["false"])
    assert result.returncode == 1


def test_process_result_has_expected_fields():
    result = run_process(["true"])
    assert hasattr(result, "returncode")
    assert hasattr(result, "pid")
    assert hasattr(result, "duration")
    assert hasattr(result, "timed_out")
