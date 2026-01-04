"""Unit tests for core.helpers module.

This module tests utility functions used throughout the Context Fabric codebase.
Tests cover escaping functions, type checking, string manipulation, set operations,
and data conversion utilities.
"""

import pytest
from datetime import datetime, timezone

from core.helpers import (
    utcnow,
    versionSort,
    var,
    isInt,
    mathEsc,
    mdEsc,
    htmlEsc,
    xmlEsc,
    mdhtmlEsc,
    tsvEsc,
    pandasEsc,
    camel,
    check32,
    cleanName,
    isClean,
    flattenToSet,
    setFromSpec,
    rangesFromSet,
    rangesFromList,
    specFromRanges,
    specFromRangesLogical,
    valueFromTf,
    tfFromValue,
    makeIndex,
    makeInverse,
    makeInverseVal,
    nbytes,
    itemize,
    fitemize,
    project,
    setFromValue,
    setFromStr,
    mergeDictOfSets,
    mergeDict,
    formatMeta,
    deepSize,
)


class TestUtcnow:
    """Tests for utcnow() function."""

    def test_returns_datetime(self):
        """utcnow() should return a datetime object."""
        result = utcnow()
        assert isinstance(result, datetime)

    def test_returns_utc_timezone(self):
        """utcnow() should return a datetime with UTC timezone."""
        result = utcnow()
        assert result.tzinfo == timezone.utc

    def test_is_recent(self):
        """utcnow() should return a time close to now."""
        before = datetime.now(timezone.utc)
        result = utcnow()
        after = datetime.now(timezone.utc)
        assert before <= result <= after


class TestVersionSort:
    """Tests for versionSort() function."""

    def test_simple_version(self):
        """Simple version strings should sort correctly."""
        versions = ["1.0", "2.0", "1.1", "10.0"]
        sorted_versions = sorted(versions, key=versionSort)
        assert sorted_versions == ["1.0", "1.1", "2.0", "10.0"]

    def test_complex_version(self):
        """Complex version strings with alpha suffixes should sort correctly."""
        versions = ["1.0a", "1.0b", "1.0", "2.0"]
        sorted_versions = sorted(versions, key=versionSort)
        # Numeric part 1 < 2, and empty alpha < 'a' < 'b'
        assert sorted_versions[0] == "1.0"
        assert "2.0" in sorted_versions

    def test_multipart_version(self):
        """Multi-part versions like semver should sort correctly."""
        versions = ["1.2.3", "1.2.10", "1.10.0", "2.0.0"]
        sorted_versions = sorted(versions, key=versionSort)
        assert sorted_versions == ["1.2.3", "1.2.10", "1.10.0", "2.0.0"]


class TestVar:
    """Tests for var() function (environment variable access)."""

    def test_existing_var(self, monkeypatch):
        """Should return value of existing environment variable."""
        monkeypatch.setenv("TEST_VAR_CF", "test_value")
        assert var("TEST_VAR_CF") == "test_value"

    def test_nonexistent_var(self):
        """Should return None for non-existent environment variable."""
        assert var("DEFINITELY_NOT_A_REAL_ENV_VAR_XYZ") is None


class TestIsInt:
    """Tests for isInt() function."""

    def test_integer_string(self):
        """String containing integer should return True."""
        assert isInt("42") is True
        assert isInt("-123") is True
        assert isInt("0") is True

    def test_float_string(self):
        """String containing float should return False."""
        assert isInt("3.14") is False
        assert isInt("1.0") is False

    def test_non_numeric_string(self):
        """Non-numeric string should return False."""
        assert isInt("hello") is False
        assert isInt("") is False

    def test_actual_integer(self):
        """Actual integer should return True."""
        assert isInt(42) is True
        assert isInt(-1) is True

    def test_none_value(self):
        """None should return False."""
        assert isInt(None) is False


class TestMathEsc:
    """Tests for mathEsc() function (escape dollar signs)."""

    def test_escapes_dollar(self):
        """Dollar signs should be wrapped in span elements."""
        assert mathEsc("$100") == "<span>$</span>100"
        assert mathEsc("cost: $50") == "cost: <span>$</span>50"

    def test_multiple_dollars(self):
        """Multiple dollar signs should all be escaped."""
        assert mathEsc("$a + $b") == "<span>$</span>a + <span>$</span>b"

    def test_none_input(self):
        """None input should return empty string."""
        assert mathEsc(None) == ""

    def test_no_dollars(self):
        """String without dollars should be unchanged."""
        assert mathEsc("hello world") == "hello world"


