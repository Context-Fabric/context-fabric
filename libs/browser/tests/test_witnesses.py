"""Tests for reading a corpus of manuscripts by biblical book (the DSS).

The fixture is a miniature DSS: signs are the slots, sections are scroll,
fragment and line, and words carry `book`, `chapter` and `verse`:

    scroll 1Q1   f1 line 1: בר א      Gen 1:1
                 f1 line 2: ת         Exod 2:3
    scroll 4Q10  f1 line 1: ש         Gen 1:1 (biblical=2)
    scroll 4Q2   f1 line 1: [בר] ק#   Gen 1:1 (reconstructed), then a word
                                       not identified with a verse, whose
                                       last letter is illegible (` # `)

The scroll 4Q10 comes before 4Q2 in the corpus, but after it by siglum.
"""

from pathlib import Path

import pytest
from cfabric_browser.app import create_app
from cfabric_browser.compare import witness_letters
from cfabric_browser.corpora import CorpusService, resolve_corpus
from cfabric_browser.witnesses import (
    format_ref,
    parse_ref,
    place,
    show_placeholders,
    siglum_key,
)
from cfabric_browser.workers import SearchPool
from fastapi.testclient import TestClient

# signs 1-9, words 10-15, lines 16-19, fragments 20-22, scrolls 23-25
SIGNS = ["ב", "ר", "א", "ת", "ש", "ב", "ר", "ק", " # "]
FILES = {
    "otype": "1-9\tsign\n10-15\tword\n16-19\tline\n20-22\tfragment\n23-25\tscroll\n",
    "oslots": "10\t1-2\n11\t3\n12\t4\n13\t5\n14\t6-7\n15\t8-9\n"
    "16\t1-3\n17\t4\n18\t5\n19\t6-9\n20\t1-4\n21\t5\n22\t6-9\n23\t1-4\n24\t5\n25\t6-9\n",
    "glyph": "".join(f"{i + 1}\t{c}\n" for (i, c) in enumerate(SIGNS)),
    "after": "2\t \n3\t \n4\t \n5\t \n7\t \n9\t \n",
    "rec": "6\t1\n7\t1\n",
    "type": "1-8\tcons\n9\tunc\n",
    "scroll": "23\t1Q1\n24\t4Q10\n25\t4Q2\n",
    "fragment": "20\tf1\n21\tf1\n22\tf1\n",
    "line": "16\t1\n17\t2\n18\t1\n19\t1\n",
    "book": "10\tGen\nGen\nEx\nGen\nGen\n4Q2\n",
    "chapter": "10\t1\n1\n2\n1\n1\nf1\n",
    "verse": "10\t1\n1\n3\n1\n1\n1\n",
    "biblical": "10\t1\n1\n1\n2\n1\n1\n",
}
EDGES = {"oslots"}
INTS = {"rec", "biblical"}


# a reference text (MT) with Genesis 1:1 = בורא ש
MT_FILES = {
    "otype": "1-2\tword\n3\tbook\n4\tchapter\n5\tverse\n",
    "oslots": "3\t1-2\n4\t1-2\n5\t1-2\n",
    "g_cons_utf8": "1\tבורא\n2\tש\n",
    "book": "3\tGenesis\n",
    "chapter": "4\t1\n",
    "verse": "5\t1\n",
}


def write_tf(data: Path, files: dict[str, str], ints: set[str], otext: str) -> None:
    data.mkdir(parents=True)
    for name, body in files.items():
        kind = "@edge" if name in EDGES else "@node"
        vtype = "int" if name in ints else "str"
        (data / f"{name}.tf").write_text(
            f"{kind}\n@valueType={vtype}\n\n{body}", encoding="utf-8"
        )
    (data / "otext.tf").write_text(f"@config\n{otext}", encoding="utf-8")


