"""Numbers that are not nodes of the corpus in node-list arguments.

Calls that order or select nodes (``N.sortNodes``, ``N.walk(nodes=...)``,
search ``sets=``) raise ``IndexError`` naming the number, for anything outside
``1..=maxNode`` (Text-Fabric raises ``IndexError`` above ``maxNode`` too).
Per-node lookups over a list (``F.<f>.vs``, ``L.u_many``, ``T.text_many``,
...) answer each such number like their single-node call: ``None`` / ``()`` /
``""``.

``mini_corpus`` has nodes 1 to 8.
"""

from __future__ import annotations

import pytest

NOT_NODES = [10**20, -1, 0, 9]


@pytest.mark.parametrize("number", NOT_NODES)
def test_sort_nodes_raises_index_error_naming_the_number(loaded_api, number):
    with pytest.raises(IndexError, match=rf"^node {number} is out of range: the nodes are 1 to 8$"):
        loaded_api.N.sortNodes([1, number])


@pytest.mark.parametrize("number", NOT_NODES)
def test_walk_over_nodes_raises_index_error(loaded_api, number):
    with pytest.raises(IndexError, match=rf"^node {number} "):
        loaded_api.N.walk(nodes=[1, number])


@pytest.mark.parametrize("number", NOT_NODES)
def test_search_sets_raise_index_error(loaded_api, number):
    with pytest.raises(IndexError, match=rf"^node {number} "):
        loaded_api.S.search("x", sets={"x": {1, number}})
    with pytest.raises(IndexError, match=rf"^node {number} "):
        loaded_api.S.study("x", sets={"x": [number]})


def test_corpus_nodes_are_accepted(loaded_api):
    assert loaded_api.N.sortNodes([2, 8]) == [8, 2]
    assert tuple(loaded_api.S.search("x", sets={"x": {1, 8}})) == ((1,), (8,))


@pytest.mark.parametrize("number", NOT_NODES)
def test_batch_lookups_answer_like_single_calls(loaded_api, number):
    api = loaded_api
    assert api.F.word.vs([1, number]) == [api.F.word.v(1), api.F.word.v(number)]
    assert api.F.word.v(number) is None
    assert api.L.u_many([1, number]) == [api.L.u(1), api.L.u(number)]
    assert api.L.d_many([8, number]) == [api.L.d(8), api.L.d(number)]
    assert api.L.u(number) == () and api.L.d(number) == ()
    assert api.T.text_many([1, number, [1, number]]) == [
        api.T.text(1),
        api.T.text(number),
        api.T.text(1),
    ]
    assert api.T.text([1, number]) == api.T.text(1)
    assert api.N.sortKeyTuple((1, number)) == (api.N.sortKey(1), api.N.sortKey(number))
    assert api.S.glean((1, number)) == f"{api.S.glean((1,))} {api.S.glean((number,))}"


@pytest.mark.parametrize("number", NOT_NODES)
def test_vs_array_fills_numbers_that_are_not_nodes(loaded_api, number):
    pytest.importorskip("numpy")
    assert loaded_api.F.number.vs_array([number], fill=-7).tolist() == [-7]
