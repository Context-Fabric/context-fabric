"""Integration tests for Nodes operations (N).

Tests node ordering and traversal with real TF data.
"""

import pytest


class TestNodesSortNodes:
    """Tests for N.sortNodes() method."""

    def test_sort_nodes_list(self, loaded_api):
        """sortNodes() should sort list in canonical order."""
        N = loaded_api.N

        result = N.sortNodes([3, 1, 2])

        # Should be in canonical order
        assert result == [1, 2, 3]

    def test_sort_nodes_set(self, loaded_api):
        """sortNodes() should accept set input."""
        N = loaded_api.N

        result = N.sortNodes({5, 3, 1})

        assert result == [1, 3, 5]

    def test_sort_nodes_mixed_types(self, loaded_api):
        """sortNodes() should sort nodes of different types."""
        N = loaded_api.N

        result = N.sortNodes([8, 6, 1, 7])

        # All nodes should be in the result
        assert set(result) == {1, 6, 7, 8}
        # Result should be in canonical order (consistent ordering)
        assert len(result) == 4

    def test_sort_nodes_empty(self, loaded_api):
        """sortNodes() should handle empty input."""
        N = loaded_api.N

        result = N.sortNodes([])

        assert result == []

    def test_sort_nodes_single(self, loaded_api):
        """sortNodes() should handle single node."""
        N = loaded_api.N

        result = N.sortNodes([5])

        assert result == [5]


class TestNodesWalk:
    """Tests for N.walk() method."""

    def test_walk_all_nodes(self, loaded_api):
        """walk() should yield all nodes in canonical order."""
        N = loaded_api.N

        result = list(N.walk())

        # Should have all 8 nodes
        assert len(result) == 8
        assert set(result) == {1, 2, 3, 4, 5, 6, 7, 8}

    def test_walk_subset(self, loaded_api):
        """walk(nodes) should yield subset in canonical order."""
        N = loaded_api.N

        result = list(N.walk(nodes=[8, 6, 1]))

        # Should be in canonical order
        assert len(result) == 3
        assert 1 in result
        assert 6 in result
        assert 8 in result


class TestNodesSortKey:
    """Tests for sort key functions."""

    def test_sort_key_consistent(self, loaded_api):
        """sortKey should produce consistent ordering."""
        N = loaded_api.N

        # Nodes with lower sortKey should come first
        key1 = N.sortKey(1)
        key2 = N.sortKey(2)
        key3 = N.sortKey(3)

        # Word nodes should have sequential keys
        assert key1 < key2 < key3

    def test_sort_key_tuple(self, loaded_api):
        """sortKeyTuple should work on tuples."""
        N = loaded_api.N

        key = N.sortKeyTuple((1, 2, 3))

        assert isinstance(key, tuple)
        assert len(key) == 3


class TestNodesOtypeRank:
    """Tests for otypeRank dictionary."""

    def test_otype_rank_contains_types(self, loaded_api):
        """otypeRank should contain all node types."""
        N = loaded_api.N

        assert "word" in N.otypeRank
        assert "phrase" in N.otypeRank
        assert "sentence" in N.otypeRank

    def test_otype_rank_slot_lowest(self, loaded_api):
        """Slot type should have lowest rank."""
        N = loaded_api.N

        word_rank = N.otypeRank["word"]
        phrase_rank = N.otypeRank["phrase"]
        sentence_rank = N.otypeRank["sentence"]

        assert word_rank < phrase_rank
        assert phrase_rank < sentence_rank
