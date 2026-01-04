"""Unit tests for core.nodes module.

This module tests the Nodes class that handles canonical node ordering
and node traversal operations.
"""

import pytest
from unittest.mock import MagicMock


class TestNodesInit:
    """Tests for Nodes class initialization."""

    def test_nodes_creation(self):
        """Nodes should initialize with an API object."""
        from cfabric.core.nodes import Nodes

        # Create mock API with required C (computed) attributes
        mock_api = MagicMock()
        mock_api.C.rank.data = [0, 1, 2, 3, 4]  # rank data for nodes 1-5
        mock_api.C.levels.data = [
            ("sentence", 4, 5, 3.0),
            ("phrase", 3, 3, 2.0),
            ("word", 1, 2, 1.0),
        ]

        nodes = Nodes(mock_api)

        assert nodes.api is mock_api

    def test_otype_rank_created(self):
        """Nodes should create otypeRank dictionary."""
        from cfabric.core.nodes import Nodes

        mock_api = MagicMock()
        mock_api.C.rank.data = [0, 1, 2, 3, 4]
        mock_api.C.levels.data = [
            ("sentence", 4, 5, 3.0),
            ("phrase", 3, 3, 2.0),
            ("word", 1, 2, 1.0),
        ]

        nodes = Nodes(mock_api)

        # Reversed order: word=0, phrase=1, sentence=2
        assert nodes.otypeRank["word"] == 0
        assert nodes.otypeRank["phrase"] == 1
        assert nodes.otypeRank["sentence"] == 2

    def test_sort_key_function(self):
        """Nodes should have sortKey function."""
        from cfabric.core.nodes import Nodes

        mock_api = MagicMock()
        mock_api.C.rank.data = [10, 20, 30]  # ranks for nodes 1, 2, 3
        mock_api.C.levels.data = [("word", 1, 3, 1.0)]

        nodes = Nodes(mock_api)

        # sortKey should return rank for node n (using Crank[n-1])
        assert nodes.sortKey(1) == 10
        assert nodes.sortKey(2) == 20
        assert nodes.sortKey(3) == 30

    def test_sort_key_tuple_function(self):
        """Nodes should have sortKeyTuple function for tuple sorting."""
        from cfabric.core.nodes import Nodes

        mock_api = MagicMock()
        mock_api.C.rank.data = [10, 20, 30]
        mock_api.C.levels.data = [("word", 1, 3, 1.0)]

        nodes = Nodes(mock_api)

        result = nodes.sortKeyTuple((1, 2, 3))

        assert result == (10, 20, 30)


class TestSortNodes:
    """Tests for sortNodes method."""

    def test_sort_empty_set(self):
        """sortNodes should handle empty input."""
        from cfabric.core.nodes import Nodes

        mock_api = MagicMock()
        mock_api.C.rank.data = [0, 1, 2]
        mock_api.C.levels.data = [("word", 1, 3, 1.0)]

        nodes = Nodes(mock_api)

        result = nodes.sortNodes([])

        assert result == []

    def test_sort_single_node(self):
        """sortNodes should handle single node."""
        from cfabric.core.nodes import Nodes

        mock_api = MagicMock()
        mock_api.C.rank.data = [0, 1, 2]
        mock_api.C.levels.data = [("word", 1, 3, 1.0)]

        nodes = Nodes(mock_api)

        result = nodes.sortNodes([2])

        assert result == [2]

    def test_sort_multiple_nodes(self):
        """sortNodes should sort nodes by canonical order."""
        from cfabric.core.nodes import Nodes

        mock_api = MagicMock()
        # Ranks: node 1 has rank 30, node 2 has rank 10, node 3 has rank 20
        mock_api.C.rank.data = [30, 10, 20]
        mock_api.C.levels.data = [("word", 1, 3, 1.0)]

        nodes = Nodes(mock_api)

        # Input in arbitrary order
        result = nodes.sortNodes([1, 3, 2])

        # Should be sorted by rank: node 2 (rank 10), node 3 (rank 20), node 1 (rank 30)
        assert result == [2, 3, 1]

    def test_sort_preserves_all_nodes(self):
        """sortNodes should preserve all input nodes."""
        from cfabric.core.nodes import Nodes

        mock_api = MagicMock()
        mock_api.C.rank.data = [0, 1, 2, 3, 4]
        mock_api.C.levels.data = [("word", 1, 5, 1.0)]

        nodes = Nodes(mock_api)

        input_nodes = [5, 3, 1, 4, 2]
        result = nodes.sortNodes(input_nodes)

        assert set(result) == set(input_nodes)
        assert len(result) == len(input_nodes)

    def test_sort_accepts_set(self):
        """sortNodes should accept a set as input."""
        from cfabric.core.nodes import Nodes

        mock_api = MagicMock()
        mock_api.C.rank.data = [0, 1, 2]
        mock_api.C.levels.data = [("word", 1, 3, 1.0)]

        nodes = Nodes(mock_api)

        result = nodes.sortNodes({3, 1, 2})

        assert len(result) == 3


