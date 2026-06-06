from __future__ import annotations

from pathlib import Path

import pytest

from cfabric import Fabric


@pytest.fixture
def mini_corpus_path() -> Path:
    return Path(__file__).resolve().parents[1] / "fixtures" / "mini_corpus"


@pytest.fixture
def loaded_api(mini_corpus_path: Path):
    return Fabric(locations=str(mini_corpus_path), silent="deep").loadAll(silent="deep")
