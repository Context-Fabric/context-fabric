"""The lexicon of a corpus with lexeme nodes (BHSA): its lexemes, and where they occur.

A lexeme is a node of type `lex` whose words are the slots it contains. Its
features are read by role, from the first feature of that role the corpus has
(see `ROLES`): the BHSA has all of them. A corpus has a lexicon when it has
lexeme nodes with glosses; the DSS has lexeme nodes, but no glosses.
"""

from __future__ import annotations

import threading
from bisect import bisect_right
from collections import Counter
from dataclasses import dataclass, field
from typing import Any

from cfabric_browser.display import (
    DisplayConfig,
    RenderOptions,
    render_node,
    section_ref_for,
)

LEXEME_TYPE = "lex"
VERSE_CACHE = 32  # lexemes whose verses are kept

# role: candidate features of a lexeme node, the first one present is used
ROLES: dict[str, tuple[str, ...]] = {
    "lex": ("lex",),  # transliteration, unique within a language: BR>[
    "form": ("voc_lex_utf8", "lex_utf8"),  # shown: בָּרָא
    "cons": ("lex_utf8",),  # consonants, which give the first letter
    "gloss": ("gloss",),
    "sp": ("sp",),
    "language": ("language",),
    "freq": ("freq_lex",),
    "rank": ("rank_lex",),
    "nametype": ("nametype",),
}

# the ETCBC transliteration in the order of the Hebrew alphabet; F (sin) and C
# (shin) are both ש, so they sort together
ALPHABET = ">BGDHWZXVJKLMNS<PYQRFCT"
ORDER = {c: i for (i, c) in enumerate(ALPHABET)} | {"C": ALPHABET.index("F")}

# the Hebrew letters a word can start with (final forms cannot)
LETTERS = "אבגדהוזחטיכלמנסעפצקרשת"


def has_lexicon(api: Any) -> bool:
    """Whether a corpus has lexeme nodes with glosses."""
    F = api.F
    if LEXEME_TYPE not in F.otype.all:
        return False
    gloss = getattr(F, "gloss", None)
    if gloss is None:
        return False
    first = F.otype.s(LEXEME_TYPE)
    return len(first) > 0 and gloss.v(int(first[0])) is not None


def sort_key(lex: str) -> tuple[int, ...]:
    """Alphabetical order of a transliterated lexeme, in Hebrew order."""
    return tuple(ORDER[c] for c in lex if c in ORDER)


def first_letter(cons: str | None) -> str | None:
    """The Hebrew letter a lexeme starts with (vowels and marks left out)."""
    for c in cons or "":
        if c in LETTERS:
            return c
    return None


