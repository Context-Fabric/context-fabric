"""Unit tests for core.generic module.

This module tests the AttrDict class and utility functions for
converting between dict and AttrDict structures.
"""

import pytest

from cfabric.core.generic import AttrDict, deepdict, deepAttrDict, isIterable


class TestAttrDict:
    """Tests for AttrDict class."""

    def test_basic_creation(self):
        """AttrDict should be creatable from a dictionary."""
        d = AttrDict({"a": 1, "b": 2})
        assert d["a"] == 1
        assert d["b"] == 2

    def test_attribute_access(self):
        """AttrDict should support attribute access."""
        d = AttrDict({"name": "test", "value": 42})
        assert d.name == "test"
        assert d.value == 42

    def test_mixed_access(self):
        """Both dict and attribute access should work together."""
        d = AttrDict({"x": 1})
        assert d["x"] == d.x == 1

    def test_missing_key_returns_none(self):
        """Accessing missing key via [] should return None."""
        d = AttrDict({"a": 1})
        assert d["nonexistent"] is None

    def test_missing_attr_returns_none(self):
        """Accessing missing attribute should return None."""
        d = AttrDict({"a": 1})
        assert d.nonexistent is None

    def test_attribute_assignment(self):
        """Should support attribute assignment."""
        d = AttrDict()
        d.name = "test"
        assert d.name == "test"
        assert d["name"] == "test"

    def test_dict_assignment(self):
        """Should support dict-style assignment."""
        d = AttrDict()
        d["name"] = "test"
        assert d.name == "test"
        assert d["name"] == "test"

    def test_nested_dict(self):
        """Nested dicts should work but won't auto-convert to AttrDict."""
        d = AttrDict({"outer": {"inner": 1}})
        assert d.outer == {"inner": 1}
        # Inner dict is not automatically an AttrDict
        assert isinstance(d.outer, dict)

    def test_is_dict_subclass(self):
        """AttrDict should be a dict subclass."""
        d = AttrDict({"a": 1})
        assert isinstance(d, dict)

    def test_dict_methods(self):
        """Standard dict methods should work."""
        d = AttrDict({"a": 1, "b": 2})
        assert list(d.keys()) == ["a", "b"]
        assert list(d.values()) == [1, 2]
        assert list(d.items()) == [("a", 1), ("b", 2)]

    def test_update(self):
        """Update method should work."""
        d = AttrDict({"a": 1})
        d.update({"b": 2})
        assert d.b == 2

    def test_deepdict_method(self):
        """AttrDict.deepdict() should return a regular dict."""
        d = AttrDict({"a": 1})
        result = d.deepdict()
        assert result == {"a": 1}
        assert type(result) is dict

    def test_empty_attrdict(self):
        """Empty AttrDict should work correctly."""
        d = AttrDict()
        assert len(d) == 0
        assert d.anything is None

    def test_kwargs_creation(self):
        """AttrDict should accept kwargs."""
        d = AttrDict(a=1, b=2)
        assert d.a == 1
        assert d.b == 2

    def test_iteration(self):
        """AttrDict should be iterable over keys."""
        d = AttrDict({"a": 1, "b": 2})
        keys = list(d)
        assert "a" in keys
        assert "b" in keys


class TestDeepdict:
    """Tests for deepdict() function."""

    def test_simple_attrdict(self):
        """Should convert simple AttrDict to dict."""
        d = AttrDict({"a": 1, "b": 2})
        result = deepdict(d)
        assert result == {"a": 1, "b": 2}
        assert type(result) is dict

    def test_nested_attrdict(self):
        """Should recursively convert nested AttrDicts."""
        inner = AttrDict({"x": 1})
        outer = AttrDict({"inner": inner})
        result = deepdict(outer)
        assert result == {"inner": {"x": 1}}
        assert type(result["inner"]) is dict

    def test_preserves_tuples(self):
        """Should preserve tuples by default."""
        d = AttrDict({"data": (1, 2, 3)})
        result = deepdict(d)
        assert result["data"] == (1, 2, 3)
        assert type(result["data"]) is tuple

    def test_preserves_frozensets(self):
        """Should preserve frozensets by default."""
        d = AttrDict({"data": frozenset({1, 2, 3})})
        result = deepdict(d)
        assert result["data"] == frozenset({1, 2, 3})
        assert type(result["data"]) is frozenset

    def test_ordinary_mode(self):
        """With ordinary=True, tuples become lists and frozensets become sets."""
        d = AttrDict({"tup": (1, 2), "fset": frozenset({3, 4})})
        result = deepdict(d, ordinary=True)
        assert result["tup"] == [1, 2]
        assert type(result["tup"]) is list
        assert result["fset"] == {3, 4}
        assert type(result["fset"]) is set

    def test_lists_preserved(self):
        """Lists should be preserved."""
        d = AttrDict({"data": [1, 2, 3]})
        result = deepdict(d)
        assert result["data"] == [1, 2, 3]
        assert type(result["data"]) is list

    def test_sets_preserved(self):
        """Sets should be preserved."""
        d = AttrDict({"data": {1, 2, 3}})
        result = deepdict(d)
        assert result["data"] == {1, 2, 3}
        assert type(result["data"]) is set

    def test_nested_in_list(self):
        """Should convert AttrDicts nested in lists."""
        inner = AttrDict({"x": 1})
        d = AttrDict({"nested": [inner, 2, 3]})
        result = deepdict(d)
        assert result["nested"][0] == {"x": 1}
        assert type(result["nested"][0]) is dict

    def test_atomic_values(self):
        """Atomic values should pass through unchanged."""
        d = AttrDict({"int": 42, "str": "hello", "bool": True, "none": None})
        result = deepdict(d)
        assert result["int"] == 42
        assert result["str"] == "hello"
        assert result["bool"] is True
        assert result["none"] is None

    def test_regular_dict_input(self):
        """Should also work with regular dict input."""
        d = {"a": {"b": 1}}
        result = deepdict(d)
        assert result == {"a": {"b": 1}}
        assert type(result) is dict


