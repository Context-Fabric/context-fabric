"""Unit tests for core.oslotsfeature module.

This module tests the OslotsFeature class that provides optimized
access to slot containment (oslots) feature data.
"""

import pytest
from unittest.mock import MagicMock


class TestOslotsFeatureInit:
    """Tests for OslotsFeature initialization."""

    def test_basic_creation(self):
        """OslotsFeature should initialize with API, metadata, and data."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        metaData = {"description": "slot containment"}
        # data is (slotData, maxSlot, maxNode)
        # slotData contains tuples of slots for non-slot nodes
        data = ([(1, 2), (1, 2, 3)], 3, 5)

        oslots = OslotsFeature(mock_api, metaData, data)

        assert oslots.api is mock_api
        assert oslots.meta == metaData
        assert oslots.maxSlot == 3
        assert oslots.maxNode == 5

    def test_data_attribute(self):
        """OslotsFeature should store slot data for non-slot nodes."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        metaData = {}
        # Non-slot node 4 contains slots (1, 2), node 5 contains (1, 2, 3)
        slot_data = [(1, 2), (1, 2, 3)]
        data = (slot_data, 3, 5)

        oslots = OslotsFeature(mock_api, metaData, data)

        assert oslots.data == slot_data


class TestOslotsS:
    """Tests for s() method - get slots of a node."""

    def test_s_node_zero(self):
        """s(0) should return empty tuple."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        data = ([(1, 2)], 3, 4)

        oslots = OslotsFeature(mock_api, {}, data)

        assert oslots.s(0) == ()

    def test_s_slot_node(self):
        """s() for slot node should return tuple containing just that node."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        # maxSlot = 3, so nodes 1, 2, 3 are slots
        data = ([(1, 2)], 3, 4)

        oslots = OslotsFeature(mock_api, {}, data)

        assert oslots.s(1) == (1,)
        assert oslots.s(2) == (2,)
        assert oslots.s(3) == (3,)

    def test_s_non_slot_node(self):
        """s() for non-slot node should return its contained slots."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        # maxSlot = 3, node 4 contains (1, 2), node 5 contains (1, 2, 3)
        data = ([(1, 2), (1, 2, 3)], 3, 5)

        oslots = OslotsFeature(mock_api, {}, data)

        assert oslots.s(4) == (1, 2)
        assert oslots.s(5) == (1, 2, 3)

    def test_s_out_of_range_node(self):
        """s() should return empty tuple for out-of-range nodes."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        data = ([(1, 2)], 3, 4)

        oslots = OslotsFeature(mock_api, {}, data)

        assert oslots.s(100) == ()

    def test_s_boundary_node(self):
        """s() should handle boundary between slots and non-slots."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        data = ([(1, 2, 3)], 3, 4)

        oslots = OslotsFeature(mock_api, {}, data)

        # Node 3 is last slot - should return itself
        assert oslots.s(3) == (3,)
        # Node 4 is first non-slot - should return its slots
        assert oslots.s(4) == (1, 2, 3)

    def test_s_large_slot_set(self):
        """s() should handle nodes with many slots."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        # Node 11 contains slots 1-10
        large_slot_set = tuple(range(1, 11))
        data = ([large_slot_set], 10, 11)

        oslots = OslotsFeature(mock_api, {}, data)

        assert oslots.s(11) == large_slot_set
        assert len(oslots.s(11)) == 10


class TestOslotsItems:
    """Tests for items() method."""

    def test_items_yields_non_slot_nodes(self):
        """items() should yield (node, slots) for non-slot nodes only."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        # 3 slots, 2 non-slots (4, 5)
        data = ([(1, 2), (2, 3)], 3, 5)

        oslots = OslotsFeature(mock_api, {}, data)

        result = list(oslots.items())

        expected = [
            (4, (1, 2)),
            (5, (2, 3)),
        ]
        assert result == expected

    def test_items_empty_non_slots(self):
        """items() should yield nothing if no non-slot nodes."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        # 3 slots, no non-slots
        data = ([], 3, 3)

        oslots = OslotsFeature(mock_api, {}, data)

        result = list(oslots.items())

        assert result == []

    def test_items_is_generator(self):
        """items() should return a generator."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        data = ([(1, 2)], 3, 4)

        oslots = OslotsFeature(mock_api, {}, data)

        result = oslots.items()

        # Should be a generator, not a list
        assert hasattr(result, "__next__")

    def test_items_single_non_slot(self):
        """items() should handle single non-slot node."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        data = ([(1, 2, 3)], 3, 4)

        oslots = OslotsFeature(mock_api, {}, data)

        result = list(oslots.items())

        assert result == [(4, (1, 2, 3))]

    def test_items_multiple_non_slots(self):
        """items() should yield all non-slot nodes in order."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        # 5 slots, 3 non-slots (6, 7, 8)
        data = ([(1, 2), (3, 4), (1, 2, 3, 4, 5)], 5, 8)

        oslots = OslotsFeature(mock_api, {}, data)

        result = list(oslots.items())

        assert len(result) == 3
        assert result[0] == (6, (1, 2))
        assert result[1] == (7, (3, 4))
        assert result[2] == (8, (1, 2, 3, 4, 5))


class TestOslotsMetadata:
    """Tests for metadata access."""

    def test_meta_attribute(self):
        """meta attribute should contain feature metadata."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        metaData = {
            "description": "slot containment",
            "valueType": "int",
            "edgeValues": True,
        }
        data = ([(1, 2)], 3, 4)

        oslots = OslotsFeature(mock_api, metaData, data)

        assert oslots.meta == metaData
        assert oslots.meta["description"] == "slot containment"

    def test_max_slot_attribute(self):
        """maxSlot should report correct value."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        data = ([(1,)], 100, 150)

        oslots = OslotsFeature(mock_api, {}, data)

        assert oslots.maxSlot == 100

    def test_max_node_attribute(self):
        """maxNode should report correct value."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        data = ([(1,)], 100, 150)

        oslots = OslotsFeature(mock_api, {}, data)

        assert oslots.maxNode == 150


class TestOslotsEdgeCases:
    """Tests for edge cases and boundary conditions."""

    def test_empty_slot_data(self):
        """Should handle empty slot data tuple."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        # Node with no slots (shouldn't happen but testing)
        data = ([()], 3, 4)

        oslots = OslotsFeature(mock_api, {}, data)

        assert oslots.s(4) == ()

    def test_single_slot_per_node(self):
        """Should handle nodes containing single slot."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        data = ([(1,), (2,), (3,)], 3, 6)

        oslots = OslotsFeature(mock_api, {}, data)

        assert oslots.s(4) == (1,)
        assert oslots.s(5) == (2,)
        assert oslots.s(6) == (3,)

    def test_overlapping_slot_sets(self):
        """Should handle overlapping slot sets between nodes."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        # Node 4 and 5 both contain slots 1, 2
        data = ([(1, 2), (1, 2, 3)], 3, 5)

        oslots = OslotsFeature(mock_api, {}, data)

        assert oslots.s(4) == (1, 2)
        assert oslots.s(5) == (1, 2, 3)

    def test_negative_node(self):
        """Negative nodes are treated as slot nodes (n < maxSlot + 1)."""
        from core.oslotsfeature import OslotsFeature

        mock_api = MagicMock()
        data = ([(1, 2)], 3, 4)

        oslots = OslotsFeature(mock_api, {}, data)

        # Negative numbers are less than maxSlot + 1, so treated as slots
        assert oslots.s(-1) == (-1,)