@dataclass
class Lexicon:
    api: Any
    features: dict[str, str]  # role: feature of the corpus
    entries: list[dict[str, Any]]
    byNode: dict[int, dict[str, Any]]
    _verses: dict[tuple[int, str | None, str], list[tuple[int, list[int]]]] = field(
        default_factory=dict
    )
    _byLex: dict[tuple[Any, Any], dict[str, Any]] = field(default_factory=dict)
    # per language of book names: the first slot of each book, and its name
    _books: dict[str, tuple[list[int], list[int], list[str]]] = field(
        default_factory=dict
    )
    # requests come in on several threads
    _lock: threading.Lock = field(default_factory=threading.Lock, repr=False)

    @classmethod
    def build(cls, api: Any) -> Lexicon | None:
        if not has_lexicon(api):
            return None
        F = api.F
        features = {}
        for role, candidates in ROLES.items():
            for name in candidates:
                if getattr(F, name, None) is not None:
                    features[role] = name
                    break
        values = {role: getattr(F, name).v for role, name in features.items()}
        entries = []
        for n in F.otype.s(LEXEME_TYPE):
            n = int(n)
            entry: dict[str, Any] = {"node": n}
            for role, value in values.items():
                v = value(n)
                entry[role] = int(v) if role in ("freq", "rank") and v else v
            if "freq" not in features:
                entry["freq"] = len(api.L.d(n, F.otype.slotType))
            entry["letter"] = first_letter(entry.get("cons") or entry.get("form"))
            entries.append(entry)
        entries.sort(
            key=lambda e: (e.get("language") or "", sort_key(e.get("lex") or ""))
        )
        return cls(api, features, entries, {e["node"]: e for e in entries})

    def entry(self, node: int) -> dict[str, Any] | None:
        return self.byNode.get(node)

    def gloss(self, lex: str, language: str | None = None) -> str | None:
        """The gloss of a lexeme by its transliteration (`BR>[`) and language
        (without one, Hebrew); if the lexeme is not in that language, as in
        another one (the DSS marks some words Aramaic whose lexeme the BHSA only
        has in Hebrew)."""
        entry = self.lookup(lex, language)
        return entry.get("gloss") if entry else None

    def lookup(self, lex: str, language: str | None = None) -> dict[str, Any] | None:
        """The lexeme by its transliteration and language (without one,
        Hebrew), else in another language, as `gloss` does."""
        return self.find(lex, language or "Hebrew") or self.find(lex, None)

    def find(self, lex: str, language: str | None) -> dict[str, Any] | None:
        """The lexeme with this transliteration in this language (None: any)."""
        with self._lock:
            if not self._byLex:
                for e in self.entries:
                    self._byLex.setdefault((e.get("lex"), e.get("language")), e)
                    self._byLex.setdefault((e.get("lex"), None), e)
        return self._byLex.get((lex, language))

    def words(self, node: int) -> list[int]:
        """The words of a lexeme, in canonical order."""
        return [int(w) for w in self.api.L.d(node, self.api.F.otype.slotType)]

    def verses(
        self, node: int, book: str | None = None, lang: str = "en"
    ) -> list[tuple[int, list[int]]]:
        """The verses a lexeme occurs in, each with its words there; with
        `book`, only those in the book of that name (in language `lang`)."""
        key = (node, book, lang)
        with self._lock:
            cached = self._verses.get(key)
        if cached is not None:
            return cached
        api = self.api
        verseType = api.T.sectionTypes[-1]
        verses: dict[int, list[int]] = {}
        for w in self.words(node):
            if book is not None and self.book_of(w, lang) != book:
                continue
            up = api.L.u(w, verseType)
            if up:
                verses.setdefault(int(up[0]), []).append(w)
        result = list(verses.items())
        # paging through a frequent lexeme asks for the same list again and again
        with self._lock:
            if len(self._verses) >= VERSE_CACHE:
                self._verses.pop(next(iter(self._verses)))
            self._verses[key] = result
        return result

    def book_of(self, word: int, lang: str = "en") -> str:
        """The name of the book a word is in; by its slot among the books'
        slot ranges (asking `L.u` for each word is slow for frequent
        lexemes)."""
        api = self.api
        if lang not in self._books:
            starts, ends, names = [], [], []
            for b in api.F.otype.s(api.T.sectionTypes[0]):
                slots = api.E.oslots.s(int(b))
                starts.append(int(slots[0]))
                ends.append(int(slots[-1]))
                names.append(str(api.T.sectionFromNode(int(b), lang=lang)[0]))
            self._books[lang] = (starts, ends, names)
        starts, ends, names = self._books[lang]
        i = bisect_right(starts, word) - 1
        if i >= 0 and word <= ends[i] and api.F.otype.v(word) == api.F.otype.slotType:
            return names[i]
        return book_name(api, word, api.T.sectionTypes[0], lang)

    def distribution(self, node: int, lang: str = "en") -> list[dict[str, Any]]:
        """How often a lexeme occurs per book, in the order of the books."""
        counts: Counter[str] = Counter()
        order: list[str] = []
        for w in self.words(node):
            name = self.book_of(w, lang)
            if name not in counts:
                order.append(name)
            counts[name] += 1
        return [{"book": name, "count": counts[name]} for name in order]


def book_name(api: Any, node: int, bookType: str, lang: str = "en") -> str:
    up = api.L.u(node, bookType)
    return str(api.T.sectionFromNode(up[0] if up else node, lang=lang)[0])


def render_verse(
    api: Any,
    config: DisplayConfig,
    verse: int,
    words: list[int],
    fmt: str | None = None,
    lang: str = "en",
) -> dict[str, Any]:
    """A verse as running text, with the words of a lexeme marked."""
    options = RenderOptions(
        fmt=fmt, full=True, lang=lang, highlights={w: "" for w in words}
    )
    return {
        "node": verse,
        "ref": section_ref_for(api, config, verse, lang=lang),
        "words": words,
        "tree": render_node(api, config, verse, "plain", options).to_dict(),
    }