class TestMdEsc:
    """Tests for mdEsc() function (escape markdown characters)."""

    def test_escapes_special_chars(self):
        """Special markdown characters should be escaped."""
        result = mdEsc("*bold* and _italic_")
        assert "&#42;" in result  # asterisk
        assert "&#95;" in result  # underscore

    def test_escapes_brackets(self):
        """Brackets should be escaped."""
        result = mdEsc("[link]")
        assert "&#91;" in result

    def test_none_input(self):
        """None input should return empty string."""
        assert mdEsc(None) == ""

    def test_math_mode(self):
        """With math=True, dollar signs should not be escaped."""
        assert "$" in mdEsc("$x$", math=True)
        assert "<span>" not in mdEsc("$x$", math=True)

    def test_no_math_mode(self):
        """With math=False (default), dollar signs should be escaped."""
        assert "<span>$</span>" in mdEsc("$x$", math=False)


class TestHtmlEsc:
    """Tests for htmlEsc() function (escape HTML characters)."""

    def test_escapes_ampersand(self):
        """Ampersand should be escaped."""
        assert htmlEsc("A & B") == "A &amp; B"

    def test_escapes_angle_brackets(self):
        """Angle brackets should be escaped."""
        assert htmlEsc("<tag>") == "&lt;tag&gt;"

    def test_none_input(self):
        """None input should return empty string."""
        assert htmlEsc(None) == ""

    def test_math_mode(self):
        """With math=True, dollar signs should not be escaped."""
        assert "$" in htmlEsc("$x$", math=True)

    def test_combined_escaping(self):
        """Multiple special characters should all be escaped."""
        result = htmlEsc("<a & b>")
        assert "&lt;" in result
        assert "&amp;" in result
        assert "&gt;" in result


class TestXmlEsc:
    """Tests for xmlEsc() function (escape XML characters)."""

    def test_escapes_quotes(self):
        """Quotes should be escaped."""
        assert "&apos;" in xmlEsc("it's")
        assert "&quot;" in xmlEsc('say "hello"')

    def test_escapes_html_entities(self):
        """HTML entities should also be escaped."""
        result = xmlEsc("<tag & attr='val'>")
        assert "&lt;" in result
        assert "&amp;" in result
        assert "&apos;" in result
        assert "&gt;" in result

    def test_none_input(self):
        """None input should return empty string."""
        assert xmlEsc(None) == ""


class TestMdhtmlEsc:
    """Tests for mdhtmlEsc() function (escape markdown and HTML)."""

    def test_escapes_pipe(self):
        """Pipe character should be escaped (for tables)."""
        assert "&#124;" in mdhtmlEsc("a | b")

    def test_escapes_html(self):
        """HTML characters should be escaped."""
        result = mdhtmlEsc("<div>")
        assert "&lt;" in result
        assert "&gt;" in result

    def test_none_input(self):
        """None input should return empty string."""
        assert mdhtmlEsc(None) == ""


class TestTsvEsc:
    """Tests for tsvEsc() function (escape for TSV)."""

    def test_escapes_leading_quote(self):
        """Leading quotes should be escaped with backslash."""
        assert tsvEsc('"hello') == '\\"hello'
        assert tsvEsc("'hello") == "\\'hello"

    def test_no_escape_internal_quote(self):
        """Internal quotes should not be escaped."""
        assert tsvEsc('say "hi"') == 'say "hi"'

    def test_empty_string(self):
        """Empty string should remain empty."""
        assert tsvEsc("") == ""

    def test_normal_string(self):
        """Normal strings should be unchanged."""
        assert tsvEsc("hello") == "hello"


class TestPandasEsc:
    """Tests for pandasEsc() function."""

    def test_replaces_tabs(self):
        """Tabs should be replaced with spaces."""
        assert pandasEsc("a\tb") == "a b"

    def test_escapes_quote_char(self):
        """Quote character should be escaped."""
        result = pandasEsc('say "hi"')
        assert "\u0001" in result  # PANDAS_ESCAPE char

    def test_empty_string(self):
        """Empty string should remain empty."""
        assert pandasEsc("") == ""


