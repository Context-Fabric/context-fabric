from __future__ import annotations

import os
from pathlib import Path
from typing import Any

import pytest

from cfabric import Fabric

pytestmark = pytest.mark.skipif(
    os.environ.get("CONTEXT_FABRIC_RUN_PARITY") != "1",
    reason="set CONTEXT_FABRIC_RUN_PARITY=1 to run Text-Fabric oracle parity tests",
)

ROOT = Path(__file__).resolve().parents[4]
CORPORA = ROOT / "libs" / "benchmarks" / ".corpora"


def _tf_fabric() -> Any:
    return pytest.importorskip("tf.fabric").Fabric


def _corpus_path(name: str) -> Path:
    path = CORPORA / name / "tf"
    if not (path / "otype.tf").exists():
        pytest.skip(f"missing corpus at {path}")
    return path


def _load_pair(name: str) -> tuple[Any, Any]:
    path = _corpus_path(name)
    tf_api = _tf_fabric()(locations=str(path), silent="deep").loadAll(silent="deep")
    cf_api = Fabric(locations=str(path), silent="deep").loadAll(silent="deep")
    return tf_api, cf_api


def _normalize(value: Any) -> Any:
    if isinstance(value, tuple):
        return tuple(_normalize(item) for item in value)
    if isinstance(value, list):
        return tuple(_normalize(item) for item in value)
    if isinstance(value, dict):
        return {key: _normalize(item) for key, item in value.items()}
    try:
        import numpy as np

        if isinstance(value, np.integer):
            return int(value)
    except Exception:
        pass
    return value


def _as_multiset(rows: Any) -> dict[Any, int]:
    if isinstance(rows, int):
        return {(rows,): 1}
    counts: dict[Any, int] = {}
    for row in _normalize(rows):
        counts[row] = counts.get(row, 0) + 1
    return counts


def _sample_nodes(api: Any, per_type: int = 5) -> list[int]:
    nodes: set[int] = set()
    for _node_type, _level, first, last in api.C.levels.data:
        first = int(first)
        last = int(last)
        if first > last:
            continue
        midpoint = first + ((last - first) // 2)
        step = max(1, (last - first) // max(1, per_type - 1))
        candidates = {first, midpoint, last}
        candidates.update(range(first, min(last + 1, first + step * per_type), step))
        nodes.update(node for node in candidates if first <= node <= last)
    return sorted(nodes)


@pytest.mark.parametrize("corpus", ["banks", "n1904", "bhsa"])
def test_text_navigation_and_nodes_match_text_fabric(corpus: str) -> None:
    tf_api, cf_api = _load_pair(corpus)
    sample = _sample_nodes(cf_api, per_type=4)

    assert set(cf_api.N.otypeRank) == set(tf_api.N.otypeRank)
    assert _as_multiset(cf_api.N.walk()) == _as_multiset(tf_api.N.walk())
    assert tuple(cf_api.N.sortNodes(reversed(sample))) == tuple(tf_api.N.sortNodes(reversed(sample)))
    assert _normalize(cf_api.T.top()) == _normalize(tf_api.T.top())
    assert _normalize(cf_api.T.structure()) == _normalize(tf_api.T.structure())

    for node in sample:
        assert cf_api.T.text(node) == tf_api.T.text(node)
        assert _normalize(cf_api.T.sectionFromNode(node)) == _normalize(tf_api.T.sectionFromNode(node))
        assert _normalize(cf_api.T.headingFromNode(node)) == _normalize(tf_api.T.headingFromNode(node))
        assert _normalize(cf_api.L.u(node)) == _normalize(tf_api.L.u(node))
        assert _as_multiset(cf_api.L.d(node)) == _as_multiset(tf_api.L.d(node))
        assert _normalize(cf_api.L.n(node)) == _normalize(tf_api.L.n(node))
        assert _normalize(cf_api.L.p(node)) == _normalize(tf_api.L.p(node))
        assert set(cf_api.L.i(node)) == set(tf_api.L.i(node))


@pytest.mark.parametrize("corpus", ["banks", "n1904", "bhsa"])
def test_feature_values_and_frequencies_match_text_fabric(corpus: str) -> None:
    tf_api, cf_api = _load_pair(corpus)
    sample = _sample_nodes(cf_api, per_type=4)

    for feature in cf_api.Fall(warp=False):
        tf_feature = tf_api.Fs(feature, warn=False)
        cf_feature = cf_api.Fs(feature, warn=False)
        assert cf_feature is not None
        assert tf_feature is not None
        for node in sample:
            assert _normalize(cf_feature.v(node)) == _normalize(tf_feature.v(node))
        assert _as_multiset(cf_feature.freqList()) == _as_multiset(tf_feature.freqList())

    for feature in cf_api.Eall(warp=False):
        tf_feature = tf_api.Es(feature, warn=False)
        cf_feature = cf_api.Es(feature, warn=False)
        assert cf_feature is not None
        assert tf_feature is not None
        for node in sample:
            assert _normalize(cf_feature.f(node)) == _normalize(tf_feature.f(node))
            assert _normalize(cf_feature.t(node)) == _normalize(tf_feature.t(node))
            assert _as_multiset(cf_feature.b(node)) == _as_multiset(tf_feature.b(node))
        tf_freq = None
        try:
            tf_freq = tf_feature.freqList()
        except TypeError:
            # Text-Fabric cannot sort mixed None/int valued-edge frequency rows.
            pass
        if tf_freq is not None:
            assert _as_multiset(cf_feature.freqList()) == _as_multiset(tf_freq)


def test_curated_bhsa_queries_match_text_fabric() -> None:
    tf_api, cf_api = _load_pair("bhsa")
    queries = pytest.importorskip(
        "cfabric_benchmarks.queries.curated"
    ).get_bhsa_queries()

    for query in queries:
        expected = sorted(_normalize(list(tf_api.S.search(query.template))))
        actual = sorted(_normalize(list(cf_api.S.search(query.template))))
        assert actual == expected, query.id
