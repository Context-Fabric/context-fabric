"""``T.text`` expands a node to the format's target type with ``L.d(node,
otype=target)``, as Text-Fabric does (``tf/core/text.py``): only nodes whose
slots lie inside the node, never the node's own embedders, in canonical order.
"""

from __future__ import annotations

import pytest

from conftest import LOAD_MODES, open_corpus

OTEXT = (
    "@fmt:phrase-id=phrase#{phrase_id}|",
    "@sectionFeatures=sentence_id",
    "@sectionTypes=sentence",
)


@pytest.mark.parametrize("mode", LOAD_MODES)
def test_text_descends_to_embedded_target_nodes(mini_corpus_with_otext, tmp_path, mode):
    api = open_corpus([mini_corpus_with_otext(*OTEXT)], mode, tmp_path)
    T = api.T

    assert T.text(8, fmt="phrase-id") == "1|2|"
    assert T.text(6, fmt="phrase-id") == "1|"
    # A slot embeds no phrase, even one with that single slot.
    assert T.text(1, fmt="phrase-id") == ""


@pytest.mark.parametrize("mode", LOAD_MODES)
def test_text_descend_uses_slot_sets_not_spans(mini_corpus_with_otext, tmp_path, mode):
    corpus = mini_corpus_with_otext(*OTEXT)
    # sentence 8 is gappy (slots 1 and 5); phrase 7 (slots 2-4) lies inside its
    # span but shares none of its slots.
    (corpus / "oslots.tf").write_text(
        "@edge\n@valueType=int\n\n6\t1\n7\t2-4\n8\t1,5\n", encoding="utf-8"
    )
    api = open_corpus([corpus], mode, tmp_path)

    assert api.T.text(8, fmt="phrase-id") == "1|"
    assert api.T.text(1, fmt="phrase-id") == ""


@pytest.mark.parametrize("mode", LOAD_MODES)
def test_text_descend_single_slot_target(mini_corpus_with_otext, tmp_path, mode):
    corpus = mini_corpus_with_otext(*OTEXT)
    # phrase 7 has exactly one slot (5), like a one-word line in BANKS.
    (corpus / "oslots.tf").write_text(
        "@edge\n@valueType=int\n\n6\t1-4\n7\t5\n8\t1-5\n", encoding="utf-8"
    )
    api = open_corpus([corpus], mode, tmp_path)

    assert api.T.text(5, fmt="phrase-id") == ""
    assert api.T.text(5, fmt="phrase-id", descend=True) == ""
    assert api.T.text(7, fmt="phrase-id") == "2|"