class TestCamel:
    """Tests for camel() function (convert to camelCase)."""

    def test_snake_to_camel(self):
        """Snake case should convert to camelCase."""
        assert camel("hello_world") == "helloWorld"
        assert camel("my_variable_name") == "myVariableName"

    def test_already_lowercase(self):
        """Single word should have lowercase first letter."""
        assert camel("hello") == "hello"

    def test_empty_string(self):
        """Empty string should return empty string."""
        assert camel("") == ""

    def test_none_input(self):
        """None should return None."""
        assert camel(None) is None


class TestCheck32:
    """Tests for check32() function (check 32-bit Python)."""

    def test_returns_tuple(self):
        """Should return a tuple of (is32bit, warning, message)."""
        result = check32()
        assert isinstance(result, tuple)
        assert len(result) == 3

    def test_tuple_contents(self):
        """Tuple should contain boolean and strings."""
        on32, warn, msg = check32()
        assert isinstance(on32, bool)
        assert isinstance(warn, str)
        assert isinstance(msg, str)


class TestCleanName:
    """Tests for cleanName() function."""

    def test_valid_name(self):
        """Valid names should be unchanged."""
        assert cleanName("myVariable") == "myVariable"
        assert cleanName("var_123") == "var_123"

    def test_invalid_start(self):
        """Names starting with non-letter should be prefixed."""
        assert cleanName("123abc").startswith("x")
        assert cleanName("_test").startswith("x")

    def test_invalid_chars(self):
        """Invalid characters should be replaced with underscore."""
        result = cleanName("my-var.name")
        assert "-" not in result
        assert "." not in result

    def test_mql_keywords(self):
        """MQL keywords should be mapped to safe alternatives."""
        assert cleanName("type") == "typ"
        assert cleanName("database") == "dbase"


class TestIsClean:
    """Tests for isClean() function."""

    def test_clean_names(self):
        """Valid identifiers should return True."""
        assert isClean("myVar") is True
        assert isClean("var_123") is True
        assert isClean("ABC") is True

    def test_invalid_start(self):
        """Names starting with non-letter should return False."""
        assert isClean("123abc") is False
        assert isClean("_test") is False

    def test_none_or_empty(self):
        """None or empty string should return False."""
        assert isClean(None) is False
        assert isClean("") is False


class TestSetFromSpec:
    """Tests for setFromSpec() function (parse range specifications)."""

    def test_single_number(self):
        """Single number should parse to set with one element."""
        assert setFromSpec("5") == {5}

    def test_range(self):
        """Range specification should expand correctly."""
        assert setFromSpec("1-5") == {1, 2, 3, 4, 5}

    def test_comma_separated(self):
        """Comma-separated values should parse correctly."""
        assert setFromSpec("1,3,5") == {1, 3, 5}

    def test_mixed(self):
        """Mix of singles and ranges should parse correctly."""
        assert setFromSpec("1-3,5,7-9") == {1, 2, 3, 5, 7, 8, 9}

    def test_reverse_range(self):
        """Reverse range (e.g., 5-1) should still work."""
        assert setFromSpec("5-1") == {1, 2, 3, 4, 5}


class TestRangesFromSet:
    """Tests for rangesFromSet() function."""

    def test_consecutive_numbers(self):
        """Consecutive numbers should form single range."""
        result = list(rangesFromSet({1, 2, 3, 4, 5}))
        assert result == [(1, 5)]

    def test_separate_numbers(self):
        """Non-consecutive numbers should form separate ranges."""
        result = list(rangesFromSet({1, 3, 5}))
        assert result == [(1, 1), (3, 3), (5, 5)]

    def test_mixed(self):
        """Mixed consecutive and separate should produce correct ranges."""
        result = list(rangesFromSet({1, 2, 3, 5, 7, 8}))
        assert result == [(1, 3), (5, 5), (7, 8)]

    def test_empty_set(self):
        """Empty set should produce no ranges."""
        result = list(rangesFromSet(set()))
        assert result == []


