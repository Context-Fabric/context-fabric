"""Shared fixtures for MCP integration tests.

Provides fixtures that load corpora into the corpus_manager for testing
the full MCP tool stack.
"""

import pytest


@pytest.fixture(scope="module")
def mini_corpus_path(fixtures_dir):
    """Path to minimal test corpus A (5 words: hello, beautiful, world, good, morning)."""
    return str(fixtures_dir / "mini_corpus")


@pytest.fixture(scope="module")
def mini_corpus_b_path(fixtures_dir):
    """Path to minimal test corpus B (3 words: alpha, beta, gamma)."""
    return str(fixtures_dir / "mini_corpus_b")


@pytest.fixture(scope="module")
def mini_corpus_c_path(fixtures_dir):
    """Path to minimal test corpus C (4 words: red, green, blue, yellow)."""
    return str(fixtures_dir / "mini_corpus_c")


@pytest.fixture(scope="module")
def loaded_corpus(mini_corpus_path):
    """Load mini_corpus into the corpus_manager.

    mini_corpus structure:
    - Nodes 1-5: slot nodes (type "word")
      - word values: "hello", "beautiful", "world", "good", "morning"
      - pos values: "interjection", "adjective", "noun", "adjective", "noun"
    - Node 6: phrase containing slots 1-3 ("hello beautiful world")
    - Node 7: phrase containing slots 4-5 ("good morning")
    - Node 8: sentence containing slots 1-5

    Parent edges:
    - Words 1,2,3 -> phrase 6
    - Words 4,5 -> phrase 7
    - Phrases 6,7 -> sentence 8

    Features:
    - word: text content of word nodes
    - pos: part-of-speech tag
    - number: word count per phrase
    - score: test for None vs 0 distinction
    - parent (edge): parent relationships with values
    - distance (edge): edge distances (tests None values)

    Yields:
        The corpus name (for use in multi-corpus tests)
    """
    from cfabric_mcp.corpus_manager import corpus_manager

    # Clear any existing corpora
    for name in list(corpus_manager.list_corpora()):
        corpus_manager.unload(name)

    # Load the mini corpus
    corpus_manager.load(mini_corpus_path, name="mini")

    yield "mini"

    # Cleanup after tests
    for name in list(corpus_manager.list_corpora()):
        corpus_manager.unload(name)


@pytest.fixture(scope="module")
def loaded_api(loaded_corpus):
    """Get the API object from the loaded corpus.

    Use this fixture when you need direct API access for assertions.
    """
    from cfabric_mcp.corpus_manager import corpus_manager

    return corpus_manager.get_api(loaded_corpus)


@pytest.fixture(scope="module")
def multi_corpus(mini_corpus_path, mini_corpus_b_path, mini_corpus_c_path):
    """Load three different corpora for multi-corpus testing.

    Loads three distinct corpora with different data to properly test
    corpus isolation and multi-corpus functionality.

    Corpora:
    - corpus_a: 5 words (hello, beautiful, world, good, morning), 2 phrases, 1 sentence
    - corpus_b: 3 words (alpha, beta, gamma), 1 phrase, 1 sentence
    - corpus_c: 4 words (red, green, blue, yellow), 2 phrases, 1 sentence

    Yields:
        Tuple of (corpus_a_name, corpus_b_name, corpus_c_name)
    """
    from cfabric_mcp.corpus_manager import corpus_manager

    # Clear any existing corpora
    for name in list(corpus_manager.list_corpora()):
        corpus_manager.unload(name)

    # Load three different corpora
    corpus_manager.load(mini_corpus_path, name="corpus_a")
    corpus_manager.load(mini_corpus_b_path, name="corpus_b")
    corpus_manager.load(mini_corpus_c_path, name="corpus_c")

    yield ("corpus_a", "corpus_b", "corpus_c")

    # Cleanup after tests
    for name in list(corpus_manager.list_corpora()):
        corpus_manager.unload(name)
