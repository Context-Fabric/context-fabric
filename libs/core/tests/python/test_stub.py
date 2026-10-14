"""The type stub `cfabric/_core.pyi` matches the compiled extension.

Runs mypy's `stubtest`, which imports `cfabric._core` and compares every
runtime name, signature (parameter names, kinds, defaults), and class against
the stub. Skipped when mypy is not installed.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest

pytest.importorskip("mypy.stubtest")

ALLOWLIST = Path(__file__).with_name("stubtest_allowlist.txt")


def test_stub_matches_runtime_extension():
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "mypy.stubtest",
            "cfabric._core",
            "--allowlist",
            str(ALLOWLIST),
            "--concise",
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stdout + result.stderr


def test_stub_and_marker_ship_with_the_package():
    import cfabric

    package = Path(cfabric.__file__).parent
    assert (package / "_core.pyi").is_file()
    assert (package / "py.typed").is_file()
