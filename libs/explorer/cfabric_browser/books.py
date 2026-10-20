"""A concordance of biblical book names across corpora.

Corpora name their books differently: `1_Samuel` (BHSA), `1Sam` (LXX),
`Samuel_1` (Peshitta). To show the same passage in several corpora, book
names are mapped to one canonical id (OSIS book ids). Only mappings that are
safe are listed: for instance the LXX's `2Esdr` (Ezra and Nehemiah together)
is not mapped to Ezra, because its chapters do not correspond.

Names are compared after normalisation: lower case, without spaces,
underscores and punctuation, so `Song_of_songs`, `Song of Songs` and
`songofsongs` are the same name.
"""

from __future__ import annotations

import re

# canonical id: other names (normalised automatically)
BOOKS: dict[str, tuple[str, ...]] = {
    "Gen": ("Genesis", "Ge", "Gn"),
    "Exod": ("Exodus", "Ex", "Exo"),
    "Lev": ("Leviticus", "Lv"),
    "Num": ("Numbers", "Nu", "Nm"),
    "Deut": ("Deuteronomy", "Dt", "Deu"),
    "Josh": ("Joshua", "Jos"),
    "Judg": ("Judges", "Jdg", "Jdgs"),
    "Ruth": ("Ru", "Rut"),
    "1Sam": ("1 Samuel", "Samuel 1", "1Sa", "I Samuel"),
    "2Sam": ("2 Samuel", "Samuel 2", "2Sa", "II Samuel"),
    "1Kgs": ("1 Kings", "Kings 1", "1Ki", "I Kings"),
    "2Kgs": ("2 Kings", "Kings 2", "2Ki", "II Kings"),
    "1Chr": ("1 Chronicles", "Chronicles 1", "1Ch", "I Chronicles"),
    "2Chr": ("2 Chronicles", "Chronicles 2", "2Ch", "II Chronicles"),
    "Ezra": ("Ezr",),
    "Neh": ("Nehemiah", "Nehemia"),
    "Esth": ("Esther", "Est"),
    "Job": ("Jb",),
    "Ps": ("Psalms", "Psalm", "Psa", "Pss"),
    "Prov": ("Proverbs", "Pr", "Prv"),
    "Eccl": ("Ecclesiastes", "Qoh", "Qohelet", "Ec", "Ecc"),
    "Song": ("Song of Songs", "Song of Solomon", "Cant", "Canticles", "SoS"),
    "Isa": ("Isaiah", "Is"),
    "Jer": ("Jeremiah", "Je"),
    "Lam": ("Lamentations", "La"),
    "Ezek": ("Ezekiel", "Eze", "Ezk"),
    "Dan": ("Daniel", "Da", "Dn"),
    "Hos": ("Hosea", "Ho"),
    "Joel": ("Jl",),
    "Amos": ("Am",),
    "Obad": ("Obadiah", "Ob"),
    "Jonah": ("Jon",),
    "Mic": ("Micah", "Mi"),
    "Nah": ("Nahum", "Na"),
    "Hab": ("Habakkuk", "Hb"),
    "Zeph": ("Zephaniah", "Zep"),
    "Hag": ("Haggai", "Hg"),
    "Zech": ("Zechariah", "Zec"),
    "Mal": ("Malachi", "Ml"),
    # books outside the Hebrew canon
    # where a corpus has two forms of a text, both point to the book; the
    # first form found in a corpus is used (see book_index)
    "Tob": ("Tobit", "Tb", "TobBA", "TobS", "Tobit A", "Tobit B"),
    "Jdt": ("Judith", "Jth"),
    "Wis": ("Wisdom", "Wisdom of Solomon", "Ws"),
    "Sir": ("Sirach", "Ecclesiasticus", "Ben Sira"),
    "Bar": ("Baruch",),
    "EpJer": ("Letter of Jeremiah", "Epistle of Jeremiah"),
    "Sus": ("Susanna",),
    "Bel": ("Bel and the Dragon",),
    "1Macc": ("1 Maccabees", "1Mac", "Maccabees 1", "Maccabees 1 A", "Maccabees 1 B"),
    "2Macc": ("2 Maccabees", "2Mac", "Maccabees 2"),
    "3Macc": ("3 Maccabees", "3Mac", "Maccabees 3"),
    "4Macc": ("4 Maccabees", "4Mac", "Maccabees 4"),
    "1Esd": ("1 Esdras", "1Esdr"),
    "PrMan": ("Prayer of Manasseh", "Prayer of Manasseh A", "Prayer of Manasseh B"),
    "Odes": ("Od", "Ode"),
    "PssSol": ("Psalms of Solomon", "PsSol"),
}


def normalise(name: str) -> str:
    return re.sub(r"[^0-9a-z]", "", str(name).lower())


ALIASES: dict[str, str] = {}
for canonical, others in BOOKS.items():
    for name in (canonical, *others):
        ALIASES.setdefault(normalise(name), canonical)


def canonical_book(name: str) -> str | None:
    """The canonical id of a book name, or None if it is not in the concordance."""
    return ALIASES.get(normalise(name))


def book_index(names: list[str]) -> dict[str, str]:
    """Map canonical ids to a corpus' own book names.

    When a corpus has several books for one id (the LXX has `Dan` and
    `DanTh`), the first one, normally the main text, is used.
    """
    index: dict[str, str] = {}
    for name in names:
        canonical = canonical_book(name)
        if canonical is not None:
            index.setdefault(canonical, name)
    return index
