"""Tests for the ``sets=`` and ``shallow=`` arguments of the search bindings.

Covers TF-compatible behavior wired in ``libs/core/src/bindings/accessors.rs``:

* ``sets=`` restricts an atom to a custom node set referenced by name.
* ``shallow=True`` (or ``1``) returns a set of first components.
* ``shallow=k`` (k>1) returns a set of k-prefix tuples.
* ``shallow=False``/``0``/``None`` returns the full tuple-of-tuples.

The ``loaded_api`` / ``mini_corpus_path`` fixtures come from ``conftest.py``.
"""

from __future__ import annotations


# In the mini corpus, nodes 1-5 are words, 6 and 7 are phrases (6 covers words
# 1-3, 7 covers words 4-5), and node 8 is the sentence.


def test_sets_restricts_candidates(loaded_api):
    search = loaded_api.S

    # A custom set is referenced by its name in the otype position of an atom.
    restricted = search.search("noun", sets={"noun": [1, 3, 5]})
    assert restricted == ((1,), (3,), (5,))

    # The labelled form `label:setname` also resolves the custom set.
    labelled = search.search("x:noun", sets={"noun": [2, 4]})
    assert labelled == ((2,), (4,))

    # Without the set the same template (a name that is not an otype) is empty.
    assert search.search("noun") == ()


def test_sets_intersect_with_constraints(loaded_api):
    search = loaded_api.S

    # Custom set restricts candidates; feature constraints still apply on top.
    results = search.search('noun pos=noun', sets={"noun": [1, 2, 3, 4, 5]})
    # pos: 1=interjection, 2=adjective, 3=noun, 4=adjective, 5=noun
    assert results == ((3,), (5,))


def test_shallow_true_returns_set_of_ints(loaded_api):
    search = loaded_api.S

    result = search.search("phrase\n  word", shallow=True)
    assert isinstance(result, set)
    # First component (the phrase node) of each matching tuple.
    assert result == {6, 7}
    assert all(isinstance(item, int) for item in result)

    # shallow=1 is equivalent to shallow=True.
    assert search.search("phrase\n  word", shallow=1) == {6, 7}


def test_shallow_k_returns_set_of_prefix_tuples(loaded_api):
    search = loaded_api.S

    result = search.search("phrase\n  word", shallow=2)
    assert isinstance(result, set)
    assert result == {(6, 1), (6, 2), (6, 3), (7, 4), (7, 5)}
    assert all(isinstance(item, tuple) and len(item) == 2 for item in result)


def test_shallow_falsey_returns_full_tuples(loaded_api):
    search = loaded_api.S

    expected = ((6, 1), (6, 2), (6, 3), (7, 4), (7, 5))
    assert search.search("phrase\n  word") == expected
    assert search.search("phrase\n  word", shallow=False) == expected
    assert search.search("phrase\n  word", shallow=0) == expected
    assert search.search("phrase\n  word", shallow=None) == expected


def test_study_fetch_count_honor_sets(loaded_api):
    search = loaded_api.S

    search.study("noun", sets={"noun": [2, 4]})
    assert search.exe.good is True
    assert search.fetch() == ((2,), (4,))
    assert search.count() == 2


def test_study_fetch_count_honor_shallow(loaded_api):
    search = loaded_api.S

    search.study("phrase\n  word", shallow=2)
    fetched = search.fetch()
    assert isinstance(fetched, set)
    assert fetched == {(6, 1), (6, 2), (6, 3), (7, 4), (7, 5)}
    assert search.count() == 5

    search.study("phrase\n  word", shallow=True)
    assert search.fetch() == {6, 7}
    assert search.count() == 2


def test_sets_and_shallow_combined(loaded_api):
    search = loaded_api.S

    # Restrict words to a custom set, then project to first components.
    result = search.search(
        "phrase\n  myword", sets={"myword": [1, 2, 4]}, shallow=True
    )
    assert isinstance(result, set)
    # Phrase 6 covers words 1-3 (sees 1,2); phrase 7 covers 4-5 (sees 4).
    assert result == {6, 7}
