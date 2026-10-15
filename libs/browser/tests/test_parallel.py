"""Tests for the parallel interface's rows: a chapter in several corpora, verse by verse.

Three small corpora besides the miniature DSS of `test_witnesses`:

    mt   Genesis 1:1 בורא ש, 1:2 ארץ;  Exodus 2:3 תבה
    sp   Genesis 1:1 ברא ש,  1:3 אור   (no 1:2, no Exodus)

The DSS has Genesis 1:1 in three scrolls and Exodus 2:3 in one.
"""

import pytest
from cfabric_browser.app import create_app
from cfabric_browser.corpora import CorpusService, resolve_corpus
from cfabric_browser.workers import SearchPool
from fastapi.testclient import TestClient
from test_witnesses import FILES, INTS, leaves, write_tf

OTEXT = (
    "@fmt:text-orig-full={g_cons_utf8} \n"
    "@sectionFeatures=book,chapter,verse\n@sectionTypes=book,chapter,verse\n"
)

MT = {
    "otype": "1-4\tword\n5-6\tbook\n7-8\tchapter\n9-11\tverse\n",
    "oslots": "5\t1-3\n6\t4\n7\t1-3\n8\t4\n9\t1-2\n10\t3\n11\t4\n",
    "g_cons_utf8": "1\tבורא\n2\tש\n3\tארץ\n4\tתבה\n",
    "book": "5\tGenesis\n6\tExodus\n",
    "chapter": "7\t1\n8\t2\n",
    "verse": "9\t1\n10\t2\n11\t3\n",
}

SP = {
    "otype": "1-3\tword\n4\tbook\n5\tchapter\n6-7\tverse\n",
    "oslots": "4\t1-3\n5\t1-3\n6\t1-2\n7\t3\n",
    "g_cons_utf8": "1\tברא\n2\tש\n3\tאור\n",
    "book": "4\tGenesis\n",
    "chapter": "5\t1\n",
    "verse": "6\t1\n7\t3\n",
}


@pytest.fixture(scope="module")
def client(tmp_path_factory):
    root = tmp_path_factory.mktemp("parallel")
    write_tf(
        root / "Scrolls" / "mini" / "tf" / "1.0",
        FILES,
        INTS,
        "@fmt:text-orig-full={glyph}{after}\n"
        "@sectionFeatures=scroll,fragment,line\n@sectionTypes=scroll,fragment,line\n",
    )
    for repo, files in (("mt", MT), ("sp", SP)):
        write_tf(
            root / "Hebrew" / repo / "tf" / "1.0", files, {"chapter", "verse"}, OTEXT
        )
    roots = (str(root),)
    names = ("Scrolls/mini", "Hebrew/mt", "Hebrew/sp")
    service = CorpusService([resolve_corpus(c, roots) for c in names], roots=roots)
    app = create_app(service, SearchPool([]), manage_pool=False)
    with TestClient(app) as c:
        yield c


def get(client, ref: str, texts: str, lead: str = "mt", **params):
    return client.get(
        "/api/parallel", params={"ref": ref, "lead": lead, "texts": texts, **params}
    )


def words(cell) -> list[str]:
    return [t for tree in cell["trees"] for (t, _) in leaves(tree["fragments"])]


class TestRows:
    def test_a_row_per_verse_in_any_text(self, client):
        data = get(client, "Genesis 1", "mt,sp,mini").json()
        assert data["ref"] == "Genesis 1"
        assert [r["verse"] for r in data["rows"]] == [1, 2, 3]
        assert [r["ref"] for r in data["rows"]] == [
            "Genesis 1:1",
            "Genesis 1:2",
            "Genesis 1:3",
        ]
        cells = [sorted(r["cells"]) for r in data["rows"]]
        # 1:2 only in the MT, 1:3 only in the SP
        assert cells == [["mini", "mt", "sp"], ["mt"], ["sp"]]

    def test_cells(self, client):
        row = get(client, "Genesis 1", "mt,sp,mini").json()["rows"][0]
        assert words(row["cells"]["mt"]) == ["בורא", "ש"]
        assert words(row["cells"]["sp"]) == ["ברא", "ש"]
        # the DSS: per verse the text of each scroll, by siglum
        witnesses = row["cells"]["mini"]["witnesses"]
        assert [w["scroll"] for w in witnesses] == ["1Q1", "4Q2", "4Q10"]
        assert witnesses[0]["words"] == [10, 11]

    def test_columns(self, client):
        data = get(client, "Genesis 1", "sp,mini,mt").json()
        assert data["columns"] == [
            {"name": "sp", "kind": "text", "ref": "Genesis 1"},
            {"name": "mini", "kind": "witnesses", "ref": "Genesis 1"},
            {"name": "mt", "kind": "text", "ref": "Genesis 1"},
        ]

    def test_lead_need_not_be_shown(self, client):
        # the rows follow the lead's numbering even when it is not a column
        data = get(client, "Genesis 1", "sp").json()
        assert [r["verse"] for r in data["rows"]] == [1, 3]
        assert data["rows"][1]["ref"] == "Genesis 1:3"

    def test_chapter_a_text_lacks(self, client):
        data = get(client, "Exodus 2", "mt,sp,mini").json()
        assert data["columns"][1] == {"name": "sp", "kind": "text", "ref": None}
        assert [sorted(r["cells"]) for r in data["rows"]] == [["mini", "mt"]]
        assert data["prev"]["ref"] == "Genesis 1"
        assert data["next"] is None

    def test_format_another_corpus_lacks(self, client):
        # each corpus that lacks the format uses its default
        response = get(client, "Genesis 1", "mt,mini", fmt="text-orig-plain")
        assert response.status_code == 200

    def test_missing(self, client):
        assert get(client, "Genesis 9", "mt").status_code == 404
        assert get(client, "Genesis 1", "mt,nothing").status_code == 404
        assert get(client, "Genesis 1", "mt", lead="nothing").status_code == 404
