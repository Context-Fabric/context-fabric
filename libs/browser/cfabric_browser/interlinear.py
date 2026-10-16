"""The interlinear: a passage word by word, with lines under each word.

Per word: its text, and the values of the lines, read by role as on the card
shown when hovering over a word (`cfabric_browser.summary`): transliteration
(of the lexeme), lexeme, gloss, part of speech and morphology in words; and
the values of any other features asked for. Per verse also the phrases and
clauses its words are in, with their phrase function and clause type, for
showing the structure. Optionally a second corpus under the first, word by
word: the words that correspond, found as in the comparison
(`cfabric_browser.compare.align_words`), with their differences.
"""

from __future__ import annotations

from collections.abc import Callable
from typing import Any

from cfabric.display import RenderOptions, section_ref_for

from cfabric_browser import formats
from cfabric_browser.compare import align_words, diff_words, verse_words, word_type
from cfabric_browser.summary import summary

# the lines built from the summary, by its labels
MORPHOLOGY = ("stem", "tense", "person", "gender", "number", "state")
PHRASE_LABEL = "function"
CLAUSE_LABEL = "typ"


def _groups(
    api, words: list[int], nType: str, feature: str
) -> tuple[list[int], list[dict[str, Any]]]:
    """Per word the index of its group (the `nType` node it lies in, or -1), and
    the groups: node and label (the value of `feature`)."""
    if nType not in api.F.otype.all:
        return ([-1] * len(words), [])
    fLabel = api.Fs(feature) if feature in api.Fall() else None
    index: dict[int, int] = {}
    groups: list[dict[str, Any]] = []
    owners: list[int] = []
    for w in words:
        ups = api.L.u(w, otype=nType)
        if not ups:
            owners.append(-1)
            continue
        node = int(ups[0])
        if node not in index:
            index[node] = len(groups)
            label = fLabel.v(node) if fLabel else None
            groups.append(
                {"node": node, "label": None if label is None else str(label)}
            )
        owners.append(index[node])
    return (owners, groups)


def word_lines(items: list[dict[str, str]]) -> dict[str, str]:
    """The lines of a word from its summary items."""
    by = {i["label"]: i["value"] for i in items}
    lines: dict[str, str] = {}
    for line, label in (
        ("translit", "transliteration"),
        ("lexeme", "lexeme"),
        ("gloss", "gloss"),
        ("pos", "part of speech"),
    ):
        if label in by:
            lines[line] = by[label]
    morph = " ".join(by[m] for m in MORPHOLOGY if m in by)
    if "suffix" in by:
        morph = f"{morph} + {by['suffix']}" if morph else f"+ {by['suffix']}"
    if morph:
        lines["morph"] = morph
    if "language" in by:
        lines["language"] = by["language"]
    return lines


def interlinear_verse(
    service,
    corpus: str,
    verse: int,
    opts: RenderOptions,
    features: list[str],
    other: str | None,
    bdb_for: Callable[[int], str | None] | None = None,
) -> dict[str, Any]:
    """A verse word by word; see the module. `bdb_for(word)` gives BDB's
    short definition of a word, for the line `bdb`."""
    api = service.api(corpus)
    config = service.config(corpus)
    fmt = formats.chosen(config, opts.fmt) or formats.default(config)
    empty = {str(v) for v in config.none_values if v is not None}
    glosses = service.glosses(corpus)
    wType = word_type(api, config)
    words = (
        [int(w) for w in api.L.d(verse, otype=wType)]
        if api.F.otype.v(verse) != wType
        else [verse]
    )
    extra = [(f, api.Fs(f)) for f in features if f in api.Fall()]

    rows = []
    for w in words:
        if glosses is None:
            items = summary(api, w, empty)
        else:
            (gloss_for, source) = glosses
            items = summary(api, w, empty, gloss_for, f"gloss in {source}")
        values = {}
        for name, feature in extra:
            value = feature.v(w)
            if value is not None and str(value) not in empty:
                values[name] = str(value)
        lines = word_lines(items)
        definition = bdb_for(w) if bdb_for else None
        if definition:
            lines["bdb"] = definition
        rows.append(
            {
                "node": w,
                "text": api.T.text(w, fmt=fmt).strip(),
                "lines": lines,
                "features": values,
            }
        )

    (phraseOf, phrases) = _groups(api, words, "phrase", PHRASE_LABEL)
    (clauseOf, clauses) = _groups(api, words, "clause", CLAUSE_LABEL)
    for row, p, c in zip(rows, phraseOf, clauseOf, strict=True):
        row["phrase"] = p
        row["clause"] = c

    ref = section_ref_for(api, config, verse, lang=opts.lang)
    number = api.T.sectionFromNode(verse, lang="en")[-1]
    result: dict[str, Any] = {
        "verse": int(number) if str(number).isdigit() else str(number),
        "ref": ref,
        "words": rows,
        "phrases": phrases,
        "clauses": clauses,
    }
    if other:
        result["under"] = under_verse(service, corpus, ref, verse, other, opts)
    return result


def under_verse(
    service, corpus: str, ref: str, verse: int, other: str, opts: RenderOptions
) -> dict[str, Any] | None:
    """The verse in `other`, word by word under the words of `corpus`, with the
    words that differ (`matres` or `variant`, as in the comparison).

    A translation is not paired word by word: it comes as the verse's text
    (`translation`, `text`, and a `note` printed after it, if any)."""
    aligned = service.align(corpus, ref, other, opts.lang)
    if aligned is None or not aligned["exact"]:
        return None
    oApi = service.api(other)
    oConfig = service.config(other)
    if service.translation(other) is not None:
        node = aligned["node"]
        noteFeature = getattr(oApi.F, "note", None)
        return {
            "corpus": other,
            "ref": aligned["ref"],
            "translation": True,
            "text": oApi.T.text(node, fmt=formats.default(oConfig)).strip(),
            "note": noteFeature.v(node) if noteFeature else None,
            "words": [],
            "under": [],
            "marks": {},
            "marksHere": {},
        }
    a = verse_words(service.api(corpus), service.config(corpus), verse)
    b = verse_words(oApi, oConfig, aligned["node"])
    fmt = formats.chosen(oConfig, opts.fmt) or formats.default(oConfig)
    (marksA, marksB) = diff_words(a, b)
    return {
        "corpus": other,
        "ref": aligned["ref"],
        "words": [{"node": n, "text": oApi.T.text(n, fmt=fmt).strip()} for (n, _) in b],
        # per word of the corpus read, the nodes of the words under it
        "under": align_words(a, b),
        # the words that differ, under and in the corpus read (node ids of two
        # corpora may coincide, so they are kept apart)
        "marks": {str(n): k for (n, k) in marksB.items()},
        "marksHere": {str(n): k for (n, k) in marksA.items()},
    }
