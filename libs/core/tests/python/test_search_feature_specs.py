"""Feature and edge-value conditions in search templates follow Text-Fabric
(``tf/search/syntax.py::parseFeatureVals``, ``tf/search/relations.py``).

Expected results were produced by Text-Fabric 13.0.19 on ``mini_corpus``,
whose ``distance`` edges carry int values, some of them missing (None).
"""

from __future__ import annotations

import pytest

from conftest import LOAD_MODES, open_corpus

CASES = {
    # node features: `f*` imposes nothing, `f` needs a value, `f#` needs none
    "word score*": [(1,), (2,), (3,), (4,), (5,)],
    "phrase score*": [(6,), (7,)],
    "word score": [(1,), (2,), (4,), (5,)],
    "word score#": [(3,)],
    "word score>-1": [(1,), (2,), (4,), (5,)],
    # edge values
    "w1:word\nw2:word\nw1 -distance> w2": [(1, 2), (1, 3), (2, 3), (3, 4), (4, 5)],
    "w1:word\nw2:word\nw1 -distance*> w2": [(1, 2), (1, 3), (2, 3), (3, 4), (4, 5)],
    "w1:word\nw2:word\nw1 -distance#> w2": [(2, 3)],
    "w1:word\nw2:word\nw1 -distance=5> w2": [(1, 3)],
    "w1:word\nw2:word\nw1 -distance#0> w2": [(1, 3), (2, 3), (4, 5)],
    "w1:word\nw2:word\nw1 -distance>0> w2": [(1, 3), (4, 5)],
    "w1:word\nw2:word\nw1 -distance<5> w2": [(1, 2), (3, 4)],
    "w1:word\nw2:word\nw1 <distance>0- w2": [(3, 1), (5, 4)],
    "w1:word\nw2:word\nw1 <distance#0> w2": [(1, 3), (2, 3), (3, 1), (3, 2), (4, 5), (5, 4)],
    "w1:word\nw2:word\nw1 <distance=0|10> w2": [(1, 2), (2, 1), (3, 4), (4, 3), (4, 5), (5, 4)],
    "w1:word\np:phrase\nw1 -relation#subject> p": [(2, 6), (3, 6), (5, 7)],
    "w1:word\np:phrase\nw1 -relation~ect$> p": [(1, 6), (3, 6), (4, 7)],
}


@pytest.mark.parametrize("mode", LOAD_MODES)
def test_feature_specs_match_text_fabric(mini_corpus_path, tmp_path, mode):
    api = open_corpus([mini_corpus_path], mode, tmp_path, features="")
    failures = {
        template: (sorted(api.S.search(template)), expected)
        for template, expected in CASES.items()
        if sorted(api.S.search(template)) != expected
    }
    assert not failures
