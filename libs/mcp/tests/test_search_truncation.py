"""Exact totals must not depend on the number of retained cursor results."""

import csv
from types import SimpleNamespace
from unittest.mock import Mock

import pytest
from cfabric_mcp import tools
from cfabric_mcp.cache import SearchCache


@pytest.fixture
def bounded_search(monkeypatch):
    cache = SearchCache(max_results_per_entry=3)
    search_api = Mock()
    search_api.exe = SimpleNamespace(good=True)
    search_api.search.return_value = [(n,) for n in range(1, 6)]
    api = SimpleNamespace(
        S=search_api,
        F=SimpleNamespace(otype=SimpleNamespace(v=lambda n: "word")),
        T=SimpleNamespace(),
        Fs=lambda name, warn=False: SimpleNamespace(
            v=lambda n: "tail" if n > 3 else "prefix"
        ),
    )
    monkeypatch.setattr(tools, "get_cache", lambda: cache)
    monkeypatch.setattr(tools.corpus_manager, "get_api", lambda corpus: api)
    monkeypatch.setattr(
        tools.NodeInfo,
        "from_api",
        lambda api, n: SimpleNamespace(
            node=n,
            otype="word",
            text=str(n),
            section_ref=str(n),
            to_dict=lambda: {"node": n, "otype": "word", "text": str(n)},
        ),
    )
    return cache, search_api


def test_count_stays_exact_on_cache_miss_and_hit(bounded_search):
    cache, search_api = bounded_search
    for _ in range(2):
        assert (
            tools.search("word", return_type="count", corpus="test")["total_count"] == 5
        )
    search_api.search.assert_called_once_with("word")
    assert cache.stats()["total_results_cached"] == 3


def test_statistics_include_matches_beyond_cached_prefix(bounded_search):
    tools.search("word", return_type="count", corpus="test")
    result = tools.search(
        "word", return_type="statistics", aggregate_features=["kind"], corpus="test"
    )
    assert result["total_count"] == 5
    assert result["nodes"]["0_word"]["count"] == 5
    assert {
        item["value"]: item["count"]
        for item in result["nodes"]["0_word"]["distributions"]["kind"]
    } == {"prefix": 3, "tail": 2}


def test_statistics_on_cache_miss_execute_only_once(bounded_search):
    _, search_api = bounded_search
    result = tools.search(
        "word", return_type="statistics", aggregate_features=["kind"], corpus="test"
    )
    assert result["total_count"] == 5
    search_api.search.assert_called_once_with("word")


def test_statistics_reexecution_errors_are_not_reported_as_zero(bounded_search):
    _, search_api = bounded_search
    tools.search("word", return_type="count", corpus="test")
    search_api.search.side_effect = RuntimeError("query failed")
    result = tools.search(
        "word", return_type="statistics", aggregate_features=["kind"], corpus="test"
    )
    assert "query failed" in result["error"]
    assert "total_count" not in result


def test_pages_report_full_total_and_explicit_cache_limit(bounded_search):
    first = tools.search("word", corpus="test", limit=2)
    assert first["total_count"] == 5
    assert first["cached_count"] == 3
    assert first["results_truncated"] is True
    assert first["cursor"]["has_more"] is True
    last = tools.search_continue(first["cursor"]["id"], offset=2, limit=2)
    assert last["total_count"] == 5
    assert last["cached_count"] == 3
    assert last["results_truncated"] is True
    assert last["cursor"]["has_more"] is False
    assert last["results"][0][0]["node"] == 3


def test_search_passages_report_full_total_and_cache_limit(bounded_search):
    result = tools.search("word", return_type="passages", corpus="test", limit=3)
    assert result["total_count"] == 5
    assert result["cached_count"] == 3
    assert result["results_truncated"] is True
    assert result["has_more"] is False


@pytest.mark.parametrize("size", [0, 3])
def test_untruncated_counts_remain_exact(bounded_search, size):
    _, search_api = bounded_search
    search_api.search.return_value = [(n,) for n in range(size)]
    assert (
        tools.search("word", return_type="count", corpus="test")["total_count"] == size
    )


def test_cache_page_total_is_not_the_retained_length():
    cache = SearchCache(max_results_per_entry=3)
    entry = cache.get_or_execute("test", "word", lambda: [(n,) for n in range(5)])
    assert cache.get_page(entry.cursor_id, offset=2, limit=2) == ([(2,)], False, 5)


@pytest.mark.parametrize("return_type", ["results", "passages"])
def test_empty_pages_report_no_cache_truncation(bounded_search, return_type):
    _, search_api = bounded_search
    search_api.search.return_value = []
    result = tools.search("word", return_type=return_type, corpus="test")
    assert result["total_count"] == 0
    assert result["cached_count"] == 0
    assert result["results_truncated"] is False


@pytest.mark.parametrize("warm", [False, True])
@pytest.mark.parametrize("limit", [2, 5])
def test_csv_export_uses_full_matches_when_needed(
    bounded_search, tmp_path, warm, limit
):
    cache, search_api = bounded_search
    tools.set_transport("stdio")
    if warm:
        tools.search("word", return_type="count", corpus="test")
    path = tmp_path / "matches.csv"
    result = tools.search_csv("word", str(path), limit=limit, corpus="test")
    assert result["total_count"] == 5
    assert result["rows_written"] == limit
    with path.open(newline="") as stream:
        rows = list(csv.DictReader(stream))
    assert [int(row["node0_node"]) for row in rows] == list(range(1, limit + 1))
    assert search_api.search.call_count == (2 if warm and limit > 3 else 1)
    assert cache.stats()["total_results_cached"] == 3


def test_csv_reexecution_error_does_not_write_partial_export(bounded_search, tmp_path):
    _, search_api = bounded_search
    tools.set_transport("stdio")
    tools.search("word", return_type="count", corpus="test")
    search_api.search.side_effect = RuntimeError("query failed")
    path = tmp_path / "matches.csv"
    result = tools.search_csv("word", str(path), limit=5, corpus="test")
    assert "query failed" in result["error"]
    assert not path.exists()
