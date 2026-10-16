"""Tests for working with several corpora: book concordance, alignment, library.

Two copies of the display_corpus fixture stand in for two editions of a text:
`heb` calls its book "Genesis", `gr` calls it "Gen" and numbers the second verse
of chapter 1 as verse 3, so that verse 1:2 has no exact counterpart there.
"""

import shutil
import time
from pathlib import Path

import pytest
from cfabric_browser.app import create_app
from cfabric_browser.books import book_index, canonical_book
from cfabric_browser.cli import main, parse_args
from cfabric_browser.corpora import CorpusService, discover, resolve_corpus
from cfabric_browser.workers import SearchPool
from fastapi.testclient import TestClient

FIXTURE = Path(__file__).resolve().parent / "fixtures" / "display_corpus"


def make_corpus(
    root: Path,
    org: str,
    repo: str,
    book: str,
    verses: dict[int, int],
    title: str = "Display test corpus",
) -> Path:
    target = root / org / repo
    shutil.copytree(FIXTURE, target, ignore=shutil.ignore_patterns(".cfm"))
    config = target / "app" / "config.yaml"
    config.write_text(
        config.read_text(encoding="utf-8").replace("Display test corpus", title),
        encoding="utf-8",
    )
    data = target / "tf" / "1.0"
    (data / "book.tf").write_text(
        f"@node\n@description=book name\n@valueType=str\n\n13\t{book}\n",
        encoding="utf-8",
    )
    lines = "".join(f"{n}\t{v}\n" for (n, v) in verses.items())
    (data / "verse.tf").write_text(
        f"@node\n@description=verse number\n@valueType=int\n\n{lines}", encoding="utf-8"
    )
    return target


@pytest.fixture(scope="module")
def roots(tmp_path_factory):
    root = tmp_path_factory.mktemp("corpora")
    make_corpus(root, "Hebrew", "heb", "Genesis", {16: 1, 17: 2, 18: 1}, "Hebrew text")
    make_corpus(root, "Greek", "gr", "Gen", {16: 1, 17: 3, 18: 1}, "Greek text")
    make_corpus(root, "Other", "notes", "Alpha Book", {16: 1, 17: 2, 18: 1}, "Notes")
    return (str(root),)


@pytest.fixture(scope="module")
def client(roots):
    service = CorpusService([resolve_corpus("heb", roots)], roots=roots)
    pool = SearchPool([])  # searching is not tested here
    app = create_app(service, pool, manage_pool=False)
    with TestClient(app) as c:
        yield c


class TestBooks:
    @pytest.mark.parametrize(
        "name,canonical",
        [
            ("Genesis", "Gen"),
            ("Gen", "Gen"),
            ("1_Samuel", "1Sam"),
            ("Samuel_1", "1Sam"),
            ("1Sam", "1Sam"),
            ("Song_of_songs", "Song"),
            ("Cant", "Song"),
            ("Qoh", "Eccl"),
            ("Nehemia", "Neh"),
            ("Maccabees_1_A", "1Macc"),
        ],
    )
    def test_canonical(self, name, canonical):
        assert canonical_book(name) == canonical

    @pytest.mark.parametrize(
        "name", ["2Esdr", "DanTh", "Esdras_3", "Alpha Book", "1QS"]
    )
    def test_unmapped(self, name):
        assert canonical_book(name) is None

    def test_index_prefers_first_form(self):
        assert book_index(["TobBA", "TobS", "Dan", "DanTh"]) == {
            "Tob": "TobBA",
            "Dan": "Dan",
        }


class TestAlign:
    def test_same_verse_in_other_corpus(self, client):
        assert client.post("/api/corpora", json={"id": "Greek/gr"}).status_code == 201
        data = client.get(
            "/api/corpora/gr/align", params={"from": "heb", "ref": "Genesis 1.1"}
        ).json()
        assert data == {"node": 16, "ref": "Gen 1.1", "exact": True}

    def test_falls_back_to_chapter(self, client):
        client.post("/api/corpora", json={"id": "Greek/gr"})
        data = client.get(
            "/api/corpora/gr/align", params={"from": "heb", "ref": "Genesis 1.2"}
        ).json()
        assert data == {"node": 14, "ref": "Gen 1", "exact": False}

    def test_chapter_and_back(self, client):
        client.post("/api/corpora", json={"id": "Greek/gr"})
        data = client.get(
            "/api/corpora/heb/align", params={"from": "gr", "ref": "Gen 2"}
        ).json()
        assert data["ref"] == "Genesis 2"
        assert data["exact"] is True

    def test_no_counterpart(self, client):
        client.post("/api/corpora", json={"id": "Other/notes"})
        response = client.get(
            "/api/corpora/notes/align", params={"from": "heb", "ref": "Genesis 1.1"}
        )
        assert response.status_code == 404

    def test_bad_source_ref(self, client):
        response = client.get(
            "/api/corpora/heb/align", params={"from": "heb", "ref": "Exodus 1.1"}
        )
        assert response.status_code == 404


