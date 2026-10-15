"""Tests for comparing the consonantal text of two corpora (MT and SP)."""

import pytest
from cfabric_browser.compare import (
    align_words,
    consonants,
    diff_units,
    diff_words,
    merge_units,
    with_finals,
    witness_reading,
)


def words(text: str, start: int = 1) -> list[tuple[int, str]]:
    """Words as (node, consonants), numbered from `start`."""
    return [(start + i, consonants(w)) for (i, w) in enumerate(text.split())]


class TestConsonants:
    def test_strips_vowels_and_accents(self):
        assert consonants("בְּרֵאשִׁ֖ית") == "בראשית"

    def test_precomposed_shin(self):
        # SP's g_cons_utf8 writes shin with its dot as one character
        assert consonants("ראשׁית") == "ראשית"

    def test_final_forms(self):
        assert consonants("אלהים ארץ") == "אלהימארצ"


class TestDiffWords:
    def test_identical(self):
        assert diff_words(words("בראשית ברא"), words("בראשית ברא", 10)) == ({}, {})

    def test_word_division_is_no_variation(self):
        # final forms change with word division too
        assert diff_words(words("ב ראשית"), words("בראשית", 10)) == ({}, {})
        assert diff_words(words("כי מים"), words("כימ ים", 10)) == ({}, {})

    @pytest.mark.parametrize(
        "mt,sp",
        [
            ("כל", "כול"),  # added inside the word
            ("גדל", "גדול"),
            ("מארת", "מאורות"),  # twice in one word
            ("ירב", "ירבה"),  # at the end of the word
        ],
    )
    def test_matres_marked_in_both(self, mt, sp):
        assert diff_words(words(mt), words(sp, 10)) == ({1: "matres"}, {10: "matres"})

    def test_other_letters_are_variants(self):
        assert diff_words(words("הוא אמר"), words("היא אמר", 10)) == (
            {1: "variant"},
            {10: "variant"},
        )
        assert diff_words(words("ממשלת"), words("ממלכת", 10)) == (
            {1: "variant"},
            {10: "variant"},
        )

    def test_plus_marked_in_the_text_that_has_it(self):
        mt = words("שמים ל הבדיל")
        sp = words("שמים ל האיר על ה ארץ ו ל הבדיל", 10)
        assert diff_words(mt, sp) == (
            {},
            {
                11: "variant",
                12: "variant",
                13: "variant",
                14: "variant",
                15: "variant",
                16: "variant",
            },
        )

    def test_one_letter_word_is_a_variant(self):
        # the article and the conjunction are words, not spelling
        assert diff_words(words("כל חיה"), words("כל ה חיה", 10)) == (
            {},
            {11: "variant"},
        )

    def test_ambiguous_plus_becomes_a_word(self):
        # SP's extra ו could also be read as the end of צלמנו
        assert diff_words(
            words("ב צלמנו כ דמותנו"), words("ב צלמנו ו כ דמותנו", 10)
        ) == (
            {},
            {12: "variant"},
        )

    def test_variant_wins_over_matres(self):
        (a, b) = diff_words(words("מארת שמש"), words("מאורות שמשא", 10))
        assert a == {1: "matres", 2: "matres"}
        assert b == {10: "matres", 11: "matres"}
        (a, b) = diff_words(words("עשב"), words("עושה", 10))
        assert a == {1: "variant"}
        assert b == {10: "variant"}


def letters(text: str, reconstructed: str = "") -> tuple[list, list[bool]]:
    """A manuscript's words, numbered from 10, and per letter whether it is
    preserved: letters under a `^` in `reconstructed` (laid out like `text`,
    spaces included) are reconstructed."""
    marks = reconstructed.ljust(len(text))
    known = [m != "^" for (t, m) in zip(text, marks) if t != " "]
    return words(text, 10), known


class TestDiffWitness:
    def test_reconstructed_letters_do_not_count(self):
        (b, known) = letters("ברא אלוהים", "      ^")
        assert diff_words(words("ברא אלהים"), b, known) == ({}, {})

    def test_preserved_letters_count(self):
        (b, known) = letters("ברא אלוהים")
        assert diff_words(words("ברא אלהים"), b, known) == (
            {2: "matres"},
            {11: "matres"},
        )

    def test_lost_edges_do_not_count(self):
        # a fragment with only the middle of the verse
        (b, known) = letters("אלהים")
        assert diff_words(words("ברא אלהים את"), b, known) == ({}, {})

    def test_minus_between_preserved_letters(self):
        (b, known) = letters("את ל ימים")
        assert diff_words(words("את ו ל ימים"), b, known) == ({2: "variant"}, {})

    def test_minus_next_to_reconstruction_does_not_count(self):
        (b, known) = letters("את ל ימים", " ^")
        assert diff_words(words("את ו ל ימים"), b, known) == ({}, {})

    def test_edge_replacement_stays_local(self):
        # a last surviving letter that differs does not mark the rest of `a`
        (b, known) = letters("ברא ק")
        (a, _) = diff_words(words("ברא אלהים את השמים"), b, known)
        assert a == {2: "variant"}


class TestThreeWay:
    def test_reading_between_anchors(self):
        # MT בורא, a scroll ברא: where MT has ו the scroll has nothing
        assert witness_reading("בוראש", "בראש", [True] * 4, 1, 2) == (1, "")
        assert witness_reading("בוראש", "בוראש", [True] * 5, 1, 2) == (1, "ו")

    def test_lost_anchor_is_unknown(self):
        known = [True, False, True, True]  # the ר is reconstructed
        assert witness_reading("בוראש", "בראש", known, 1, 2) is None

    def test_edges(self):
        # the scroll starts after the place: nothing is known there
        assert witness_reading("בראש", "ראש", [True] * 3, 0, 1) is None
        # but a letter it does have at the start is a reading
        assert witness_reading("בראש", "פראש", [True] * 4, 0, 1) == (0, "פ")

    def test_same_words_are_one_place(self):
        units = merge_units(diff_units(words("מארת ב"), words("מאורות ב", 10)))
        assert len(units) == 1
        (unit,) = units
        assert (unit.aWords, unit.bWords) == ([1], [10])

    def test_final_forms_for_people(self):
        assert with_finals("אלהימ") == "אלהים"
        assert with_finals("ארצ") == "ארץ"
        assert with_finals("את") == "את"
        assert with_finals("") == ""


class TestAlignWords:
    def test_word_for_word(self):
        a = words("בראשית ברא אלהים")
        b = words("בראשית ברא אלהים", 11)
        assert align_words(a, b) == [[11], [12], [13]]

    def test_other_word_division(self):
        # one word in SP where MT has two: it goes with the first
        a = words("כי טוב ויהי")
        b = words("כיטוב ויהי", 11)
        assert align_words(a, b) == [[11], [], [12]]

    def test_two_words_under_one(self):
        a = words("כיטוב ויהי")
        b = words("כי טוב ויהי", 11)
        assert align_words(a, b) == [[11, 12], [13]]

    def test_a_plus_goes_with_the_word_before(self):
        a = words("ויאמר אלהים")
        b = words("ויאמר משה אלהים", 11)
        assert align_words(a, b) == [[11, 12], [13]]

    def test_spelling_differs(self):
        a = words("בורא שמים")
        b = words("ברא שמים", 11)
        assert align_words(a, b) == [[11], [12]]

    def test_empty(self):
        assert align_words([], words("ברא")) == []
        assert align_words(words("ברא"), []) == [[]]
