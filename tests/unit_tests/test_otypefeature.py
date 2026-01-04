"""Unit tests for core.otypefeature module.

This module tests the OtypeFeature class that provides optimized
access to node type (otype) feature data.
"""

import pytest
from unittest.mock import MagicMock


class TestOtypeFeatureInit:
    """Tests for OtypeFeature initialization."""

    def test_basic_creation(self):
        """OtypeFeature should initialize with API, metadata, and data."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        metaData = {"description": "node type"}
        # data is (typeData, maxSlot, maxNode, slotType)
        data = (["phrase", "sentence"], 3, 5, "word")

        otype = OtypeFeature(mock_api, metaData, data)

        assert otype.api is mock_api
        assert otype.meta == metaData
        assert otype.maxSlot == 3
        assert otype.maxNode == 5
        assert otype.slotType == "word"

    def test_data_attribute(self):
        """OtypeFeature should store type data for non-slot nodes."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        metaData = {}
        # Non-slot nodes 4 and 5 have types "phrase" and "sentence"
        data = (["phrase", "sentence"], 3, 5, "word")

        otype = OtypeFeature(mock_api, metaData, data)

        assert otype.data == ["phrase", "sentence"]

    def test_all_attribute_initially_none(self):
        """OtypeFeature.all should initially be None."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        data = (["phrase"], 3, 4, "word")

        otype = OtypeFeature(mock_api, {}, data)

        assert otype.all is None


class TestOtypeV:
    """Tests for v() method - get node type."""

    def test_v_node_zero(self):
        """v(0) should return None."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        data = (["phrase"], 3, 4, "word")

        otype = OtypeFeature(mock_api, {}, data)

        assert otype.v(0) is None

    def test_v_slot_node(self):
        """v() should return slotType for slot nodes."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        # maxSlot = 3, so nodes 1, 2, 3 are slots
        data = (["phrase"], 3, 4, "word")

        otype = OtypeFeature(mock_api, {}, data)

        assert otype.v(1) == "word"
        assert otype.v(2) == "word"
        assert otype.v(3) == "word"

    def test_v_non_slot_node(self):
        """v() should return type from data for non-slot nodes."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        # maxSlot = 3, node 4 is "phrase", node 5 is "sentence"
        data = (["phrase", "sentence"], 3, 5, "word")

        otype = OtypeFeature(mock_api, {}, data)

        assert otype.v(4) == "phrase"
        assert otype.v(5) == "sentence"

    def test_v_out_of_range_node(self):
        """v() should return None for out-of-range nodes."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        data = (["phrase"], 3, 4, "word")

        otype = OtypeFeature(mock_api, {}, data)

        assert otype.v(100) is None
        assert otype.v(-1) == "word"  # negative treated as slot

    def test_v_boundary_node(self):
        """v() should handle boundary between slots and non-slots."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        data = (["phrase"], 3, 4, "word")

        otype = OtypeFeature(mock_api, {}, data)

        # Node 3 is last slot
        assert otype.v(3) == "word"
        # Node 4 is first non-slot
        assert otype.v(4) == "phrase"


class TestOtypeS:
    """Tests for s() method - query nodes by type."""

    def test_s_slot_type(self):
        """s() should return slot nodes for slot type."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        # Mock C.rank.data for sorting
        mock_api.C.rank.data = [0, 1, 2]  # ranks for nodes 1, 2, 3
        data = (["phrase"], 3, 4, "word")

        otype = OtypeFeature(mock_api, {}, data)
        # Add support attribute (normally added by pre-computing)
        otype.support = {"word": (1, 3), "phrase": (4, 4)}

        result = otype.s("word")

        assert set(result) == {1, 2, 3}

    def test_s_non_slot_type(self):
        """s() should return non-slot nodes for non-slot type."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        mock_api.C.rank.data = [0, 1, 2, 3, 4]
        data = (["phrase", "phrase"], 3, 5, "word")

        otype = OtypeFeature(mock_api, {}, data)
        otype.support = {"word": (1, 3), "phrase": (4, 5)}

        result = otype.s("phrase")

        assert set(result) == {4, 5}

    def test_s_unknown_type(self):
        """s() should return empty tuple for unknown type."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        data = (["phrase"], 3, 4, "word")

        otype = OtypeFeature(mock_api, {}, data)
        otype.support = {"word": (1, 3), "phrase": (4, 4)}

        result = otype.s("unknown_type")

        assert result == ()

    def test_s_returns_canonical_order(self):
        """s() should return nodes in canonical order."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        # Ranks: node 4 has rank 10, node 5 has rank 5, node 6 has rank 15
        mock_api.C.rank.data = [0, 0, 0, 10, 5, 15]
        data = (["phrase", "phrase", "phrase"], 3, 6, "word")

        otype = OtypeFeature(mock_api, {}, data)
        otype.support = {"phrase": (4, 6)}

        result = otype.s("phrase")

        # Should be sorted by rank: 5 (rank 5), 4 (rank 10), 6 (rank 15)
        assert result == (5, 4, 6)


