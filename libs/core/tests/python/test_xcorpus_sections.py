"""Regression tests for the section-heading defects (BUG-4 / BUG-5).

BUG-4: the top section heading must use the configured ``sectionFeatures[0]``
(and only its ``@<lang>`` variants), never an unrelated feature that happens to
carry a ``languageCode`` (e.g. quran's ``name@en``).

BUG-5: ``T.nodeFromSection`` must resolve via the CFRSECT1 index and return
``None`` immediately on a miss — never enter the multi-second linear scan.

These compile the corpora shipped in ``libs/benchmarks/.corpora`` to a temp
``.cfr``; they skip automatically when a corpus is unavailable.
"""

from __future__ import annotations

import time
from pathlib import Path

import pytest

from cfabric import Fabric

CORPORA = Path(__file__).resolve().parents[2].parent / "benchmarks" / ".corpora"


def _open(corpus: str, tmp_path: Path):
    tf_dir = CORPORA / corpus / "tf"
    if not tf_dir.is_dir():
        pytest.skip(f"{corpus} corpus not available at {tf_dir}")
    cache = tmp_path / f"{corpus}.cfr"
    fab = Fabric(locations=[str(tf_dir)], silent="deep")
    fab.compile(str(cache), None, "deep")
    return fab.openMapped(str(cache))


def test_quran_top_section_uses_number_not_name(tmp_path):
    """quran @sectionFeatures=number,number — sura heading must be the number."""
    co = _open("quran", tmp_path)
    assert tuple(co.T.sectionFeatures) == ("number", "number")
    # node 128220 is sura 1 (TF: (1, 1), CF previously returned 'The Opening').
    assert co.T.sectionFromNode(128220) == (1, 1)
    assert co.T.sectionFromNode(128450) == (2, 224)


def test_quran_node_from_section_round_trips_and_fast_miss(tmp_path):
    co = _open("quran", tmp_path)
    # Round trip via the number-based heading (the TF head shape).
    assert co.T.nodeFromSection((1, 1)) == 128220
    assert co.T.nodeFromSection((2, 224)) == 128450
    # A miss must return None quickly (regression guard against the ~26s scan).
    start = time.time()
    assert co.T.nodeFromSection((999, 999)) is None
    assert co.T.nodeFromSection((1, 99999)) is None
    elapsed = time.time() - start
    assert elapsed < 5.0, f"nodeFromSection miss took {elapsed:.2f}s (expected fast index miss)"


def test_bhsa_book_section_language_variants(tmp_path):
    """The BUG-4 fix must keep BHSA's book@<lang> variants working."""
    tf_dir = CORPORA / "bhsa" / "tf"
    if not tf_dir.is_dir():
        pytest.skip("bhsa corpus not available")
    cache = tmp_path / "bhsa.cfr"
    fab = Fabric(locations=[str(tf_dir)], silent="deep")
    # Compile a small subset that includes the English book-name variant.
    fab.compile(str(cache), "otype oslots book book@en chapter verse", "deep")
    co = fab.openMapped(str(cache))

    # Find the 'Numeri' (Latin base) book node.
    book = next(
        n
        for n in range(1, co.F.otype.maxNode + 1)
        if co.F.otype.v(n) == "book" and co.F.book.v(n) == "Numeri"
    )
    assert co.T.sectionFromNode(book, lang="en") == ("Numbers",)
    # Unconfigured language falls back to the base 'book' feature (Latin).
    assert co.T.sectionFromNode(book, lang="xx") == ("Numeri",)
    # Round trip via the English name resolves back to the same node, fast.
    assert co.T.nodeFromSection(("Numbers",), lang="en") == book