class TestWalk:
    """Tests for walk method."""

    def test_walk_all_nodes(self):
        """walk() without args should yield all nodes in order."""
        from cfabric.core.nodes import Nodes

        mock_api = MagicMock()
        mock_api.C.rank.data = [0, 1, 2]
        mock_api.C.order.data = [1, 2, 3]  # canonical order
        mock_api.C.levels.data = [("word", 1, 3, 1.0)]

        nodes = Nodes(mock_api)

        result = list(nodes.walk())

        assert result == [1, 2, 3]

    def test_walk_subset_of_nodes(self):
        """walk(nodes) should yield given nodes in canonical order."""
        from cfabric.core.nodes import Nodes

        mock_api = MagicMock()
        # Ranks: node 1=30, node 2=10, node 3=20
        mock_api.C.rank.data = [30, 10, 20]
        mock_api.C.levels.data = [("word", 1, 3, 1.0)]

        nodes = Nodes(mock_api)

        # Walk subset in canonical order
        result = list(nodes.walk(nodes=[3, 1]))

        # Should be sorted: node 3 (rank 20) before node 1 (rank 30)
        assert result == [3, 1]

    def test_walk_with_events_slots(self):
        """walk(events=True) should yield (node, None) for slots."""
        from cfabric.core.nodes import Nodes

        mock_api = MagicMock()
        mock_api.C.rank.data = [0, 1, 2]
        mock_api.C.order.data = [1, 2, 3]
        mock_api.C.boundary.data = [[], [[], [], []]]  # endSlots for slots 1,2,3
        mock_api.C.levels.data = [("word", 1, 3, 1.0)]

        # F.otype for determining slot type
        mock_otype = MagicMock()
        mock_otype.v.return_value = "word"
        mock_otype.slotType = "word"
        mock_api.F.otype = mock_otype

        nodes = Nodes(mock_api)

        result = list(nodes.walk(events=True))

        # All are slots, so (node, None) for each
        assert (1, None) in result
        assert (2, None) in result
        assert (3, None) in result


class TestSortKeyChunk:
    """Tests for sortKeyChunk function."""

    def test_sort_key_chunk_exists(self):
        """Nodes should have sortKeyChunk function."""
        from cfabric.core.nodes import Nodes

        mock_api = MagicMock()
        mock_api.C.rank.data = [0, 1, 2]
        mock_api.C.levels.data = [("word", 1, 3, 1.0)]
        mock_api.F.otype = MagicMock()
        mock_api.F.otype.v.return_value = "word"

        nodes = Nodes(mock_api)

        assert nodes.sortKeyChunk is not None
        assert callable(nodes.sortKeyChunk)

    def test_sort_key_chunk_length_exists(self):
        """Nodes should have sortKeyChunkLength function."""
        from cfabric.core.nodes import Nodes

        mock_api = MagicMock()
        mock_api.C.rank.data = [0, 1, 2]
        mock_api.C.levels.data = [("word", 1, 3, 1.0)]
        mock_api.F.otype = MagicMock()
        mock_api.F.otype.v.return_value = "word"

        nodes = Nodes(mock_api)

        assert nodes.sortKeyChunkLength is not None
        assert callable(nodes.sortKeyChunkLength)


class TestOtypeRank:
    """Tests for otypeRank dictionary."""

    def test_otype_rank_empty_levels(self):
        """otypeRank should be empty if no levels defined."""
        from cfabric.core.nodes import Nodes

        mock_api = MagicMock()
        mock_api.C.rank.data = []
        mock_api.C.levels.data = []

        nodes = Nodes(mock_api)

        assert nodes.otypeRank == {}

    def test_otype_rank_single_type(self):
        """otypeRank should handle single type."""
        from cfabric.core.nodes import Nodes

        mock_api = MagicMock()
        mock_api.C.rank.data = [0, 1, 2]
        mock_api.C.levels.data = [("word", 1, 3, 1.0)]

        nodes = Nodes(mock_api)

        assert nodes.otypeRank == {"word": 0}

    def test_otype_rank_multiple_types(self):
        """otypeRank should rank types from slot to most encompassing."""
        from cfabric.core.nodes import Nodes

        mock_api = MagicMock()
        mock_api.C.rank.data = [0, 1, 2, 3, 4, 5]
        # Levels ordered from most encompassing to least
        mock_api.C.levels.data = [
            ("book", 6, 6, 100.0),
            ("chapter", 5, 5, 50.0),
            ("sentence", 4, 4, 10.0),
            ("word", 1, 3, 1.0),
        ]

        nodes = Nodes(mock_api)

        # After reversing: word=0, sentence=1, chapter=2, book=3
        assert nodes.otypeRank["word"] == 0
        assert nodes.otypeRank["sentence"] == 1
        assert nodes.otypeRank["chapter"] == 2
        assert nodes.otypeRank["book"] == 3
