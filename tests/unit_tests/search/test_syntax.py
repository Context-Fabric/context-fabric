"""Unit tests for core.search.syntax module.

This module tests the search query syntax parsing functionality,
including pattern matching for atoms, features, relations, and quantifiers.
"""

import pytest
import re

from core.search.syntax import (
    QWHERE,
    QHAVE,
    QWITHOUT,
    QWITH,
    QOR,
    QEND,
    QINIT,
    QCONT,
    QTERM,
    PARENT_REF,
    atomRe,
    atomOpRe,
    compRe,
    identRe,
    indentLineRe,
    kRe,
    nameRe,
    namesRe,
    numRe,
    noneRe,
    opLineRe,
    opStripRe,
    trueRe,
    relRe,
    reRe,
    whiteRe,
    quLineRe,
)


class TestQuantifierConstants:
    """Tests for quantifier keyword constants."""

    def test_qwhere_value(self):
        """QWHERE should be /where/."""
        assert QWHERE == "/where/"

    def test_qhave_value(self):
        """QHAVE should be /have/."""
        assert QHAVE == "/have/"

    def test_qwithout_value(self):
        """QWITHOUT should be /without/."""
        assert QWITHOUT == "/without/"

    def test_qwith_value(self):
        """QWITH should be /with/."""
        assert QWITH == "/with/"

    def test_qor_value(self):
        """QOR should be /or/."""
        assert QOR == "/or/"

    def test_qend_value(self):
        """QEND should be /-/."""
        assert QEND == "/-/"

    def test_qinit_contains_init_keywords(self):
        """QINIT should contain initialization quantifiers."""
        assert QWHERE in QINIT
        assert QWITHOUT in QINIT
        assert QWITH in QINIT

    def test_qcont_contains_continuation_keywords(self):
        """QCONT should contain continuation quantifiers."""
        assert QHAVE in QCONT
        assert QOR in QCONT

    def test_qterm_contains_termination_keywords(self):
        """QTERM should contain termination quantifiers."""
        assert QEND in QTERM

    def test_parent_ref(self):
        """PARENT_REF should be '..'."""
        assert PARENT_REF == ".."


class TestAtomRegex:
    """Tests for atom pattern matching."""

    def test_simple_atom(self):
        """Should match simple node type."""
        match = atomRe.match("word")
        assert match is not None
        assert match.group(2) == "word"

    def test_atom_with_indent(self):
        """Should capture indentation."""
        match = atomRe.match("  word")
        assert match is not None
        assert match.group(1) == "  "
        assert match.group(2) == "word"

    def test_atom_with_features(self):
        """Should capture following features."""
        match = atomRe.match("word pos=noun")
        assert match is not None
        assert match.group(2) == "word"
        assert match.group(3) == "pos=noun"

    def test_atom_with_multiple_features(self):
        """Should capture all features after node type."""
        match = atomRe.match("word pos=noun number=sg")
        assert match is not None
        assert "pos=noun" in match.group(3)


class TestAtomOpRegex:
    """Tests for atom with operator pattern matching."""

    def test_atom_with_operator(self):
        """Should match atom with preceding operator."""
        match = atomOpRe.match("  < word")
        assert match is not None
        assert match.group(2) == "<"
        assert match.group(3) == "word"

    def test_atom_with_complex_operator(self):
        """Should match complex operators like <<."""
        match = atomOpRe.match("  << phrase")
        assert match is not None
        assert match.group(2) == "<<"


class TestIdentRegex:
    """Tests for identifier pattern matching (feature=value)."""

    def test_equals_pattern(self):
        """Should match feature=value pattern."""
        match = identRe.match("pos=noun")
        assert match is not None
        assert match.group(1) == "pos"
        assert match.group(2) == "="
        assert match.group(3) == "noun"

    def test_hash_pattern(self):
        """Should match feature#value pattern."""
        match = identRe.match("pos#noun")
        assert match is not None
        assert match.group(1) == "pos"
        assert match.group(2) == "#"
        assert match.group(3) == "noun"

    def test_with_underscores(self):
        """Should match names with underscores."""
        match = identRe.match("word_type=common_noun")
        assert match is not None


