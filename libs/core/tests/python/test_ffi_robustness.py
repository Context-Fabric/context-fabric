"""Edge inputs across the Python surface never escape as Rust panics.

A Rust panic crossing the extension boundary surfaces as
``pyo3_runtime.PanicException`` (a ``BaseException``, so ``except Exception``
does not catch it) or aborts the interpreter. Every public accessor is
hammered with invalid nodes, wrong types, malformed sections, broken search
templates, and corrupted caches: each call must return a value or raise an
ordinary ``Exception``.

Where Text-Fabric answers an invalid node with ``None`` / ``()``, so does
Context-Fabric, including for integers outside the node-number range.
"""

from __future__ import annotations

import itertools
import subprocess
import sys
import shutil
from collections.abc import Callable, Iterable
from pathlib import Path
from typing import Any

import pytest

from cfabric import Fabric

OUT_OF_RANGE_NODES = (-1, -(2**31), 2**32, 2**63, 2**70)
EDGE_NODES = (0, 1, 8, 9, 10**6, *OUT_OF_RANGE_NODES)
WRONG_TYPES = (None, "1", 1.5, [1], (1,), object())
OTYPES = (None, "", "bogus", "word", ("word", "phrase"), {"sentence"}, 5)
SECTIONS = (None, (), (None,), ("",), ("S1",), ("S1", 1), ("S1", -1), ("S1", 2**40),
            ("S1", 1, 1, 1, 1), "S1", 1, [None], ((1,),))
TEMPLATES = ("", "\n", "word", "bogus", "word pos=", "word pos~(", "  word",
             "word\n  phrase", "w:word\np:phrase\nw <99999999999999999999: p",
             "word\n/with/\n", "/without/", "word\n/or/\n", "word\n/-/",
             "word score>abc", "word score<99999999999999999999999", ".. word", "%",
             "x:word\nx < y")
SEARCH_OPTIONS = ({}, {"limit": 0}, {"limit": -1}, {"limit": 2**40}, {"shallow": True},
                  {"shallow": 2}, {"sets": {"word": [0, -1, 2**33]}}, {"sets": 5})


def _assert_no_panic(call: Callable[[], Any]) -> None:
    try:
        result = call()
        if isinstance(result, Iterable) and not isinstance(result, (str, bytes, dict)):
            list(itertools.islice(iter(result), 10))
    except Exception:
        pass


@pytest.fixture(scope="module")
def api(tmp_path_factory):
    source = Path(__file__).resolve().parents[1] / "fixtures" / "mini_corpus"
    corpus = tmp_path_factory.mktemp("ffi") / "mini"
    shutil.copytree(source, corpus, ignore=shutil.ignore_patterns(".cf*"))
    return Fabric(locations=str(corpus), silent="deep").loadAll(silent="deep")


# --------------------------------------------------------------------------- #
# Invalid nodes answer like Text-Fabric
# --------------------------------------------------------------------------- #


@pytest.mark.parametrize("node", OUT_OF_RANGE_NODES)
def test_out_of_range_integers_are_absent_nodes(api, node):
    for name in api.Fall():
        assert api.Fs(name).v(node) is None
    for name in api.Eall():
        edge = api.Es(name)
        for method in (edge.f, edge.t, edge.b):
            assert method(node) == ()
    for method in (api.L.u, api.L.d, api.L.n, api.L.p, api.L.i):
        assert method(node) == ()


@pytest.mark.parametrize("node", (0, 9, 10**6, -1, 2**40))
def test_oslots_of_a_nonexistent_node_is_empty(api, node):
    assert api.E.oslots.s(node) == ()


def test_wrong_node_types_raise_type_error(api):
    for value in WRONG_TYPES:
        with pytest.raises(TypeError):
            api.F.word.v(value)


# --------------------------------------------------------------------------- #
# Fuzz: every public accessor with edge inputs
# --------------------------------------------------------------------------- #


