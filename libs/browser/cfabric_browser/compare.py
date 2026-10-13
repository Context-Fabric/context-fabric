"""Textual differences between two corpora at the same passage.

Two Hebrew texts (e.g. Genesis 1 in the MT/BHSA and in the SP) are compared
verse by verse on their consonants:

*   vowels, accents and other marks don't count;
*   matres lectionis do count, but a difference made only of matres
    (plene against defective spelling) is reported as `matres`, every other
    difference as `variant`;
*   word division doesn't count: the consonants of a verse are compared
    without word boundaries, and the differences are mapped back to the
    words that contain them;
*   a plus of whole words is marked in the text that has it; the other text
    has nothing to mark at that place.

The consonants of a word come from `g_cons_utf8` when the corpus has it, and
are otherwise stripped from the word's text in the original script.
"""

from __future__ import annotations

import re
import unicodedata
from dataclasses import dataclass, field
from difflib import SequenceMatcher
from typing import Any

from cfabric.display import DisplayConfig, node_from_section_ref

from cfabric_browser.books import canonical_book
from cfabric_browser.corpora import CorpusService
from cfabric_browser.witnesses import (
    PLACEHOLDER_TYPES,
    RECONSTRUCTED,
    SIGN_TYPE,
    format_ref,
    parse_ref,
    siglum_key,
)

CONSONANT_FEATURE = "g_cons_utf8"
HEBREW_LETTER = re.compile("[א-ת]")
# final forms depend on word division, which doesn't count
FINALS = str.maketrans("ךםןףץ", "כמנפצ")
TO_FINAL = {normal: final for (final, normal) in zip("ךםןףץ", "כמנפצ")}


def with_finals(word: str) -> str:
    """A word's consonants for people: with its last letter in final form."""
    return word[:-1] + TO_FINAL.get(word[-1], word[-1]) if word else word


MATRES = frozenset("אהוי")
UNKNOWN = "◦"  # a letter that is there but cannot be read

MATRES_ONLY = "matres"
VARIANT = "variant"

Words = list[tuple[int, str]]
Marks = dict[int, str]


def word_type(api, config: DisplayConfig) -> str:
    """The node type of words: `word`, or else a base type, or else the slot."""
    types = api.F.otype.all
    if "word" in types:
        return "word"
    bases = [t for t in types if t in config.base_types and t != config.slot_type]
    return str(bases[0]) if bases else config.slot_type


def consonants(text: str) -> str:
    """The Hebrew letters of a text, with final forms made normal."""
    # decomposed, a precomposed shin with dot (U+FB2A, as in SP) is a plain shin
    text = unicodedata.normalize("NFD", text)
    return "".join(HEBREW_LETTER.findall(text.translate(FINALS)))


def verse_words(api, config: DisplayConfig, verse: int) -> Words:
    """The words of a verse with their consonants."""
    wType = word_type(api, config)
    feature = api.Fs(CONSONANT_FEATURE) if CONSONANT_FEATURE in api.Fall() else None
    fmt = next(
        (f for f in config.text_formats if f.startswith("text-orig")),
        config.default_format,
    )
    words = api.L.d(verse, otype=wType) if api.F.otype.v(verse) != wType else [verse]
    result: Words = []
    for w in words:
        w = int(w)
        value = feature.v(w) if feature else None
        if value is None:
            value = api.T.text(w, fmt=fmt)
        result.append((w, consonants(str(value))))
    return result


def _mark(marks: Marks, node: int, kind: str) -> None:
    if marks.get(node) != VARIANT:
        marks[node] = kind


@dataclass
class Unit:
    """A place where two texts differ: letters `a[i1:i2]` against `b[j1:j2]`."""

    kind: str
    i1: int
    i2: int
    j1: int
    j2: int
    aWords: list[int] = field(default_factory=list)
    bWords: list[int] = field(default_factory=list)


def diff_words(
    a: Words, b: Words, known: list[bool] | None = None
) -> tuple[Marks, Marks]:
    """The words of `a` and `b` that differ, each with the kind of difference.

    See `diff_units`.
    """
    marksA: Marks = {}
    marksB: Marks = {}
    for unit in diff_units(a, b, known):
        for w in unit.aWords:
            _mark(marksA, w, unit.kind)
        for w in unit.bWords:
            _mark(marksB, w, unit.kind)
    return marksA, marksB


