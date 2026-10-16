"""Linking the lexemes of the BHSA to entries of BDB.

The BHSA's lexemes (`BR>[`, `DBR/`) carry no dictionary numbers. The Open
Scriptures Hebrew Bible (OSHB, `morphhb`, CC BY 4.0) gives every word a lemma:
the (augmented) Strong's numbers of its morphemes, as `b/7225` for בְּ/רֵאשִׁית.
Both are the Leningrad Codex, so the words of a verse are aligned by their
consonants, and each BHSA word gets the lemma of the OSHB morpheme it overlaps
most. A lexeme then gets the lemmas of its words, counted. Open Scriptures'
`AugIndex.xml` maps a lemma to an entry of its `LexicalIndex.xml`, which names
the BDB entry (`a.ab.ab`), the Strong's number and the root.

Differences in segmentation:
- the BHSA writes prefixes (`B`, `W`, `H`, `L`) as words of their own, as the
  OSHB splits them off with `/`;
- an article that is only a vowel (לַמֶּלֶךְ) is a word without consonants in
  the BHSA and a lemma `d` without a part of the text in the OSHB; neither
  takes part in the alignment;
- a pronominal suffix is part of the word in the BHSA and a part of its own
  without a lemma in the OSHB: its letters count for no lemma.
"""

from __future__ import annotations

import xml.etree.ElementTree as ET
from collections import Counter
from dataclasses import dataclass, field
from difflib import SequenceMatcher
from pathlib import Path

OSIS = "{http://www.bibletechnologies.net/2003/OSIS/namespace}"
OSHB = "{http://openscriptures.github.com/morphhb/namespace}"

# Hebrew letters as in the ETCBC's transliteration; shin and sin are one
# letter here (C), because unpointed text cannot tell them apart
HEBREW = dict(
    zip(
        "אבגדהוזחטיכךלמםנןסעפףצץקרשת",
        ">BGDHWZXVJKKLMMNNS<PPYYQRCT",
    )
)
ETCBC = set(">BGDHWZXVJKLMNS<PYQRFCT#")

# the letters a prefix of the OSHB's lemmas is written with
PREFIXES = {"b": "B", "c": "W", "d": "H", "i": "H", "k": "K", "l": "L", "m": "M"}
PREFIXES["s"] = "C"


def consonants(text: str) -> str:
    """The consonants of Hebrew text, in the ETCBC's letters."""
    return "".join(HEBREW[c] for c in text if c in HEBREW)


def etcbc_consonants(cons: str) -> str:
    """ETCBC consonants (`g_cons`) as `consonants` gives them: sin and shin
    (and an unpointed shin) as C."""
    return "".join("C" if c in "FC#" else c for c in cons if c in ETCBC)


def lemma_id(part: str) -> str:
    """An augmented Strong's number as `AugIndex.xml` writes it: `834 a` is
    `834a`; `+` (part of a name of several words) is left out."""
    return part.replace(" ", "").replace("+", "")


@dataclass(frozen=True)
class Segment:
    """A part of an OSHB word: its consonants, and the lemma it belongs to
    (None for a suffix)."""

    cons: str
    lemma: str | None
    language: str  # H or A, from the morphology


def segments(text: str, lemma: str, morph: str) -> list[Segment]:
    """The parts of an OSHB word with their lemmas.

    The text and the morphology have the same parts (`בְּ/רֵאשִׁית`, `HR/Ncfsa`);
    the lemma has the prefixes and the main word (`b/7225`), not the suffixes,
    and may have a prefix that has no part in the text (the article in
    לַמֶּלֶךְ: `l/d/4428` against `לַ/מֶּלֶךְ` and `HRd/Ncmsa`)."""
    parts = text.split("/")
    language = morph[:1] or "H"
    kinds = morph[1:].split("/")
    lemmas = lemma.split("/")
    main = lemma_id(lemmas[-1])
    prefixes = lemmas[:-1]
    if len(kinds) != len(parts):
        return [Segment(consonants(text), main, language)]
    suffixes = 0
    while suffixes < len(kinds) - 1 and kinds[len(kinds) - 1 - suffixes][:1] == "S":
        suffixes += 1
    m = len(parts) - suffixes - 1  # the part of the main word
    result = []
    for j in range(m):
        cons = consonants(parts[j])
        own = None
        # the next prefix written with this letter; prefixes without a letter
        # of their own (an article in a vowel) are passed over
        for k, p in enumerate(prefixes):
            if PREFIXES.get(p) == cons:
                own = p
                prefixes = prefixes[k + 1 :]
                break
        if own is None and prefixes:
            own = prefixes.pop(0)
        result.append(Segment(cons, own, language))
    result.append(Segment(consonants(parts[m]), main, language))
    for j in range(m + 1, len(parts)):
        result.append(Segment(consonants(parts[j]), None, language))
    return result