class TestCompRegex:
    """Tests for comparison pattern matching (feature<value or feature>value)."""

    def test_less_than(self):
        """Should match feature<value."""
        match = compRe.match("chapter<10")
        assert match is not None
        assert match.group(1) == "chapter"
        assert match.group(2) == "<"
        assert match.group(3) == "10"

    def test_greater_than(self):
        """Should match feature>value."""
        match = compRe.match("verse>5")
        assert match is not None
        assert match.group(2) == ">"


class TestNameRegex:
    """Tests for name pattern matching."""

    def test_simple_name(self):
        """Should match simple alphanumeric names."""
        assert nameRe.match("word")
        assert nameRe.match("clause_atom")
        assert nameRe.match("verse123")

    def test_with_special_chars(self):
        """Should match names with allowed special chars."""
        assert nameRe.match("half-verse")
        assert nameRe.match("word.type")
        assert nameRe.match("word_type")

    def test_invalid_names(self):
        """Should not match invalid names."""
        assert not nameRe.match("word type")  # space
        assert not nameRe.match("word=type")  # equals


class TestNumRegex:
    """Tests for number pattern matching."""

    def test_positive_integer(self):
        """Should match positive integers."""
        assert numRe.match("123")
        assert numRe.match("0")

    def test_negative_integer(self):
        """Should match negative integers."""
        assert numRe.match("-42")

    def test_non_integer(self):
        """Should not match non-integers."""
        assert not numRe.match("3.14")
        assert not numRe.match("abc")


class TestNoneRegex:
    """Tests for none value pattern matching."""

    def test_simple_none(self):
        """Should match feature name alone (none value)."""
        match = noneRe.match("pos")
        assert match is not None
        assert match.group(1) == "pos"

    def test_none_with_hash(self):
        """Should match feature# pattern."""
        match = noneRe.match("pos#")
        assert match is not None
        assert match.group(2) == "#"


class TestTrueRegex:
    """Tests for true value pattern matching (feature*)."""

    def test_star_pattern(self):
        """Should match feature* pattern."""
        match = trueRe.match("gloss*")
        assert match is not None
        assert match.group(1) == "gloss"


class TestRelRegex:
    """Tests for relation pattern matching."""

    def test_simple_relation(self):
        """Should match simple relation patterns."""
        match = relRe.match("  word < clause")
        assert match is not None
        # Groups: (indent, name1, operator, name2)

    def test_complex_operator(self):
        """Should match relations with complex operators."""
        match = relRe.match("a << b")
        assert match is not None


class TestWhiteRegex:
    """Tests for whitespace/comment pattern matching."""

    def test_empty_line(self):
        """Should match empty lines."""
        assert whiteRe.match("")
        assert whiteRe.match("   ")

    def test_comment_line(self):
        """Should match comment lines starting with %."""
        assert whiteRe.match("% this is a comment")
        assert whiteRe.match("  % indented comment")


class TestQuLineRegex:
    """Tests for quantifier line pattern matching."""

    def test_where_line(self):
        """Should match /where/ line."""
        match = quLineRe.match("  /where/")
        assert match is not None
        assert match.group(2) == "/where/"

    def test_have_line(self):
        """Should match /have/ line."""
        match = quLineRe.match("/have/")
        assert match is not None

    def test_end_line(self):
        """Should match /-/ line."""
        match = quLineRe.match("/-/")
        assert match is not None

    def test_non_quantifier_line(self):
        """Should not match regular lines."""
        assert quLineRe.match("word") is None
        assert quLineRe.match("  phrase") is None


class TestEscapeSequences:
    """Tests for escape sequence handling."""

    def test_escapes_defined(self):
        """ESCAPES should contain common escape sequences."""
        from core.search.syntax import ESCAPES

        assert "\\t" in ESCAPES
        assert "\\n" in ESCAPES
        assert "\\\\" in ESCAPES

    def test_val_escapes_defined(self):
        """VAL_ESCAPES should contain value-specific escapes."""
        from core.search.syntax import VAL_ESCAPES

        assert "\\|" in VAL_ESCAPES
        assert "\\=" in VAL_ESCAPES