class TestRangesFromList:
    """Tests for rangesFromList() function."""

    def test_sorted_list(self):
        """Sorted list should produce correct ranges."""
        result = list(rangesFromList([1, 2, 3, 5, 6]))
        assert result == [(1, 3), (5, 6)]

    def test_single_element(self):
        """Single element list should produce one range."""
        result = list(rangesFromList([42]))
        assert result == [(42, 42)]

    def test_empty_list(self):
        """Empty list should produce no ranges."""
        result = list(rangesFromList([]))
        assert result == []


class TestSpecFromRanges:
    """Tests for specFromRanges() function."""

    def test_single_ranges(self):
        """Single-element ranges should format as numbers."""
        result = specFromRanges([(1, 1), (3, 3)])
        assert result == "1,3"

    def test_multi_element_ranges(self):
        """Multi-element ranges should format with dash."""
        result = specFromRanges([(1, 5)])
        assert result == "1-5"

    def test_mixed_ranges(self):
        """Mixed ranges should format correctly."""
        result = specFromRanges([(1, 3), (5, 5), (7, 10)])
        assert result == "1-3,5,7-10"


class TestSpecFromRangesLogical:
    """Tests for specFromRangesLogical() function."""

    def test_single_ranges(self):
        """Single-element ranges should return single numbers."""
        result = specFromRangesLogical([(1, 1), (3, 3)])
        assert result == [1, 3]

    def test_multi_element_ranges(self):
        """Multi-element ranges should return [start, end] lists."""
        result = specFromRangesLogical([(1, 5)])
        assert result == [[1, 5]]


class TestValueFromTf:
    """Tests for valueFromTf() function (parse TF format)."""

    def test_escaped_tab(self):
        """Escaped tab should be unescaped."""
        assert valueFromTf("a\\tb") == "a\tb"

    def test_escaped_newline(self):
        """Escaped newline should be unescaped."""
        assert valueFromTf("a\\nb") == "a\nb"

    def test_escaped_backslash(self):
        """Escaped backslash should be unescaped."""
        assert valueFromTf("a\\\\b") == "a\\b"

    def test_plain_text(self):
        """Plain text should be unchanged."""
        assert valueFromTf("hello world") == "hello world"


class TestTfFromValue:
    """Tests for tfFromValue() function (convert to TF format)."""

    def test_tab_escaped(self):
        """Tab should be escaped."""
        assert tfFromValue("a\tb") == "a\\tb"

    def test_newline_escaped(self):
        """Newline should be escaped."""
        assert tfFromValue("a\nb") == "a\\nb"

    def test_backslash_escaped(self):
        """Backslash should be escaped."""
        assert tfFromValue("a\\b") == "a\\\\b"

    def test_integer(self):
        """Integer should be converted to string."""
        assert tfFromValue(42) == "42"

    def test_invalid_type(self):
        """Invalid type should return None."""
        assert tfFromValue([1, 2, 3]) is None
        assert tfFromValue({"a": 1}) is None


class TestMakeIndex:
    """Tests for makeIndex() function."""

    def test_creates_inverse_mapping(self):
        """Should create inverse mapping from values to keys."""
        data = {1: "a", 2: "a", 3: "b"}
        result = makeIndex(data)
        assert result["a"] == {1, 2}
        assert result["b"] == {3}


class TestMakeInverse:
    """Tests for makeInverse() function."""

    def test_inverts_edge_data(self):
        """Should invert edge data (from source to target)."""
        data = {1: {2, 3}, 2: {3}}
        result = makeInverse(data)
        assert result[2] == {1}
        assert result[3] == {1, 2}


class TestMakeInverseVal:
    """Tests for makeInverseVal() function."""

    def test_inverts_valued_edges(self):
        """Should invert edges while preserving values."""
        data = {1: {2: "x", 3: "y"}}
        result = makeInverseVal(data)
        assert result[2] == {1: "x"}
        assert result[3] == {1: "y"}


class TestNbytes:
    """Tests for nbytes() function (format byte sizes)."""

    def test_bytes(self):
        """Small values should show as bytes."""
        assert "B" in nbytes(100)

    def test_kilobytes(self):
        """~1KB should show as KB."""
        assert "KB" in nbytes(2048)

    def test_megabytes(self):
        """~1MB should show as MB."""
        assert "MB" in nbytes(2 * 1024 * 1024)

    def test_gigabytes(self):
        """~1GB should show as GB."""
        assert "GB" in nbytes(2 * 1024 * 1024 * 1024)