def diff_units(a: Words, b: Words, known: list[bool] | None = None) -> list[Unit]:
    """The places where `a` and `b` differ, with the words that are marked there.

    The consonants are compared without word boundaries. Where letters are
    present in one text only, the other text marks the word they fall into,
    if they are attached to a word there; a plus of whole words leaves the
    other text unmarked.

    `known` flags, per letter of `b`, whether it is preserved (a manuscript)
    rather than reconstructed by an editor. Reconstructed letters keep the
    texts aligned but are never marked, and neither is what `a` has in their
    place. Letters of `a` that `b` lacks count only between two preserved
    letters: at the edge of what survives, or next to a reconstruction, they
    may just be lost.
    """
    textA = "".join(c for (_, c) in a)
    textB = "".join(c for (_, c) in b)
    ownerA = [w for (w, c) in a for _ in c]
    ownerB = [w for (w, c) in b for _ in c]
    units: list[Unit] = []

    def attached(owner: list[int], i: int, j: int) -> bool:
        # letters i and j (j = i + 1) are in the same word
        return 0 <= i and j < len(owner) and owner[i] == owner[j]

    def whole_words(owner: list[int], i1: int, i2: int) -> bool:
        # letters i1..i2 are one or more complete words
        return not attached(owner, i1 - 1, i1) and not attached(owner, i2 - 1, i2)

    def slide(
        text: str, owner: list[int], lo: int, hi: int, left: int, right: int
    ) -> int:
        # Letters present in one text only can often sit in several places
        # (בצלמנו ו כדמותנו: the extra ו is the word ו or the end of צלמנו).
        # Shift them within the equal stretches around them (`left`, `right`
        # letters) to the first place where they are whole words, if any.
        shifts = [0]
        s = 0
        while s < right and hi + s < len(text) and text[lo + s] == text[hi + s]:
            s += 1
            shifts.append(s)
        s = 0
        while s < left and lo - s > 0 and text[lo - s - 1] == text[hi - s - 1]:
            s += 1
            shifts.append(-s)
        for s in sorted(shifts):
            if whole_words(owner, lo + s, hi + s):
                return s
        return 0

    matcher = SequenceMatcher(None, textA, textB, autojunk=False)
    opcodes = matcher.get_opcodes()
    for k, (tag, i1, i2, j1, j2) in enumerate(opcodes):
        if tag == "equal":
            continue
        if tag in ("insert", "delete"):
            prev = opcodes[k - 1] if k > 0 and opcodes[k - 1][0] == "equal" else None
            nxt = (
                opcodes[k + 1]
                if k + 1 < len(opcodes) and opcodes[k + 1][0] == "equal"
                else None
            )
            left = prev[2] - prev[1] if prev else 0
            right = nxt[2] - nxt[1] if nxt else 0
            if tag == "insert":
                s = slide(textB, ownerB, j1, j2, left, right)
            else:
                s = slide(textA, ownerA, i1, i2, left, right)
            (i1, i2, j1, j2) = (i1 + s, i2 + s, j1 + s, j2 + s)
        kept = [j for j in range(j1, j2) if known is None or known[j]]
        if known is not None:
            if tag != "delete" and not kept:
                continue
            if tag == "delete" and not (
                0 < j1 < len(known) and known[j1 - 1] and known[j1]
            ):
                continue
            # at the edge of what survives, `a` goes on where `b` is lost:
            # only as many letters of `a` as `b` has stand against them
            if tag == "replace" and j2 == len(known):
                i2 = min(i2, i1 + (j2 - j1))
            if tag == "replace" and j1 == 0:
                i1 = max(i1, i2 - (j2 - j1))
        # spelling: matres added or left out within a word; a word of its
        # own (the article ה, the conjunction ו) or another letter is a variant
        if tag == "insert":
            spelling = {textB[j] for j in kept} <= MATRES and not whole_words(
                ownerB, j1, j2
            )
        elif tag == "delete":
            spelling = set(textA[i1:i2]) <= MATRES and not whole_words(ownerA, i1, i2)
        else:
            spelling = False
        unit = Unit(MATRES_ONLY if spelling else VARIANT, i1, i2, j1, j2)
        aWords = [ownerA[i] for i in range(i1, i2)]
        bWords = [ownerB[j] for j in kept]
        # letters only in b: mark the words of a they are attached to
        if i1 == i2:
            if attached(ownerB, j1 - 1, j1) and i1 > 0:
                aWords.append(ownerA[i1 - 1])
            if attached(ownerB, j2 - 1, j2) and i1 < len(ownerA):
                aWords.append(ownerA[i1])
        # letters only in a: likewise for b
        if j1 == j2:
            if attached(ownerA, i1 - 1, i1) and j1 > 0:
                bWords.append(ownerB[j1 - 1])
            if attached(ownerA, i2 - 1, i2) and j1 < len(ownerB):
                bWords.append(ownerB[j1])
        unit.aWords = sorted(set(aWords))
        unit.bWords = sorted(set(bWords))
        units.append(unit)
    return units


