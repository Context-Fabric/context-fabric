"""Building a translation from an OSIS source (the Statenvertaling), with a
small OSIS file written here, so without the network."""

from __future__ import annotations

import importlib
from dataclasses import replace
from pathlib import Path

import pytest
from cfabric import Fabric
from cfabric_browser.net import sha256_bytes
from cfabric_browser.translations import TRANSLATIONS, BuildError, build
from cfabric_browser.translations.build import (
    SourceVerse,
    Word,
    fetch,
    read_osis,
    renumber_by_order,
)
from cfabric_browser.translations.catalog import OSIS_BOOKS, Join, Split

# the module (the package's `build` is the function)
building = importlib.import_module("cfabric_browser.translations.build")

STV = TRANSLATIONS["statenvertaling"]


def osis(books: str, title: str = "De Statenvertaling") -> bytes:
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<osis xmlns="http://www.bibletechnologies.net/2003/OSIS/namespace">
  <osisText osisIDWork="STV" xml:lang="nl">
    <header><work osisWork="STV"><title>{title}</title></work></header>
    {books}
  </osisText>
</osis>""".encode()


# Alpha: three verses in the source; the BHSA (here) has 2 + 2, after
# dividing 1:2 and joining 2:1 with 2:2. Beta: chapters 1 and 2 of the source
# are one chapter here.
SOURCE = osis("""
<div osisID="Alp" type="book"><title type="main">Alpha</title>
  <chapter osisID="Alp.1">
    <verse osisID="Alp.1.1">In den <w lemma="strong:H7225">beginne</w> <w lemma="strong:H1254 strong:H8804">schiep</w> <divineName><w lemma="strong:H3068">Heere</w></divineName> <w lemma="strong:H853"></w>, <transChange type="added">was</transChange> het.<note><catchWord>beginne</catchWord> Van den tijd, <reference osisRef="Ps.90.2">Psa 90:2</reference> .</note><note>Zonder vangwoord.</note></verse>
    <verse osisID="Alp.1.2">Toen zeide hij: Ga. Daarna stond hij op.</verse>
  </chapter>
  <chapter osisID="Alp.2">
    <verse osisID="Alp.2.1">Hun paarden;</verse>
    <verse osisID="Alp.2.2">hun muildieren.<note><catchWord>muildieren</catchWord> Ezels.</note></verse>
  </chapter>
</div>
<div osisID="Bet" type="book">
  <chapter osisID="Bet.1"><verse osisID="Bet.1.1"><title type="acrostic">Aleph.</title> Een.</verse></chapter>
  <chapter osisID="Bet.2"><verse osisID="Bet.2.1">Twee.</verse></chapter>