def test_node_accessors_never_panic(api):
    for node in (*EDGE_NODES, *WRONG_TYPES):
        for name in api.Fall():
            feature = api.Fs(name)
            for method in (feature.v, feature.s):
                _assert_no_panic(lambda: method(node))
            _assert_no_panic(lambda: feature.vs([node, 1]))
            _assert_no_panic(lambda: feature.vs_array([node, 1]))
        for name in api.Eall():
            edge = api.Es(name)
            for method in (edge.f, edge.t, edge.s, edge.b):
                _assert_no_panic(lambda: method(node))
        for otype in OTYPES:
            for method in (api.L.u, api.L.d, api.L.n, api.L.p, api.L.i):
                _assert_no_panic(lambda: method(node, otype=otype))
            _assert_no_panic(lambda: api.L.u_many([node], otype))
            _assert_no_panic(lambda: api.L.d_many([node], otype))
        text = api.T
        section_0_name = getattr(text, f"{text.sectionTypes[0]}Name")
        for method in (text.text, text.sectionFromNode, text.sectionTuple,
                       text.headingFromNode, text.structure, text.up, text.down,
                       text.structurePretty, section_0_name):
            _assert_no_panic(lambda: method(node))
        for fmt in (None, "", "bogus", "text-orig-full", 5):
            _assert_no_panic(lambda: text.text(node, fmt=fmt, descend=True))
            _assert_no_panic(lambda: text.text([node, node], fmt=fmt))
            _assert_no_panic(lambda: text.text_many([node], fmt=fmt))
        _assert_no_panic(lambda: text.sectionFromNode(node, lastSlot=True, fillup=True))
        _assert_no_panic(lambda: text.sectionFromNode(node, lang="zz", level=99))
        nodes = api.N
        _assert_no_panic(lambda: nodes.sortKey(node))
        _assert_no_panic(lambda: nodes.sortKeyTuple([node, 1]))
        _assert_no_panic(lambda: nodes.sortNodes([node, 1]))
        _assert_no_panic(lambda: nodes.sortKeyChunk((node, (node, node))))
        _assert_no_panic(lambda: nodes.sortKeyChunkLength((node, (node, 0))))
        _assert_no_panic(lambda: nodes.walk(nodes=[node]))
        _assert_no_panic(lambda: api.S.glean((node,)))
        for computed in api.Call():
            data = api.Cs(computed).data
            _assert_no_panic(lambda: data[node])
            _assert_no_panic(lambda: node in data)


def test_section_and_heading_lookups_never_panic(api):
    for section in SECTIONS:
        for lang in ("en", "zz", None):
            _assert_no_panic(lambda: api.T.nodeFromSection(section, lang=lang))
        _assert_no_panic(lambda: api.T.nodeFromHeading(section))
        _assert_no_panic(lambda: getattr(api.T, f"{api.T.sectionTypes[0]}Node")(section))
    _assert_no_panic(api.T.top)
    _assert_no_panic(api.T.structure)
    _assert_no_panic(api.T.structurePretty)


def test_search_never_panics(api):
    for template in TEMPLATES:
        _assert_no_panic(lambda: api.S.study(template))
        for options in SEARCH_OPTIONS:
            _assert_no_panic(lambda: api.S.search(template, **options))
        _assert_no_panic(lambda: api.S.count())
        _assert_no_panic(lambda: api.S.fetch(limit=-1))
        _assert_no_panic(lambda: api.S.showPlan(details=True))


def test_feature_lookups_never_panic(api):
    for name in api.Fall():
        feature = api.Fs(name)
        for value in (None, "", 0, -1, 2**70, 1.5, [1]):
            _assert_no_panic(lambda: feature.s(value))
        for node_types in (None, "", "bogus", 5, ["word"], {"word"}):
            _assert_no_panic(lambda: feature.freqList(node_types))
    for name in api.Eall():
        for node_types in (None, "", "bogus", 5, ["word"]):
            _assert_no_panic(lambda: api.Es(name).freqList(node_types))
    for name in ("", "bogus", "0", "__class__"):
        _assert_no_panic(lambda: getattr(api.F, name))
        _assert_no_panic(lambda: getattr(api.E, name))
        _assert_no_panic(lambda: api.Fs(name))
        _assert_no_panic(lambda: api.Es(name))
        _assert_no_panic(lambda: api.Cs(name))