def make_scrolls(root: Path) -> None:
    write_tf(
        root / "Scrolls" / "mini" / "tf" / "1.0",
        FILES,
        INTS,
        "@fmt:text-orig-full={glyph}{after}\n"
        "@sectionFeatures=scroll,fragment,line\n@sectionTypes=scroll,fragment,line\n",
    )
    write_tf(
        root / "Hebrew" / "mt" / "tf" / "1.0",
        MT_FILES,
        {"chapter", "verse"},
        "@fmt:text-orig-full={g_cons_utf8} \n"
        "@sectionFeatures=book,chapter,verse\n@sectionTypes=book,chapter,verse\n",
    )


@pytest.fixture(scope="module")
def service(tmp_path_factory):
    root = tmp_path_factory.mktemp("scrolls")
    make_scrolls(root)
    roots = (str(root),)
    specs = [resolve_corpus(c, roots) for c in ("Scrolls/mini", "Hebrew/mt")]
    return CorpusService(specs, roots=roots)


@pytest.fixture(scope="module")
def client(service):
    app = create_app(service, SearchPool([]), manage_pool=False)
    with TestClient(app) as c:
        yield c


def leaves(fragments) -> list[tuple[str, bool]]:
    """The texts in render tree fragments: (text, reconstructed)."""
    result = []
    for frag in fragments:
        if frag.get("children"):
            result.extend(leaves(frag["children"]))
        elif "text" in frag:
            result.append((frag["text"].strip(), bool(frag.get("dim"))))
    return result


def texts(row) -> list[tuple[str, bool]]:
    return leaves(row["tree"]["fragments"])


class TestRefs:
    @pytest.mark.parametrize(
        "ref,parsed",
        [
            ("Genesis 1", ("Gen", 1, None)),
            ("Genesis 1:3", ("Gen", 1, 3)),
            ("Gen 1:3", ("Gen", 1, 3)),
            ("1_Samuel 2:4", ("1Sam", 2, 4)),
            ("Is 6", ("Isa", 6, None)),
            ("4Q2 f1:1", None),
            ("Genesis", None),
        ],
    )
    def test_parse(self, ref, parsed):
        assert parse_ref(ref) == parsed

    def test_format(self):
        assert format_ref("Gen", 1) == "Genesis 1"
        assert format_ref("1Sam", 2, 4) == "1 Samuel 2:4"
        assert format_ref("Ruth", 1) == "Ruth 1"

    def test_sigla(self):
        names = ["Mas1", "4Q10", "11Q5", "4Q2", "1Q1", "4Q8d", "4Q8a"]
        assert sorted(names, key=siglum_key) == [
            "1Q1",
            "4Q2",
            "4Q8a",
            "4Q8d",
            "4Q10",
            "11Q5",
            "Mas1",
        ]


class TestBooks:
    def test_card(self, client):
        assert client.get("/api/corpora/mini").json()["bookView"] is True

    def test_books_in_canonical_order(self, client):
        books = client.get("/api/corpora/mini/books").json()["books"]
        assert [b["label"] for b in books] == ["Genesis", "Exodus"]
        (gen, exod) = books
        assert gen["scrolls"] == 3
        assert gen["chapters"] == [
            {"chapter": 1, "ref": "Genesis 1", "verses": 1, "witnesses": 3}
        ]
        assert exod["ref"] == "Exodus 2"

    def test_place(self, service):
        api = service.api("mini")
        assert place(api, [10, 11]) == "f1:1"
        assert place(api, [10, 12]) == "f1:1–2"


