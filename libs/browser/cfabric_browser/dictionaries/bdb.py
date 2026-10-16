"""BDB's entries: Open Scriptures' ids and short definitions joined with
Sefaria's text, and Sefaria's HTML cleaned.

Open Scriptures' `BrownDriverBriggs.xml` gives every entry an id (`a.ac.aa`:
letter, root group, entry), its part of speech and short definitions, and the
links from the BHSA go to these ids. Its text is half skeletons, so the text
comes from Sefaria. Both follow BDB's order, so the two lists are joined by
order and by the consonants of their headwords (`join`).

Sefaria's HTML is reduced to a few tags (`clean`): references to verses
become `<a data-osis="2Sam.12.4">`, references to other entries
`<a data-entry="BDB, אבה">`; any tag not in `TAGS` stops the build, so that nothing
slips in unseen.
"""

from __future__ import annotations

import html
import re
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from difflib import SequenceMatcher
from html.parser import HTMLParser
from pathlib import Path

from cfabric_browser.books import canonical_book
from cfabric_browser.dictionaries.link import OSHB, consonants
from cfabric_browser.dictionaries.sefaria import plain

# tags kept, with the attributes kept
TAGS: dict[str, tuple[str, ...]] = {
    "big": (),
    "sub": (),
    "sup": (),
    "strong": (),
    "em": (),
    "br": (),
    "span": ("dir",),
    "a": (),  # rewritten: data-osis or data-entry
}
VOID = {"br"}


class CleanError(Exception):
    """Sefaria's HTML has something the cleaner doesn't know."""


@dataclass
class Entry:
    """An entry of Open Scriptures' BDB."""

    id: str  # a.ac.aa
    headword: str
    cons: str  # its consonants, as `link.consonants` gives them
    pos: str | None
    defs: list[str]
    root: bool  # the entry of a root (type="root")
    homonym: str | None  # mod="II"
    aramaic: bool

    @property
    def section(self) -> str:
        return self.id.rsplit(".", 1)[0]


def read_entries(path: Path) -> list[Entry]:
    """The entries of `BrownDriverBriggs.xml`, in its order."""
    root = ET.parse(path).getroot()
    entries = []
    for part in root.iter(OSHB + "part"):
        aramaic = part.get("{http://www.w3.org/XML/1998/namespace}lang") == "arc"
        for e in part.iter(OSHB + "entry"):
            w = e.find(OSHB + "w")
            head = "".join(w.itertext()) if w is not None else ""
            pos = e.find(OSHB + "pos")
            # the definitions at the head of the entry, before its senses
            defs = [
                "".join(d.itertext()).strip()
                for d in e.findall(OSHB + "def")
                if "".join(d.itertext()).strip()
            ]
            entries.append(
                Entry(
                    id=e.get("id", ""),
                    headword=head,
                    cons=consonants(head),
                    pos=pos.text if pos is not None else None,
                    defs=defs,
                    root=e.get("type") == "root",
                    homonym=e.get("mod"),
                    aramaic=aramaic,
                )
            )
    return entries


def headword_consonants(headword: str) -> str:
    return consonants(plain(headword))


def join(ours: list[str], theirs: list[str], reach: int = 3) -> dict[int, int]:
    """Pairs of positions in two lists of headword consonants in the same
    order: equal runs are joined, and so are the entries between two equal
    runs when both have the same number of them (spellings that differ).
    Then entries left over on both sides with the same consonants are joined
    when they are at most `reach` places from where they would be expected
    (two entries in the other order)."""
    pairs: dict[int, int] = {}
    matcher = SequenceMatcher(None, ours, theirs, autojunk=False)
    for tag, i1, i2, j1, j2 in matcher.get_opcodes():
        if tag == "equal" or (tag == "replace" and i2 - i1 == j2 - j1):
            for k in range(i2 - i1):
                pairs[i1 + k] = j1 + k
    taken = set(pairs.values())
    left = [j for j in range(len(theirs)) if j not in taken]
    offset = 0
    for i in range(len(ours)):
        if i in pairs:
            offset = pairs[i] - i
            continue
        for j in left:
            if abs(j - (i + offset)) <= reach and theirs[j] == ours[i]:
                pairs[i] = j
                left.remove(j)
                break
    return pairs


def osis(ref: str) -> str | None:
    """Sefaria's reference `II Samuel 12:4` as `2Sam.12.4` (also a chapter,
    or a range, whose start is taken)."""
    m = re.fullmatch(r"(.+?) (\d+)(?::(\d+))?(?:[-–].*)?", ref.strip())
    if not m:
        return None
    book = canonical_book(m.group(1))
    if book is None:
        return None
    return ".".join(x for x in (book, m.group(2), m.group(3)) if x)


@dataclass
class Cleaned:
    html: str
    refs: list[str] = field(default_factory=list)  # OSIS references
    entries: list[str] = field(default_factory=list)  # `BDB, אבה`
    unknown: list[str] = field(default_factory=list)  # references not understood


class _Cleaner(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.out: list[str] = []
        self.open: list[str] = []
        self.result = Cleaned("")

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if tag not in TAGS:
            raise CleanError(f"tag <{tag}>")
        a = dict(attrs)
        if tag == "a":
            ref = a.get("data-ref") or ""
            if ref.startswith(("BDB, ", "BDB Aramaic, ")):
                # another entry, as `BDB, אבה`: resolved to its id in the build
                self.result.entries.append(ref)
                self.out.append(f'<a data-entry="{html.escape(ref)}">')
            elif (o := osis(ref)) is not None:
                self.result.refs.append(o)
                self.out.append(f'<a data-osis="{o}">')
            else:
                self.result.unknown.append(ref)
                self.out.append("<a>")
        else:
            kept = "".join(
                f' {k}="{html.escape(v)}"'
                for (k, v) in attrs
                if k in TAGS[tag] and v is not None
            )
            self.out.append(f"<{tag}{kept}>")
        if tag not in VOID:
            self.open.append(tag)

    def handle_startendtag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        self.handle_starttag(tag, attrs)
        if tag not in VOID and self.open and self.open[-1] == tag:
            self.open.pop()
            self.out.append(f"</{tag}>")

    def handle_endtag(self, tag: str) -> None:
        if tag not in TAGS:
            raise CleanError(f"tag </{tag}>")
        if tag in VOID:
            return
        if tag in self.open:
            # close what is still open inside it
            while self.open:
                t = self.open.pop()
                self.out.append(f"</{t}>")
                if t == tag:
                    break

    def handle_data(self, data: str) -> None:
        self.out.append(html.escape(data, quote=False))


def clean(raw: str) -> Cleaned:
    """Sefaria's HTML of an entry with only the tags in `TAGS`."""
    cleaner = _Cleaner()
    cleaner.feed(raw)
    cleaner.close()
    for t in reversed(cleaner.open):
        cleaner.out.append(f"</{t}>")
    cleaner.result.html = "".join(cleaner.out).strip()
    return cleaner.result
