"""Structure navigation (``T.top/structure/up/down/headingFromNode/...``).

Expected values are Text-Fabric 13.0.19's answers on the same ``mini_corpus``
configurations (``tf/core/text.py``, ``tf/core/prepare.py::structure``):

* configured structure: ``T.up`` gives the parent *structural* node as an int
  (or ``None``), ``T.down`` the tuple of structural children; both return
  ``None`` for nodes whose type is not a structure type;
* no structure configured: every structure accessor returns ``None`` and
  ``T.headings`` is ``()``;
* an unusable configuration (a repeated level, or a different number of types
  and features) makes Text-Fabric crash while loading; Context-Fabric loads the
  corpus and treats it as "no structure configured".

``T.nodeFromHeading`` returns ``None`` whenever structure is not configured,
where Text-Fabric raises ``AttributeError`` after logging the error.
"""

from __future__ import annotations

import pytest

from conftest import LOAD_MODES, open_corpus

SECTIONS = ("@sectionFeatures=sentence_id,phrase_id", "@sectionTypes=sentence,phrase")
STRUCTURE = ("@structureFeatures=sentence_id,phrase_id", "@structureTypes=sentence,phrase")

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

NO_STRUCTURES = {
    "empty": ("@structureFeatures=", "@structureTypes="),
    "absent": (),
}


@pytest.mark.parametrize("mode", LOAD_MODES)
def test_configured_structure_matches_text_fabric(mini_corpus_with_otext, tmp_path, mode):
    api = open_corpus([mini_corpus_with_otext(*SECTIONS, *STRUCTURE)], mode, tmp_path)
    T = api.T

    assert T.headings == (("sentence", "sentence_id"), ("phrase", "phrase_id"))
    assert T.top() == (8,)
    assert T.structure() == ((8, ((6, ()), (7, ()))),)
    assert T.structure(8) == (8, ((6, ()), (7, ())))
    assert T.structure(6) == (6, ())
    assert T.structure(1) is None

    assert T.up(6) == 8
    assert T.up(7) == 8
    assert T.up(8) is None
    assert T.up(1) is None
    assert T.down(8) == (6, 7)
    assert T.down(6) == ()
    assert T.down(1) is None

    assert T.headingFromNode(6) == (("sentence", "S1"), ("phrase", 1))
    assert T.headingFromNode(8) == (("sentence", "S1"),)
    assert T.headingFromNode(1) is None
    assert T.nodeFromHeading((("sentence", "S1"), ("phrase", 1))) == 6
    assert T.nodeFromHeading((("sentence", "S1"),)) == 8
    assert T.nodeFromHeading((("sentence", "zz"),)) is None

    assert T.structurePretty() == "    sentence:S1\n        phrase:1\n        phrase:2"
    assert T.structurePretty(8) == "  sentence:S1\n      phrase:1\n      phrase:2"
    assert T.structurePretty(8, fullHeading=True) == (
        "  sentence:S1\n      sentence:S1-phrase:1\n      sentence:S1-phrase:2"
    )
    assert T.structurePretty(1) is None

    assert "structure" in api.Call()
    assert api.C.structure.data == (
        {6: (("sentence", "S1"), ("phrase", 1)), 7: (("sentence", "S1"), ("phrase", 2)),
         8: (("sentence", "S1"),)},
        {(("sentence", "S1"), ("phrase", 1)): 6, (("sentence", "S1"), ("phrase", 2)): 7,
         (("sentence", "S1"),): 8},
        {},
        (8,),
        {6: 8, 7: 8},
        {8: (6, 7)},
    )


@pytest.mark.parametrize("mode", LOAD_MODES)
def test_heading_values_keep_feature_types(mini_corpus_with_otext, tmp_path, mode):
    """Heading values are the feature values: int for int features, str for str
    features (also when they look numeric), and None where the node has none."""
    corpus = mini_corpus_with_otext(
        *SECTIONS, "@structureFeatures=sentence_id,word", "@structureTypes=sentence,phrase"
    )
    (corpus / "sentence_id.tf").write_text(
        "@node\n@valueType=str\n\n8\t12\n", encoding="utf-8"
    )
    api = open_corpus([corpus], mode, tmp_path)
    T = api.T

    assert T.headingFromNode(8) == (("sentence", "12"),)
    assert T.headingFromNode(6) == (("sentence", "12"), ("phrase", None))
    assert T.nodeFromHeading((("sentence", "12"),)) == 8
    assert T.nodeFromHeading((("sentence", 12),)) is None
    assert T.structurePretty(8, fullHeading=True).splitlines()[1] == (
        "      sentence:12-phrase:None"
    )
    # Both phrases share a heading: the last one wins, both are in hdMult.
    (_, node_from_heading, multiple, *_) = api.C.structure.data
    assert node_from_heading[(("sentence", "12"), ("phrase", None))] == 7
    assert multiple == {(("sentence", "12"), ("phrase", None)): (6, 7)}


@pytest.mark.parametrize("mode", LOAD_MODES)
@pytest.mark.parametrize("structure", NO_STRUCTURES.values(), ids=NO_STRUCTURES)
def test_no_structure_returns_none(mini_corpus_with_otext, tmp_path, mode, structure):
    api = open_corpus([mini_corpus_with_otext(*SECTIONS, *structure)], mode, tmp_path)
    _assert_no_structure(api)


@pytest.mark.parametrize("mode", LOAD_MODES)
@pytest.mark.parametrize("structure", UNUSABLE_STRUCTURES.values(), ids=UNUSABLE_STRUCTURES)
def test_unusable_structure_degrades_to_no_structure(
    mini_corpus_with_otext, tmp_path, mode, structure
):
    api = open_corpus([mini_corpus_with_otext(*SECTIONS, *structure)], mode, tmp_path)
    _assert_no_structure(api)
    assert api.F.word.v(1) == "hello"


def _assert_no_structure(api) -> None:
    assert "structure" not in api.Call()
    with pytest.raises(AttributeError):
        api.C.structure
    T = api.T
    assert T.headings == ()
    assert T.top() is None
    assert T.structure() is None
    assert T.structure(8) is None
    assert T.structurePretty() is None
    for node in (1, 6, 8):
        assert T.up(node) is None
        assert T.down(node) is None
        assert T.headingFromNode(node) is None
    assert T.nodeFromHeading((("sentence", "S1"),)) is None
