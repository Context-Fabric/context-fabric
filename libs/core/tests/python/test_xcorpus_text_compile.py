"""Regression tests for the text-format and multi-location compile defects.

BUG-2: the default text format ``{qere_utf8/g_word_utf8}{qere_trailer_utf8/
trailer_utf8}`` must emit a present empty ``qere_trailer_utf8`` as ``""`` rather
than falling back to ``trailer_utf8`` (a spurious space).

BUG-1: ``Fabric(locations=[a, b]).compile(...)`` must merge warp features from
every location (otype/oslots from one, added features from another) instead of
failing with "missing required feature: otype".

Both need the BHSA corpus (see ``tests/conftest.py``) and the second also the
local parallels module; they skip when those are unavailable.
"""

from __future__ import annotations

from pathlib import Path

import pytest
from cfabric import Fabric

PARALLELS_2021 = Path.home() / "text-fabric-data" / "github" / "etcbc" / "parallels" / "tf" / "2021"


def test_bhsa_qere_empty_trailer_no_spurious_space(bhsa_tf_dir, tmp_path):
    tf_dir = bhsa_tf_dir
    cache = tmp_path / "bhsa.cfr"
    fab = Fabric(locations=[str(tf_dir)], silent="deep")
    fab.compile(
        str(cache),
        "otype oslots qere_utf8 g_word_utf8 qere_trailer_utf8 trailer_utf8",
        "deep",
    )
    co = fab.openMapped(str(cache))
    # word 363617: qere_trailer_utf8 == '' (present) -> no trailing space.
    assert co.F.qere_trailer_utf8.v(363617) == ""
    assert co.T.text(363617) == "מִ"
    # word 4420: qere_trailer_utf8 == '׃' (non-empty) -> emitted as-is.
    assert co.T.text(4420) == "אָהֳלֹֽו׃"


def test_multi_location_compile_merges_otype_and_crossref(bhsa_tf_dir, tmp_path):
    bhsa = bhsa_tf_dir
    if not PARALLELS_2021.is_dir():
        pytest.skip(f"parallels module not available at {PARALLELS_2021}")
    cache = tmp_path / "merged.cfr"
    fab = Fabric(locations=[str(bhsa), str(PARALLELS_2021)], silent="deep")
    # Must not raise "missing required feature: otype".
    fab.compile(str(cache), None, "deep")
    co = fab.openMapped(str(cache))
    assert co.F.otype.v(1) == "word"
    crossref = co.Es("crossref")
    assert crossref is not None
    # Valued crossref edge merged from the parallels location.
    assert crossref.f(1414401) == ((1414407, 84), (1414411, 89))