class TestItemize:
    """Tests for itemize() function."""

    def test_default_split(self):
        """Default should split on whitespace."""
        assert itemize("a b c") == ["a", "b", "c"]

    def test_custom_separator(self):
        """Custom separator should be used."""
        assert itemize("a,b,c", sep=",") == ["a", "b", "c"]

    def test_empty_input(self):
        """Empty input should return empty list."""
        assert itemize("") == []
        assert itemize(None) == []


class TestFitemize:
    """Tests for fitemize() function."""

    def test_string_input(self):
        """String should be split on whitespace/commas."""
        result = fitemize("a, b, c")
        assert "a" in result
        assert "b" in result
        assert "c" in result

    def test_numeric_input(self):
        """Numeric values should be stringified."""
        assert fitemize(42) == ["42"]
        assert fitemize(True) == ["True"]

    def test_empty_input(self):
        """Empty input should return empty list."""
        assert fitemize("") == []
        assert fitemize(None) == []


class TestProject:
    """Tests for project() function."""

    def test_project_first(self):
        """Should project first element of tuples."""
        data = {(1, 2, 3), (4, 5, 6)}
        result = project(data, 1)
        assert result == {1, 4}

    def test_project_two(self):
        """Should project first two elements."""
        data = {(1, 2, 3), (4, 5, 6)}
        result = project(data, 2)
        assert result == {(1, 2), (4, 5)}


class TestSetFromValue:
    """Tests for setFromValue() function."""

    def test_set_input(self):
        """Set input should return same set."""
        s = {1, 2, 3}
        assert setFromValue(s) == s

    def test_string_input(self):
        """String should be split and converted."""
        result = setFromValue("1 2 3", asInt=True)
        assert result == {1, 2, 3}

    def test_none_input(self):
        """None should return empty set."""
        assert setFromValue(None) == set()


class TestSetFromStr:
    """Tests for setFromStr() function."""

    def test_splits_on_whitespace(self):
        """Should split on whitespace and punctuation."""
        result = setFromStr("a b c")
        assert result == {"a", "b", "c"}

    def test_none_input(self):
        """None should return empty set."""
        assert setFromStr(None) == set()


class TestMergeDictOfSets:
    """Tests for mergeDictOfSets() function."""

    def test_merges_sets(self):
        """Should merge sets for same keys."""
        d1 = {1: {10, 20}}
        d2 = {1: {20, 30}, 2: {40}}
        mergeDictOfSets(d1, d2)
        assert d1[1] == {10, 20, 30}
        assert d1[2] == {40}


class TestMergeDict:
    """Tests for mergeDict() function."""

    def test_simple_merge(self):
        """Should override values."""
        source = {"a": 1, "b": 2}
        overrides = {"b": 3, "c": 4}
        mergeDict(source, overrides)
        assert source == {"a": 1, "b": 3, "c": 4}

    def test_recursive_merge(self):
        """Should merge nested dicts recursively."""
        source = {"a": {"x": 1, "y": 2}}
        overrides = {"a": {"y": 3, "z": 4}}
        mergeDict(source, overrides)
        assert source["a"] == {"x": 1, "y": 3, "z": 4}


class TestFormatMeta:
    """Tests for formatMeta() function."""

    def test_combines_desc_and_eg(self):
        """Should combine 'desc' and 'eg' into 'description'."""
        meta = {"feat": {"desc": "A feature", "eg": "example"}}
        result = formatMeta(meta)
        assert result["feat"]["description"] == "A feature (example)"
        assert "desc" not in result["feat"]
        assert "eg" not in result["feat"]


class TestDeepSize:
    """Tests for deepSize() function."""

    def test_basic_types(self):
        """Should calculate size of basic types."""
        assert deepSize(42) > 0
        assert deepSize("hello") > 0
        assert deepSize([1, 2, 3]) > deepSize([1])

    def test_nested_structures(self):
        """Should handle nested structures."""
        nested = {"a": [1, 2, {"b": 3}]}
        size = deepSize(nested)
        assert size > 0

    def test_empty_structures(self):
        """Empty structures should still have non-zero size."""
        assert deepSize([]) > 0
        assert deepSize({}) > 0