</div>
""")

COUNTS = {"Alpha": [2, 2], "Beta": [2]}

TEST = replace(
    STV,
    id="test-osis",
    repo="test-osis",
    url="https://example.org/stv.xml",
    books=(("Alpha", "Alp"), ("Beta", "Bet")),
    checksum=sha256_bytes(SOURCE),
    splits=(Split("Alpha", 1, 2, ("Daarna stond",)),),
    joins=(Join("Alpha", 2, 1),),
)


@pytest.fixture
def counts(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(building, "hebrew_verses", lambda: COUNTS)


def test_read_osis_gives_words_with_what_they_carry() -> None:
    books = read_osis(TEST, SOURCE)
    first = books["Alpha"][0]
    assert (first.chapter, first.verse) == (1, 1)
    assert "".join(w.text + w.trailer for w in first.words) == (
        "In den beginne schiep Heere, was het."
    )
    by_text = {w.text: w for w in first.words}
    assert by_text["beginne"].strong == "H7225"
    # Strong's codes for verb forms (8675 and up) are not words
    assert by_text["schiep"].strong == "H1254"
    assert by_text["Heere"].strong == "H3068"
    assert by_text["was"].added and not by_text["het"].added
    # an empty <w> before a comma leaves no space before it
    assert by_text["Heere"].trailer == ", "
    assert first.margin == ("beginne — Van den tijd, Psa 90:2. | Zonder vangwoord.")
    # the letters of an acrostic are text
    assert [w.text for w in books["Beta"][0].words] == ["Aleph", "Een"]


def test_read_osis_checks_the_title_and_markup() -> None:
    with pytest.raises(BuildError, match="source is 'Iets anders'"):
        read_osis(TEST, osis("", title="Iets anders"))
    with pytest.raises(BuildError, match=r"Alpha 1:1: unexpected markup <hi>"):
        read_osis(
            TEST,
            osis(
                '<div osisID="Alp" type="book"><chapter osisID="Alp.1">'
                '<verse osisID="Alp.1.1">Een <hi>woord</hi>.</verse>'
                '</chapter></div><div osisID="Bet" type="book"/>'
            ),
        )
    with pytest.raises(BuildError, match="Beta is missing"):
        read_osis(
            TEST,
            osis(
                '<div osisID="Alp" type="book"><chapter osisID="Alp.1">'
                '<verse osisID="Alp.1.1">Een.</verse></chapter></div>'
            ),
        )


def words(text: str) -> list[Word]:
    parts = text.split(" ")
    return [Word(w, " " if i < len(parts) - 1 else "") for i, w in enumerate(parts)]


def test_renumber_by_order_divides_joins_and_keeps_the_source_number() -> None:
    source = [
        SourceVerse(1, 1, words("Een."), None),
        SourceVerse(1, 2, words("Twee. Drie."), "n — twee"),
        SourceVerse(2, 1, words("Vier"), "a — vier"),
        SourceVerse(2, 2, words("en vijf."), "b — vijf"),
    ]
    chapters = renumber_by_order(
        "Alpha",
        source,
        [2, 2],
        (Split("Alpha", 1, 2, ("Drie.",)),),
        (Join("Alpha", 2, 1),),
    )
    found = [
        [
            (
                v.verse,
                "".join(w.text + w.trailer for w in v.words or []),
                v.printed,
                v.margin,
            )
            for v in verses
        ]
        for verses in chapters
    ]
    assert found == [
        [(1, "Een. ", None, None), (2, "Twee. ", "1:2", None)],
        [
            # the second part of 1:2 is the BHSA's 2:1; its notes go with it
            (1, "Drie. ", "1:2", "n — twee"),
            (2, "Vier en vijf. ", "2:1-2", "a — vier | b — vijf"),
        ],
    ]


def test_renumber_by_order_stops_on_a_wrong_total_or_a_missing_place() -> None:
    source = [SourceVerse(1, 1, words("Een."), None)]
    with pytest.raises(BuildError, match="1 verses after dividing and joining"):
        renumber_by_order("Alpha", source, [2])
    with pytest.raises(BuildError, match="no verse 3:1 to divide or join"):
        renumber_by_order("Alpha", source, [1], (), (Join("Alpha", 3, 1),))
    with pytest.raises(BuildError, match="cannot divide the verse before 'Nee'"):
        renumber_by_order("Alpha", source, [1], (Split("Alpha", 1, 1, ("Nee",)),))


def test_fetch_checks_the_pinned_checksum(tmp_path: Path) -> None:
    asked: list[str] = []

    def download(url: str) -> bytes:
        asked.append(url)
        return SOURCE

    assert fetch(TEST, cache=tmp_path, download=download) == {"": SOURCE}
    # from the cache the second time
    assert fetch(TEST, cache=tmp_path, download=download) == {"": SOURCE}
    assert asked == ["https://example.org/stv.xml"]
    changed = replace(TEST, id="other", checksum="0" * 64)
    with pytest.raises(BuildError, match="not the version this program"):
        fetch(changed, cache=tmp_path, download=download)


def test_build_writes_a_corpus_from_osis(tmp_path: Path, counts: None) -> None:
    data = build(
        TEST,
        root=tmp_path / "root",
        cache=tmp_path / "cache",
        download=lambda url: SOURCE,
    )
    api = Fabric(locations=str(data), silent="deep").loadAll(silent="deep")
    assert api
    (F, L, T) = (api.F, api.L, api.T)

    def verse(book: str, c: int, v: int) -> int:
        return T.nodeFromSection((book, c, v))

    def text(node: int) -> str:
        return "".join(F.text.v(w) + F.trailer.v(w) for w in L.d(node, otype="word"))

    assert text(verse("Alpha", 1, 2)) == "Toen zeide hij: Ga. "
    assert text(verse("Alpha", 2, 1)) == "Daarna stond hij op. "
    assert F.printed.v(verse("Alpha", 2, 1)) == "1:2"
    assert text(verse("Alpha", 2, 2)) == "Hun paarden; hun muildieren. "
    assert F.printed.v(verse("Alpha", 2, 2)) == "2:1-2"
    assert F.margin.v(verse("Alpha", 2, 2)) == "muildieren — Ezels."
    assert F.printed.v(verse("Beta", 1, 2)) == "2:1"
    added = [F.text.v(w) for w in F.otype.s("word") if F.added.v(w)]
    assert added == ["was"]
    assert {F.text.v(w): F.strong.v(w) for w in L.d(verse("Alpha", 1, 1))}[
        "beginne"
    ] == "H7225"
    assert "strong" in api.Fall() and "margin" in api.Fall()


def test_the_statenvertaling_has_the_bhsa_books_in_order() -> None:
    from cfabric_browser.translations.build import hebrew_verses

    assert [name for (name, _) in STV.books] == list(hebrew_verses())
    assert STV.books == OSIS_BOOKS
    assert STV.url.endswith("/STV.osis.xml") and STV.checksum
