"""Tests for the lexicon of a corpus with lexeme nodes (the BHSA).

The fixture is a miniature BHSA, without `freq_lex`, so frequencies are counted:

    Genesis 1:1  ב ראשית ברא       in, beginning, create
    Genesis 1:2  ברא אלהים ברא     create, god, create (twice in one verse)
    Exodus 1:1   ב ארק             in, earth (Aramaic)
"""

from pathlib import Path

import pytest
from cfabric_browser.app import create_app
from cfabric_browser.corpora import CorpusService, resolve_corpus
from cfabric_browser.lexicon import first_letter, has_lexicon, sort_key
from cfabric_browser.workers import SearchPool
from fastapi.testclient import TestClient

# words 1-8, books 9-10, chapters 11-12, verses 13-15, lexemes 16-20
WORDS = ["בְּ", "רֵאשִׁית", "בָּרָא", "בָּרָא", "אֱלֹהִים", "בָּרָא", "בְּ", "אֲרַק"]
FILES = {
    "otype": "1-8\tword\n9-10\tbook\n11-12\tchapter\n13-15\tverse\n16-20\tlex\n",
    "oslots": "9\t1-6\n10\t7-8\n11\t1-6\n12\t7-8\n13\t1-3\n14\t4-6\n15\t7-8\n"
    "16\t1,7\n17\t2\n18\t3-4,6\n19\t5\n20\t8\n",
    "g_word_utf8": "".join(f"{i + 1}\t{w}\n" for (i, w) in enumerate(WORDS)),
    "book": "9\tGenesis\nExodus\n",
    "chapter": "11\t1\n1\n",
    "verse": "13\t1\n2\n1\n",
    "lex": "16\tB\nR>CJT/\nBR>[\n>LHJM/\n>RQ/\n",
    "lex_utf8": "16\tב\nראשׁית\nברא\nאלהים\nארק\n",
    "voc_lex_utf8": "16\tבְּ\nרֵאשִׁית\nברא\nאֱלֹהִים\nאֲרַק\n",
    "gloss": "16\tin\nbeginning\ncreate\ngod(s)\nearth\n",
    "sp": "16\tprep\nsubs\nverb\nsubs\nsubs\n",
    "language": "16\tHebrew\nHebrew\nHebrew\nHebrew\nAramaic\n",
}
EDGES = {"oslots"}
INTS = {"chapter", "verse"}


def write_tf(data: Path) -> None:
    data.mkdir(parents=True)
    for name, body in FILES.items():
        kind = "@edge" if name in EDGES else "@node"
        vtype = "int" if name in INTS else "str"
        (data / f"{name}.tf").write_text(
            f"{kind}\n@valueType={vtype}\n\n{body}", encoding="utf-8"
        )
    (data / "otext.tf").write_text(
        "@config\n@fmt:text-orig-full={g_word_utf8} \n"
        "@sectionFeatures=book,chapter,verse\n@sectionTypes=book,chapter,verse\n",
        encoding="utf-8",
    )


@pytest.fixture(scope="module")
def client(tmp_path_factory):
    root = tmp_path_factory.mktemp("lexicon")
    write_tf(root / "Hebrew" / "bible" / "tf" / "1.0")
    roots = (str(root),)
    service = CorpusService([resolve_corpus("Hebrew/bible", roots)], roots=roots)
    app = create_app(service, SearchPool([]), manage_pool=False)
    with TestClient(app) as c:
        yield c


def entries(client) -> list[dict]:
    return client.get("/api/corpora/bible/lexicon").json()["entries"]


def node_of(client, lex: str) -> int:
    return next(e["node"] for e in entries(client) if e["lex"] == lex)


def marked(fragments) -> list[int]:
    """The nodes marked in render tree fragments."""
    result = []
    for frag in fragments:
        if frag.get("highlight") is not None:
            result.append(frag["node"])
        result.extend(marked(frag.get("children") or []))
    return result