class TestOtypeSInterval:
    """Tests for sInterval() method."""

    def test_sinterval_valid_type(self):
        """sInterval() should return interval for valid type."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        data = (["phrase", "phrase"], 3, 5, "word")

        otype = OtypeFeature(mock_api, {}, data)
        otype.support = {"word": (1, 3), "phrase": (4, 5)}

        result = otype.sInterval("phrase")

        assert result == (4, 5)

    def test_sinterval_slot_type(self):
        """sInterval() should return slot interval for slot type."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        data = (["phrase"], 3, 4, "word")

        otype = OtypeFeature(mock_api, {}, data)
        otype.support = {"word": (1, 3), "phrase": (4, 4)}

        result = otype.sInterval("word")

        assert result == (1, 3)

    def test_sinterval_unknown_type(self):
        """sInterval() should return empty tuple for unknown type."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        data = (["phrase"], 3, 4, "word")

        otype = OtypeFeature(mock_api, {}, data)
        otype.support = {"word": (1, 3), "phrase": (4, 4)}

        result = otype.sInterval("unknown")

        assert result == ()


class TestOtypeItems:
    """Tests for items() method."""

    def test_items_yields_all_nodes(self):
        """items() should yield (node, type) for all nodes."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        # 3 slots (word), 2 non-slots (phrase, sentence)
        data = (["phrase", "sentence"], 3, 5, "word")

        otype = OtypeFeature(mock_api, {}, data)

        result = list(otype.items())

        expected = [
            (1, "word"),
            (2, "word"),
            (3, "word"),
            (4, "phrase"),
            (5, "sentence"),
        ]
        assert result == expected

    def test_items_empty_corpus(self):
        """items() should yield nothing for empty corpus."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        # No slots, no non-slots
        data = ([], 0, 0, "word")

        otype = OtypeFeature(mock_api, {}, data)

        result = list(otype.items())

        assert result == []

    def test_items_slots_only(self):
        """items() should handle corpus with only slots."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        # 3 slots, no non-slots
        data = ([], 3, 3, "word")

        otype = OtypeFeature(mock_api, {}, data)

        result = list(otype.items())

        expected = [(1, "word"), (2, "word"), (3, "word")]
        assert result == expected

    def test_items_is_generator(self):
        """items() should return a generator."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        data = (["phrase"], 3, 4, "word")

        otype = OtypeFeature(mock_api, {}, data)

        result = otype.items()

        # Should be a generator, not a list
        assert hasattr(result, "__next__")


class TestOtypeMetadata:
    """Tests for metadata access."""

    def test_meta_attribute(self):
        """meta attribute should contain feature metadata."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        metaData = {
            "description": "node type",
            "valueType": "str",
            "custom": "value",
        }
        data = (["phrase"], 3, 4, "word")

        otype = OtypeFeature(mock_api, metaData, data)

        assert otype.meta == metaData
        assert otype.meta["description"] == "node type"
        assert otype.meta["valueType"] == "str"
        assert otype.meta["custom"] == "value"

    def test_max_slot_attribute(self):
        """maxSlot should report correct value."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        data = (["phrase"], 100, 150, "word")

        otype = OtypeFeature(mock_api, {}, data)

        assert otype.maxSlot == 100

    def test_max_node_attribute(self):
        """maxNode should report correct value."""
        from core.otypefeature import OtypeFeature

        mock_api = MagicMock()
        data = (["phrase"], 100, 150, "word")

        otype = OtypeFeature(mock_api, {}, data)

        assert otype.maxNode == 150
