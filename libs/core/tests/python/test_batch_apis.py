"""Tests for the additive batch APIs (latency workstream W4).

Covers ``F.<feat>.vs`` / ``F.<feat>.vs_array`` (NodeFeature), ``L.u_many`` /
``L.d_many`` (Locality) and ``T.text_many`` (Text). Every test proves parity
with the corresponding per-call scalar method so the batch path can never
silently diverge in semantics.

The headline "10k random nodes" parity requirement is exercised on a 10k random
sample drawn (with replacement) from the corpus node-id range, plus deliberate
out-of-range / absent ids, so the absent-value branch is covered too.
"""

from __future__ import annotations

import random

import numpy as np
import pytest


@pytest.fixture
def api(loaded_api):
    return loaded_api


@pytest.fixture
def all_nodes(api):
    return list(range(1, api.F.otype.maxNode + 1))


@pytest.fixture
def words(api, all_nodes):
    return [n for n in all_nodes if api.F.otype.v(n) == "word"]


def _random_sample(max_node: int, count: int = 10_000) -> list[int]:
    rng = random.Random(20260610)
    # In-range ids plus absent/out-of-range ids (0 and beyond maxNode) so the
    # None / fill branch is covered.
    sample = [rng.randint(1, max_node) for _ in range(count)]
    sample += [0, max_node + 1, max_node + 50]
    rng.shuffle(sample)
    return sample


# --------------------------------------------------------------------------- #
# F.<feat>.vs
# --------------------------------------------------------------------------- #
def test_vs_parity_string_feature(api, all_nodes):
    F = api.F
    assert F.pos.vs(all_nodes) == [F.pos.v(n) for n in all_nodes]


def test_vs_parity_int_feature(api, all_nodes):
    F = api.F
    assert F.number.vs(all_nodes) == [F.number.v(n) for n in all_nodes]


def test_vs_accepts_diverse_iterables(api, words):
    F = api.F
    expected = [F.pos.v(n) for n in words]
    assert F.pos.vs(tuple(words)) == expected
    assert F.pos.vs(iter(words)) == expected
    assert F.pos.vs(np.array(words, dtype=np.int64)) == expected
    assert F.pos.vs(np.array(words, dtype=np.uint32)) == expected
    lo, hi = min(words), max(words) + 1
    assert F.pos.vs(range(lo, hi)) == [F.pos.v(n) for n in range(lo, hi)]


def test_vs_returns_list(api, words):
    assert isinstance(api.F.pos.vs(words), list)


def test_vs_10k_random_parity(api):
    F = api.F
    sample = _random_sample(F.otype.maxNode)
    assert F.pos.vs(sample) == [F.pos.v(n) for n in sample]
    assert F.number.vs(sample) == [F.number.v(n) for n in sample]


# --------------------------------------------------------------------------- #
# F.<feat>.vs_array
# --------------------------------------------------------------------------- #
def test_vs_array_dtype_and_parity(api, all_nodes):
    F = api.F
    arr = F.number.vs_array(all_nodes)
    assert isinstance(arr, np.ndarray)
    assert arr.dtype == np.int64
    expected = [v if (v := F.number.v(n)) is not None else 0 for n in all_nodes]
    assert arr.tolist() == expected


def test_vs_array_fill_sentinel(api):
    F = api.F
    sample = _random_sample(F.otype.maxNode)
    arr = F.number.vs_array(sample, fill=-1)
    expected = [v if (v := F.number.v(n)) is not None else -1 for n in sample]
    assert arr.tolist() == expected


def test_vs_array_rejects_non_int_feature(api, words):
    with pytest.raises(TypeError):
        api.F.pos.vs_array(words)


# --------------------------------------------------------------------------- #
# L.u_many / L.d_many
# --------------------------------------------------------------------------- #
def test_u_many_parity(api, all_nodes):
    L = api.L
    assert L.u_many(all_nodes) == [L.u(n) for n in all_nodes]


def test_d_many_parity(api, all_nodes):
    L = api.L
    assert L.d_many(all_nodes) == [L.d(n) for n in all_nodes]


def test_u_many_otype_filter_parity(api, words):
    L = api.L
    assert L.u_many(words, otype="sentence") == [L.u(n, otype="sentence") for n in words]
    assert L.d_many(words, otype="word") == [L.d(n, otype="word") for n in words]


def test_u_many_returns_list_of_tuples(api, words):
    rows = api.L.u_many(words)
    assert isinstance(rows, list)
    assert all(isinstance(r, tuple) for r in rows)


def test_u_d_many_10k_random_parity(api):
    L = api.L
    sample = [n for n in _random_sample(api.F.otype.maxNode) if 1 <= n <= api.F.otype.maxNode]
    assert L.u_many(sample) == [L.u(n) for n in sample]
    assert L.d_many(sample) == [L.d(n) for n in sample]


# --------------------------------------------------------------------------- #
# T.text_many
# --------------------------------------------------------------------------- #
def test_text_many_scalar_parity(api, all_nodes):
    T = api.T
    assert T.text_many(all_nodes) == [T.text(n) for n in all_nodes]


def test_text_many_node_lists_parity(api, words):
    T = api.T
    chunks = [words, words[:1], words[1:3]]
    assert T.text_many(chunks) == [T.text(c) for c in chunks]


def test_text_many_mixed_scalars_and_lists(api, words):
    T = api.T
    items = [words[0], words[:2], words[2]]
    assert T.text_many(items) == [
        T.text(words[0]),
        T.text(words[:2]),
        T.text(words[2]),
    ]


def test_text_many_respects_fmt(api, words):
    T = api.T
    fmt = next(iter(T.formats), None)
    assert T.text_many(words, fmt=fmt) == [T.text(n, fmt=fmt) for n in words]


def test_text_many_returns_list_of_str(api, words):
    out = api.T.text_many(words)
    assert isinstance(out, list)
    assert all(isinstance(s, str) for s in out)
