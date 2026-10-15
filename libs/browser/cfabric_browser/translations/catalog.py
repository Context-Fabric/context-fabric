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
class Translation:
    """A translation that can be downloaded and built into a corpus."""

    id: str
    title: str
    language: str
    # where the corpus goes, in Text-Fabric layout: <org>/<repo>/tf/<version>
    org: str
    repo: str
    version: str
    # what the source must say about itself
    source_title: str
    source_licence: str
    # the source's address per book, with {path} for a book's path
    url: str
    # (BHSA English name, path at the source), in the BHSA's order
    books: tuple[tuple[str, str], ...]
    licence: str
    licence_note: str
    source: str
    cite: str
    splits: tuple[Split, ...] = field(default_factory=tuple)

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

TRANSLATIONS: dict[str, Translation] = {t.id: t for t in (JPS1917,)}
