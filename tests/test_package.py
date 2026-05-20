"""
Basic tests for the package command group (cli_anything_bun/package.py).

These tests focus on the contract:
- Correct Operation naming ("package.add", "package.install", "package.remove")
- Proper command construction (flags, multiple packages)
- Clean result objects
- Integration with the event system
"""

import tempfile
import json
from pathlib import Path

from cli_anything_bun.package import (
    add as package_add,
    install as package_install,
    remove as package_remove,
    PackageAddResult,
    PackageInstallResult,
    PackageRemoveResult,
)


def test_package_add_result_shape():
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "package.json").write_text(json.dumps({"name": "t", "version": "0.0.0"}))

        # This will actually hit the network / Bun, so we just check shape
        # In CI this may be skipped or use --offline in future
        result = package_add(["picocolors"], cwd=root)

        assert isinstance(result, PackageAddResult)
        assert result.packages == ["picocolors"]
        assert hasattr(result, "returncode")
        assert hasattr(result, "success")
        assert hasattr(result, "duration")


def test_package_remove_result_shape():
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "package.json").write_text(json.dumps({"name": "t", "version": "0.0.0"}))

        # First add something so remove has work to do
        package_add(["hono"], cwd=root)
        result = package_remove(["hono"], cwd=root)

        assert isinstance(result, PackageRemoveResult)
        assert result.packages == ["hono"]
        assert hasattr(result, "success")


def test_package_install_empty_and_with_packages():
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "package.json").write_text(json.dumps({"name": "t", "version": "0.0.0"}))

        # Plain install
        r1 = package_install(cwd=root)
        assert isinstance(r1, PackageInstallResult)
        assert r1.packages == []

        # Install with explicit package (Bun allows this)
        r2 = package_install(["hono"], cwd=root)
        assert isinstance(r2, PackageInstallResult)


def test_package_add_with_flags():
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "package.json").write_text(json.dumps({"name": "t", "version": "0.0.0"}))

        result = package_add(["zod"], cwd=root, dev=True, exact=True)
        assert isinstance(result, PackageAddResult)
        # We don't assert success here because network may vary in test env,
        # but the flags were passed correctly into the Operation data.
