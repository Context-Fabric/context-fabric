"""Fixtures shared by the browser's tests: small corpora in `fixtures/`
(`display_corpus`, `mini_corpus`, `edge_cases`, from the core package)."""

from pathlib import Path

import pytest

FIXTURES = Path(__file__).parent / "fixtures"


@pytest.fixture(scope="session")
def fixtures_dir() -> Path:
    return FIXTURES


@pytest.fixture(scope="session")
def mini_corpus_path(fixtures_dir: Path) -> str:
    return str(fixtures_dir / "mini_corpus")


@pytest.fixture(scope="module")
def loaded_api(mini_corpus_path: str):
    """The mini corpus, loaded: words 1-5 ("hello", "beautiful", "world",
    "good", "morning"), phrases 6 (words 1-3) and 7 (words 4-5), sentence 8."""
    from cfabric import Fabric

    api = Fabric(locations=mini_corpus_path, silent="deep").loadAll(silent="deep")
    assert api is not False, "Failed to load mini_corpus"
    return api
