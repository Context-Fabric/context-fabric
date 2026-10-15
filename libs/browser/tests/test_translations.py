"""Building a translation into a corpus, on small source files (no network)."""

from __future__ import annotations

import json
import time
from dataclasses import replace
from pathlib import Path

import pytest
from cfabric import Fabric
from cfabric_browser.translations import TRANSLATIONS, BuildError, Split, build
from cfabric_browser.translations.build import (
    Verse,
    check_counts,
    clean,
    fetch,
    hebrew_verses,
    lower,
    prepare,
    read_book,
    renumber,
    tokenize,
)

JPS = TRANSLATIONS["jps1917"]

# two books that are not in the BHSA, so their verses are not counted
TEST = replace(
    JPS,
    id="test",
    repo="test-translation",
    url="https://example.org/{path}",
    books=(("Alpha", "alpha.json"), ("Beta", "beta.json")),
    splits=(
        Split("Alpha", 1, 2, ("Thou shalt not steal.",)),
        Split("Beta", 2, 1, ("that the LORD spoke",), to_previous_chapter=True),
    ),
)

SOURCES = {
    "alpha.json": [
        [
            "IN THE beginning, ‘Speak thou with us.’",
            "Thou shalt not murder. Thou shalt not steal.",
            "And [the LORD] spoke.",
        ],
        ["The end.<br><small>[The end, again.]</small>"],
    ],
    "beta.json": [
        ["One.", "Two."],
        [
            "And it came to pass after the plague, that the LORD spoke unto Moses.",
            "Three.",
        ],
    ],
}


def source_file(text: list[list[str]], **extra: str) -> bytes:
    data = {
        "versionTitle": JPS.source_title,
        "license": JPS.source_licence,
        "text": text,
        **extra,
    }
    return json.dumps(data).encode("utf-8")


class Downloads:
    def __init__(self) -> None:
        self.urls: list[str] = []

    def __call__(self, url: str) -> bytes:
        self.urls.append(url)
        return source_file(SOURCES[url.rsplit("/", 1)[1]])


def test_clean_keeps_a_note_apart() -> None:
    raw = "lest I come and smite the land.<br><small>[Behold, I will send you Elijah.]</small>"
    assert clean("Malachi 3:24", raw) == (
        "lest I come and smite the land.",
        "Behold, I will send you Elijah.",
    )


def test_clean_tidies_space_and_entities() -> None:
    assert clean("x", "  Salt &amp; light\n here ") == ("Salt & light here", None)


def test_clean_stops_at_other_markup() -> None:
    with pytest.raises(BuildError, match="Genesis 1:1: unexpected markup '<i>'"):
        clean("Genesis 1:1", "In the <i>beginning</i>")


def test_tokenize_gives_the_text_back() -> None:
    for text in (
        "And they said unto Moses: ‘Speak thou with us, and we will hear.’",
        "and [the LORD] spoke with Moses.",
        "because he had fought against him—because Hadarezer had wars",
        "Then spoke we ’ unto them [, wrote they]: What?",
    ):
        words = tokenize(text)
        assert "".join(w + t for (w, t) in words) == text


def test_tokenize_puts_punctuation_before_a_word_with_it() -> None:
    assert tokenize("unto Moses: ‘Speak [the LORD]—now") == [
        ("unto", " "),
        ("Moses", ": "),
        ("‘Speak", " "),
        ("[the", " "),
        ("LORD", "]—"),
        ("now", ""),
    ]


def test_lower() -> None:
    assert [
        lower(w)
        for w in ("‘Speak", "[the", "LORD", "Jacob’s", "man-servant", "fathers’")
    ] == [
        "speak",
        "the",
        "lord",
        "jacob's",
        "man-servant",
        "fathers",
    ]


def test_renumber_divides_as_the_bhsa() -> None:
    chapters = [
        [("A.", None), ("Not murder. Not steal.", None), ("C.", None)],
        [("After the plague, the LORD spoke.", "a note"), ("D.", None)],
    ]
    splits = (
        Split("Book", 1, 2, ("Not steal.",)),
        Split("Book", 2, 1, ("the LORD",), to_previous_chapter=True),
    )
    result = renumber("Book", chapters, splits)
    assert [[(v.chapter, v.verse, v.text, v.printed) for v in ch] for ch in result] == [
        [
            (1, 1, "A.", None),
            (1, 2, "Not murder.", "1:2"),
            (1, 3, "Not steal.", "1:2"),
            (1, 4, "C.", "1:3"),
            (1, 5, "After the plague,", "2:1"),
        ],
        [(2, 1, "the LORD spoke.", "2:1"), (2, 2, "D.", None)],
    ]
    # a note stays with the end of the verse
    assert result[1][0].note == "a note"