class TestBookView:
    def test_corpus_with_books_has_none(self, client):
        # its sections are books already; see test_witnesses.py for the DSS
        assert client.get("/api/corpora/heb").json()["bookView"] is False
        assert client.get("/api/corpora/heb/books").status_code == 404


class TestCompare:
    # the diff itself is tested in test_compare.py; the fixture is not Hebrew

    def compare(self, client, aref: str, bref: str):
        client.post("/api/corpora", json={"id": "Greek/gr"})
        return client.get(
            "/api/compare", params={"a": "heb", "aref": aref, "b": "gr", "bref": bref}
        )

    def test_not_hebrew(self, client):
        data = self.compare(client, "Genesis 1", "Gen 1").json()
        assert data["comparable"] is False
        assert "Hebrew" in data["reason"]

    def test_different_passages(self, client):
        data = self.compare(client, "Genesis 1", "Gen 2").json()
        assert data["comparable"] is False
        assert "different passages" in data["reason"]

    def test_bad_ref(self, client):
        assert self.compare(client, "Exodus 1", "Gen 1").status_code == 404

    def test_unknown_corpus(self, client):
        response = client.get(
            "/api/compare",
            params={"a": "heb", "aref": "Genesis 1", "b": "nope", "bref": "Gen 1"},
        )
        assert response.status_code == 404


class TestDisplayOverrides:
    def test_overrides_layer_over_app_config(self, roots, tmp_path):
        overrides = tmp_path / "display" / "Hebrew"
        overrides.mkdir(parents=True)
        # for all versions: hide phrases and label clauses differently
        (overrides / "heb.yaml").write_text(
            "typeDisplay:\n  phrase:\n    hidden: true\n  clause:\n    label: '[{typ}]'\n",
            encoding="utf-8",
        )
        # for version 1.0 only: a different clause label, which wins
        (overrides / "heb@1.0.yaml").write_text(
            "typeDisplay:\n  clause:\n    label: '<{typ}>'\n", encoding="utf-8"
        )
        spec = resolve_corpus("heb", roots)
        service = CorpusService([spec], roots=roots, overrides=tmp_path / "display")
        config = service.config("heb")
        assert config.types["phrase"].hidden is True
        assert config.types["clause"].label == "<{typ}>"
        # the rest of the app config still applies
        assert config.types["word"].features == ("sp",)
        assert config.section_sep2 == "."

    def test_no_overrides(self, roots, tmp_path):
        spec = resolve_corpus("heb", roots)
        service = CorpusService([spec], roots=roots, overrides=tmp_path / "none")
        assert service.config("heb").types["clause"].label == "{typ}"


