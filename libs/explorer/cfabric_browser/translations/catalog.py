"""The translations the browser can add, and what is known about their sources.

A translation is described by a `Translation`: where its books are, what the
source must say about itself (title, licence), and the few places where its
verse division differs from the Hebrew Bible's in the BHSA (`Split`). The
builder (`build.py`) turns it into an ordinary corpus.
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True)
class Split:
    """A source verse that the BHSA divides into several verses.

    The verse `chapter:verse` (in the source's numbering) is cut before each
    text in `before`; the following verses of the chapter move up. With
    `to_previous_chapter`, the first part is the last verse of the previous
    chapter instead (Numbers 25:19, which the source joins to 26:1).
    """

    book: str
    chapter: int
    verse: int
    before: tuple[str, ...]
    to_previous_chapter: bool = False


@dataclass(frozen=True)
class Join:
    """Source verses that the BHSA has as one: `chapter:verse` (in the
    source's numbering) and the `more` verses after it, even across a chapter
    boundary (an OSIS source only, whose verses are renumbered by order)."""

    book: str
    chapter: int
    verse: int
    more: int = 1


@dataclass(frozen=True)
class Translation:
    """A translation that can be downloaded and built into a corpus."""

    id: str
    # short, as a column heading, and in full
    title: str
    name: str
    language: str
    # where the corpus goes, in Text-Fabric layout: <org>/<repo>/tf/<version>
    org: str
    repo: str
    version: str
    # what the source must say about itself (an OSIS source: its title; its
    # licence is not in the file, so `checksum` pins the file instead)
    source_title: str
    source_licence: str
    # the source's address per book, with {path} for a book's path; an OSIS
    # source is one file, at `url` itself
    url: str
    # (BHSA English name, path at the source), in the BHSA's order; for an
    # OSIS source the book's OSIS id instead of a path
    books: tuple[tuple[str, str], ...]
    licence: str
    licence_note: str
    source: str
    cite: str
    splits: tuple[Split, ...] = field(default_factory=tuple)
    # "sefaria": a JSON file per book in Sefaria's export, in the BHSA's
    # numbering but for `splits`; "osis": one OSIS file, whose verses are
    # numbered as the BHSA's by their order in each book, after `splits` and
    # `joins`
    format: str = "sefaria"
    joins: tuple[Join, ...] = field(default_factory=tuple)
    # the sha256 of an OSIS source file
    checksum: str | None = None
    # how the source's own verse numbers are described (feature `printed`)
    printed_as: str = "the 1917 print"

    @property
    def corpus_id(self) -> str:
        return f"{self.org}/{self.repo}"


SEFARIA = "https://storage.googleapis.com/sefaria-export/json/Tanakh/{path}"

JPS_FILE = "English/The Holy Scriptures A New Translation JPS 1917.json"

JPS_BOOKS = tuple(
    (name, f"{section}/{book}/{JPS_FILE}")
    for (name, section, book) in (
        ("Genesis", "Torah", "Genesis"),
        ("Exodus", "Torah", "Exodus"),
        ("Leviticus", "Torah", "Leviticus"),
        ("Numbers", "Torah", "Numbers"),
        ("Deuteronomy", "Torah", "Deuteronomy"),
        ("Joshua", "Prophets", "Joshua"),
        ("Judges", "Prophets", "Judges"),
        ("1_Samuel", "Prophets", "I Samuel"),
        ("2_Samuel", "Prophets", "II Samuel"),
        ("1_Kings", "Prophets", "I Kings"),
        ("2_Kings", "Prophets", "II Kings"),
        ("Isaiah", "Prophets", "Isaiah"),
        ("Jeremiah", "Prophets", "Jeremiah"),
        ("Ezekiel", "Prophets", "Ezekiel"),
        ("Hosea", "Prophets", "Hosea"),
        ("Joel", "Prophets", "Joel"),
        ("Amos", "Prophets", "Amos"),
        ("Obadiah", "Prophets", "Obadiah"),
        ("Jonah", "Prophets", "Jonah"),
        ("Micah", "Prophets", "Micah"),
        ("Nahum", "Prophets", "Nahum"),
        ("Habakkuk", "Prophets", "Habakkuk"),
        ("Zephaniah", "Prophets", "Zephaniah"),
        ("Haggai", "Prophets", "Haggai"),
        ("Zechariah", "Prophets", "Zechariah"),
        ("Malachi", "Prophets", "Malachi"),
        ("Psalms", "Writings", "Psalms"),
        ("Job", "Writings", "Job"),
        ("Proverbs", "Writings", "Proverbs"),
        ("Ruth", "Writings", "Ruth"),
        ("Song_of_songs", "Writings", "Song of Songs"),
        ("Ecclesiastes", "Writings", "Ecclesiastes"),
        ("Lamentations", "Writings", "Lamentations"),
        ("Esther", "Writings", "Esther"),
        ("Daniel", "Writings", "Daniel"),
        ("Ezra", "Writings", "Ezra"),
        ("Nehemiah", "Writings", "Nehemiah"),
        ("1_Chronicles", "Writings", "I Chronicles"),
        ("2_Chronicles", "Writings", "II Chronicles"),
    )
)

JPS1917 = Translation(
    id="jps1917",
    title="JPS 1917",
    name="The Holy Scriptures: A New Translation (Jewish Publication Society, 1917)",
    language="eng",
    org="Context-Fabric",
    repo="jps1917",
    version="1917",
    source_title="The Holy Scriptures: A New Translation (JPS 1917)",
    source_licence="Public Domain",
    url=SEFARIA,
    books=JPS_BOOKS,
    licence="Public domain in the United States",
    licence_note=(
        "Published in 1917, so in the public domain in the United States. "
        "Elsewhere it depends on the translators' dates; check the rules in "
        "your country."
    ),
    source=(
        "Sefaria's export (sefaria.org), from the Open Siddur Project's "
        "scans of the 1917 print, which JPS shared"
    ),
    cite=(
        "The Holy Scriptures According to the Masoretic Text: A New "
        "Translation (Philadelphia: Jewish Publication Society, 1917), "
        "from Sefaria's export"
    ),
    splits=(
        # the Ten Commandments: the BHSA has one verse per commandment
        Split(
            "Exodus",
            20,
            13,
            (
                "Thou shalt not commit adultery.",
                "Thou shalt not steal.",
                "Thou shalt not bear false witness",
            ),
        ),
        Split(
            "Deuteronomy",
            5,
            17,
            (
                "Neither shalt thou commit adultery.",
                "Neither shalt thou steal.",
                "Neither shalt thou bear false witness",
            ),
        ),
        Split("Numbers", 26, 1, ("that the LORD spoke",), to_previous_chapter=True),
    ),
)

STV_COMMIT = "1d1860efa1c56eb50a6cdb3c656a6a45bdc66788"

# the BHSA's books with their OSIS ids, in the BHSA's order
OSIS_BOOKS = (
    ("Genesis", "Gen"),
    ("Exodus", "Exod"),
    ("Leviticus", "Lev"),
    ("Numbers", "Num"),
    ("Deuteronomy", "Deut"),
    ("Joshua", "Josh"),
    ("Judges", "Judg"),
    ("1_Samuel", "1Sam"),
    ("2_Samuel", "2Sam"),
    ("1_Kings", "1Kgs"),
    ("2_Kings", "2Kgs"),
    ("Isaiah", "Isa"),
    ("Jeremiah", "Jer"),
    ("Ezekiel", "Ezek"),
    ("Hosea", "Hos"),
    ("Joel", "Joel"),
    ("Amos", "Amos"),
    ("Obadiah", "Obad"),
    ("Jonah", "Jonah"),
    ("Micah", "Mic"),
    ("Nahum", "Nah"),
    ("Habakkuk", "Hab"),
    ("Zephaniah", "Zeph"),
    ("Haggai", "Hag"),
    ("Zechariah", "Zech"),
    ("Malachi", "Mal"),
    ("Psalms", "Ps"),
    ("Job", "Job"),
    ("Proverbs", "Prov"),
    ("Ruth", "Ruth"),
    ("Song_of_songs", "Song"),
    ("Ecclesiastes", "Eccl"),
    ("Lamentations", "Lam"),
    ("Esther", "Esth"),
    ("Daniel", "Dan"),
    ("Ezra", "Ezra"),
    ("Nehemiah", "Neh"),
    ("1_Chronicles", "1Chr"),
    ("2_Chronicles", "2Chr"),
)

STATENVERTALING = Translation(
    id="statenvertaling",
    title="StV",
    name="Statenvertaling (1637), text of the 1888 edition",
    language="nld",
    org="Context-Fabric",
    repo="statenvertaling",
    version="1888",
    source_title="De Statenvertaling",
    source_licence="CC0-1.0",
    url=(
        "https://raw.githubusercontent.com/jajpater/statenvertaling/"
        f"{STV_COMMIT}/STV.osis.xml"
    ),
    books=OSIS_BOOKS,
    licence="Public domain (CC0 1.0)",
    licence_note=(
        "Translated in 1637, so in the public domain; the transcription of the "
        "1888 edition is dedicated to the public domain (CC0 1.0)."
    ),
    source=(
        "the OSIS transcription by Isidore-Guild and jajpater "
        "(github.com/jajpater/statenvertaling), with the translators' "
        "kanttekeningen and Strong's numbers"
    ),
    cite=(
        "Statenvertaling (1637), text of the 1888 edition, transcription by "
        "Isidore-Guild and jajpater (CC0), "
        f"github.com/jajpater/statenvertaling @ {STV_COMMIT[:7]}"
    ),
    format="osis",
    checksum="cd3f8ab208febebc9b3eb9fee463707988e3ac3f9eb159a7f39ce55f6404d25c",
    printed_as="the Statenvertaling",
    # where the BHSA divides the verses otherwise than by their order alone;
    # found by comparing the words' Strong's numbers with the OSHB's
    splits=(
        # the first part is the BHSA's 25:19
        Split("Numbers", 26, 1, ("dat de Heere sprak",)),
        # the second part is the BHSA's 21:1
        Split("1_Samuel", 20, 42, ("Daarna stond hij op",)),
        Split("1_Chronicles", 12, 4, ("en Jirmeja,",)),
    ),
    joins=(
        # the horses and mules (7:68) are not in the Hebrew
        Join("Nehemiah", 7, 67),
        Join("Nehemiah", 7, 73),
        Join("Isaiah", 63, 19),
    ),
)

TRANSLATIONS: dict[str, Translation] = {t.id: t for t in (JPS1917, STATENVERTALING)}
