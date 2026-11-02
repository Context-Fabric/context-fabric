"""Tests for the essentials of a word shown when hovering over it."""

from types import SimpleNamespace

from cfabric_browser.summary import summary


def corpus(**features: dict[int, str]) -> SimpleNamespace:
    """A stand-in for a loaded corpus with only these node features."""
    F = SimpleNamespace(
        **{
            name: SimpleNamespace(v=lambda n, values=values: values.get(n))
            for name, values in features.items()
        }
    )
    return SimpleNamespace(F=F)


def pairs(items: list[dict]) -> list[tuple[str, str]]:
    return [(i["label"], i["value"]) for i in items]


def test_bhsa_verb():
    api = corpus(
        gloss={1: "create"},
        voc_lex_utf8={1: "ברא"},
        lex={1: "BR>["},
        sp={1: "verb"},
        vs={1: "qal"},
        vt={1: "perf"},
        ps={1: "p3"},
        gn={1: "m"},
        nu={1: "sg"},
        st={1: "NA"},
        language={1: "Hebrew"},
        g_word_utf8={1: "בָּרָ֣א"},
        freq_lex={1: "48"},
    )
    assert pairs(summary(api, 1)) == [
        ("gloss", "create"),
        ("lexeme", "ברא"),
        ("transliteration", "BR>["),
        ("part of speech", "verb"),
        ("stem", "qal"),
        ("tense", "perfect"),
        ("person", "3rd"),
        ("gender", "masculine"),
        ("number", "singular"),
        # no state (NA), no language (Hebrew), no g_ features, no frequency
    ]


def test_suffix_and_aramaic():
    api = corpus(
        gloss={1: "kind"},
        sp={1: "subs"},
        prs_ps={1: "p3"},
        prs_gn={1: "m"},
        prs_nu={1: "sg"},
        language={1: "Aramaic"},
    )
    assert pairs(summary(api, 1)) == [
        ("gloss", "kind"),
        ("part of speech", "noun"),
        ("language", "Aramaic"),
        ("suffix", "3rd masculine singular"),
    ]


def test_dss_prefers_etcbc_features():
    # the DSS: its own analysis and the ETCBC's; the ETCBC's first, the own one
    # where the ETCBC has nothing
    api = corpus(
        lex={1: "כֹּל", 2: "וְ"},
        glex={1: "כֹּל"},
        lex_etcbc={1: "KL/", 2: "W"},
        lex_utf8_etcbc={1: "כֹּל", 2: "וְ"},
        sp={1: "subs", 2: "ptcl"},
        sp_etcbc={1: "subs", 2: ""},
        nu={1: "s"},
        nu_etcbc={1: "sg"},
        gn={1: "m"},
        gn_etcbc={1: "NA"},
        st={1: "c"},
        lang={1: "h", 2: "h"},
        lang_etcbc={1: "Hebrew", 2: "Hebrew"},
    )
    items = summary(api, 1)
    assert pairs(items) == [
        ("lexeme", "כֹּל"),
        ("transliteration", "KL/"),
        ("part of speech", "noun"),
        ("gender", "masculine"),
        ("number", "singular"),
        ("state", "construct"),
    ]
    assert [i["feature"] for i in items] == [
        "lex_utf8_etcbc",
        "lex_etcbc",
        "sp_etcbc",
        "gn",  # gn_etcbc is NA
        "nu_etcbc",
        "st",
    ]
    # no sp_etcbc: the DSS's own part of speech
    assert ("part of speech", "particle") in pairs(summary(api, 2))


def test_gloss_from_another_corpus():
    api = corpus(
        lex_etcbc={1: "KL/", 2: "XYZ/"}, lang_etcbc={1: "Aramaic", 2: "Hebrew"}
    )
    glosses = {("KL/", "Aramaic"): "whole"}
    items = summary(
        api,
        1,
        None,
        lambda lex, language: glosses.get((lex, language)),
        "gloss in bhsa",
    )
    assert items[0] == {"label": "gloss", "value": "whole", "feature": "gloss in bhsa"}
    # a lexeme the other corpus does not know: no gloss
    assert not any(
        i["label"] == "gloss" for i in summary(api, 2, None, lambda *_: None)
    )


def test_sp_lexemes():
    # the SP writes lexemes without final forms; the BHSA marks homonyms
    sp = corpus(lex_utf8={1: "אלהימ/", 2: "ראשׁית֜"})
    assert pairs(summary(sp, 1)) == [("lexeme", "אלהים")]
    assert pairs(summary(sp, 2)) == [("lexeme", "ראשׁית")]


def test_corpus_empty_values():
    api = corpus(gloss={1: "?"}, sp={1: "verb"})
    assert pairs(summary(api, 1, {"?"})) == [("part of speech", "verb")]
