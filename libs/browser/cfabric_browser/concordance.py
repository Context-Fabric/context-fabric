"""The results of a search as a concordance: lines with the hit in context.

As in Sketch Engine's KWIC view (Key Word In Context), each result is a line
with its *hit* (the words of the result's innermost nodes) in the middle and a
number of words to the left and right of it, within the same book. Lines can
be sorted by a feature of the hit or of a word at a position before or after
it (`L1` is the word just before the hit, `R2` the second word after it),
filtered by features of the hit's first word, reduced to one per verse, and
sampled. Facets count the values of a feature of the hit (or its book), and
collocations count the values of a feature of the words near the hit, with
their logDice score against the corpus.

Positions and sorting follow the corpus' word order, so for Hebrew "left"
means "before" (on the right of the page).
"""

from __future__ import annotations

import math
import random
import re
import threading
from bisect import bisect_right
from collections import Counter
from typing import Any

from cfabric_browser import formats
from cfabric_browser.compare import consonants, word_type

MAX_SIDE = 15
POSITION = re.compile(r"^(hit|[LR][1-9][0-9]?)$")
BOOK = "book"  # the pseudo-feature for the book of the hit
TEXT = "text"  # the pseudo-feature for the word's consonants

_frequencies: dict[tuple[int, str], Counter] = {}
_frequencies_lock = threading.Lock()


def parse_filters(value: str | None) -> list[tuple[str, str]]:
    """`vs:qal,sp:verb` → [("vs", "qal"), ("sp", "verb")]."""
    filters = []
    for part in (value or "").split(","):
        feature, sep, wanted = part.partition(":")
        if sep and feature.strip():
            filters.append((feature.strip(), wanted.strip()))
    return filters


