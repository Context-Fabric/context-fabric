"""Fixtures shared by the Python and parity suites for the real corpora.

Cross-corpus tests run against full corpora (BHSA, N1904, ...) when they are
available and skip otherwise.

Environment:
    CFABRIC_CORPORA: directory holding ``<name>/tf`` corpora
        (default: ``libs/benchmarks/.corpora`` in this repository).
    CFABRIC_BHSA_CFR: explicit compiled BHSA cache to open (compiled there if
        missing). By default the BHSA API is loaded with ``loadAll``, which
        builds and reuses the cache that ``Fabric`` keeps next to the sources.
"""

from __future__ import annotations

import os
from collections.abc import Callable
from pathlib import Path
from typing import Any

import pytest
from cfabric import Fabric

REPO_CORPORA = Path(__file__).resolve().parents[2] / "benchmarks" / ".corpora"


def corpora_dir() -> Path:
    """Directory holding the real corpora (``CFABRIC_CORPORA`` overrides)."""
    return Path(os.environ.get("CFABRIC_CORPORA") or REPO_CORPORA)


@pytest.fixture(scope="session")
def corpus_tf_dir() -> Callable[[str], Path]:
    """Return ``<corpora>/<name>/tf``, skipping the test when it is absent."""

    def resolve(name: str) -> Path:
        path = corpora_dir() / name / "tf"
        if not (path / "otype.tf").exists():
            pytest.skip(f"{name} corpus not available at {path}")
        return path

    return resolve


@pytest.fixture(scope="session")
def bhsa_tf_dir(corpus_tf_dir: Callable[[str], Path]) -> Path:
    return corpus_tf_dir("bhsa")


@pytest.fixture(scope="session")
def bhsa_explicit_cache() -> Path | None:
    """The ``CFABRIC_BHSA_CFR`` override, or ``None`` to use the default cache."""
    value = os.environ.get("CFABRIC_BHSA_CFR")
    return Path(value) if value else None


@pytest.fixture(scope="session")
def bhsa_api(bhsa_tf_dir: Path, bhsa_explicit_cache: Path | None) -> Any:
    """BHSA loaded once per session (compiling its cache on first use)."""
    fabric = Fabric(locations=str(bhsa_tf_dir), silent="deep")
    if bhsa_explicit_cache is None:
        return fabric.loadAll(silent="deep")
    if not bhsa_explicit_cache.exists():
        fabric.compile(str(bhsa_explicit_cache), None, "deep")
    return fabric.openMapped(str(bhsa_explicit_cache))