def text(fragments) -> str:
    return "".join(
        frag.get("text", "") + text(frag.get("children") or []) for frag in fragments
    )


class TestHelpers:
    def test_sort_key(self):
        # in Hebrew order: alef, bet, ..., resh, sin and shin as one letter (so
        # the next letter decides: FDH before CM), tav
        words = ["T", "R>CJT/", "B", ">LHJM/", "FDH/", "CM/", "BR>["]
        assert sorted(words, key=sort_key) == [
            ">LHJM/",
            "B",
            "BR>[",
            "R>CJT/",
            "FDH/",
            "CM/",
            "T",
        ]

    def test_first_letter(self):
        assert first_letter("בְּ") == "ב"
        assert first_letter("ראשׁית֜") == "ר"
        assert first_letter(None) is None


class TestLexicon:
    def test_card(self, client):
        assert client.get("/api/corpora/bible").json()["lexicon"] is True

    def test_entries(self, client):
        found = entries(client)
        # per language, in Hebrew alphabetical order
        assert [(e["language"], e["lex"]) for e in found] == [
            ("Aramaic", ">RQ/"),
            ("Hebrew", ">LHJM/"),
            ("Hebrew", "B"),
            ("Hebrew", "BR>["),
            ("Hebrew", "R>CJT/"),
        ]
        create = found[3]
        assert create["gloss"] == "create"
        assert create["sp"] == "verb"
        assert create["letter"] == "ב"
        assert create["form"] == "ברא"
        # no freq_lex in this corpus: counted from the words
        assert create["freq"] == 3

    def test_entry(self, client):
        entry = client.get(f"/api/corpora/bible/lexicon/{node_of(client, 'B')}").json()
        assert entry["verses"] == 2
        assert entry["books"] == [
            {"book": "Genesis", "count": 1},
            {"book": "Exodus", "count": 1},
        ]

    def test_occurrences(self, client):
        node = node_of(client, "BR>[")
        page = client.get(f"/api/corpora/bible/lexicon/{node}/occurrences").json()
        assert page["total"] == 2
        assert [row["ref"] for row in page["rows"]] == ["Genesis 1:1", "Genesis 1:2"]
        # a verse once, with every occurrence in it marked, as running text
        second = page["rows"][1]
        assert second["words"] == [4, 6]
        assert marked(second["tree"]["fragments"]) == [4, 6]
        assert text(second["tree"]["fragments"]).split() == WORDS[3:6]

    def test_paging_and_book(self, client):
        node = node_of(client, "B")
        url = f"/api/corpora/bible/lexicon/{node}/occurrences"
        page = client.get(url, params={"offset": 1, "limit": 1}).json()
        assert [row["ref"] for row in page["rows"]] == ["Exodus 1:1"]
        page = client.get(url, params={"book": "Exodus"}).json()
        assert page["total"] == 1

    def test_tsv(self, client):
        node = node_of(client, "BR>[")
        response = client.get(f"/api/corpora/bible/lexicon/{node}/occurrences.tsv")
        assert response.status_code == 200
        assert "bible-BR.tsv" in response.headers["content-disposition"]
        lines = response.text.splitlines()
        assert lines[0].split("\t") == [
            "BOOK",
            "CHAPTER",
            "VERSE",
            "WORD",
            "TEXT",
            "VERSE_TEXT",
        ]
        # one row per occurrence, also two in one verse
        assert [line.split("\t")[:4] for line in lines[1:]] == [
            ["Genesis", "1", "1", "3"],
            ["Genesis", "1", "2", "4"],
            ["Genesis", "1", "2", "6"],
        ]

    def test_not_a_lexeme(self, client):
        assert client.get("/api/corpora/bible/lexicon/1").status_code == 404


def test_no_lexicon():
    """A corpus without lexeme nodes, or with lexemes without glosses."""

    class Otype:
        all = ("word", "verse")

    class F:
        otype = Otype()

    class Api:
        pass

    api = Api()
    api.F = F()  # type: ignore[attr-defined]
    assert has_lexicon(api) is False