class TestLibrary:
    def test_discover(self, roots):
        found = discover(roots)
        assert [c["id"] for c in found] == ["Greek/gr", "Hebrew/heb", "Other/notes"]
        assert found[1]["title"] == "Hebrew text"
        assert found[1]["versions"] == ["1.0"]

    def test_discover_collapses_copies_and_hides(self, tmp_path):
        roots = (str(tmp_path),)
        # two copies of one corpus (same app title) and an unrelated one
        make_corpus(tmp_path, "Main", "text", "Genesis", {16: 1, 17: 2, 18: 1})
        fork = make_corpus(tmp_path, "Fork", "text", "Genesis", {16: 1, 17: 2, 18: 1})
        shutil.copytree(
            fork / "tf" / "1.0",
            tmp_path / "Main" / "text" / "tf" / "2.0",
            ignore=shutil.ignore_patterns(".cfm"),
        )
        make_corpus(tmp_path, "Other", "letters", "Alpha Book", {16: 1, 17: 2, 18: 1})
        (tmp_path / "Other" / "letters" / "app" / "config.yaml").write_text(
            "provenanceSpec:\n  corpus: Letters\n", encoding="utf-8"
        )

        everything = discover(roots, collapse=False)
        assert [c["id"] for c in everything] == [
            "Fork/text",
            "Main/text",
            "Other/letters",
        ]

        listed = discover(roots)
        # the copy with the newest data stays, the other is named as a copy
        assert [c["id"] for c in listed] == ["Main/text", "Other/letters"]
        assert listed[0]["copies"] == ["Fork/text"]
        assert listed[0]["versions"] == ["1.0", "2.0"]
        # the version that opens is the one the app config names (1.0)
        assert listed[0]["version"] == "1.0"

        assert [c["id"] for c in discover(roots, hide=["other/LETTERS"])] == [
            "Main/text"
        ]

    def test_library_lists_open_state(self, client):
        corpora = client.get("/api/library").json()["corpora"]
        byId = {c["id"]: c for c in corpora}
        assert byId["Hebrew/heb"]["open"] is True

    def test_open_by_id_only(self, client, roots):
        response = client.post(
            "/api/corpora", json={"id": str(Path(roots[0]) / "Other" / "notes")}
        )
        assert response.status_code == 404
        response = client.post("/api/corpora", json={"id": "nowhere/nothing"})
        assert response.status_code == 404

    def test_open_twice(self, client):
        first = client.post("/api/corpora", json={"id": "Other/notes"}).json()
        again = client.post("/api/corpora", json={"id": "other/NOTES"}).json()
        assert again == {"name": "notes", "opened": False}
        assert first["name"] == "notes"
        names = [c["name"] for c in client.get("/api/corpora").json()["corpora"]]
        assert "notes" in names

    def test_open_a_version(self, client, roots):
        response = client.post("/api/corpora", json={"id": "Greek/gr@9.9"})
        assert response.status_code == 404
        assert "on this computer: 1.0" in response.json()["detail"]
        response = client.post(
            "/api/corpora", json={"id": "Greek/gr@latest", "name": "grlatest"}
        )
        assert response.status_code == 201

    def test_name_clash(self, client):
        response = client.post("/api/corpora", json={"id": "Greek/gr", "name": "heb"})
        assert response.status_code == 409

    def test_library_can_be_switched_off(self, roots):
        service = CorpusService([resolve_corpus("heb", roots)], roots=roots)
        app = create_app(service, SearchPool([]), manage_pool=False, library=False)
        with TestClient(app) as c:
            assert c.get("/api/library").json() == {"enabled": False, "corpora": []}
            assert c.post("/api/corpora", json={"id": "Greek/gr"}).status_code == 403


class TestStartEmpty:
    """Started without corpora, the browser opens them from the library."""

    def test_open_and_search(self, roots):
        service = CorpusService([], roots=roots)
        pool = SearchPool(
            [],
            sort_key=lambda corpus: service.api(corpus).N.sortKeyTuple,
            workers=1,
        )
        pool.start()
        app = create_app(service, pool, manage_pool=False)
        try:
            with TestClient(app) as c:
                assert c.get("/api/corpora").json() == {"corpora": []}
                library = c.get("/api/library").json()
                assert library["enabled"] is True
                assert not any(entry["open"] for entry in library["corpora"])

                opened = c.post("/api/corpora", json={"id": "Hebrew/heb"}).json()
                assert opened == {"name": "heb", "opened": True}
                names = [x["name"] for x in c.get("/api/corpora").json()["corpora"]]
                assert names == ["heb"]

                # the worker, started without corpora, can search the new one
                job = c.post(
                    "/api/corpora/heb/search", json={"template": "word"}
                ).json()
                deadline = time.monotonic() + 20
                while time.monotonic() < deadline:
                    status = c.get(f"/api/searches/{job['id']}").json()
                    if status["state"] not in ("queued", "running"):
                        break
                    time.sleep(0.05)
                assert status["state"] == "done"
                assert status["count"] > 0
        finally:
            pool.shutdown()


class TestCommandLine:
    def test_corpora_are_optional(self):
        assert parse_args([]).corpora == []

    def test_no_library_needs_a_corpus(self, capsys):
        assert main(["--no-library"]) == 2
        assert "--no-library" in capsys.readouterr().err


class TestOnly:
    """A library limited to a few corpora, e.g. for a test version."""

    def test_discover_only(self, roots):
        found = discover(roots, only=["hebrew/HEB", "Other/notes"])
        assert [c["id"] for c in found] == ["Hebrew/heb", "Other/notes"]

    def test_library_and_opening(self, roots):
        service = CorpusService([], roots=roots, only=["Hebrew/heb"])
        app = create_app(service, SearchPool([]), manage_pool=False)
        with TestClient(app) as c:
            corpora = c.get("/api/library").json()["corpora"]
            assert [x["id"] for x in corpora] == ["Hebrew/heb"]
            # a corpus outside the limit cannot be opened either
            assert c.post("/api/corpora", json={"id": "Greek/gr"}).status_code == 404
            assert c.post("/api/corpora", json={"id": "Hebrew/heb"}).status_code == 201
