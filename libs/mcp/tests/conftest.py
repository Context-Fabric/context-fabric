"""Shared fixtures for the cfabric-mcp test suite."""

import shutil
from pathlib import Path

import pytest

SOURCE_FIXTURES = Path(__file__).parent / "fixtures"


@pytest.fixture(scope="session")
def fixtures_dir(tmp_path_factory) -> Path:
    """Session-scoped copy of the ``.tf`` fixture corpora.

    Loading a corpus compiles a ``.cfr`` cache next to its sources, so tests
    load from this copy to keep the source tree untouched.
    """
    copy = tmp_path_factory.mktemp("fixtures")
    for corpus in SOURCE_FIXTURES.iterdir():
        if corpus.is_dir():
            shutil.copytree(corpus, copy / corpus.name, ignore=shutil.ignore_patterns(".cf*"))
    return copy
