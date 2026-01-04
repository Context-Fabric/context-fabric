"""Unit tests for core.edgefeature module.

This module tests EdgeFeature class which provides access to edge feature data,
including forward/backward/bidirectional edge traversal and frequency analysis.
"""

import pytest
from unittest.mock import MagicMock

from core.edgefeature import EdgeFeature, EdgeFeatures


class TestEdgeFeatures:
    """Tests for EdgeFeatures container class."""

    def test_is_empty_class(self):
        """EdgeFeatures should be an empty container class."""
        ef = EdgeFeatures()
        assert ef is not None


class TestEdgeFeatureInit:
    """Tests for EdgeFeature initialization."""

    def test_initialization_without_values(self, mock_api, sample_edge_data):
        """EdgeFeature should initialize for edges without values."""
        metadata = {"description": "parent relationship"}
        ef = EdgeFeature(mock_api, metadata, sample_edge_data, doValues=False)

        assert ef.api is mock_api
        assert ef.meta == metadata
        assert ef.data == sample_edge_data
        assert ef.doValues is False

    def test_initialization_with_values(self, mock_api, sample_edge_data_with_values):
        """EdgeFeature should initialize for edges with values."""
        metadata = {"description": "relationship with type"}
        ef = EdgeFeature(
            mock_api, metadata, sample_edge_data_with_values, doValues=True
        )

        assert ef.doValues is True
        assert ef.data == sample_edge_data_with_values

    def test_creates_inverse_data(self, mock_api, sample_edge_data):
        """EdgeFeature should create inverse mapping on init."""
        ef = EdgeFeature(mock_api, {}, sample_edge_data, doValues=False)

        # Check that inverse data was created
        assert hasattr(ef, "dataInv")
        # Node 6 should have incoming edges from 1, 2, 3
        assert 1 in ef.dataInv.get(6, set())

    def test_tuple_data_format(self, mock_api):
        """EdgeFeature should handle tuple format (data, dataInv)."""
        data = {1: frozenset({2})}
        dataInv = {2: {1}}
        ef = EdgeFeature(mock_api, {}, (data, dataInv), doValues=False)

        assert ef.data == data
        assert ef.dataInv == dataInv


class TestEdgeFeatureF:
    """Tests for EdgeFeature.f() method (outgoing edges from node)."""

    def test_returns_outgoing_nodes(self, mock_api):
        """f() should return nodes connected by outgoing edges."""
        mock_api.C = MagicMock()
        mock_api.C.rank = MagicMock()
        mock_api.C.rank.data = list(range(20))

        data = {1: frozenset({5, 6}), 2: frozenset({6})}
        ef = EdgeFeature(mock_api, {}, data, doValues=False)

        result = ef.f(1)
        assert set(result) == {5, 6}

    def test_returns_empty_for_no_edges(self, mock_api):
        """f() should return empty tuple for nodes without outgoing edges."""
        mock_api.C = MagicMock()
        mock_api.C.rank = MagicMock()
        mock_api.C.rank.data = list(range(20))

        data = {1: frozenset({5})}
        ef = EdgeFeature(mock_api, {}, data, doValues=False)

        result = ef.f(999)
        assert result == ()

    def test_with_values(self, mock_api):
        """f() should return tuples of (node, value) when doValues=True."""
        mock_api.C = MagicMock()
        mock_api.C.rank = MagicMock()
        mock_api.C.rank.data = list(range(20))

        data = {1: {5: "parent", 6: "sibling"}}
        ef = EdgeFeature(mock_api, {}, data, doValues=True)

        result = ef.f(1)
        # Should be list of (node, value) tuples
        result_dict = dict(result)
        assert result_dict[5] == "parent"
        assert result_dict[6] == "sibling"


class TestEdgeFeatureT:
    """Tests for EdgeFeature.t() method (incoming edges to node)."""

    def test_returns_incoming_nodes(self, mock_api):
        """t() should return nodes that have edges to this node."""
        mock_api.C = MagicMock()
        mock_api.C.rank = MagicMock()
        mock_api.C.rank.data = list(range(20))

        data = {1: frozenset({6}), 2: frozenset({6}), 3: frozenset({6})}
        ef = EdgeFeature(mock_api, {}, data, doValues=False)

        result = ef.t(6)
        assert set(result) == {1, 2, 3}

    def test_returns_empty_for_no_incoming(self, mock_api):
        """t() should return empty tuple for nodes without incoming edges."""
        mock_api.C = MagicMock()
        mock_api.C.rank = MagicMock()
        mock_api.C.rank.data = list(range(20))

        data = {1: frozenset({6})}
        ef = EdgeFeature(mock_api, {}, data, doValues=False)

        result = ef.t(1)  # Node 1 has outgoing but no incoming
        assert result == ()