def test_renumber_stops_when_a_division_is_not_found() -> None:
    with pytest.raises(BuildError, match="cannot divide the verse before 'Not steal.'"):
        renumber(
            "Book", [[("Not murder.", None)]], (Split("Book", 1, 1, ("Not steal.",)),)
        )
    with pytest.raises(BuildError, match="no verse 3:1 to divide"):
        renumber("Book", [[("A.", None)]], (Split("Book", 3, 1, ("x",)),))


def test_check_counts() -> None:
    verses = [[Verse(1, 1, "a"), Verse(1, 2, "b")], [Verse(2, 1, "c")]]
    check_counts("Book", verses, [2, 1])
    with pytest.raises(BuildError, match="Book: chapter 2 has 1 verses, not 3"):
        check_counts("Book", verses, [2, 3])
    with pytest.raises(BuildError, match="2 chapters, but the BHSA has 3"):
        check_counts("Book", verses, [2, 1, 1])


def test_read_book_checks_the_version_and_licence() -> None:
    assert read_book(JPS, "Alpha", source_file([["A."]])) == [["A."]]
    with pytest.raises(BuildError, match="the source is 'Other'"):
        read_book(JPS, "Alpha", source_file([["A."]], versionTitle="Other"))
    with pytest.raises(BuildError, match="licence is 'CC-BY-NC'"):
        read_book(JPS, "Alpha", source_file([["A."]], license="CC-BY-NC"))


def test_fetch_uses_the_cache_while_checksums_match(tmp_path: Path) -> None:
    downloads = Downloads()
    files = fetch(TEST, cache=tmp_path, download=downloads)
    assert sorted(files) == ["Alpha", "Beta"]
    assert downloads.urls == [
        "https://example.org/alpha.json",
        "https://example.org/beta.json",
    ]

    fetch(TEST, cache=tmp_path, download=downloads)
    assert len(downloads.urls) == 2

    (tmp_path / "test" / "Beta.json").write_text("changed", encoding="utf-8")
    assert fetch(TEST, cache=tmp_path, download=downloads)["Beta"] == files["Beta"]
    assert downloads.urls[2:] == ["https://example.org/beta.json"]


def test_build_writes_a_corpus(tmp_path: Path) -> None:
    steps: list[tuple[str, int, int]] = []
    data = build(
        TEST,
        root=tmp_path / "root",
        cache=tmp_path / "cache",
        progress=lambda *step: steps.append(step),
        download=Downloads(),
    )
    assert (
        data
        == tmp_path / "root" / "Context-Fabric" / "test-translation" / "tf" / "1917"
    )
    assert steps[-1] == ("build", 1, 1)
    config = (data.parent.parent / "app" / "config.yaml").read_text(encoding="utf-8")
    assert "writing: eng" in config
    assert "licenceNote:" in config

    api = Fabric(locations=str(data), silent="deep").loadAll(silent="deep")
    F, T = api.F, api.T
    assert [T.sectionFromNode(v) for v in F.otype.s("verse")][:4] == [
        ("Alpha", 1, 1),
        ("Alpha", 1, 2),
        ("Alpha", 1, 3),
        ("Alpha", 1, 4),
    ]
    first = T.nodeFromSection(("Alpha", 1, 1))
    assert T.text(first) == "IN THE beginning, ‘Speak thou with us.’ "
    assert [F.lower.v(w) for w in api.L.d(first, otype="word")][-2:] == ["with", "us"]
    steal = T.nodeFromSection(("Alpha", 1, 3))
    assert (T.text(steal), F.printed.v(steal)) == ("Thou shalt not steal. ", "1:2")
    end = T.nodeFromSection(("Alpha", 2, 1))
    assert (T.text(end), F.note.v(end)) == ("The end. ", "The end, again.")
    plague = T.nodeFromSection(("Beta", 1, 3))
    assert (T.text(plague), F.printed.v(plague)) == (
        "And it came to pass after the plague, ",
        "2:1",
    )
    assert len(list(api.S.search("word lower=lord"))) == 2


def test_build_stops_on_a_wrong_count(tmp_path: Path) -> None:
    wrong = replace(TEST, books=(("Obadiah", "alpha.json"),), splits=())
    with pytest.raises(BuildError, match="Obadiah: 2 chapters, but the BHSA has 1"):
        prepare(wrong, {"Obadiah": source_file(SOURCES["alpha.json"])})


def test_the_jps_catalogue_has_the_bhsa_books_in_order() -> None:
    assert [name for (name, _) in JPS.books] == list(hebrew_verses())
    assert sum(map(sum, hebrew_verses().values())) == 23213


# adding and removing, through the API


