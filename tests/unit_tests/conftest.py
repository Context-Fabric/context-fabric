"""Shared fixtures for Context Fabric unit tests."""

import pytest
from pathlib import Path
from unittest.mock import MagicMock


@pytest.fixture
def fixtures_dir():
    """Path to test fixtures directory."""
    return Path(__file__).parent.parent / "fixtures"


@pytest.fixture
def mini_corpus_path(fixtures_dir):
    """Path to minimal test corpus."""
    return fixtures_dir / "mini_corpus"


@pytest.fixture
def invalid_fixtures_path(fixtures_dir):
    """Path to invalid TF files for error testing."""
    return fixtures_dir / "invalid"


@pytest.fixture
def edge_cases_path(fixtures_dir):
    """Path to edge case TF files."""
    return fixtures_dir / "edge_cases"


@pytest.fixture
def mock_timestamp():
    """Mock timestamp object for silent testing.

    Provides a mock object that implements the tmObj interface
    used throughout the codebase for logging and timing.
    """
    tm = MagicMock()
    tm.isSilent.return_value = True
    tm.setSilent = MagicMock()
    tm.indent = MagicMock()
    tm.info = MagicMock()
    tm.error = MagicMock()
    return tm


@pytest.fixture
def sample_node_data():
    """Sample node feature data dict.

    Represents a simple node feature mapping node IDs to string values.
    """
    return {1: "word1", 2: "word2", 3: "word3", 4: "word4", 5: "word5"}


@pytest.fixture
def sample_edge_data():
    """Sample edge feature data dict.

    Represents edge relationships where nodes 1-3 connect to node 6,
    and nodes 4-5 connect to node 7.
    """
    return {
        1: frozenset({6}),
        2: frozenset({6}),
        3: frozenset({6}),
        4: frozenset({7}),
        5: frozenset({7}),
    }


@pytest.fixture
def sample_edge_data_with_values():
    """Sample edge feature data with values.

    Represents edges with associated values (e.g., weights or labels).
    """
    return {
        1: {6: "child"},
        2: {6: "child"},
        3: {6: "child"},
        4: {7: "child"},
        5: {7: "child"},
    }


@pytest.fixture
def temp_tf_file(tmp_path):
    """Factory fixture for creating temporary TF files.

    Usage:
        def test_something(temp_tf_file):
            path = temp_tf_file("myfeature", "@node\\n@valueType=str\\n\\nvalue1\\n")
            # use path...
    """

    def _create(name, content):
        path = tmp_path / f"{name}.tf"
        path.write_text(content)
        return path

    return _create


@pytest.fixture
def mock_api():
    """Mock API object for testing feature classes.

    Provides a minimal API mock with the necessary attributes
    for testing NodeFeature, EdgeFeature, etc.
    """
    api = MagicMock()
    api.F = MagicMock()
    api.E = MagicMock()
    api.T = MagicMock()
    api.L = MagicMock()
    api.N = MagicMock()
    api.TF = MagicMock()

    # Set up otype data structure (maxSlot, maxNode, slotType)
    api.F.otype = MagicMock()
    api.F.otype.v = MagicMock(side_effect=lambda n: "word" if n <= 5 else "phrase")
    api.F.otype.s = MagicMock(
        side_effect=lambda t: range(1, 6) if t == "word" else [6, 7, 8]
    )
    api.F.otype.all = ("word", "phrase", "sentence")
    api.F.otype.data = (("phrase", "phrase", "sentence"), 5, 8, "word")

    # Set up oslots
    api.E.oslots = MagicMock()
    api.E.oslots.s = MagicMock(
        side_effect=lambda n: (
            (1, 2, 3) if n == 6 else (4, 5) if n == 7 else (1, 2, 3, 4, 5)
        )
    )

    return api
