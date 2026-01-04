"""Integration tests for feature access (F and E).

Tests NodeFeature and EdgeFeature access with real TF data.
"""

import pytest


class TestNodeFeatureAccess:
    """Tests for accessing node feature values."""

    def test_word_v_returns_values(self, loaded_api):
        """F.word.v(n) should return word text."""
        assert loaded_api.F.word.v(1) == "hello"
        assert loaded_api.F.word.v(2) == "beautiful"
        assert loaded_api.F.word.v(3) == "world"
        assert loaded_api.F.word.v(4) == "good"
        assert loaded_api.F.word.v(5) == "morning"

    def test_word_v_non_slot_returns_none(self, loaded_api):
        """F.word.v(n) for non-slot nodes should return None."""
        # Nodes 6, 7, 8 are phrase/sentence, not words
        assert loaded_api.F.word.v(6) is None
        assert loaded_api.F.word.v(7) is None
        assert loaded_api.F.word.v(8) is None

    def test_otype_v_slot_nodes(self, loaded_api):
        """F.otype.v(n) should return 'word' for slot nodes."""
        for n in range(1, 6):
            assert loaded_api.F.otype.v(n) == "word"

    def test_otype_v_phrase_nodes(self, loaded_api):
        """F.otype.v(n) should return 'phrase' for phrase nodes."""
        assert loaded_api.F.otype.v(6) == "phrase"
        assert loaded_api.F.otype.v(7) == "phrase"

    def test_otype_v_sentence_node(self, loaded_api):
        """F.otype.v(n) should return 'sentence' for sentence node."""
        assert loaded_api.F.otype.v(8) == "sentence"


class TestNodeFeatureSearch:
    """Tests for finding nodes by feature value."""

    def test_word_s_finds_node(self, loaded_api):
        """F.word.s(val) should return nodes with that value."""
        result = loaded_api.F.word.s("hello")
        assert 1 in result

    def test_word_s_not_found(self, loaded_api):
        """F.word.s(val) should return empty for unknown value."""
        result = loaded_api.F.word.s("nonexistent")
        assert len(result) == 0

    def test_otype_s_finds_all_words(self, loaded_api):
        """F.otype.s('word') should return all word nodes."""
        result = loaded_api.F.otype.s("word")
        assert set(result) == {1, 2, 3, 4, 5}

    def test_otype_s_finds_phrases(self, loaded_api):
        """F.otype.s('phrase') should return phrase nodes."""
        result = loaded_api.F.otype.s("phrase")
        assert set(result) == {6, 7}

    def test_otype_s_finds_sentence(self, loaded_api):
        """F.otype.s('sentence') should return sentence node."""
        result = loaded_api.F.otype.s("sentence")
        assert set(result) == {8}


class TestNodeFeatureIteration:
    """Tests for iterating over feature data."""

    def test_word_items_iterates_all(self, loaded_api):
        """F.word.items() should yield all (node, value) pairs."""
        items = list(loaded_api.F.word.items())

        # Should have 5 word values
        assert len(items) == 5

        # Check all expected pairs present
        node_values = dict(items)
        assert node_values[1] == "hello"
        assert node_values[5] == "morning"

    def test_otype_items_iterates_all(self, loaded_api):
        """F.otype.items() should yield all node types."""
        items = list(loaded_api.F.otype.items())

        # Should have 8 nodes total
        assert len(items) == 8


class TestNodeFeatureFreqList:
    """Tests for frequency analysis."""

    def test_word_freqlist(self, loaded_api):
        """F.word.freqList() should return value frequencies."""
        freq = loaded_api.F.word.freqList()

        # All words appear once
        assert len(freq) == 5
        for word, count in freq:
            assert count == 1

    def test_otype_s_query_all_types(self, loaded_api):
        """F.otype.s() should allow querying all types."""
        # OtypeFeature uses s() method instead of freqList()
        words = loaded_api.F.otype.s("word")
        phrases = loaded_api.F.otype.s("phrase")
        sentences = loaded_api.F.otype.s("sentence")

        assert len(words) == 5
        assert len(phrases) == 2
        assert len(sentences) == 1


class TestEdgeFeatureAccess:
    """Tests for accessing edge feature values."""

    def test_oslots_s_phrase(self, loaded_api):
        """E.oslots.s(n) should return slots for phrase."""
        result = loaded_api.E.oslots.s(6)
        assert list(result) == [1, 2, 3]

        result = loaded_api.E.oslots.s(7)
        assert list(result) == [4, 5]

    def test_oslots_s_sentence(self, loaded_api):
        """E.oslots.s(n) should return all slots for sentence."""
        result = loaded_api.E.oslots.s(8)
        assert list(result) == [1, 2, 3, 4, 5]

    def test_parent_f_from_word(self, loaded_api):
        """E.parent.f(n) should return parent nodes."""
        result = loaded_api.E.parent.f(1)
        assert 6 in result

        result = loaded_api.E.parent.f(4)
        assert 7 in result

    def test_parent_t_to_phrase(self, loaded_api):
        """E.parent.t(n) should return child nodes."""
        result = loaded_api.E.parent.t(6)
        assert set(result) == {1, 2, 3}

        result = loaded_api.E.parent.t(7)
        assert set(result) == {4, 5}

    def test_parent_f_from_phrase(self, loaded_api):
        """E.parent.f(phrase) should return sentence."""
        result = loaded_api.E.parent.f(6)
        assert 8 in result


class TestApiFeatureListing:
    """Tests for API feature listing methods."""

    def test_fall_lists_node_features(self, loaded_api):
        """api.Fall() should list node features."""
        features = loaded_api.Fall()

        assert "word" in features
        assert "otype" in features

    def test_eall_lists_edge_features(self, loaded_api):
        """api.Eall() should list edge features."""
        features = loaded_api.Eall()

        assert "parent" in features
        assert "oslots" in features

    def test_call_lists_computed(self, loaded_api):
        """api.Call() should list computed data."""
        computed = loaded_api.Call()

        assert "levels" in computed
        assert "order" in computed
        assert "rank" in computed
