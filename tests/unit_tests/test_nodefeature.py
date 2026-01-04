"""Unit tests for core.nodefeature module.

This module tests NodeFeature class which provides access to node feature data,
including value retrieval, node selection, and frequency analysis.
"""

import pytest
from unittest.mock import MagicMock

from core.nodefeature import NodeFeature, NodeFeatures


class TestNodeFeatures:
    """Tests for NodeFeatures container class."""

    def test_is_empty_class(self):
        """NodeFeatures should be an empty container class."""
        nf = NodeFeatures()
        assert nf is not None


class TestNodeFeatureInit:
    """Tests for NodeFeature initialization."""

    def test_initialization(self, mock_api, sample_node_data):
        """NodeFeature should initialize with api, metadata, and data."""
        metadata = {"valueType": "str", "description": "test feature"}
        nf = NodeFeature(mock_api, metadata, sample_node_data)

        assert nf.api is mock_api
        assert nf.meta == metadata
        assert nf.data == sample_node_data

    def test_empty_data(self, mock_api):
        """NodeFeature should work with empty data."""
        nf = NodeFeature(mock_api, {}, {})
        assert nf.data == {}


class TestNodeFeatureV:
    """Tests for NodeFeature.v() method (get value for node)."""

    def test_returns_value_for_existing_node(self, mock_api, sample_node_data):
        """v() should return value for nodes that have a value."""
        nf = NodeFeature(mock_api, {}, sample_node_data)

        assert nf.v(1) == "word1"
        assert nf.v(2) == "word2"
        assert nf.v(3) == "word3"

    def test_returns_none_for_missing_node(self, mock_api, sample_node_data):
        """v() should return None for nodes without a value."""
        nf = NodeFeature(mock_api, {}, sample_node_data)

        assert nf.v(999) is None
        assert nf.v(0) is None

    def test_works_with_integer_values(self, mock_api):
        """v() should work with integer feature values."""
        data = {1: 100, 2: 200, 3: 300}
        nf = NodeFeature(mock_api, {"valueType": "int"}, data)

        assert nf.v(1) == 100
        assert nf.v(2) == 200


class TestNodeFeatureItems:
    """Tests for NodeFeature.items() method."""

    def test_yields_all_items(self, mock_api, sample_node_data):
        """items() should yield all (node, value) pairs."""
        nf = NodeFeature(mock_api, {}, sample_node_data)

        items_list = list(nf.items())
        assert len(items_list) == len(sample_node_data)

        for node, value in items_list:
            assert sample_node_data[node] == value

    def test_empty_data_yields_nothing(self, mock_api):
        """items() on empty data should yield no items."""
        nf = NodeFeature(mock_api, {}, {})
        items_list = list(nf.items())
        assert items_list == []


class TestNodeFeatureS:
    """Tests for NodeFeature.s() method (select nodes by value)."""

    def test_returns_nodes_with_value(self, mock_api):
        """s() should return all nodes with the specified value."""
        # Setup mock C.rank.data for sorting
        mock_api.C = MagicMock()
        mock_api.C.rank = MagicMock()
        mock_api.C.rank.data = list(range(10))  # Simple rank data

        data = {1: "a", 2: "b", 3: "a", 4: "a", 5: "c"}
        nf = NodeFeature(mock_api, {}, data)

        result = nf.s("a")
        assert set(result) == {1, 3, 4}

    def test_returns_empty_for_nonexistent_value(self, mock_api):
        """s() should return empty tuple for values not in data."""
        mock_api.C = MagicMock()
        mock_api.C.rank = MagicMock()
        mock_api.C.rank.data = list(range(10))

        data = {1: "a", 2: "b"}
        nf = NodeFeature(mock_api, {}, data)

        result = nf.s("nonexistent")
        assert result == ()

    def test_returns_tuple(self, mock_api):
        """s() should always return a tuple."""
        mock_api.C = MagicMock()
        mock_api.C.rank = MagicMock()
        mock_api.C.rank.data = list(range(10))

        data = {1: "a"}
        nf = NodeFeature(mock_api, {}, data)

        result = nf.s("a")
        assert isinstance(result, tuple)


class TestNodeFeatureFreqList:
    """Tests for NodeFeature.freqList() method."""

    def test_counts_all_values(self, mock_api):
        """freqList() should count all value occurrences."""
        data = {1: "a", 2: "b", 3: "a", 4: "a", 5: "b"}
        nf = NodeFeature(mock_api, {}, data)

        result = nf.freqList()

        # Convert to dict for easier testing
        freq_dict = dict(result)
        assert freq_dict["a"] == 3
        assert freq_dict["b"] == 2

    def test_orders_by_frequency_descending(self, mock_api):
        """freqList() should order by frequency, highest first."""
        data = {1: "rare", 2: "common", 3: "common", 4: "common"}
        nf = NodeFeature(mock_api, {}, data)

        result = nf.freqList()

        # First item should be most frequent
        assert result[0][0] == "common"
        assert result[0][1] == 3

    def test_filters_by_node_types(self, mock_api):
        """freqList() should filter by nodeTypes if provided."""
        # Setup otype feature
        mock_api.F = MagicMock()
        mock_api.F.otype = MagicMock()
        mock_api.F.otype.v = MagicMock(
            side_effect=lambda n: "word" if n <= 3 else "phrase"
        )

        data = {1: "a", 2: "a", 3: "b", 4: "a", 5: "b"}
        nf = NodeFeature(mock_api, {}, data)

        # Only count values for "word" type nodes (1, 2, 3)
        result = nf.freqList(nodeTypes={"word"})

        freq_dict = dict(result)
        assert freq_dict["a"] == 2  # nodes 1, 2
        assert freq_dict["b"] == 1  # node 3

    def test_empty_data_returns_empty(self, mock_api):
        """freqList() on empty data should return empty tuple."""
        nf = NodeFeature(mock_api, {}, {})
        result = nf.freqList()
        assert result == ()


class TestNodeFeatureMetadata:
    """Tests for NodeFeature metadata access."""

    def test_meta_accessible(self, mock_api, sample_node_data):
        """meta attribute should contain the metadata."""
        metadata = {
            "valueType": "str",
            "description": "Test feature",
            "author": "Test Author",
        }
        nf = NodeFeature(mock_api, metadata, sample_node_data)

        assert nf.meta["valueType"] == "str"
        assert nf.meta["description"] == "Test feature"
        assert nf.meta["author"] == "Test Author"


class TestNodeFeatureEdgeCases:
    """Tests for edge cases and error conditions."""

    def test_none_value_in_data(self, mock_api):
        """v() should handle None values in data correctly."""
        data = {1: "value", 2: None, 3: "other"}
        nf = NodeFeature(mock_api, {}, data)

        assert nf.v(1) == "value"
        assert nf.v(2) is None  # Explicit None in data
        assert nf.v(999) is None  # Missing key

    def test_zero_node_id(self, mock_api):
        """v() should handle node id 0 correctly."""
        data = {0: "zero", 1: "one"}
        nf = NodeFeature(mock_api, {}, data)

        assert nf.v(0) == "zero"

    def test_large_node_ids(self, mock_api):
        """Should handle large node IDs correctly."""
        large_id = 10**9
        data = {large_id: "large"}
        nf = NodeFeature(mock_api, {}, data)

        assert nf.v(large_id) == "large"
        assert nf.v(large_id + 1) is None