class TestKNearnessRegex:
    """Tests for k-nearness pattern matching (:k:, =k:, etc.)."""

    def test_colon_k_colon(self):
        """Should match :k: pattern (within k slots)."""
        match = kRe.match(":3:")
        assert match is not None
        assert match.group(2) == "3"

    def test_equals_k_colon(self):
        """Should match =k: pattern (start within k)."""
        match = kRe.match("=5:")
        assert match is not None
        assert match.group(1) == "="
        assert match.group(2) == "5"

    def test_colon_k_equals(self):
        """Should match :k= pattern (end within k)."""
        match = kRe.match(":2=")
        assert match is not None
        assert match.group(2) == "2"
        assert match.group(3) == "="

    def test_less_k_colon(self):
        """Should match <k: pattern (before within k)."""
        match = kRe.match("<4:")
        assert match is not None
        assert match.group(1) == "<"
        assert match.group(2) == "4"

    def test_colon_k_greater(self):
        """Should match :k> pattern (after within k)."""
        match = kRe.match(":1>")
        assert match is not None
        assert match.group(2) == "1"
        assert match.group(3) == ">"

    def test_multi_digit_k(self):
        """Should match multi-digit k values."""
        match = kRe.match(":10:")
        assert match is not None
        assert match.group(2) == "10"


class TestNamesRegex:
    """Tests for named node pattern matching (name:type)."""

    def test_simple_named_node(self):
        """Should match w:word pattern."""
        match = namesRe.match("w:word")
        assert match is not None
        assert match.group(1) == "w"

    def test_named_node_with_features(self):
        """Should match named node with features."""
        match = namesRe.match("vb:word pos=verb")
        assert match is not None
        assert match.group(1) == "vb"

    def test_long_name(self):
        """Should match longer variable names."""
        match = namesRe.match("first_word:word")
        assert match is not None
        assert match.group(1) == "first_word"

    def test_with_leading_whitespace(self):
        """Should match named node with leading whitespace."""
        match = namesRe.match("  w:word")
        assert match is not None
        assert match.group(1) == "w"


class TestRegexFeatureRegex:
    """Tests for regex feature constraint pattern (feature~regex)."""

    def test_simple_regex(self):
        """Should match feature~pattern."""
        match = reRe.match("word~hello")
        assert match is not None
        assert match.group(1) == "word"
        assert match.group(2) == "hello"

    def test_regex_with_anchors(self):
        """Should match regex with anchors."""
        match = reRe.match("word~^hello$")
        assert match is not None
        assert match.group(2) == "^hello$"

    def test_regex_with_character_class(self):
        """Should match regex with character classes."""
        match = reRe.match("pos~[nv]oun")
        assert match is not None
        assert match.group(2) == "[nv]oun"

    def test_regex_with_quantifiers(self):
        """Should match regex with quantifiers."""
        match = reRe.match("word~hel+o.*")
        assert match is not None
        assert match.group(2) == "hel+o.*"


class TestOpLineRegex:
    """Tests for standalone operator line pattern."""

    def test_simple_operator_line(self):
        """Should match standalone operator."""
        match = opLineRe.match("  [[")
        assert match is not None
        assert match.group(2) == "[["

    def test_edge_operator_line(self):
        """Should match edge operator."""
        match = opLineRe.match("  -parent>")
        assert match is not None
        assert match.group(2) == "-parent>"

    def test_complex_operator(self):
        """Should match complex operators."""
        match = opLineRe.match("<:>")
        assert match is not None


class TestOpStripRegex:
    """Tests for operator stripping pattern."""

    def test_strip_operator(self):
        """Should strip operator and capture rest."""
        match = opStripRe.match("  < word pos=noun")
        assert match is not None
        assert match.group(1) == "word pos=noun"

    def test_strip_complex_operator(self):
        """Should strip complex operator."""
        match = opStripRe.match("<< phrase")
        assert match is not None
        assert match.group(1) == "phrase"


class TestIndentLineRegex:
    """Tests for indent line pattern."""

    def test_capture_indent(self):
        """Should capture leading whitespace."""
        match = indentLineRe.match("    content")
        assert match is not None
        assert match.group(1) == "    "
        assert match.group(2) == "content"

    def test_no_indent(self):
        """Should match lines without indent."""
        match = indentLineRe.match("content")
        assert match is not None
        assert match.group(1) == ""