def _words(element: ET.Element) -> list[ET.Element]:
    """The words of a verse in the text, not those in notes (the qere)."""
    found = []
    for child in element:
        if child.tag == OSIS + "note":
            continue
        if child.tag == OSIS + "w":
            found.append(child)
        else:
            found.extend(_words(child))
    return found


def read_oshb(path: Path) -> dict[tuple[str, int, int], list[Segment]]:
    """The segments per verse of an OSHB book file (`Gen.xml`), by
    (OSIS book, chapter, verse)."""
    verses: dict[tuple[str, int, int], list[Segment]] = {}
    root = ET.parse(path).getroot()
    for verse in root.iter(OSIS + "verse"):
        book, chapter, number = verse.get("osisID", "").split(".")
        found: list[Segment] = []
        for w in _words(verse):
            found.extend(segments(w.text or "", w.get("lemma", ""), w.get("morph", "")))
        verses[(book, int(chapter), int(number))] = found
    return verses


def align(words: list[tuple[int, str]], found: list[Segment]) -> dict[int, Segment]:
    """The OSHB segment (with a lemma) each BHSA word of a verse overlaps
    most, by consonants; `words` are (node, ETCBC consonants)."""
    a: list[int] = []
    for i, (_, cons) in enumerate(words):
        a.extend([i] * len(etcbc_consonants(cons)))
    b: list[int] = []
    for j, s in enumerate(found):
        b.extend([j] * len(s.cons))
    textA = "".join(etcbc_consonants(cons) for (_, cons) in words)
    textB = "".join(s.cons for s in found)
    overlap: dict[int, Counter[int]] = {}
    matcher = SequenceMatcher(None, textA, textB, autojunk=False)
    for block in matcher.get_matching_blocks():
        for k in range(block.size):
            j = b[block.b + k]
            if found[j].lemma is not None:
                overlap.setdefault(a[block.a + k], Counter())[j] += 1
    return {
        words[i][0]: found[counts.most_common(1)[0][0]]
        for (i, counts) in overlap.items()
    }


@dataclass
class IndexEntry:
    """An entry of Open Scriptures' lexical index."""

    id: str  # aaf
    word: str  # אָבַד
    language: str  # heb, arc
    pos: str | None = None
    gloss: str | None = None
    bdb: str | None = None  # a.ac.aa
    strong: str | None = None
    twot: str | None = None
    root: str | None = None  # the entry of the root (for derived words)
    derived: list[str] = field(default_factory=list)


def read_lexical_index(path: Path) -> dict[str, IndexEntry]:
    entries = {}
    root = ET.parse(path).getroot()
    for part in root.iter(OSHB + "part"):
        language = part.get("{http://www.w3.org/XML/1998/namespace}lang", "heb")
        for e in part.iter(OSHB + "entry"):
            w = e.find(OSHB + "w")
            xref = e.find(OSHB + "xref")
            pos = e.find(OSHB + "pos")
            gloss = e.find(OSHB + "def")
            etym = e.find(OSHB + "etym")
            entry = IndexEntry(
                id=e.get("id", ""),
                word=(w.text or "") if w is not None else "",
                language=language,
                pos=pos.text if pos is not None else None,
                gloss=gloss.text if gloss is not None else None,
            )
            if xref is not None:
                entry.bdb = xref.get("bdb")
                entry.strong = xref.get("strong")
                entry.twot = xref.get("twot")
            if etym is not None and etym.text:
                ids = [x.strip() for x in etym.text.split(",") if x.strip()]
                if etym.get("type") == "sub":
                    entry.root = ids[0] if ids else None
                else:
                    entry.derived = ids
            entries[entry.id] = entry
    return entries


def read_aug_index(path: Path) -> dict[str, str]:
    """Augmented Strong's number (`834a`): lexical index id."""
    root = ET.parse(path).getroot()
    return {w.get("aug", ""): (w.text or "").strip() for w in root.iter(OSHB + "w")}
