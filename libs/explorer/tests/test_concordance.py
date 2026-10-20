"""Tests for the concordance: search results as lines with the hit in context.

On the `display_corpus` fixture (see `test_api`): one book with the words

    1 the/art  2 word/subs  3 said/verb  4 to/prep        verse 1:1
    5 man/subs  6 go/verb  7 and/conj  8 woman/subs      verse 1:2
    9 came/verb  10 the/art  11 child/subs  12 NA         verse 2:1

(gloss/part of speech). `word sp=verb` finds the words 3, 6 and 9.
"""

import math
import time

import pytest
from cfabric_browser.app import create_app
from cfabric_browser.corpora import CorpusService, CorpusSpec
from cfabric_browser.workers import SearchPool
from fastapi.testclient import TestClient
from test_api import DATA


@pytest.fixture(scope="module")
def client():
    spec = CorpusSpec(name="alpha", path=DATA)
    service = CorpusService([spec])
    pool = SearchPool(
        [(spec.name, str(spec.path))],
        sort_key=lambda c: service.api(c).N.sortKeyTuple,
        workers=1,
        timeout=20,
        max_results=1000,
    )
    pool.start(wait=True)
    app = create_app(service, pool, manage_pool=False)
    with TestClient(app) as c:
        yield c
    pool.shutdown()


def search(client, template: str) -> str:
    started = client.post("/api/corpora/alpha/search", json={"template": template})
    search_id = started.json()["id"]
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        status = client.get(f"/api/searches/{search_id}").json()
        if status["state"] not in ("queued", "running"):
            return search_id
        time.sleep(0.05)
    raise AssertionError("search did not finish")


@pytest.fixture(scope="module")
def verbs(client):
    return search(client, "word sp=verb")


@pytest.fixture(scope="module")
def words(client):
    return search(client, "word")


def kwic(client, search_id, **params):
    response = client.get(f"/api/searches/{search_id}/kwic", params=params)
    assert response.status_code == 200, response.text
    return response.json()


def nodes(line, side):
    return [w["node"] for w in line[side]]


class TestLines:
    def test_hit_in_context(self, client, verbs):
        data = kwic(client, verbs, left=2, right=1, features="gloss")
        assert data["total"] == 3
        assert data["words"] == 12
        lines = data["lines"]
        assert [nodes(line, "hit") for line in lines] == [[3], [6], [9]]
        assert [nodes(line, "left") for line in lines] == [[1, 2], [4, 5], [7, 8]]
        assert [nodes(line, "right") for line in lines] == [[4], [7], [10]]
        assert [line["ref"] for line in lines] == [
            "Alpha Book 1.1",
            "Alpha Book 1.2",
            "Alpha Book 2.1",
        ]
        assert lines[0]["hit"][0]["features"] == {"gloss": "said"}

    def test_context_stops_at_the_corpus_edge(self, client, words):
        data = kwic(client, words, left=3, right=3)
        assert nodes(data["lines"][0], "left") == []
        assert nodes(data["lines"][-1], "right") == []

    def test_the_hit_is_the_innermost_node(self, client):
        # the clause is the container, the word the hit
        search_id = search(client, "clause\n  word sp=conj")
        (line,) = kwic(client, search_id, left=1, right=1)["lines"]
        assert nodes(line, "hit") == [7]

    def test_paging(self, client, words):
        data = kwic(client, words, offset=10, limit=5)
        assert data["total"] == 12
        assert [line["index"] for line in data["lines"]] == [10, 11]


class TestSortAndFilter:
    def test_sort_by_the_word_before(self, client, verbs):
        # before the verbs: word, man, woman
        data = kwic(client, verbs, sort="L1.gloss")
        assert [nodes(line, "hit") for line in data["lines"]] == [[6], [9], [3]]

    def test_sort_by_the_hit(self, client, words):
        data = kwic(client, words, sort="hit.sp", limit=3)
        assert [line["hit"][0]["node"] for line in data["lines"]] == [1, 10, 7]

    def test_bad_sort(self, client, verbs):
        response = client.get(
            f"/api/searches/{verbs}/kwic", params={"sort": "middle.gloss"}
        )
        assert response.status_code == 400

    def test_filter(self, client, words):
        data = kwic(client, words, filter="sp:subs,gloss:man")
        assert [nodes(line, "hit") for line in data["lines"]] == [[5]]
        assert kwic(client, words, filter="book:Alpha Book")["total"] == 12

    def test_one_per_verse(self, client, words):
        data = kwic(client, words, per="verse")
        assert [nodes(line, "hit") for line in data["lines"]] == [[1], [5], [9]]

    def test_sample(self, client, words):
        first = kwic(client, words, sample=4, seed=7)
        again = kwic(client, words, sample=4, seed=7)
        assert first["total"] == 4
        indexes = [line["index"] for line in first["lines"]]
        assert indexes == [line["index"] for line in again["lines"]]
        assert indexes == sorted(indexes)


class TestFacets:
    def test_values(self, client, words):
        data = client.get(
            f"/api/searches/{words}/kwic/facets", params={"feature": "sp"}
        ).json()
        assert data["total"] == 12
        assert data["values"][0] == {"value": "subs", "count": 4}
        assert {"value": "verb", "count": 3} in data["values"]

    def test_books_in_order(self, client, words):
        data = client.get(
            f"/api/searches/{words}/kwic/facets", params={"feature": "book"}
        ).json()
        assert data["values"] == [{"value": "Alpha Book", "count": 12}]

    def test_filtered(self, client, words):
        data = client.get(
            f"/api/searches/{words}/kwic/facets",
            params={"feature": "gloss", "filter": "sp:verb"},
        ).json()
        assert sorted(v["value"] for v in data["values"]) == ["came", "go", "said"]


class TestCollocations:
    def test_next_to_the_verbs(self, client, verbs):
        data = client.get(
            f"/api/searches/{verbs}/collocations", params={"feature": "sp", "window": 1}
        ).json()
        # before each verb a noun (subs); after them prep, conj and art once each
        (row,) = data["values"]
        assert (row["value"], row["count"], row["frequency"]) == ("subs", 3, 4)
        assert row["logDice"] == round(14 + math.log2(2 * 3 / (3 + 4)), 2)

    def test_min_count(self, client, verbs):
        data = client.get(
            f"/api/searches/{verbs}/collocations",
            params={"feature": "sp", "window": 1, "min": 1},
        ).json()
        assert sorted(v["value"] for v in data["values"]) == [
            "art",
            "conj",
            "prep",
            "subs",
        ]
