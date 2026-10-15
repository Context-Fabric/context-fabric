"""``freqList`` ordering matches Text-Fabric (``tf/core/nodefeature.py``):
highest frequency first, ties broken by ascending value, where int values
compare numerically (so negative numbers sort before positive ones)."""

from __future__ import annotations

import pytest

from conftest import LOAD_MODES, open_corpus


@pytest.mark.parametrize("mode", LOAD_MODES)
def test_int_freq_list_breaks_ties_numerically(mini_corpus_with_otext, tmp_path, mode):
    corpus = mini_corpus_with_otext("@sectionFeatures=sentence_id", "@sectionTypes=sentence")
    (corpus / "dist.tf").write_text(
        "@node\n@valueType=int\n\n-1\n-2\n10\n9\n-1\n6\t-10\n7\t-9\n", encoding="utf-8"
    )
    api = open_corpus([corpus], mode, tmp_path, features="dist")

    assert api.F.dist.freqList() == ((-1, 2), (-10, 1), (-9, 1), (-2, 1), (9, 1), (10, 1))
    assert api.F.dist.freqList(nodeTypes={"word"}) == ((-1, 2), (-2, 1), (9, 1), (10, 1))


@pytest.mark.parametrize("mode", LOAD_MODES)
def test_str_freq_list_breaks_ties_by_code_point(mini_corpus_with_otext, tmp_path, mode):
    corpus = mini_corpus_with_otext("@sectionFeatures=sentence_id", "@sectionTypes=sentence")
    (corpus / "label.tf").write_text(
        "@node\n@valueType=str\n\nb\nB\na\nä\n10\n", encoding="utf-8"
    )
    api = open_corpus([corpus], mode, tmp_path, features="label")

    assert api.F.label.freqList() == (("10", 1), ("B", 1), ("a", 1), ("b", 1), ("ä", 1))