def _verses(api, node: int) -> list[int]:
    verseType = api.T.sectionTypes[-1]
    if api.F.otype.v(node) == verseType:
        return [node]
    return [int(v) for v in api.L.d(node, otype=verseType)]


def compare_passages(
    service: CorpusService, a: str, aRef: str, b: str, bRef: str, lang: str = "en"
) -> dict[str, Any]:
    """Compare the passage `aRef` of corpus `a` with `bRef` of corpus `b`.

    The passages must correspond (`bRef` is where `aRef` aligns to in `b`);
    their verses are paired by chapter and verse number.

    Raises
    ------
    ValueError
        If a reference is not a passage of its corpus.
    """
    aApi, bApi = service.api(a), service.api(b)
    aConfig, bConfig = service.config(a), service.config(b)
    aNode = node_from_section_ref(
        aApi, aRef, aConfig.section_sep1, aConfig.section_sep2, lang
    )
    bNode = node_from_section_ref(
        bApi, bRef, bConfig.section_sep1, bConfig.section_sep2, lang
    )

    def incomparable(reason: str) -> dict[str, Any]:
        return {"comparable": False, "reason": reason, "verses": 0, "a": {}, "b": {}}

    aligned = service.align(a, aRef, b, lang)
    if aligned is None or aligned["node"] != bNode:
        return incomparable("The panes show different passages.")
    bBook = bApi.T.sectionFromNode(bNode, lang="en")[0]

    marksA: Marks = {}
    marksB: Marks = {}
    paired = 0
    letters = False
    for aVerse in _verses(aApi, aNode):
        section = aApi.T.sectionFromNode(aVerse, lang="en")
        bVerse = bApi.T.nodeFromSection((bBook, *section[1:]), lang="en")
        if bVerse is None:
            continue
        aWords = verse_words(aApi, aConfig, aVerse)
        bWords = verse_words(bApi, bConfig, int(bVerse))
        if not any(c for (_, c) in aWords) or not any(c for (_, c) in bWords):
            continue
        letters = True
        paired += 1
        (mA, mB) = diff_words(aWords, bWords)
        marksA.update(mA)
        marksB.update(mB)

    if not letters:
        return incomparable("Only Hebrew texts can be compared so far.")
    return {
        "comparable": True,
        "reason": None,
        "verses": paired,
        "a": {str(n): k for (n, k) in marksA.items()},
        "b": {str(n): k for (n, k) in marksB.items()},
    }


def witness_letters(api, words: list[int]) -> tuple[Words, list[bool]]:
    """The consonants of the words of a manuscript, and which are preserved.

    A letter is preserved unless its slot (sign) is flagged as reconstructed.
    A lost or illegible sign (`ε`, `#`, `?` in the DSS) stands for letters
    that are there but unknown: it becomes a placeholder that is not preserved.
    """
    rec = api.Fs(RECONSTRUCTED).v if RECONSTRUCTED in api.Fall() else None
    signType = api.Fs(SIGN_TYPE).v if SIGN_TYPE in api.Fall() else None
    # a format of the slots: text-orig-extra (DSS) is one of words
    formats = api.T.formats
    fmt = "text-orig-full" if "text-orig-full" in formats else None
    result: Words = []
    known: list[bool] = []
    for w in words:
        letters = ""
        for s in api.E.oslots.s(w):
            if signType and signType(int(s)) in PLACEHOLDER_TYPES:
                letters += UNKNOWN
                known.append(False)
                continue
            c = consonants(str(api.T.text([int(s)], fmt=fmt)))
            letters += c
            known.extend([not (rec and rec(int(s)))] * len(c))
        result.append((w, letters))
    return result, known


