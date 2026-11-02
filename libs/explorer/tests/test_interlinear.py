"""Tests for the interlinear: a passage word by word, with lines under each word.

The structure is tested on the `display_corpus` fixture of the core package
(see `test_api`): verse "Alpha Book 1:1" has the words 1-4, in the phrases 22
(Subj, words 1-2) and 23 (Pred, words 3-5), and the clauses 19 (Main, words
1-2 and 7-8) and 20 (Rel, words 3-6). The second corpus under the first is
tested on the MT and SP of `test_parallel`.
"""

import pytest
from cfabric_browser.app import create_app
from cfabric_browser.corpora import CorpusService, CorpusSpec
from cfabric_browser.workers import SearchPool
from fastapi.testclient import TestClient
from test_api import DATA
from test_parallel import service  # noqa: F401 (a fixture)


@pytest.fixture(scope="module")
def alpha():
    app = create_app(
        CorpusService([CorpusSpec(name="alpha", path=DATA)]),
        SearchPool([]),
        manage_pool=False,
    )
    with TestClient(app) as c:
        yield c


@pytest.fixture(scope="module")
def hebrew(service):  # noqa: F811
    app = create_app(service, SearchPool([]), manage_pool=False)
    with TestClient(app) as c:
        yield c


def get(client, corpus, ref, **params):
    return client.get(
        f"/api/corpora/{corpus}/interlinear", params={"ref": ref, **params}
    )


class TestWords:
    def test_chapter(self, alpha):
        data = get(alpha, "alpha", "Alpha Book 1").json()
        assert data["ref"] == "Alpha Book 1"
        assert [v["ref"] for v in data["verses"]] == [
            "Alpha Book 1.1",
            "Alpha Book 1.2",
        ]
        assert data["next"]["ref"] == "Alpha Book 2"

    def test_lines(self, alpha):
        (verse,) = get(alpha, "alpha", "Alpha Book 1.1").json()["verses"]
        assert verse["verse"] == 1
        assert [w["node"] for w in verse["words"]] == [1, 2, 3, 4]
        assert [w["lines"]["gloss"] for w in verse["words"]] == [
            "the",
            "word",
            "said",
            "to",
        ]
        assert verse["words"][2]["lines"]["pos"] == "verb"

    def test_extra_features(self, alpha):
        (verse,) = get(
            alpha, "alpha", "Alpha Book 1.1", features="trans,nothing"
        ).json()["verses"]
        assert [w["features"] for w in verse["words"]] == [
            {"trans": "a"},
            {"trans": "b"},
            {"trans": "g"},
            {"trans": "d"},
        ]

    def test_structure(self, alpha):
        (verse,) = get(alpha, "alpha", "Alpha Book 1.1").json()["verses"]
        assert verse["phrases"] == [
            {"node": 22, "label": "Subj"},
            {"node": 23, "label": "Pred"},
        ]
        assert verse["clauses"] == [
            {"node": 19, "label": "Main"},
            {"node": 20, "label": "Rel"},
        ]
        assert [(w["phrase"], w["clause"]) for w in verse["words"]] == [
            (0, 0),
            (0, 0),
            (1, 1),
            (1, 1),
        ]

    def test_missing(self, alpha):
        assert get(alpha, "alpha", "Zeta 1").status_code == 404
        assert get(alpha, "alpha", "Alpha Book 1", under="nothing").status_code == 404


class TestUnder:
    def test_word_by_word(self, hebrew):
        (verse, _) = get(hebrew, "mt", "Genesis 1", under="sp").json()["verses"]
        under = verse["under"]
        assert under["corpus"] == "sp"
        assert under["ref"] == "Genesis 1:1"
        assert [w["node"] for w in under["words"]] == [1, 2]
        # MT בורא over SP ברא, ש over ש
        assert under["under"] == [[1], [2]]
        # the MT has a mater the SP lacks; both corpora number their words 1, 2
        assert under["marksHere"] == {"1": "matres"}
        assert under["marks"] == {"1": "matres"}

    def test_verse_the_other_lacks(self, hebrew):
        (_, verse) = get(hebrew, "mt", "Genesis 1", under="sp").json()["verses"]
        assert verse["ref"] == "Genesis 1:2"
        assert verse["under"] is None