@pytest.fixture
def library(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    from cfabric_browser.app import create_app
    from cfabric_browser.corpora import CorpusService
    from cfabric_browser.translations.manage import Jobs
    from cfabric_browser.workers import SearchPool
    from fastapi.testclient import TestClient

    monkeypatch.setitem(TRANSLATIONS, "test", TEST)
    root = tmp_path / "root"
    root.mkdir()
    service = CorpusService([], roots=(str(root),))
    jobs = Jobs(
        root,
        cache=tmp_path / "cache",
        pending=tmp_path / "remove.json",
        download=Downloads(),
    )
    app = create_app(service, SearchPool([]), manage_pool=False, translations=jobs)
    with TestClient(app) as client:
        yield (client, service, jobs)


def added(client, tid: str = "test") -> dict:
    for _ in range(200):
        entry = client.get(f"/api/translations/{tid}").json()
        if entry["job"] and entry["job"]["state"] != "running":
            return entry
        time.sleep(0.05)
    raise AssertionError("the build did not finish")


def test_add_and_remove_a_translation(library) -> None:
    (client, _, _) = library
    entries = {
        t["id"]: t for t in client.get("/api/translations").json()["translations"]
    }
    assert entries["test"]["added"] is False
    assert entries["test"]["licenceNote"].startswith("Published in 1917")

    started = client.post("/api/translations/test")
    assert started.status_code == 202
    assert started.json()["job"]["state"] == "running"
    entry = added(client)
    assert entry["job"] == {
        "state": "done",
        "step": "build",
        "done": 1,
        "total": 1,
        "error": None,
    }
    assert entry["added"] is True

    corpora = client.get("/api/library").json()["corpora"]
    assert [(c["id"], c["title"]) for c in corpora] == [
        ("Context-Fabric/test-translation", f"JPS 1917 = {TEST.name}")
    ]

    removed = client.delete("/api/translations/test").json()
    assert (removed["added"], removed["removing"]) == (False, False)
    assert client.get("/api/library").json()["corpora"] == []
    assert client.delete("/api/translations/test").status_code == 404


def test_an_open_translation_is_removed_at_the_next_start(
    library, tmp_path: Path
) -> None:
    from cfabric_browser.corpora import resolve_corpus
    from cfabric_browser.translations.manage import remove_pending

    (client, service, jobs) = library
    client.post("/api/translations/test")
    added(client)
    # open, without loading it
    service.add(resolve_corpus("Context-Fabric/test-translation", service.roots))

    entry = client.delete("/api/translations/test").json()
    assert (entry["open"], entry["added"], entry["removing"]) == (True, True, True)
    # adding it again cancels the removal
    assert client.post("/api/translations/test").json()["removing"] is False
    client.delete("/api/translations/test")

    assert remove_pending(jobs.root, jobs.pending) == ["test"]
    assert not (jobs.root / "Context-Fabric" / "test-translation").exists()
    assert not (tmp_path / "remove.json").exists()


def test_a_failed_build_says_why(library, tmp_path: Path) -> None:
    (client, _, jobs) = library
    jobs.build_options["download"] = lambda url: source_file(
        [["A."]], license="CC-BY-NC"
    )
    client.post("/api/translations/test")
    entry = added(client)
    assert entry["job"]["state"] == "failed"
    assert "licence is 'CC-BY-NC'" in entry["job"]["error"]
    assert entry["added"] is False


def test_translations_outside_only_are_not_offered(tmp_path: Path) -> None:
    from cfabric_browser.app import create_app
    from cfabric_browser.corpora import CorpusService
    from cfabric_browser.workers import SearchPool
    from fastapi.testclient import TestClient

    service = CorpusService([], roots=(str(tmp_path),), only=["ETCBC/bhsa"])
    with TestClient(create_app(service, SearchPool([]), manage_pool=False)) as client:
        assert client.get("/api/translations").json()["translations"] == []
        assert client.post("/api/translations/jps1917").status_code == 404


def test_an_open_translation_shows_its_notes_and_print_numbers(library) -> None:
    from cfabric_browser.corpora import resolve_corpus

    (client, service, _) = library
    client.post("/api/translations/test")
    added(client)
    service.add(resolve_corpus("tr=Context-Fabric/test-translation", service.roots))

    assert client.get("/api/corpora").json()["corpora"][0]["translation"] is True
    card = client.get("/api/corpora/tr").json()
    assert card["translation"]["id"] == "test"
    assert card["display"]["direction"] == "ltr"

    alpha = client.get("/api/corpora/tr/passage", params={"ref": "Alpha 1"}).json()
    assert sorted(alpha["printeds"].values()) == ["1:2", "1:2", "1:3"]
    end = client.get("/api/corpora/tr/passage", params={"ref": "Alpha 2"}).json()
    assert list(end["notes"].values()) == ["The end, again."]

    # an English word: no gloss or morphology, but the word to search for
    summary = client.get("/api/corpora/tr/nodes/1/summary").json()["summary"]
    assert summary == [{"label": "word", "value": "in", "feature": "lower"}]