def compare_witnesses(
    service: CorpusService, a: str, aRef: str, b: str, bRef: str, lang: str = "en"
) -> dict[str, Any]:
    """Compare passage `aRef` of `a` (MT, SP) with the scrolls at `bRef` of `b`.

    `bRef` is a passage in the book view of `b` (DSS). Each scroll that
    contains a verse (a witness) is compared with that verse of `a`, its
    reconstructed letters not counting (see `diff_words`). `a` marks a word
    when any witness differs there; `aWitnesses` names them.

    Raises
    ------
    ValueError
        If `aRef` is not a passage of `a`.
    """
    aApi, bApi = service.api(a), service.api(b)
    aConfig = service.config(a)
    index = service.book_index(b)
    aNode = node_from_section_ref(
        aApi, aRef, aConfig.section_sep1, aConfig.section_sep2, lang
    )

    def incomparable(reason: str) -> dict[str, Any]:
        return {"comparable": False, "reason": reason, "verses": 0, "a": {}, "b": {}}

    parsed = parse_ref(bRef)
    if index is None or parsed is None:
        return incomparable(f"{b} cannot be read by biblical book.")
    aligned = service.align(a, aRef, b, lang, view="books")
    if aligned is None or aligned["ref"] != format_ref(*parsed):
        return incomparable("The panes show different passages.")

    marksA: Marks = {}
    marksB: Marks = {}
    who: dict[int, set[str]] = {}
    verses = 0
    letters = False
    for aVerse in _verses(aApi, aNode):
        section = aApi.T.sectionFromNode(aVerse, lang="en")
        book = canonical_book(str(section[0]))
        if book is None or len(section) < 3:
            continue
        (chapter, verse) = (int(section[1]), int(section[2]))
        if not index.has(book, chapter, verse):
            continue
        aWords = verse_words(aApi, aConfig, aVerse)
        if not any(c for (_, c) in aWords):
            continue
        verses += 1
        for witness in index.books[book][chapter][verse]:
            (bWords, known) = witness_letters(bApi, witness.words)
            if not any(known):
                continue
            letters = True
            (mA, mB) = diff_words(aWords, bWords, known)
            for node, kind in mA.items():
                _mark(marksA, node, kind)
                who.setdefault(node, set()).add(witness.scroll)
            marksB.update(mB)

    if not letters:
        return incomparable("Only Hebrew texts can be compared so far.")
    return {
        "comparable": True,
        "reason": None,
        "verses": verses,
        "a": {str(n): k for (n, k) in marksA.items()},
        "b": {str(n): k for (n, k) in marksB.items()},
        "aWitnesses": {
            str(n): sorted(scrolls, key=siglum_key) for (n, scrolls) in who.items()
        },
    }


# three texts: which scrolls agree with MT and which with SP


def witness_reading(
    textA: str, textW: str, known: list[bool], i1: int, i2: int
) -> tuple[int, str] | None:
    """What a manuscript has where `a` has `textA[i1:i2]`, or None if unknown.

    Returns where the reading starts in `textW`, and the reading.

    The reading is what the manuscript has between the letters aligned with
    the letters of `a` just before and just after: these anchors and all
    letters between them must be preserved. At the start or end of the verse
    the manuscript's own first or last letter is the anchor; there an empty
    reading is unknown, because the text may just be lost.
    """
    if not textW:
        return None
    matcher = SequenceMatcher(None, textA, textW, autojunk=False)
    aligned = {
        i + k: j + k for (i, j, n) in matcher.get_matching_blocks() for k in range(n)
    }
    edge = i1 == 0 or i2 == len(textA)
    if i1 > 0:
        left = aligned.get(i1 - 1)
        if left is None or not known[left]:
            return None
    else:
        left = -1
        if not known[0]:
            return None
    if i2 < len(textA):
        right = aligned.get(i2)
        if right is None or not known[right]:
            return None
    else:
        right = len(textW)
        if not known[-1]:
            return None
    if right <= left or not all(known[left + 1 : right]):
        return None
    reading = textW[left + 1 : right]
    if edge and not reading:
        return None
    return (left + 1, reading)


def merge_units(units: list[Unit]) -> list[Unit]:
    """Join differences in the same words into one place.

    MT מארת against SP מאורות is two differences (two ו), but one place: a
    scroll reading מארות agrees with neither as a whole.
    """
    merged: list[Unit] = []
    for unit in units:
        last = merged[-1] if merged else None
        if last and (
            set(unit.aWords) & set(last.aWords) or set(unit.bWords) & set(last.bWords)
        ):
            last.i2 = max(last.i2, unit.i2)
            last.j2 = max(last.j2, unit.j2)
            last.aWords = sorted(set(last.aWords) | set(unit.aWords))
            last.bWords = sorted(set(last.bWords) | set(unit.bWords))
            if unit.kind == VARIANT:
                last.kind = VARIANT
        else:
            merged.append(
                Unit(
                    unit.kind,
                    unit.i1,
                    unit.i2,
                    unit.j1,
                    unit.j2,
                    unit.aWords,
                    unit.bWords,
                )
            )
    return merged


