"""The essentials of a word, for the card shown when hovering over it.

Of all the features a word has (the BHSA has about 70), only these are shown:
its gloss, lexeme, part of speech and morphology, in words rather than codes.
Each is read from the first feature of its role that has a value, so that the
BHSA, the SP and the DSS all work. The DSS has its own analysis and the ETCBC's
(features ending in `_etcbc`, as in the BHSA): the ETCBC's comes first, and the
DSS's own where the ETCBC has nothing. Values that mean "not applicable" (`NA`,
`absent`, `n/a`) are left out.

The DSS has no glosses; they come from the lexicon of another open corpus (the
BHSA), by the ETCBC lexeme (`lex_etcbc`) and language (`lang_etcbc`).
"""

from __future__ import annotations

from collections.abc import Callable
from typing import Any

# label: candidate features, the first one with a value is used
ROLES: list[tuple[str, tuple[str, ...]]] = [
    ("gloss", ("gloss",)),
    ("lexeme", ("voc_lex_utf8", "lex_utf8_etcbc", "glex", "lex_utf8")),
    ("transliteration", ("lex_etcbc", "lex")),
    ("part of speech", ("sp_etcbc", "sp")),
    ("stem", ("vs_etcbc", "vs")),
    ("tense", ("vt_etcbc", "vt")),
    ("person", ("ps_etcbc", "ps")),
    ("gender", ("gn_etcbc", "gn")),
    ("number", ("nu_etcbc", "nu")),
    ("state", ("st_etcbc", "st")),
    ("language", ("lang_etcbc", "language")),
    # a translation's word, as it is searched (`cfabric_browser.translations`)
    ("word", ("lower",)),
]

# a word's lexeme and language as in the BHSA, to find its gloss there
LEXEME = ("lex_etcbc", "lex")
LANGUAGE = ("lang_etcbc", "language")

GlossFor = Callable[[str, str | None], str | None]

# the pronominal suffix: its person, gender and number, shown together
SUFFIX = (("person", "prs_ps"), ("gender", "prs_gn"), ("number", "prs_nu"))

EMPTY = {"", "NA", "na", "n/a", "absent", "none", "unknown", "0"}

PARTS = {
    "subs": "noun",
    "nmpr": "proper noun",
    "verb": "verb",
    "adjv": "adjective",
    "advb": "adverb",
    "prep": "preposition",
    "conj": "conjunction",
    "prps": "personal pronoun",
    "prde": "demonstrative pronoun",
    "prin": "interrogative pronoun",
    "intj": "interjection",
    "nega": "negation",
    "inrg": "interrogative",
    "art": "article",
    "ptcl": "particle",
    "pron": "pronoun",
    "numr": "numeral",
    "suff": "suffix",
}
VALUES: dict[str, dict[str, str]] = {
    "part of speech": PARTS,
    "tense": {
        "perf": "perfect",
        "impf": "imperfect",
        "wayq": "wayyiqtol",
        "wayy": "wayyiqtol",
        "impv": "imperative",
        "infa": "infinitive absolute",
        "infc": "infinitive construct",
        "ptca": "participle",
        "ptcp": "passive participle",
    },
    "person": {
        "p1": "1st",
        "p2": "2nd",
        "p3": "3rd",
        "1": "1st",
        "2": "2nd",
        "3": "3rd",
    },
    "gender": {"m": "masculine", "f": "feminine", "c": "common", "b": "both"},
    "number": {
        "sg": "singular",
        "pl": "plural",
        "du": "dual",
        "s": "singular",
        "p": "plural",
        "d": "dual",
    },
    "state": {"a": "absolute", "c": "construct", "e": "emphatic"},
}

# the letters and vowel points of Hebrew; accents and other marks in a lexeme
# (the BHSA's homonym mark ֜, the SP's /) are left out
HEBREW = set(range(0x05B0, 0x05C8)) | set(range(0x05D0, 0x05EB))


FINALS = {"כ": "ך", "מ": "ם", "נ": "ן", "פ": "ף", "צ": "ץ"}


def _clean_hebrew(text: str) -> str:
    """Hebrew letters and points only, with a final form at the end (the SP
    writes its lexemes without: אלהימ)."""
    chars = [c for c in text if ord(c) in HEBREW and c not in "׀׃׆"]
    letters = [i for i, c in enumerate(chars) if "א" <= c <= "ת"]
    if letters and chars[letters[-1]] in FINALS:
        chars[letters[-1]] = FINALS[chars[letters[-1]]]
    return "".join(chars).strip()


def _value(api: Any, name: str, node: int) -> Any:
    feature = getattr(api.F, name, None)
    return None if feature is None else feature.v(node)


def _first(
    api: Any, node: int, names: tuple[str, ...], empty: set[str]
) -> tuple[str, str] | None:
    """The first of these features with a value: (feature, value)."""
    for name in names:
        value = _value(api, name, node)
        if value is not None and str(value).strip() not in empty:
            return (name, str(value).strip())
    return None


def lexeme_of(
    api: Any, node: int, empty: set[str] | None = None
) -> tuple[str, str | None] | None:
    """A word's ETCBC lexeme (`BR>[`) and language, as in the BHSA, or None."""
    empty = EMPTY | (empty or set())
    lex = _first(api, node, LEXEME, empty)
    if not lex or not lex[1].isascii():
        return None
    language = _first(api, node, LANGUAGE, empty)
    return (lex[1], language[1] if language else None)


def summary(
    api: Any,
    node: int,
    empty: set[str] | None = None,
    gloss_for: GlossFor | None = None,
    gloss_source: str = "lexicon",
) -> list[dict[str, str]]:
    """The essentials of a node: a list of `{label, value, feature}`.

    `gloss_for(lex, language)` gives the gloss of an ETCBC lexeme from another
    corpus (`gloss_source`), for a corpus without glosses (the DSS).
    """
    empty = EMPTY | (empty or set())
    items = []
    for label, candidates in ROLES:
        for name in candidates:
            value = _value(api, name, node)
            if value is None:
                continue
            text = str(value).strip()
            if label == "lexeme":
                text = _clean_hebrew(text)
            elif label == "transliteration" and not text.isascii():
                # the DSS's `lex` is in Hebrew: the transliteration is `lex_etcbc`
                continue
            if text in empty:
                # nothing here: try the next feature of this role
                continue
            if label == "language" and text == "Hebrew":
                # the language only where it is not Hebrew
                break
            items.append(
                {
                    "label": label,
                    "value": VALUES.get(label, {}).get(text, text),
                    "feature": name,
                }
            )
            break
    if gloss_for and not any(item["label"] == "gloss" for item in items):
        lex = _first(api, node, LEXEME, empty)
        if lex and lex[1].isascii():
            language = _first(api, node, LANGUAGE, empty)
            gloss = gloss_for(lex[1], language[1] if language else None)
            if gloss:
                items.insert(
                    0, {"label": "gloss", "value": gloss, "feature": gloss_source}
                )
    suffix = []
    for role, name in SUFFIX:
        value = _value(api, name, node)
        if value is not None and str(value) not in empty:
            suffix.append(VALUES[role].get(str(value), str(value)))
    if suffix:
        items.append({"label": "suffix", "value": " ".join(suffix), "feature": "prs"})
    return items
