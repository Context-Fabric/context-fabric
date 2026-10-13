"""Tests for working with several corpora: book concordance, alignment, library.

Two copies of the display_corpus fixture stand in for two editions of a text:
`heb` calls its book "Genesis", `gr` calls it "Gen" and numbers the second verse
of chapter 1 as verse 3, so that verse 1:2 has no exact counterpart there.
"""

import shutil
from pathlib import Path

import pytest
from cfabric_browser.app import create_app
from cfabric_browser.books import book_index, canonical_book
from cfabric_browser.corpora import CorpusService, discover, resolve_corpus
from cfabric_browser.workers import SearchPool
from fastapi.testclient import TestClient

FIXTURE = (
    Path(__file__).resolve().parents[2]
    / "core"
    / "tests"
    / "fixtures"
    / "display_corpus"
)


def make_corpus(
    root: Path, org: str, repo: str, book: str, verses: dict[int, int]
) -> Path:
    target = root / org / repo
    shutil.copytree(FIXTURE, target, ignore=shutil.ignore_patterns(".cfm"))
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
    make_corpus(root, "Hebrew", "heb", "Genesis", {16: 1, 17: 2, 18: 1})
    make_corpus(root, "Greek", "gr", "Gen", {16: 1, 17: 3, 18: 1})
    make_corpus(root, "Other", "notes", "Alpha Book", {16: 1, 17: 2, 18: 1})
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


class TestLibrary:
    def test_discover(self, roots):
        found = discover(roots)
        assert [c["id"] for c in found] == ["Greek/gr", "Hebrew/heb", "Other/notes"]
        assert found[1]["title"] == "Display test corpus"
        assert found[1]["versions"] == ["1.0"]

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

    def test_name_clash(self, client):
        response = client.post("/api/corpora", json={"id": "Greek/gr", "name": "heb"})
        assert response.status_code == 409

    def test_library_can_be_switched_off(self, roots):
        service = CorpusService([resolve_corpus("heb", roots)], roots=roots)
        app = create_app(service, SearchPool([]), manage_pool=False, library=False)
        with TestClient(app) as c:
            assert c.get("/api/library").json() == {"enabled": False, "corpora": []}
            assert c.post("/api/corpora", json={"id": "Greek/gr"}).status_code == 403
