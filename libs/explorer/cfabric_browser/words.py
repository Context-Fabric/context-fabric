"""The words of a corpus: their node type, the words of a node, the format of
their text, and the book each lies in.

Shared by the comparison, the Concordance, the Interlinear and the lexicon.
"""

from __future__ import annotations

from bisect import bisect_right

from cfabric_browser.display import DisplayConfig


def word_type(api, config: DisplayConfig) -> str:
    """The node type of words: `word`, or else a base type, or else the slot."""
    types = api.F.otype.all
    if "word" in types:
        return "word"
    bases = [t for t in types if t in config.base_types and t != config.slot_type]
    return str(bases[0]) if bases else config.slot_type


def orig_format(config: DisplayConfig) -> str | None:
    """The first text format in the original script, or else the default."""
    return next(
        (f for f in config.text_formats if f.startswith("text-orig")),
        config.default_format,
    )


def words_of(api, wType: str, node: int) -> list[int]:
    """The words of a node, in order; for a part of a word (a sign), its word."""
    if api.F.otype.v(node) == wType:
        return [node]
    down = [int(w) for w in api.L.d(node, otype=wType)]
    if down:
        return down
    up = api.L.u(node, otype=wType)
    return [int(up[0])] if up else []


class BookSpans:
    """The books as runs of nodes of one type (words, or slots), to find the
    book of such a node quickly (asking `L.u` for each is slow for many)."""

    def __init__(self, api, unit: str) -> None:
        self._starts: list[int] = []
        self._ends: list[int] = []
        self._books: list[int] = []
        types = api.T.sectionTypes or ()
        if not types:
            return
        spans = []
        for b in api.F.otype.s(types[0]):
            units = api.L.d(int(b), otype=unit)
            if len(units):
                spans.append((int(units[0]), int(units[-1]), int(b)))
        for first, last, b in sorted(spans):
            self._starts.append(first)
            self._ends.append(last)
            self._books.append(b)

    def book(self, node: int) -> int | None:
        """The book a node of the unit type lies in, if any."""
        k = bisect_right(self._starts, node) - 1
        if k < 0 or node > self._ends[k]:
            return None
        return self._books[k]
