"""Section lookup works for every load mode and section depth.

Selecting only some features, or opening a compiled corpus, must still provide
the section index that ``T.nodeFromSection`` resolves through, including for a
corpus with a single section level.
"""

from __future__ import annotations

import pytest

from conftest import LOAD_MODES, open_corpus

SECTION_TYPES = ("sentence", "phrase", "word")
SECTION_FEATURES = ("sentence_id", "phrase_id", "word")

EXPECTED = {
    ("S1",): 8,
    ("missing",): None,
    ("S1", 1): 6,
    ("S1", 2): 7,
    ("S1", 99): None,
    ("S1", 1, "hello"): 1,
    ("S1", 2, "morning"): 5,
    ("S1", 1, "missing"): None,
}


@pytest.mark.parametrize("mode", LOAD_MODES)
@pytest.mark.parametrize("levels", [1, 2, 3])
def test_node_from_section(mini_corpus_with_otext, tmp_path, mode, levels):
    corpus = mini_corpus_with_otext(
        f"@sectionTypes={','.join(SECTION_TYPES[:levels])}",
        f"@sectionFeatures={','.join(SECTION_FEATURES[:levels])}",
    )
    T = open_corpus([corpus], mode, tmp_path).T

    for section, node in EXPECTED.items():
        if len(section) <= levels:
            assert T.nodeFromSection(section) == node, section


@pytest.mark.parametrize("mode", LOAD_MODES)
def test_duplicate_section_name_resolves_to_the_last_node(mini_corpus_with_otext, tmp_path, mode):
    # TF builds its name -> node map from the feature data, so the last node
    # with a repeated name wins (BHSA book@zh names both Job and Esther 以斯帖记).
    corpus = mini_corpus_with_otext("@sectionFeatures=label", "@sectionTypes=phrase")
    (corpus / "label.tf").write_text("@node\n@valueType=str\n\n6\tX\n7\tX\n", encoding="utf-8")
    api = open_corpus([corpus], mode, tmp_path, features="label")

    assert api.T.nodeFromSection(("X",)) == 7
