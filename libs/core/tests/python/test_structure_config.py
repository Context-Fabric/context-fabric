"""An unusable structure configuration must degrade, not abort loading.

A corpus whose ``otext`` repeats a structure level, or declares a different
number of structure types and features, loads with no structural elements.
Structure accessors then report "nothing" instead of raising, and the rest of
the API is unaffected. ``T.nodeFromHeading`` returns ``None`` whenever
structure is not configured.
"""

from __future__ import annotations

import pytest

from conftest import LOAD_MODES, open_corpus

SECTIONS = ("@sectionFeatures=sentence_id,phrase_id", "@sectionTypes=sentence,phrase")

UNUSABLE_STRUCTURES = {
    "duplicate-levels": (
        "@structureFeatures=sentence_id,phrase_id,phrase_id",
        "@structureTypes=sentence,phrase,phrase",
    ),
    "type-feature-mismatch": (
        "@structureFeatures=sentence_id",
        "@structureTypes=sentence,phrase",
    ),
}


@pytest.mark.parametrize("mode", LOAD_MODES)
@pytest.mark.parametrize("structure", UNUSABLE_STRUCTURES.values(), ids=UNUSABLE_STRUCTURES)
def test_unusable_structure_degrades_to_no_structure(
    mini_corpus_with_otext, tmp_path, mode, structure
):
    api = open_corpus([mini_corpus_with_otext(*SECTIONS, *structure)], mode, tmp_path)
    T = api.T

    assert not T.top()
    assert not T.structure()
    assert T.structure(1) is None
    assert T.headingFromNode(1) is None
    assert T.nodeFromHeading((("sentence", "s1"),)) is None
    assert api.F.word.v(1) == "hello"


@pytest.mark.parametrize("mode", LOAD_MODES)
def test_node_from_heading_without_structure(mini_corpus_with_otext, tmp_path, mode):
    corpus = mini_corpus_with_otext(*SECTIONS, "@structureFeatures=", "@structureTypes=")
    api = open_corpus([corpus], mode, tmp_path)

    assert api.T.nodeFromHeading((("sentence", "s1"),)) is None