def compare_three(
    service: CorpusService,
    a: str,
    aRef: str,
    b: str,
    bRef: str,
    c: str,
    cRef: str,
    lang: str = "en",
) -> dict[str, Any]:
    """Which scrolls of `c` side with `a` (MT) and which with `b` (SP).

    `bRef` is the passage of `b` that corresponds to `aRef`, `cRef` the one in
    the book view of `c` (DSS). For every place where `a` and `b` differ in a
    verse (see `diff_units`), each scroll that has the verse is read there
    (see `witness_reading`): it agrees with `a`, with `b`, or with neither.
    Scrolls whose text is lost or reconstructed there are left out.

    Raises
    ------
    ValueError
        If a reference is not a passage of its corpus.
    """
    aApi, bApi, cApi = service.api(a), service.api(b), service.api(c)
    aConfig, bConfig = service.config(a), service.config(b)
    index = service.book_index(c)
    aNode = node_from_section_ref(
        aApi, aRef, aConfig.section_sep1, aConfig.section_sep2, lang
    )
    bNode = node_from_section_ref(
        bApi, bRef, bConfig.section_sep1, bConfig.section_sep2, lang
    )

    def incomparable(reason: str) -> dict[str, Any]:
        return {"comparable": False, "reason": reason, "verses": []}

    parsed = parse_ref(cRef)
    if index is None or parsed is None:
        return incomparable(f"{c} cannot be read by biblical book.")
    alignedB = service.align(a, aRef, b, lang)
    alignedC = service.align(a, aRef, c, lang, view="books")
    if alignedB is None or alignedB["node"] != bNode:
        return incomparable("The panes show different passages.")
    if alignedC is None or alignedC["ref"] != format_ref(*parsed):
        return incomparable("The panes show different passages.")
    bBook = bApi.T.sectionFromNode(bNode, lang="en")[0]

    def words_text(words: Words, nodes: list[int]) -> str:
        chosen = set(nodes)
        return " ".join(with_finals(c) for (w, c) in words if w in chosen)

    verses = []
    for aVerse in _verses(aApi, aNode):
        section = aApi.T.sectionFromNode(aVerse, lang="en")
        book = canonical_book(str(section[0]))
        if book is None or len(section) < 3:
            continue
        (chapter, verse) = (int(section[1]), int(section[2]))
        bVerse = bApi.T.nodeFromSection((bBook, *section[1:]), lang="en")
        if bVerse is None or not index.has(book, chapter, verse):
            continue
        aWords = verse_words(aApi, aConfig, aVerse)
        bWords = verse_words(bApi, bConfig, int(bVerse))
        units = merge_units(diff_units(aWords, bWords))
        if not units:
            continue
        textA = "".join(c for (_, c) in aWords)
        textB = "".join(c for (_, c) in bWords)
        readings: list[list[dict[str, Any]]] = [[] for _ in units]
        for witness in index.books[book][chapter][verse]:
            (wWords, known) = witness_letters(cApi, witness.words)
            textW = "".join(c for (_, c) in wWords)
            if not any(known):
                continue
            ownerW = [w for (w, c) in wWords for _ in c]
            for k, unit in enumerate(units):
                found = witness_reading(textA, textW, known, unit.i1, unit.i2)
                if found is None:
                    continue
                (start, reading) = found
                if reading == textA[unit.i1 : unit.i2]:
                    agrees = "a"
                elif reading == textB[unit.j1 : unit.j2]:
                    agrees = "b"
                else:
                    agrees = "other"
                inSpan = sorted({ownerW[j] for j in range(start, start + len(reading))})
                readings[k].append(
                    {
                        "scroll": witness.scroll,
                        "agrees": agrees,
                        # the letters in which it differs, and its words there
                        "reading": reading,
                        "words": inSpan,
                        "text": words_text(wWords, inSpan),
                    }
                )
        places = [
            {
                "kind": unit.kind,
                "a": words_text(aWords, unit.aWords),
                "b": words_text(bWords, unit.bWords),
                "aWords": unit.aWords,
                "bWords": unit.bWords,
                "witnesses": rs,
            }
            for (unit, rs) in zip(units, readings)
            if rs
        ]
        if places:
            verses.append(
                {
                    "verse": verse,
                    "ref": format_ref(book, chapter, verse),
                    "units": places,
                }
            )
    return {"comparable": True, "reason": None, "verses": verses}