class Concordance:
    """The results of one search, by their hit words."""

    def __init__(self, api, config, results: list[tuple[int, ...]]):
        self.api = api
        self.config = config
        self.wType = word_type(api, config)
        (self.first, self.last) = (int(n) for n in api.F.otype.sInterval(self.wType))
        types = list(api.T.sectionTypes or ())
        self.bookType = types[0] if types else None
        self.verseType = types[-1] if types else None
        self.fmt = next(
            (f for f in config.text_formats if f.startswith("text-orig")),
            config.default_format,
        )
        # values that mean "no value" (NA, absent, ...) count as missing
        self.empty = {str(v) for v in config.none_values if v is not None}
        # books are runs of words: where each starts, to find a word's book quickly
        self._starts: list[int] = []
        self._startBooks: list[int] = []
        self._ends: list[int] = []
        if self.bookType:
            spans = []
            for b in api.F.otype.s(self.bookType):
                ws = api.L.d(int(b), otype=self.wType)
                if len(ws):
                    spans.append((int(ws[0]), int(ws[-1]), int(b)))
            for first, last, b in sorted(spans):
                self._starts.append(first)
                self._ends.append(last)
                self._startBooks.append(b)
        self._bookNames: dict[int, str | None] = {}
        self._values: dict[tuple[int, str], str | None] = {}
        self.hits = [self._hit(tup) for tup in results]

    # words

    def _words_of(self, node: int) -> list[int]:
        api = self.api
        nType = api.F.otype.v(node)
        if nType == self.wType:
            return [node]
        down = [int(w) for w in api.L.d(node, otype=self.wType)]
        if down:
            return down
        up = api.L.u(node, otype=self.wType)
        return [int(up[0])] if up else []

    def _hit(self, tup: tuple[int, ...]) -> list[int]:
        """The words of the result's innermost nodes, in order."""
        api = self.api
        maxSlot = api.F.otype.maxSlot
        nodes = [int(n) for n in tup]

        def slots(n: int) -> set[int]:
            return {n} if n <= maxSlot else {int(s) for s in api.E.oslots.s(n)}

        sets = {n: slots(n) for n in nodes}
        inner = [
            n for n in nodes if not any(m != n and sets[m] < sets[n] for m in nodes)
        ]
        words = sorted({w for n in inner for w in self._words_of(n)})
        return words

    def book(self, word: int) -> int | None:
        k = bisect_right(self._starts, word) - 1
        if k < 0 or word > self._ends[k]:
            return None
        return self._startBooks[k]

    def neighbour(self, hit: list[int], position: str) -> int | None:
        """The word at a position (`L2`, `R1`) of a hit, if it is in the same book."""
        if not hit:
            return None
        k = int(position[1:])
        word = hit[0] - k if position[0] == "L" else hit[-1] + k
        if not self.first <= word <= self.last:
            return None
        if self.book(word) != self.book(hit[0]):
            return None
        return word

    def value(self, word: int | None, feature: str) -> str | None:
        """A feature of a word: also `text` (its consonants) and `book`."""
        if word is None:
            return None
        api = self.api
        if feature == TEXT:
            return consonants(api.T.text(word, fmt=self.fmt))
        if feature == BOOK:
            book = self.book(word)
            if book is None:
                return None
            if book not in self._bookNames:
                section = api.T.sectionFromNode(book, lang="en")
                self._bookNames[book] = str(section[0]) if section else None
            return self._bookNames[book]
        key = (word, feature)
        if key not in self._values:
            if feature not in api.Fall():
                return None
            value = api.Fs(feature).v(word)
            self._values[key] = (
                None if value is None or str(value) in self.empty else str(value)
            )
        return self._values[key]

    def verse(self, word: int) -> int | None:
        ups = self.api.L.u(word, otype=self.verseType) if self.verseType else ()
        return int(ups[0]) if ups else None

    # selection

    def select(
        self,
        filters: list[tuple[str, str]] = (),
        per_verse: bool = False,
        sample: int | None = None,
        seed: int = 1,
    ) -> list[int]:
        """The indexes of the results kept, in result order."""
        keep = []
        seen: set[int | None] = set()
        for i, hit in enumerate(self.hits):
            if not hit:
                continue
            if any(self.value(hit[0], f) != v for (f, v) in filters):
                continue
            if per_verse:
                verse = self.verse(hit[0])
                if verse in seen:
                    continue
                seen.add(verse)
            keep.append(i)
        if sample is not None and sample < len(keep):
            keep = sorted(random.Random(seed).sample(keep, sample))
        return keep

    def sort(self, indexes: list[int], key: str | None) -> list[int]:
        """Sort by `<position>.<feature>` (`L1.lex`, `hit.text`); ties and
        missing values keep the result order."""
        if not key:
            return indexes
        (position, _, feature) = key.partition(".")
        if not POSITION.match(position) or not feature:
            raise ValueError(f"Cannot sort by {key!r}: use e.g. hit.lex or L1.text")

        def word(i: int) -> int | None:
            hit = self.hits[i]
            return hit[0] if position == "hit" else self.neighbour(hit, position)

        values = {i: self.value(word(i), feature) for i in indexes}
        return sorted(indexes, key=lambda i: (values[i] is None, values[i] or "", i))

    # output

    def line(
        self, i: int, left: int, right: int, features: list[str], fmt: str | None
    ) -> dict[str, Any]:
        api = self.api
        hit = self.hits[i]
        fmt = formats.chosen(self.config, fmt) or formats.default(self.config)

        def word(w: int) -> dict[str, Any]:
            values = {f: self.value(w, f) for f in features}
            return {
                "node": w,
                "text": api.T.text(w, fmt=fmt).strip(),
                "features": {f: v for f, v in values.items() if v is not None},
            }

        before = [self.neighbour(hit, f"L{k}") for k in range(left, 0, -1)]
        after = [self.neighbour(hit, f"R{k}") for k in range(1, right + 1)]
        verse = self.verse(hit[0]) if hit else None
        return {
            "index": i,
            "verse": verse,
            "left": [word(w) for w in before if w is not None],
            "hit": [word(w) for w in hit],
            "right": [word(w) for w in after if w is not None],
        }

    def facet(
        self, indexes: list[int], feature: str, top: int = 50
    ) -> list[dict[str, Any]]:
        """The values of a feature of the hits' first words, most frequent first;
        for `book`, in the order of the books."""
        counts: Counter = Counter()
        order: dict[str, int] = {}
        for i in indexes:
            hit = self.hits[i]
            value = self.value(hit[0], feature)
            if value is None:
                continue
            counts[value] += 1
            order.setdefault(value, hit[0])
        if feature == BOOK:
            items = sorted(counts.items(), key=lambda kv: order[kv[0]])
        else:
            items = counts.most_common(top)
        return [{"value": v, "count": c} for (v, c) in items]

    def frequencies(self, corpus_key: int, feature: str) -> Counter:
        """How often each value of a feature occurs on the corpus' words (cached)."""
        key = (corpus_key, feature)
        with _frequencies_lock:
            if key not in _frequencies:
                counts: Counter = Counter()
                api = self.api
                if feature in api.Fall() and feature not in (TEXT, BOOK):
                    for value, n in api.Fs(feature).freqList(nodeTypes={self.wType}):
                        if str(value) not in self.empty:
                            counts[str(value)] += n
                else:
                    for w in range(self.first, self.last + 1):
                        value = self.value(w, feature)
                        if value is not None:
                            counts[value] += 1
                _frequencies[key] = counts
            return _frequencies[key]

    def collocations(
        self,
        indexes: list[int],
        feature: str,
        window: int,
        corpus_key: int,
        min_count: int = 2,
        top: int = 40,
    ) -> list[dict[str, Any]]:
        """Values of a feature of the words within `window` words of the hits,
        with their count there, their frequency in the corpus and their logDice
        score (14 + log2(2·count / (hits + frequency)); higher is more typical)."""
        counts: Counter = Counter()
        for i in indexes:
            hit = self.hits[i]
            for k in range(1, window + 1):
                for position in (f"L{k}", f"R{k}"):
                    value = self.value(self.neighbour(hit, position), feature)
                    if value is not None:
                        counts[value] += 1
        frequency = self.frequencies(corpus_key, feature)
        hits = len(indexes)
        rows = []
        for value, count in counts.items():
            if count < min_count:
                continue
            freq = frequency.get(value, count)
            score = 14 + math.log2(2 * count / (hits + freq))
            rows.append(
                {
                    "value": value,
                    "count": count,
                    "frequency": freq,
                    "logDice": round(score, 2),
                }
            )
        rows.sort(key=lambda r: (-r["logDice"], -r["count"]))
        return rows[:top]
