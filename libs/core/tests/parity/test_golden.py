"""cfabric against the Text-Fabric golden files in ``tests/golden``.

``test_cfabric_matches_golden`` runs every probe through cfabric and compares
it with the recorded Text-Fabric answer; ``test_golden_files_regenerate_from_text_fabric``
checks that the committed golden files are exactly what Text-Fabric produces.
"""

from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import sys
from collections.abc import Callable
from pathlib import Path
from types import ModuleType
from typing import Any

import pytest
from cfabric import Fabric

pytestmark = pytest.mark.skipif(
    os.environ.get("CONTEXT_FABRIC_RUN_PARITY") != "1",
    reason="set CONTEXT_FABRIC_RUN_PARITY=1 to run Text-Fabric golden parity tests",
)

GOLDEN_ROOT = Path(__file__).resolve().parents[1] / "golden"


def _load_probes() -> ModuleType:
    spec = importlib.util.spec_from_file_location("golden_probes", GOLDEN_ROOT / "probes.py")
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


probes = _load_probes()


def _golden(name: str) -> dict[str, Any]:
    records = json.loads((probes.GOLDEN_DIR / f"{name}.json").read_text())
    return {record["id"]: record["result"] for record in records}


@pytest.mark.parametrize("name", probes.CORPORA)
def test_cfabric_matches_golden(name: str, corpus_tf_dir: Callable[[str], Path]) -> None:
    api = Fabric(locations=str(corpus_tf_dir(name)), silent="deep").loadAll(silent="deep")
    actual = {
        probe["id"]: probes.normalize(probes.run_probe(api, probe))
        for probe in probes.iter_probes(name)
    }
    assert actual == _golden(name)


@pytest.mark.parametrize("name", probes.CORPORA)
def test_golden_files_regenerate_from_text_fabric(
    name: str, corpus_tf_dir: Callable[[str], Path], tmp_path: Path
) -> None:
    pytest.importorskip("tf.fabric")
    corpus_tf_dir(name)
    subprocess.run(
        [sys.executable, str(GOLDEN_ROOT / "gen_golden.py"), name, "--output-dir", str(tmp_path)],
        check=True,
        capture_output=True,
    )
    regenerated = (tmp_path / f"{name}.json").read_text()
    assert regenerated == (probes.GOLDEN_DIR / f"{name}.json").read_text()