class TestWitnesses:
    def get(self, client, ref):
        return client.get("/api/corpora/mini/witnesses", params={"ref": ref})

    def test_chapter(self, client):
        data = self.get(client, "Genesis 1").json()
        assert data["ref"] == "Genesis 1"
        assert (data["prev"], data["next"]) == (None, "Exodus 2")
        (verse,) = data["verses"]
        assert verse["ref"] == "Genesis 1:1"
        # by siglum, and words not identified with a verse are left out
        assert [w["scroll"] for w in verse["witnesses"]] == ["1Q1", "4Q2", "4Q10"]

    def test_witness(self, client):
        q1 = self.get(client, "Gen 1:1").json()["verses"][0]["witnesses"][0]
        assert q1["words"] == [10, 11]
        assert q1["place"] == "f1:1"
        assert q1["ref"] == "1Q1 f1"
        assert texts(q1) == [("ב", False), ("ר", False), ("א", False)]

    def test_reconstructed_is_dim(self, client):
        q2 = self.get(client, "Genesis 1:1").json()["verses"][0]["witnesses"][1]
        assert texts(q2) == [("ב", True), ("ר", True)]

    def test_also_non_biblical(self, client):
        rows = self.get(client, "Genesis 1:1").json()["verses"][0]["witnesses"]
        assert [w["alsoNonBiblical"] for w in rows] == [False, False, True]

    def test_missing(self, client):
        assert self.get(client, "Genesis 2").status_code == 404
        assert self.get(client, "Esther 1").status_code == 404
        assert self.get(client, "4Q2 f1").status_code == 404


class TestPassageDims:
    def test_scroll_view_dims_reconstruction(self, client):
        data = client.get(
            "/api/corpora/mini/passage", params={"ref": "4Q2 f1:1", "mode": "plain"}
        ).json()
        signs = [leaf for tree in data["trees"] for leaf in leaves(tree["fragments"])]
        assert signs == [("ב", True), ("ר", True), ("ק", False), ("◦", False)]


class TestAlign:
    def test_to_book_view(self, client):
        data = client.get(
            "/api/corpora/mini/align",
            params={"from": "mini", "ref": "Gen 1:1", "view": "books"},
        ).json()
        assert data == {"node": None, "ref": "Genesis 1:1", "exact": True}

    def test_missing_verse_falls_back_to_chapter(self, client):
        data = client.get(
            "/api/corpora/mini/align",
            params={"from": "mini", "ref": "Genesis 1:5", "view": "books"},
        ).json()
        assert data == {"node": None, "ref": "Genesis 1", "exact": False}

    def test_book_not_attested(self, client):
        response = client.get(
            "/api/corpora/mini/align",
            params={"from": "mini", "ref": "Leviticus 1", "view": "books"},
        )
        assert response.status_code == 404


class TestCompareWitnesses:
    def compare(self, client, aref: str, bref: str):
        return client.get(
            "/api/compare",
            params={
                "a": "mt",
                "aref": aref,
                "b": "mini",
                "bref": bref,
                "bview": "books",
            },
        )

    def test_witnesses(self, client):
        data = self.compare(client, "Genesis 1", "Genesis 1").json()
        assert data["comparable"] is True
        # 1Q1 lacks the ו of בורא; 4Q2 is all reconstructed; 4Q10 agrees
        assert data["a"] == {"1": "matres"}
        assert data["aWitnesses"] == {"1": ["1Q1"]}
        assert data["b"] == {"10": "matres"}

    def test_other_passage(self, client):
        data = self.compare(client, "Genesis 1", "Exodus 2").json()
        assert data["comparable"] is False

    def test_align_to_book_view_and_back(self, client):
        there = client.get(
            "/api/corpora/mini/align",
            params={"from": "mt", "ref": "Genesis 1:1", "view": "books"},
        ).json()
        assert there == {"node": None, "ref": "Genesis 1:1", "exact": True}
        back = client.get(
            "/api/corpora/mt/align", params={"from": "mini", "ref": "Genesis 1:1"}
        ).json()
        assert back["ref"] == "Genesis 1:1"


class TestPlaceholders:
    def test_shown_as_in_editions(self):
        assert show_placeholders("ε ") == "[…] "
        assert show_placeholders("נפ #  # ") == "נפ◦◦"
        assert show_placeholders("מ ? ") == "מ◦"

    def test_unknown_in_comparisons(self, service):
        # an illegible letter is there but unknown, so it is not preserved
        (letters, known) = witness_letters(service.api("mini"), [15])
        assert letters == [(15, "ק◦")]
        assert known == [True, False]