# --------------------------------------------------------------------------- #
# Missing and corrupted caches fail with ordinary exceptions
# --------------------------------------------------------------------------- #


def test_opening_a_missing_cache_raises_file_not_found(tmp_path):
    with pytest.raises(FileNotFoundError):
        Fabric(silent="deep").openMapped(str(tmp_path / "missing.cfr"))


def test_loading_a_missing_corpus_raises_file_not_found(tmp_path):
    with pytest.raises(FileNotFoundError):
        Fabric(locations=str(tmp_path / "nowhere"), silent="deep").loadAll()
    assert not (tmp_path / "nowhere").exists()


def test_corrupted_caches_raise_value_error(api, tmp_path):
    source = tmp_path / "good.cfr"
    corpus = Path(__file__).resolve().parents[1] / "fixtures" / "mini_corpus"
    Fabric(locations=str(corpus), silent="deep").compile(str(source))
    data = source.read_bytes()
    damaged = tmp_path / "damaged.cfr"
    for blob in (b"", b"not a cache at all", data[:8], data[: len(data) // 2]):
        damaged.write_bytes(blob)
        with pytest.raises(ValueError, match="damaged.cfr"):
            Fabric(silent="deep").openMapped(str(damaged))


def test_vs_array_without_numpy_raises_import_error(tmp_path):
    """numpy is optional; its absence must not surface as a Rust panic."""
    corpus = Path(__file__).resolve().parents[1] / "fixtures" / "mini_corpus"
    shutil.copytree(corpus, tmp_path / "mini", ignore=shutil.ignore_patterns(".cf*"))
    script = (
        "import sys\n"
        "sys.modules['numpy'] = None  # make `import numpy` fail\n"
        "from cfabric import Fabric\n"
        f"api = Fabric(locations={str(tmp_path / 'mini')!r}, silent='deep').loadAll()\n"
        "try:\n"
        "    api.F.score.vs_array([1, 2])\n"
        "except ImportError as error:\n"
        "    print('ImportError', error)\n"
    )
    result = subprocess.run(
        [sys.executable, "-c", script], capture_output=True, text=True, cwd=tmp_path, check=False
    )
    assert "panicked" not in result.stderr, result.stderr
    assert result.stdout.startswith("ImportError"), result.stdout + result.stderr
    assert "context-fabric[numpy]" in result.stdout


def test_batch_lookups_ignore_rows_beyond_max_node_in_a_corrupted_cache(tmp_path):
    """A row naming a node past maxNode must not index out of bounds."""
    corpus = Path(__file__).resolve().parents[1] / "fixtures" / "mini_corpus"
    cache = tmp_path / "mini.cfr"
    Fabric(locations=str(corpus), silent="deep").compile(str(cache))
    data = bytearray(cache.read_bytes())
    # Node feature header: name (u32 length + bytes), encoding tag (u8), row
    # count, aux count, payload length (u32 each); the payload starts with its
    # own row count, followed by the first row's node number.
    name = b"score"
    header = len(name).to_bytes(4, "little") + name
    first_row_node = data.index(header) + len(header) + 1 + 4 * 3 + 4
    assert int.from_bytes(data[first_row_node : first_row_node + 4], "little") == 1
    data[first_row_node : first_row_node + 4] = (10**6).to_bytes(4, "little")
    cache.write_bytes(bytes(data))

    api = Fabric(silent="deep").openMapped(str(cache))
    assert api.F.score.vs(list(range(1, 9)))[0] is None
    assert api.F.score.vs_array(list(range(1, 9)))[0] == 0
