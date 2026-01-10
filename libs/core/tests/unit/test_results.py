"""Tests for cfabric.results module.

Focus on serialization correctness, especially handling of numpy types.
"""

import json
from unittest.mock import MagicMock

import numpy as np
import pytest

from cfabric.results import NodeInfo, NodeList, SearchResult, FeatureInfo


class TestNodeInfoSerialization:
    """Tests for NodeInfo JSON serialization."""

    def test_node_id_is_python_int(self):
        """Node ID should be Python int, not numpy type."""
        # Create mock API
        api = MagicMock()
        api.F.otype.v.return_value = "word"
        api.T.text.return_value = "hello"
        api.T.sectionFromNode.return_value = None

        # Pass numpy.uint32 (what Text-Fabric returns)
        numpy_node = np.uint32(42)
        info = NodeInfo.from_api(api, numpy_node)

        assert type(info.node) is int
        assert info.node == 42

    def test_to_dict_is_json_serializable(self):
        """to_dict() output must be JSON serializable."""
        api = MagicMock()
        api.F.otype.v.return_value = "word"
        api.T.text.return_value = "hello"
        api.T.sectionFromNode.return_value = None

        numpy_node = np.uint32(42)
        info = NodeInfo.from_api(api, numpy_node)

        # This should not raise
        result = json.dumps(info.to_dict())
        assert '"node": 42' in result

    def test_slots_are_python_ints(self):
        """Slot node IDs should be Python ints, not numpy types."""
        api = MagicMock()
        api.F.otype.v.return_value = "phrase"
        api.F.otype.slotType = "word"
        api.T.text.return_value = "hello world"
        api.T.sectionFromNode.return_value = None
        api.E.oslots.s.return_value = (np.uint32(1), np.uint32(2), np.uint32(3))

        numpy_node = np.uint32(100)
        info = NodeInfo.from_api(api, numpy_node, include_slots=True)

        assert info.slots is not None
        for slot in info.slots:
            assert type(slot) is int

        # Should be JSON serializable
        json.dumps(info.to_dict())


class TestSearchResultSerialization:
    """Tests for SearchResult JSON serialization."""

    def test_search_results_are_json_serializable(self):
        """Search results with numpy node IDs must be JSON serializable."""
        api = MagicMock()
        api.F.otype.v.return_value = "word"
        api.T.text.return_value = "hello"
        api.T.sectionFromNode.return_value = None

        # Simulate search results with numpy types
        numpy_results = [
            (np.uint32(1), np.uint32(2)),
            (np.uint32(3), np.uint32(4)),
        ]

        result = SearchResult.from_search(api, numpy_results, "word\nword")

        # This should not raise
        json_str = json.dumps(result.to_dict())
        assert '"node": 1' in json_str
        assert '"node": 2' in json_str


class TestNodeListSerialization:
    """Tests for NodeList JSON serialization."""

    def test_node_list_with_numpy_ids(self):
        """NodeList with numpy node IDs must be JSON serializable."""
        api = MagicMock()
        api.F.otype.v.return_value = "word"
        api.T.text.return_value = "hello"
        api.T.sectionFromNode.return_value = None

        numpy_nodes = [np.uint32(1), np.uint32(2), np.uint32(3)]
        node_list = NodeList.from_nodes(api, numpy_nodes, query="test")

        # All nodes should be Python ints
        for node_info in node_list.nodes:
            assert type(node_info.node) is int

        # Should be JSON serializable
        json.dumps(node_list.to_dict())


class TestFeatureInfoMetadata:
    """Tests for FeatureInfo metadata retrieval."""

    def test_metadata_from_tf_features_tf_format(self):
        """Should get metadata from TF.features with .tf format keys."""
        api = MagicMock()

        # Mock TF.features with .tf-style metadata
        feature_obj = MagicMock()
        feature_obj.metaData = {
            "valueType": "str",
            "description": "Test feature description",
        }
        api.CF.features.get.return_value = feature_obj

        info = FeatureInfo.from_api(api, "test_feature", "node")

        assert info is not None
        assert info.name == "test_feature"
        assert info.kind == "node"
        assert info.value_type == "str"
        assert info.description == "Test feature description"

    def test_metadata_from_tf_features_cfm_format(self):
        """Should get metadata from CF.features with .cfm format keys."""
        api = MagicMock()

        # Mock CF.features with .cfm-style metadata (value_type instead of valueType)
        feature_obj = MagicMock()
        feature_obj.metaData = {
            "value_type": "str",
            "description": "Loaded from .cfm format",
        }
        api.CF.features.get.return_value = feature_obj

        info = FeatureInfo.from_api(api, "test_feature", "node")

        assert info is not None
        assert info.value_type == "str"
        assert info.description == "Loaded from .cfm format"

    def test_edge_feature_metadata(self):
        """Should get metadata for edge features and check has_values."""
        api = MagicMock()

        feature_obj = MagicMock()
        feature_obj.metaData = {
            "value_type": "int",
            "description": "Edge feature",
        }
        api.CF.features.get.return_value = feature_obj

        edge_feature = MagicMock()
        edge_feature.doValues = True
        api.Es.return_value = edge_feature

        info = FeatureInfo.from_api(api, "test_edge", "edge")

        assert info is not None
        assert info.kind == "edge"
        assert info.description == "Edge feature"
        assert info.has_values is True

    def test_returns_none_when_feature_not_found(self):
        """Should return None when feature doesn't exist in CF.features."""
        api = MagicMock()
        api.CF.features.get.return_value = None

        info = FeatureInfo.from_api(api, "nonexistent", "node")

        assert info is None