class TestDeepAttrDict:
    """Tests for deepAttrDict() function."""

    def test_simple_dict(self):
        """Should convert simple dict to AttrDict."""
        d = {"a": 1, "b": 2}
        result = deepAttrDict(d)
        assert result.a == 1
        assert result.b == 2
        assert isinstance(result, AttrDict)

    def test_nested_dict(self):
        """Should recursively convert nested dicts."""
        d = {"outer": {"inner": 1}}
        result = deepAttrDict(d)
        assert result.outer.inner == 1
        assert isinstance(result.outer, AttrDict)

    def test_preserves_tuples(self):
        """Should preserve tuples by default."""
        d = {"items": (1, 2, 3)}
        result = deepAttrDict(d)
        assert result.items == (1, 2, 3)
        assert type(result.items) is tuple

    def test_preserves_lists(self):
        """Should preserve lists by default."""
        d = {"items": [1, 2, 3]}
        result = deepAttrDict(d)
        assert result.items == [1, 2, 3]
        assert type(result.items) is list

    def test_prefer_tuples(self):
        """With preferTuples=True, lists become tuples."""
        d = {"items": [1, 2, 3]}
        result = deepAttrDict(d, preferTuples=True)
        assert result.items == (1, 2, 3)
        assert type(result.items) is tuple

    def test_nested_in_list(self):
        """Should convert dicts nested in lists."""
        d = {"items": [{"x": 1}, {"y": 2}]}
        result = deepAttrDict(d)
        assert result.items[0].x == 1
        assert result.items[1].y == 2
        assert isinstance(result.items[0], AttrDict)

    def test_nested_in_tuple(self):
        """Should convert dicts nested in tuples."""
        d = {"items": ({"x": 1}, {"y": 2})}
        result = deepAttrDict(d)
        assert result.items[0].x == 1
        assert result.items[1].y == 2

    def test_preserves_frozensets(self):
        """Should preserve frozensets."""
        d = {"items": frozenset({1, 2, 3})}
        result = deepAttrDict(d)
        assert result.items == frozenset({1, 2, 3})
        assert type(result.items) is frozenset

    def test_preserves_sets(self):
        """Should preserve sets."""
        d = {"items": {1, 2, 3}}
        result = deepAttrDict(d)
        assert result.items == {1, 2, 3}
        assert type(result.items) is set

    def test_atomic_values(self):
        """Atomic values should pass through unchanged."""
        d = {"int": 42, "str": "hello", "bool": True, "none": None}
        result = deepAttrDict(d)
        assert result.int == 42
        assert result.str == "hello"
        assert result.bool is True
        assert result.none is None

    def test_deeply_nested(self):
        """Should handle deeply nested structures."""
        d = {"a": {"b": {"c": {"d": 1}}}}
        result = deepAttrDict(d)
        assert result.a.b.c.d == 1

    def test_already_attrdict(self):
        """Should handle AttrDict input."""
        d = AttrDict({"a": {"b": 1}})
        result = deepAttrDict(d)
        assert result.a.b == 1
        assert isinstance(result.a, AttrDict)


class TestIsIterable:
    """Tests for isIterable() function."""

    def test_list_is_iterable(self):
        """Lists should be iterable."""
        assert isIterable([1, 2, 3]) is True

    def test_tuple_is_iterable(self):
        """Tuples should be iterable."""
        assert isIterable((1, 2, 3)) is True

    def test_set_is_iterable(self):
        """Sets should be iterable."""
        assert isIterable({1, 2, 3}) is True

    def test_dict_is_iterable(self):
        """Dicts should be iterable."""
        assert isIterable({"a": 1}) is True

    def test_string_is_not_iterable(self):
        """Strings should NOT be considered iterable (special case)."""
        assert isIterable("hello") is False

    def test_int_is_not_iterable(self):
        """Integers should not be iterable."""
        assert isIterable(42) is False

    def test_none_is_not_iterable(self):
        """None should not be iterable."""
        assert isIterable(None) is False

    def test_generator_is_iterable(self):
        """Generators should be iterable."""
        gen = (x for x in range(3))
        assert isIterable(gen) is True

    def test_range_is_iterable(self):
        """Range objects should be iterable."""
        assert isIterable(range(10)) is True

    def test_attrdict_is_iterable(self):
        """AttrDict should be iterable."""
        assert isIterable(AttrDict({"a": 1})) is True


class TestRoundTrip:
    """Tests for round-trip conversions between dict and AttrDict."""

    def test_dict_to_attrdict_to_dict(self):
        """Converting dict -> AttrDict -> dict should preserve data."""
        original = {"a": 1, "b": {"c": 2}}
        as_attr = deepAttrDict(original)
        back = deepdict(as_attr)
        assert back == original

    def test_attrdict_to_dict_to_attrdict(self):
        """Converting AttrDict -> dict -> AttrDict should preserve data."""
        inner = AttrDict({"x": 1})
        original = AttrDict({"outer": inner})
        as_dict = deepdict(original)
        back = deepAttrDict(as_dict)
        assert back.outer.x == 1

    def test_complex_structure_roundtrip(self):
        """Complex nested structures should survive round-trip."""
        original = {
            "name": "test",
            "values": [1, 2, 3],
            "nested": {"deep": {"deeper": True}},
            "empty": {},
        }
        as_attr = deepAttrDict(original)
        back = deepdict(as_attr)
        assert back == original