class TestSlotComparisonOperators:
    """Tests for slot comparison operators in relations."""

    def test_same_slots(self):
        """Should match == (same slots) operator."""
        match = relRe.match("a == b")
        assert match is not None
        assert match.group(3) == "=="

    def test_different_slots(self):
        """Should match ## (different slots) operator."""
        match = relRe.match("a ## b")
        assert match is not None
        assert match.group(3) == "##"

    def test_overlap(self):
        """Should match && (overlap) operator."""
        match = relRe.match("a && b")
        assert match is not None
        assert match.group(3) == "&&"

    def test_disjoint(self):
        """Should match || (disjoint) operator."""
        match = relRe.match("a || b")
        assert match is not None
        assert match.group(3) == "||"

    def test_embeds_left(self):
        """Should match [[ (embeds) operator."""
        match = relRe.match("phrase [[ word")
        assert match is not None
        assert match.group(3) == "[["

    def test_embeds_right(self):
        """Should match ]] (embedded in) operator."""
        match = relRe.match("word ]] phrase")
        assert match is not None
        assert match.group(3) == "]]"

    def test_slot_before(self):
        """Should match << (slot before) operator."""
        match = relRe.match("a << b")
        assert match is not None
        assert match.group(3) == "<<"

    def test_slot_after(self):
        """Should match >> (slot after) operator."""
        match = relRe.match("a >> b")
        assert match is not None
        assert match.group(3) == ">>"

    def test_adjacent_before(self):
        """Should match <: (adjacent before) operator."""
        match = relRe.match("a <: b")
        assert match is not None
        assert match.group(3) == "<:"

    def test_adjacent_after(self):
        """Should match :> (adjacent after) operator."""
        match = relRe.match("a :> b")
        assert match is not None
        assert match.group(3) == ":>"

    def test_start_aligned(self):
        """Should match =: (start aligned) operator."""
        match = relRe.match("a =: b")
        assert match is not None
        assert match.group(3) == "=:"

    def test_end_aligned(self):
        """Should match := (end aligned) operator."""
        match = relRe.match("a := b")
        assert match is not None
        assert match.group(3) == ":="


class TestEdgeFeatureOperators:
    """Tests for edge feature operators."""

    def test_edge_forward(self):
        """Should match -name> (forward edge) operator."""
        match = relRe.match("w -parent> p")
        assert match is not None
        assert match.group(3) == "-parent>"

    def test_edge_backward(self):
        """Should match <name- (backward edge) operator."""
        match = relRe.match("p <parent- w")
        assert match is not None
        assert match.group(3) == "<parent-"

    def test_edge_bidirectional(self):
        """Should match <name> (bidirectional edge) operator."""
        match = relRe.match("a <link> b")
        assert match is not None
        assert match.group(3) == "<link>"

    def test_edge_with_value(self):
        """Should match edge operator with value spec."""
        match = relRe.match("w -parent=1> p")
        assert match is not None
        assert "-parent=1>" in match.group(3)


class TestFeatureRelationOperators:
    """Tests for feature-based relation operators."""

    def test_feature_equality(self):
        """Should match .f. (feature equality) pattern."""
        match = relRe.match("a .pos. b")
        assert match is not None
        assert match.group(3) == ".pos."

    def test_feature_cross_equality(self):
        """Should match .f=g. (cross feature equality) pattern."""
        match = relRe.match("a .pos=gender. b")
        assert match is not None
        assert match.group(3) == ".pos=gender."

    def test_feature_inequality(self):
        """Should match .f#g. (feature inequality) pattern."""
        match = relRe.match("a .pos#type. b")
        assert match is not None
        assert match.group(3) == ".pos#type."

    def test_feature_less_than(self):
        """Should match .f<g. (feature less than) pattern."""
        match = relRe.match("a .chapter<verse. b")
        assert match is not None
        assert match.group(3) == ".chapter<verse."

    def test_feature_greater_than(self):
        """Should match .f>g. (feature greater than) pattern."""
        match = relRe.match("a .verse>chapter. b")
        assert match is not None
        assert match.group(3) == ".verse>chapter."


class TestAtomOpSlotOperators:
    """Tests for slot operators in atom patterns."""

    def test_embedding_operator(self):
        """Should match [[ operator before atom."""
        match = atomOpRe.match("  [[ word")
        assert match is not None
        assert match.group(2) == "[["

    def test_embedded_operator(self):
        """Should match ]] operator before atom."""
        match = atomOpRe.match("  ]] phrase")
        assert match is not None
        assert match.group(2) == "]]"

    def test_adjacent_operator(self):
        """Should match <: operator before atom."""
        match = atomOpRe.match("  <: word")
        assert match is not None
        assert match.group(2) == "<:"

    def test_edge_operator_in_atom(self):
        """Should match edge operator before atom."""
        match = atomOpRe.match("  -parent> phrase")
        assert match is not None
        assert match.group(2) == "-parent>"
