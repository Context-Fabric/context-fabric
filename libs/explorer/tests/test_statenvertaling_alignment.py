"""The Statenvertaling's verses, numbered as the BHSA's, against the Hebrew:
the Strong's numbers of each verse's words compared with the lemmas of the
OSHB's verse with the same number (the OSHB numbers as the BHSA).

Needs the source (`~/.cfabric/translations/statenvertaling/source.xml`,
left by `cfabric-browser add statenvertaling`) and the OSHB's books (in the
cache of the BDB build), so only with CFABRIC_REAL_CORPORA=1.
"""

from __future__ import annotations

import os
import re
import xml.etree.ElementTree as ET
from pathlib import Path

import pytest
from cfabric_browser.dictionaries.build import OSHB_COMMIT
from cfabric_browser.translations import TRANSLATIONS
from cfabric_browser.translations.build import prepare

STV = TRANSLATIONS["statenvertaling"]
SOURCE = Path("~/.cfabric/translations/statenvertaling/source.xml").expanduser()
OSHB = Path(f"~/.cfabric/dictionaries/cache/openscriptures/{OSHB_COMMIT[:7]}")
OSHB = OSHB.expanduser()
OSIS = "{http://www.bibletechnologies.net/2003/OSIS/namespace}"

# words in nearly every verse, which say little about which verse it is
COMMON = {853, 3068, 430, 559, 1961, 3605, 834, 3588, 5921, 413, 3808, 3117}

pytestmark = pytest.mark.skipif(
    os.environ.get("CFABRIC_REAL_CORPORA") != "1"
    or not SOURCE.is_file()
    or not OSHB.is_dir(),
    reason="needs the Statenvertaling's source, the OSHB and CFABRIC_REAL_CORPORA=1",
)


def hebrew(book: str) -> dict[tuple[int, int], set[int]]:
    root = ET.parse(OSHB / f"{book}.xml").getroot()
    verses = {}
    for verse in root.iter(OSIS + "verse"):
        (_, c, v) = verse.get("osisID", "").split(".")
        lemmas: set[int] = set()
        for w in verse.iter(OSIS + "w"):
            lemmas |= {int(n) for n in re.findall(r"\d+", w.get("lemma", ""))}
        verses[(int(c), int(v))] = lemmas - COMMON
    return verses


def test_every_verse_has_the_hebrew_of_its_number() -> None:
    books = prepare(STV, {"": SOURCE.read_bytes()})
    compared = poor = 0
    worst: list[str] = []
    for name, osis in STV.books:
        heb = hebrew(osis)
        for verses in books[name]:
            for verse in verses:
                numbers = {
                    int(n[1:])
                    for w in verse.words or []
                    if w.strong
                    for n in w.strong.split()
                } - COMMON
                lemmas = heb[(verse.chapter, verse.verse)]
                if not numbers or not lemmas:
                    continue  # Hosea has no Strong's numbers in the source
                compared += 1
                overlap = len(numbers & lemmas) / len(numbers | lemmas)
                if overlap < 0.2:
                    poor += 1
                    worst.append(f"{name} {verse.chapter}:{verse.verse}")
    assert compared > 22_000
    # a verse moved by one would overlap poorly in thousands of places
    assert poor < 30, worst
