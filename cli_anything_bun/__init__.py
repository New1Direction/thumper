"""
cli-anything-bun

Agent-oriented semantic layer over Bun.
Phase 1: Event envelope + NDJSON emitter foundation.
"""

from .events import (
    BunEvent,
    EventLevel,
    SCHEMA_VERSION,
    emit,
    emit_event,
    emit_result,
    info,
    warn,
    error,
    debug,
    raw_stdout,
    raw_stderr,
    event_from_json,
)
from .operations import Operation, operation
from .process import Process, ProcessResult, run_process
from .script import run as script_run, ScriptRunResult
from .package import (
    add as package_add,
    install as package_install,
    remove as package_remove,
    PackageAddResult,
    PackageInstallResult,
    PackageRemoveResult,
)
from .cli import main as cli_main

__all__ = [
    "BunEvent",
    "EventLevel",
    "SCHEMA_VERSION",
    "emit",
    "emit_event",
    "emit_result",
    "info",
    "warn",
    "error",
    "debug",
    "raw_stdout",
    "raw_stderr",
    "event_from_json",
    "Operation",
    "operation",
    "Process",
    "ProcessResult",
    "run_process",
    "script_run",
    "ScriptRunResult",
    "package_add",
    "package_install",
    "package_remove",
    "PackageAddResult",
    "PackageInstallResult",
    "PackageRemoveResult",
    "cli_main",
]
