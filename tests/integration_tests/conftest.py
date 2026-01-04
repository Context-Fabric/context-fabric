"""Shared fixtures for integration tests.

Provides loaded API objects for testing the full Context Fabric stack.
"""

import pytest
from pathlib import Path


@pytest.fixture(scope="module")
def fixtures_dir():
    """Path to test fixtures directory."""
    return Path(__file__).parent.parent / "fixtures"


@pytest.fixture(scope="module")
def mini_corpus_path(fixtures_dir):
    """Path to minimal test corpus as string."""
    return str(fixtures_dir / "mini_corpus")


@pytest.fixture(scope="module")
def fabric_core(mini_corpus_path):
    """Create FabricCore instance for mini_corpus."""
    from core.fabric import FabricCore

    TF = FabricCore(locations=mini_corpus_path, silent="deep")
    return TF


@pytest.fixture(scope="module")
def loaded_api(fabric_core):
    """Load mini_corpus and return API object.

    mini_corpus structure:
    - Nodes 1-5: slot nodes (type "word")
      - word values: "hello", "beautiful", "world", "good", "morning"
    - Node 6: phrase containing slots 1-3
    - Node 7: phrase containing slots 4-5
    - Node 8: sentence containing slots 1-5

    Parent edges:
    - Words 1,2,3 -> phrase 6
    - Words 4,5 -> phrase 7
    - Phrases 6,7 -> sentence 8
    """
    api = fabric_core.loadAll(silent="deep")
    assert api is not False, "Failed to load mini_corpus"
    return api
