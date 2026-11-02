"""A dictionary file as the browser reads it (`bdb.json.gz`, from `build.py`).

The file has the entries in BDB's order (Hebrew, then Aramaic), the links
from the BHSA's lexemes (`"Hebrew BR>["`) to entry ids, the main entry
first, and BDB's abbreviations. An entry's `section` is Open Scriptures'
root group: the root and the words BDB derives from it, its root family.
"""

from __future__ import annotations

import gzip
import json
import re
import threading
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

# the fields of an entry in a list (the lexeme page, the root family)
SUMMARY = ("id", "lexicon", "headword", "pos", "defs", "incomplete", "ref")


# the letter of each part of BDB, by the first letter of Open Scriptures' ids
# (`b.cw.aa` is in ב; the Aramaic part's ids have `x` before it)
PARTS = dict(zip("abcdefghijklmnopqrstuvw", "אבגדהוזחטיכלמנסעפצקרששת"))


class DictionaryError(Exception):
    """The file is not a dictionary this browser can read."""


# BDB's grammatical abbreviations, which start its bold words (`n.pr.m.`,
# `prep. in, with`)
GRAMMAR = re.compile(
    r"^(?:(?:n|pr|m|f|pl|du|sg|adj|gent|loc|terr|div|mont|fl|vb|prep|adv|conj"
    r"|interj|pron|subst|denom|coll|abstr|id|abs|cstr|intens)\.\s*)+"
)


def short(entry: dict[str, Any]) -> str:
    """A short definition: Open Scriptures' definitions, else the first words
    BDB sets in bold (its glosses) once its grammatical abbreviations are
    left out; empty when only those are there (most names)."""
    if entry.get("defs"):
        return ", ".join(entry["defs"])
    for bold in re.findall(r"<strong>([^<]+)</strong>", entry.get("html") or "")[:3]:
        words = GRAMMAR.sub("", bold.strip()).strip(" ,;:()")
        if re.search(r"[a-z]{2}", words):
            return words
    return ""


@dataclass
class Dictionary:
    meta: dict[str, Any]
    entries: list[dict[str, Any]]
    links: dict[str, dict[str, Any]]
    abbreviations: dict[str, str]
    byId: dict[str, int] = field(default_factory=dict)  # id: position
    bySection: dict[str, list[int]] = field(default_factory=dict)
    lexemes: dict[str, list[str]] = field(default_factory=dict)  # id: keys

    def __post_init__(self) -> None:
        for i, e in enumerate(self.entries):
            self.byId[e["id"]] = i
            if e.get("section"):
                self.bySection.setdefault(e["section"], []).append(i)
        for key, link in self.links.items():
            for i in link["entries"]:
                self.lexemes.setdefault(i, []).append(key)

    @classmethod
    def load(cls, path: str | Path) -> Dictionary:
        try:
            data = json.loads(gzip.decompress(Path(path).read_bytes()))
        except (OSError, ValueError) as e:
            raise DictionaryError(f"{path} is not a dictionary file: {e}") from e
        if not isinstance(data, dict) or not {"meta", "entries", "links"} <= set(data):
            raise DictionaryError(f"{path} is not a dictionary file")
        return cls(
            data["meta"],
            data["entries"],
            data["links"],
            data.get("abbreviations") or {},
        )

    def summary(self, entry: dict[str, Any]) -> dict[str, Any]:
        found = {k: entry.get(k) for k in SUMMARY}
        found["short"] = short(entry)
        return found

    def for_lexeme(self, language: str | None, lex: str | None) -> dict[str, Any]:
        """The entries a lexeme links to, the main one first, and how the link
        was made (`oshb`, `cited`, `manual`); no entries if it has no link."""
        link = self.links.get(f"{language} {lex}")
        if link is None:
            return {"entries": [], "how": None}
        return {
            "entries": [
                self.summary(self.entries[self.byId[i]])
                for i in link["entries"]
                if i in self.byId
            ],
            "how": link.get("how"),
            "share": link.get("share"),
        }

    def letter(self, eid: str) -> str | None:
        """The letter of the part of BDB an entry is in (its root's letter);
        for an entry without an Open Scriptures id, that of the one before."""
        i = self.byId.get(eid)
        while i is not None and i >= 0:
            part = self.entries[i]["id"].split(".")[0].removeprefix("x")
            if part in PARTS:
                return PARTS[part]
            i -= 1
        return None

    def for_list(self, language: str | None, lex: str | None) -> dict[str, Any]:
        """What the lexicon's list shows of a lexeme's main entry: its short
        definition, its place in BDB (for ordering by root), the letter of
        its part, and whether it is a root; empty without a link."""
        link = self.links.get(f"{language} {lex}")
        if not link or link["entries"][0] not in self.byId:
            return {}
        eid = link["entries"][0]
        i = self.byId[eid]
        return {
            "bdbShort": short(self.entries[i]),
            "bdbOrder": i,
            "bdbLetter": self.letter(eid),
            "bdbRoot": bool(self.entries[i].get("root")),
        }

    def article(self, eid: str) -> dict[str, Any] | None:
        """An entry with its text, its neighbours in BDB, its root family and
        the lexemes linked to it (`"Hebrew BR>["`)."""
        i = self.byId.get(eid)
        if i is None:
            return None
        entry = self.entries[i]
        family = [
            self.summary(self.entries[j])
            for j in self.bySection.get(entry.get("section") or "", [])
        ]

        def neighbour(j: int) -> dict[str, Any] | None:
            if 0 <= j < len(self.entries):
                other = self.entries[j]
                if other["lexicon"] == entry["lexicon"]:
                    return {"id": other["id"], "headword": other["headword"]}
            return None

        return {
            **self.summary(entry),
            "html": entry.get("html") or "",
            "root": entry.get("root"),
            "strong": entry.get("strong") or [],
            "twot": entry.get("twot") or [],
            "previous": neighbour(i - 1),
            "next": neighbour(i + 1),
            "family": family if len(family) > 1 else [],
            "lexemes": self.lexemes.get(eid, []),
        }


class Dictionaries:
    """The dictionaries in a folder (`<root>/<id>/<id>.json.gz`), read when
    first asked for and again when the file changes (added, removed)."""

    def __init__(self, root: str | Path) -> None:
        self.root = Path(root).expanduser()
        self._lock = threading.Lock()
        self._loaded: dict[str, tuple[float, Dictionary]] = {}

    def path(self, did: str) -> Path:
        return self.root / did / f"{did}.json.gz"

    def get(self, did: str) -> Dictionary | None:
        path = self.path(did)
        try:
            mtime = path.stat().st_mtime
        except OSError:
            with self._lock:
                self._loaded.pop(did, None)
            return None
        with self._lock:
            found = self._loaded.get(did)
            if found is None or found[0] != mtime:
                found = (mtime, Dictionary.load(path))
                self._loaded[did] = found
            return found[1]
