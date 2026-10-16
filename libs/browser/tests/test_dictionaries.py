"""Tests for dictionaries in the browser (BDB): reading the file, adding and
removing it, and the routes. The dictionary is a few entries written in the
test; the corpus is the miniature BHSA of `test_lexicon.py`."""

import gzip
import hashlib
import json
from dataclasses import replace
from pathlib import Path

import pytest
from cfabric_browser.app import create_app
from cfabric_browser.corpora import CorpusService, resolve_corpus
from cfabric_browser.dictionaries import manage
from cfabric_browser.dictionaries.store import Dictionary, DictionaryError, short
from cfabric_browser.downloads import DownloadError
from cfabric_browser.workers import SearchPool
from fastapi.testclient import TestClient
from test_lexicon import write_tf


def entry(eid, headword, section, html="", defs=(), pos=None, lexicon="heb"):
    return {
        "id": eid,
        "lexicon": lexicon,
        "headword": headword,
        "ref": f"BDB, {headword}",
        "html": html,
        "incomplete": False,
        "refs": 0,
        "pos": pos,
        "defs": list(defs),
        "root": False,
        "section": section,
        "strong": [],
        "twot": [],
    }


ENTRIES = [
    entry("a.aa.aa", "רֵאשִׁית", "a.aa", "<strong>beginning</strong>, first", pos="n.f"),
    entry(
        "b.aa.aa",
        "בָּרָא",
        "b.aa",
        'vb. <strong>shape, create</strong> <a data-osis="Gen.1.1">Gn 1:1</a>; '
        'v. <a data-entry="b.aa.ab">בְּרִיאָה</a>',
        defs=["shape", "create"],
        pos="vb",
    ),
    entry("b.aa.ab", "בְּרִיאָה", "b.aa", "n.f. <strong>a creation</strong>"),
    entry("xa.aa.aa", "אֲרַק", "xa.aa", "n.f. earth", ["earth"], lexicon="arc"),
]
LINKS = {
    "Hebrew BR>[": {"entries": ["b.aa.aa", "b.aa.ab"], "share": 0.9, "how": "oshb"},
    "Hebrew R>CJT/": {"entries": ["a.aa.aa"], "share": 1.0, "how": "oshb"},
    "Aramaic >RQ/": {"entries": ["xa.aa.aa"], "how": "manual"},
}


