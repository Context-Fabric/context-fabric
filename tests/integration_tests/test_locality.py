"""Integration tests for Locality navigation (L).

Tests navigation methods (u, d, n, p) with real TF data.
"""

import pytest


class TestLocalityUp:
    """Tests for upward navigation L.u()."""

    def test_u_word_returns_embedders(self, loaded_api):
        """L.u(word) should return phrase and sentence containing it."""
        L = loaded_api.L

        result = L.u(1)  # First word

        # Should include phrase 6 and sentence 8
        assert 6 in result
        assert 8 in result

    def test_u_word_filter_by_phrase(self, loaded_api):
        """L.u(word, otype='phrase') should return only phrase."""
        L = loaded_api.L

        result = L.u(1, otype="phrase")

        assert result == (6,)

    def test_u_word_filter_by_sentence(self, loaded_api):
        """L.u(word, otype='sentence') should return only sentence."""
        L = loaded_api.L

        result = L.u(1, otype="sentence")

        assert result == (8,)

    def test_u_phrase_returns_sentence(self, loaded_api):
        """L.u(phrase) should return sentence containing it."""
        L = loaded_api.L

        result = L.u(6)

        assert 8 in result

    def test_u_sentence_returns_empty(self, loaded_api):
        """L.u(sentence) should return empty - no parent."""
        L = loaded_api.L

        result = L.u(8)

        assert result == ()

    def test_u_different_phrases(self, loaded_api):
        """Words in different phrases should have different embedders."""
        L = loaded_api.L

        result1 = L.u(1, otype="phrase")  # Word 1 in phrase 6
        result2 = L.u(4, otype="phrase")  # Word 4 in phrase 7

        assert result1 == (6,)
        assert result2 == (7,)


class TestLocalityDown:
    """Tests for downward navigation L.d()."""

    def test_d_sentence_returns_all(self, loaded_api):
        """L.d(sentence) should return all embedded nodes."""
        L = loaded_api.L

        result = L.d(8)

        # Should include all words and phrases
        assert set(result) == {1, 2, 3, 4, 5, 6, 7}

    def test_d_sentence_filter_words(self, loaded_api):
        """L.d(sentence, otype='word') should return only words."""
        L = loaded_api.L

        result = L.d(8, otype="word")

        assert set(result) == {1, 2, 3, 4, 5}

    def test_d_sentence_filter_phrases(self, loaded_api):
        """L.d(sentence, otype='phrase') should return only phrases."""
        L = loaded_api.L

        result = L.d(8, otype="phrase")

        assert set(result) == {6, 7}

    def test_d_phrase_returns_words(self, loaded_api):
        """L.d(phrase) should return words in that phrase."""
        L = loaded_api.L

        result = L.d(6)
        assert set(result) == {1, 2, 3}

        result = L.d(7)
        assert set(result) == {4, 5}

    def test_d_word_returns_empty(self, loaded_api):
        """L.d(word) should return empty - words have no children."""
        L = loaded_api.L

        result = L.d(1)

        assert result == ()


class TestLocalityNext:
    """Tests for next navigation L.n()."""

    def test_n_word_returns_next(self, loaded_api):
        """L.n(word) should return next nodes."""
        L = loaded_api.L

        result = L.n(1)

        # Next word should be 2
        assert 2 in result

    def test_n_last_word_empty(self, loaded_api):
        """L.n(last_word) should return empty for same type."""
        L = loaded_api.L

        result = L.n(5, otype="word")

        assert result == ()

    def test_n_phrase_returns_next_phrase(self, loaded_api):
        """L.n(phrase) should return next phrase."""
        L = loaded_api.L

        result = L.n(6, otype="phrase")

        assert result == (7,)


class TestLocalityPrev:
    """Tests for previous navigation L.p()."""

    def test_p_word_returns_prev(self, loaded_api):
        """L.p(word) should return previous nodes."""
        L = loaded_api.L

        result = L.p(5)

        # Previous word should be 4
        assert 4 in result

    def test_p_first_word_empty(self, loaded_api):
        """L.p(first_word) should return empty for same type."""
        L = loaded_api.L

        result = L.p(1, otype="word")

        assert result == ()

    def test_p_phrase_returns_prev_phrase(self, loaded_api):
        """L.p(phrase) should return previous phrase."""
        L = loaded_api.L

        result = L.p(7, otype="phrase")

        assert result == (6,)


class TestLocalityTypeFiltering:
    """Tests for otype filtering in locality methods."""

    def test_u_with_set_filter(self, loaded_api):
        """L.u() should accept set of types as filter."""
        L = loaded_api.L

        result = L.u(1, otype={"phrase", "sentence"})

        assert 6 in result
        assert 8 in result

    def test_d_with_set_filter(self, loaded_api):
        """L.d() should accept set of types as filter."""
        L = loaded_api.L

        result = L.d(8, otype={"word"})

        assert set(result) == {1, 2, 3, 4, 5}