class TestEdgeFeatureB:
    """Tests for EdgeFeature.b() method (bidirectional edges)."""

    def test_returns_both_directions(self, mock_api):
        """b() should return nodes from both incoming and outgoing edges."""
        mock_api.C = MagicMock()
        mock_api.C.rank = MagicMock()
        mock_api.C.rank.data = list(range(20))

        # Node 3 has outgoing to 6, and incoming from 2
        data = {2: frozenset({3}), 3: frozenset({6})}
        ef = EdgeFeature(mock_api, {}, data, doValues=False)

        result = ef.b(3)
        # Should include both 2 (incoming) and 6 (outgoing)
        assert set(result) == {2, 6}

    def test_returns_empty_for_isolated_node(self, mock_api):
        """b() should return empty tuple for nodes with no edges."""
        mock_api.C = MagicMock()
        mock_api.C.rank = MagicMock()
        mock_api.C.rank.data = list(range(20))

        data = {1: frozenset({2})}
        ef = EdgeFeature(mock_api, {}, data, doValues=False)

        result = ef.b(999)
        assert result == ()

    def test_with_values_outgoing_precedence(self, mock_api):
        """b() with values should give precedence to outgoing edges."""
        mock_api.C = MagicMock()
        mock_api.C.rank = MagicMock()
        mock_api.C.rank.data = list(range(20))

        # Edge from 1->2 with value "out" and 2->1 with value "in"
        data = {1: {2: "out"}, 2: {1: "in"}}
        ef = EdgeFeature(mock_api, {}, data, doValues=True)

        result = ef.b(1)
        result_dict = dict(result)
        # Outgoing edge value should take precedence
        assert result_dict[2] == "out"


class TestEdgeFeatureItems:
    """Tests for EdgeFeature.items() method."""

    def test_yields_all_items(self, mock_api, sample_edge_data):
        """items() should yield all (node, edges) pairs."""
        ef = EdgeFeature(mock_api, {}, sample_edge_data, doValues=False)

        items_list = list(ef.items())
        assert len(items_list) == len(sample_edge_data)


class TestEdgeFeatureFreqList:
    """Tests for EdgeFeature.freqList() method."""

    def test_counts_edges_without_values(self, mock_api):
        """freqList() should count total edges when doValues=False."""
        data = {1: frozenset({2, 3}), 4: frozenset({5})}
        ef = EdgeFeature(mock_api, {}, data, doValues=False)

        result = ef.freqList()
        assert result == 3  # 3 edges total

    def test_counts_values_with_values(self, mock_api):
        """freqList() should count value frequencies when doValues=True."""
        data = {1: {2: "a", 3: "b"}, 4: {5: "a"}}
        ef = EdgeFeature(mock_api, {}, data, doValues=True)

        result = ef.freqList()
        freq_dict = dict(result)
        assert freq_dict["a"] == 2
        assert freq_dict["b"] == 1

    def test_filters_by_node_types(self, mock_api):
        """freqList() should filter by nodeTypesFrom/To."""
        mock_api.F = MagicMock()
        mock_api.F.otype = MagicMock()
        mock_api.F.otype.v = MagicMock(
            side_effect=lambda n: "word" if n <= 3 else "phrase"
        )

        data = {1: frozenset({4}), 2: frozenset({5}), 4: frozenset({5})}
        ef = EdgeFeature(mock_api, {}, data, doValues=False)

        # Only count edges from "word" type nodes
        result = ef.freqList(nodeTypesFrom={"word"})
        assert result == 2  # edges from nodes 1 and 2


class TestEdgeFeatureMetadata:
    """Tests for EdgeFeature metadata access."""

    def test_meta_accessible(self, mock_api, sample_edge_data):
        """meta attribute should contain the metadata."""
        metadata = {"description": "parent edge", "valueType": "str"}
        ef = EdgeFeature(mock_api, metadata, sample_edge_data, doValues=False)

        assert ef.meta["description"] == "parent edge"


class TestEdgeFeatureEdgeCases:
    """Tests for edge cases and error conditions."""

    def test_empty_edge_set(self, mock_api):
        """Should handle empty edge sets correctly."""
        data = {1: frozenset()}
        mock_api.C = MagicMock()
        mock_api.C.rank = MagicMock()
        mock_api.C.rank.data = list(range(10))

        ef = EdgeFeature(mock_api, {}, data, doValues=False)
        result = ef.f(1)
        assert result == ()

    def test_self_referential_edge(self, mock_api):
        """Should handle edges from node to itself."""
        data = {1: frozenset({1})}
        mock_api.C = MagicMock()
        mock_api.C.rank = MagicMock()
        mock_api.C.rank.data = list(range(10))

        ef = EdgeFeature(mock_api, {}, data, doValues=False)

        result_f = ef.f(1)
        assert 1 in result_f

        result_t = ef.t(1)
        assert 1 in result_t