def write_dictionary(path: Path, short_name: str = "BDB") -> Path:
    data = {
        "meta": {"short": short_name, "title": "A Lexicon", "built": "2026-10-16"},
        "entries": ENTRIES,
        "links": LINKS,
        "abbreviations": {"Gn": "Genesis"},
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(gzip.compress(json.dumps(data).encode()))
    return path


class TestStore:
    def test_short(self):
        assert short(ENTRIES[0]) == "beginning"
        assert short(ENTRIES[1]) == "shape, create"
        assert short(entry("x", "x", None)) == ""

    def test_lexeme_and_article(self, tmp_path):
        found = Dictionary.load(write_dictionary(tmp_path / "bdb.json.gz"))
        linked = found.for_lexeme("Hebrew", "BR>[")
        assert [e["id"] for e in linked["entries"]] == ["b.aa.aa", "b.aa.ab"]
        assert linked["how"] == "oshb" and linked["share"] == 0.9
        assert found.for_lexeme("Hebrew", "XYZ/") == {"entries": [], "how": None}
        article = found.article("b.aa.aa")
        assert article["html"].startswith("vb.")
        assert article["previous"] == {"id": "a.aa.aa", "headword": "רֵאשִׁית"}
        assert article["next"] == {"id": "b.aa.ab", "headword": "בְּרִיאָה"}
        assert [e["id"] for e in article["family"]] == ["b.aa.aa", "b.aa.ab"]
        assert article["lexemes"] == ["Hebrew BR>["]
        # no neighbour across the Hebrew and Aramaic parts, no family of one
        aramaic = found.article("xa.aa.aa")
        assert aramaic["previous"] is None and aramaic["family"] == []
        assert found.article("z.zz.zz") is None

    def test_not_a_dictionary(self, tmp_path):
        bad = tmp_path / "bad.json.gz"
        bad.write_bytes(b"not gzip")
        with pytest.raises(DictionaryError):
            Dictionary.load(bad)


class TestManage:
    def test_add_file_and_remove(self, tmp_path):
        bdb = manage.DICTIONARIES["bdb"]
        source = write_dictionary(tmp_path / "built" / "bdb.json.gz")
        root = tmp_path / "dictionaries"
        assert not manage.is_added(bdb, root)
        manage.add_file(bdb, source, root)
        assert manage.is_added(bdb, root)
        assert manage.remove(bdb, root)
        assert not manage.remove(bdb, root)
        with pytest.raises(DictionaryError):
            manage.add_file(bdb, write_dictionary(tmp_path / "other.json.gz", "XYZ"))

    def test_add_by_download(self, tmp_path):
        source = write_dictionary(tmp_path / "published.json.gz")
        sha = hashlib.sha256(source.read_bytes()).hexdigest()
        bdb = replace(
            manage.DICTIONARIES["bdb"], url="https://example.org/bdb", sha256=sha
        )

        def fetch(url, target, report):
            target.write_bytes(source.read_bytes())
            report(10, 10)

        root = tmp_path / "dictionaries"
        manage.add(bdb, root, fetcher=fetch)
        assert manage.is_added(bdb, root)
        # a file with another checksum is refused
        wrong = replace(bdb, sha256="0" * 64)
        manage.remove(bdb, root)
        with pytest.raises(DownloadError):
            manage.add(wrong, root, fetcher=fetch)
        assert not manage.is_added(bdb, root)

    def test_not_published(self, tmp_path):
        with pytest.raises(DownloadError):
            manage.add(manage.DICTIONARIES["bdb"], tmp_path)


@pytest.fixture
def setup(tmp_path):
    root = tmp_path / "corpora"
    write_tf(root / "Hebrew" / "bible" / "tf" / "1.0")
    roots = (str(root),)
    service = CorpusService([resolve_corpus("Hebrew/bible", roots)], roots=roots)
    jobs = manage.DictionaryJobs(tmp_path / "dictionaries")
    app = create_app(service, SearchPool([]), manage_pool=False, dictionaries=jobs)
    with TestClient(app) as client:
        yield client, tmp_path


def node_of(client, lex):
    entries = client.get("/api/corpora/bible/lexicon").json()["entries"]
    return next(e["node"] for e in entries if e["lex"] == lex)


class TestRoutes:
    def test_without_the_dictionary(self, setup):
        client, _ = setup
        [bdb] = client.get("/api/dictionaries").json()["dictionaries"]
        assert bdb["id"] == "bdb" and not bdb["added"] and not bdb["available"]
        entry = client.get(f"/api/corpora/bible/lexicon/{node_of(client, 'BR>[')}")
        assert entry.json()["bdb"] is None
        assert client.get("/api/dictionaries/bdb/entries/b.aa.aa").status_code == 404
        # not published yet: can't be downloaded
        assert client.post("/api/dictionaries/bdb").status_code == 409
        assert client.delete("/api/dictionaries/bdb").status_code == 404
        assert client.get("/api/dictionaries/xyz").status_code == 404

    def test_with_the_dictionary(self, setup):
        client, tmp_path = setup
        write_dictionary(tmp_path / "dictionaries" / "bdb" / "bdb.json.gz")
        described = client.get("/api/dictionaries/bdb").json()
        assert described["added"] and described["meta"]["title"] == "A Lexicon"
        assert described["abbreviations"] == {"Gn": "Genesis"}

        node = node_of(client, "BR>[")
        bdb = client.get(f"/api/corpora/bible/lexicon/{node}").json()["bdb"]
        assert [e["headword"] for e in bdb["entries"]] == ["בָּרָא", "בְּרִיאָה"]
        assert bdb["entries"][0]["short"] == "shape, create"
        # Aramaic >RQ/ is linked, Hebrew B (no link) has no entries
        aramaic = client.get(f"/api/corpora/bible/lexicon/{node_of(client, '>RQ/')}")
        assert aramaic.json()["bdb"]["entries"][0]["id"] == "xa.aa.aa"
        b = client.get(f"/api/corpora/bible/lexicon/{node_of(client, 'B')}").json()
        assert b["bdb"] == {"entries": [], "how": None}

        article = client.get(
            "/api/dictionaries/bdb/entries/b.aa.aa", params={"corpus": "bible"}
        ).json()
        assert 'data-osis="Gen.1.1"' in article["html"]
        assert article["passages"] == {"Gen.1.1": "Genesis 1:1"}
        assert [(x["lex"], x["node"]) for x in article["lexemes"]] == [("BR>[", node)]
        family = {m["id"]: m["lexemes"] for m in article["family"]}
        assert [x["lex"] for x in family["b.aa.ab"]] == ["BR>["]
        # without a corpus, no lexemes
        bare = client.get("/api/dictionaries/bdb/entries/b.aa.aa").json()
        assert bare["lexemes"] == []
        assert client.get("/api/dictionaries/bdb/entries/z.zz.zz").status_code == 404

        # removed: gone from the lexicon at once
        assert client.delete("/api/dictionaries/bdb").json()["added"] is False
        assert client.get(f"/api/corpora/bible/lexicon/{node}").json()["bdb"] is None
