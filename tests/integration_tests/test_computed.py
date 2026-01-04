"""Integration tests for Computed data (C).

Tests pre-computed data structures with real TF data.
"""

import pytest


class TestComputedLevels:
    """Tests for C.levels computed data."""

    def test_levels_data_exists(self, loaded_api):
        """C.levels.data should exist and be non-empty."""
        C = loaded_api.C

        assert hasattr(C, "levels")
        assert hasattr(C.levels, "data")
        assert len(C.levels.data) > 0

    def test_levels_contains_types(self, loaded_api):
        """C.levels should contain all node types."""
        C = loaded_api.C

        types = [level[0] for level in C.levels.data]

        assert "word" in types
        assert "phrase" in types
        assert "sentence" in types

    def test_levels_order(self, loaded_api):
        """C.levels should be ordered from most to least encompassing."""
        C = loaded_api.C

        types = [level[0] for level in C.levels.data]

        # Sentence should come before phrase, phrase before word
        sentence_idx = types.index("sentence")
        phrase_idx = types.index("phrase")
        word_idx = types.index("word")

        assert sentence_idx < phrase_idx < word_idx


class TestComputedOrder:
    """Tests for C.order computed data."""

    def test_order_data_exists(self, loaded_api):
        """C.order.data should exist."""
        C = loaded_api.C

        assert hasattr(C, "order")
        assert hasattr(C.order, "data")

    def test_order_contains_all_nodes(self, loaded_api):
        """C.order should contain all nodes."""
        C = loaded_api.C

        order_data = C.order.data

        # Should have 8 nodes
        assert len(order_data) == 8

    def test_order_is_permutation(self, loaded_api):
        """C.order should be a permutation of node IDs."""
        C = loaded_api.C

        order_data = list(C.order.data)

        assert set(order_data) == {1, 2, 3, 4, 5, 6, 7, 8}


class TestComputedRank:
    """Tests for C.rank computed data."""

    def test_rank_data_exists(self, loaded_api):
        """C.rank.data should exist."""
        C = loaded_api.C

        assert hasattr(C, "rank")
        assert hasattr(C.rank, "data")

    def test_rank_length_matches_nodes(self, loaded_api):
        """C.rank should have entry for each node."""
        C = loaded_api.C

        rank_data = C.rank.data

        # Should have 8 entries (nodes 1-8)
        assert len(rank_data) == 8

    def test_rank_is_inverse_of_order(self, loaded_api):
        """C.rank should be inverse mapping of C.order."""
        C = loaded_api.C

        order = C.order.data
        rank = C.rank.data

        # rank[order[i]] should give i (0-indexed)
        for i, node in enumerate(order):
            assert rank[node - 1] == i


class TestComputedBoundary:
    """Tests for C.boundary computed data."""

    def test_boundary_data_exists(self, loaded_api):
        """C.boundary.data should exist."""
        C = loaded_api.C

        assert hasattr(C, "boundary")
        assert hasattr(C.boundary, "data")


class TestComputedConsistency:
    """Tests for consistency between computed data."""

    def test_order_rank_consistency(self, loaded_api):
        """Order and rank should be consistent with each other."""
        C = loaded_api.C

        order = C.order.data
        rank = C.rank.data

        # Walking through order, ranks should be sequential
        prev_rank = -1
        for node in order:
            node_rank = rank[node - 1]
            assert node_rank > prev_rank
            prev_rank = node_rank

    def test_levels_types_exist(self, loaded_api):
        """Levels types should exist in the corpus."""
        C = loaded_api.C
        F = loaded_api.F

        # Check that all types mentioned in levels exist
        for level in C.levels.data:
            node_type = level[0]

            # Type should return some nodes via otype.s()
            nodes = F.otype.s(node_type)
            assert len(nodes) > 0, f"Type {node_type} should have nodes"
