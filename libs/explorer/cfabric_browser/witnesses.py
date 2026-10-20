"""Reading a corpus of manuscripts by biblical book.

In the Dead Sea Scrolls the sections are scroll, fragment and line. Biblical
text is identified at the word level instead: words carry the features
`book`, `chapter` and `verse`. A `BookIndex` turns those into a second way to
read the corpus: per verse, the scrolls that contain it (its *witnesses*),
each with the words of that verse.

A corpus gets a book index when its words have these features with values
that are biblical books (`Gen`, `Is`, …; see `books.py`) and its own top
sections are not books. Words whose `book` is not a biblical book (in the
DSS: words of biblical scrolls not identified with a verse, which carry the
scroll name) are left out.

Book view references use the English book names of `books.py`
(`Genesis 1`, `Genesis 1:1`), so they match those of MT and SP; any name the
concordance knows is accepted (`Gen 1:1`).
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Any

from cfabric_browser.books import BOOKS, canonical_book

WORD = "word"
BOOK, CHAPTER, VERSE = "book", "chapter", "verse"
RECONSTRUCTED = "rec"  # on signs: reconstructed by a modern editor
BIBLICAL = "biblical"  # 2: also in Abegg's non-biblical file (DSS)

ORDER = {book: i for (i, book) in enumerate(BOOKS)}
SIGLUM = re.compile(r"^(\d+)Q(\d+)(.*)$")
REF = re.compile(r"^(.+?)\s+(\d+)(?::(\d+))?$")


def book_name(canonical: str) -> str:
    """The English name of a book: `Genesis` for `Gen`, `Ruth` for `Ruth`."""
    names = [n for n in BOOKS.get(canonical, ()) if len(n) > len(canonical)]
    return names[0] if names else canonical


def siglum_key(name: str) -> tuple:
    """Sort key for scroll names: by cave, then number (1Q1, 4Q2, 4Q10, …)."""
    match = SIGLUM.match(name)
    if match:
        return (0, int(match[1]), int(match[2]), match[3].lower())
    return (1, name.lower())


def parse_ref(ref: str) -> tuple[str, int, int | None] | None:
    """`Genesis 1:3` → (`Gen`, 1, 3); None if this is no book reference."""
    match = REF.match(ref.strip().replace("_", " "))
    if not match:
        return None
    canonical = canonical_book(match[1])
    if canonical is None:
        return None
    return (canonical, int(match[2]), int(match[3]) if match[3] else None)


def format_ref(book: str, chapter: int, verse: int | None = None) -> str:
    ref = f"{book_name(book)} {chapter}"
    return ref if verse is None else f"{ref}:{verse}"


@dataclass
class Witness:
    """The words of one verse in one scroll."""

    scroll: str
    words: list[int] = field(default_factory=list)
    also_non_biblical: bool = False


@dataclass
class BookIndex:
    """Book → chapter → verse → witnesses, sorted by scroll."""

    word_type: str
    books: dict[str, dict[int, dict[int, list[Witness]]]]

    @classmethod
    def build(cls, api) -> BookIndex | None:
        """The book index of a corpus, or None if it has no biblical words."""
        F, T = api.F, api.T
        if not {BOOK, CHAPTER, VERSE} <= set(api.Fall()):
            return None
        if WORD not in F.otype.all:
            return None
        fBook, fChapter, fVerse = api.Fs(BOOK).v, api.Fs(CHAPTER).v, api.Fs(VERSE).v
        fBiblical = api.Fs(BIBLICAL).v if BIBLICAL in api.Fall() else None
        top = T.sectionTypes[0] if T.sectionTypes else None

        canonical: dict[str, str | None] = {}
        verses: dict[tuple[str, int, int], dict[str, Witness]] = {}
        for w in F.otype.s(WORD):
            w = int(w)
            name = fBook(w)
            if not name:
                continue
            if name not in canonical:
                canonical[name] = canonical_book(str(name))
            book = canonical[name]
            if book is None:
                continue
            (chapter, verse) = (str(fChapter(w)), str(fVerse(w)))
            if not (chapter.isdigit() and verse.isdigit()):
                continue
            scroll = str(T.sectionFromNode(w, lang="en")[0]) if top else ""
            witnesses = verses.setdefault((book, int(chapter), int(verse)), {})
            witness = witnesses.get(scroll)
            if witness is None:
                witness = witnesses[scroll] = Witness(scroll)
            witness.words.append(w)
            if fBiblical and fBiblical(w) == 2:
                witness.also_non_biblical = True
        if not verses:
            return None

        books: dict[str, dict[int, dict[int, list[Witness]]]] = {}
        for key in sorted(
            verses, key=lambda k: (ORDER.get(k[0], len(ORDER)), k[1], k[2])
        ):
            witnesses = verses[key]
            books.setdefault(key[0], {}).setdefault(key[1], {})[key[2]] = [
                witnesses[s] for s in sorted(witnesses, key=siglum_key)
            ]
        return cls(WORD, books)

    def has(self, book: str, chapter: int, verse: int | None = None) -> bool:
        chapters = self.books.get(book, {})
        if verse is None:
            return chapter in chapters
        return verse in chapters.get(chapter, {})

    def summary(self) -> list[dict[str, Any]]:
        """The books with their chapters, verses and scrolls."""
        result = []
        for book, chapters in self.books.items():
            scrolls = {
                w.scroll for c in chapters.values() for v in c.values() for w in v
            }
            result.append(
                {
                    "book": book,
                    "label": book_name(book),
                    "ref": format_ref(book, next(iter(chapters))),
                    "scrolls": len(scrolls),
                    "verses": sum(len(c) for c in chapters.values()),
                    "chapters": [
                        {
                            "chapter": chapter,
                            "ref": format_ref(book, chapter),
                            "verses": len(verses),
                            "witnesses": sum(len(ws) for ws in verses.values()),
                        }
                        for (chapter, verses) in chapters.items()
                    ],
                }
            )
        return result

    def neighbours(self, book: str, chapter: int) -> dict[str, str | None]:
        """References of the chapters before and after, across books."""
        keys = [(b, c) for (b, chapters) in self.books.items() for c in chapters]
        i = keys.index((book, chapter))
        prev = keys[i - 1] if i > 0 else None
        nxt = keys[i + 1] if i + 1 < len(keys) else None
        return {
            "prev": format_ref(*prev) if prev else None,
            "next": format_ref(*nxt) if nxt else None,
        }


def place(api, words: list[int]) -> str:
    """Where words are in their scroll: `f1i:1` or `f1i:1–2` or `f1:9–f2:3`."""
    T = api.T
    first = T.sectionFromNode(words[0], lang="en")
    last = T.sectionFromNode(words[-1], lang="en")
    start = f"{first[1]}:{first[2]}" if len(first) > 2 else str(first[-1])
    if first == last:
        return start
    if len(last) > 2 and first[1] == last[1]:
        return f"{start}–{last[2]}"
    end = f"{last[1]}:{last[2]}" if len(last) > 2 else str(last[-1])
    return f"{start}–{end}"


# Placeholders in the DSS transcription (sign `type`, see the feature
# documentation of ETCBC/dss): `ε` (`missing`) is text that is lost and not
# reconstructed, ` # ` and ` ? ` (`unc`) a letter too damaged to identify.
# They are shown as in the editions: [ … ] for a gap, ◦ for an illegible letter.
# The transcription also marks morpheme breaks (`sep` `׳`, as in עמ׳י), which
# are not in the scrolls: they are left out.
SIGN_TYPE = "type"
PLACEHOLDERS = {"ε": "[…]", " # ": "◦", " ? ": "◦", "׳": ""}
PLACEHOLDER_TYPES = {"missing", "unc"}
SHOWN_TYPES = PLACEHOLDER_TYPES | {"sep"}


def show_placeholders(text: str) -> str:
    for glyph, shown in PLACEHOLDERS.items():
        text = text.replace(glyph, shown)
    return text


def dress_signs(api, fragments: list[dict[str, Any]]) -> None:
    """Prepare the signs of a manuscript (DSS) in render tree fragments.

    Parts reconstructed by an editor are flagged `dim`: slots (signs) with
    `rec`, and fragments shown as one text (a word in the structure view)
    whose slots are all reconstructed. Placeholders for lost and illegible
    text are shown as in the editions, and morpheme breaks are left out (see
    `PLACEHOLDERS`).
    """
    if RECONSTRUCTED not in api.Fall():
        return
    rec = api.Fs(RECONSTRUCTED).v
    signType = api.Fs(SIGN_TYPE).v if SIGN_TYPE in api.Fall() else None
    slotType = api.F.otype.slotType

    def visit(frags: list[dict[str, Any]]) -> None:
        for frag in frags:
            children = frag.get("children")
            if children:
                visit(children)
            elif "text" in frag:
                (begin, end) = frag["range"]
                if frag["type"] == slotType:
                    reconstructed = bool(rec(frag["node"]))
                    placeholder = signType and signType(frag["node"]) in SHOWN_TYPES
                else:
                    reconstructed = all(rec(s) for s in range(begin, end + 1))
                    placeholder = signType and any(
                        signType(s) in SHOWN_TYPES for s in range(begin, end + 1)
                    )
                if reconstructed:
                    frag["dim"] = True
                if placeholder:
                    frag["text"] = show_placeholders(frag["text"])

    visit(fragments)
