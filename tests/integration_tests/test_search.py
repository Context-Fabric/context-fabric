"""Integration tests for Search operations (S).

Tests search query execution with real TF data.
"""

import pytest


class TestSearchBasicQueries:
    """Tests for basic search queries."""

    def test_search_type_query(self, loaded_api):
        """Search for node type should find all matching nodes."""
        S = loaded_api.S

        results = list(S.search("word"))

        # Should find all 5 words
        assert len(results) == 5

        # Each result is a tuple with one node
        nodes = [r[0] for r in results]
        assert set(nodes) == {1, 2, 3, 4, 5}

    def test_search_phrase_type(self, loaded_api):
        """Search for phrase type should find phrase nodes."""
        S = loaded_api.S

        results = list(S.search("phrase"))

        assert len(results) == 2
        nodes = [r[0] for r in results]
        assert set(nodes) == {6, 7}

    def test_search_sentence_type(self, loaded_api):
        """Search for sentence type should find sentence node."""
        S = loaded_api.S

        results = list(S.search("sentence"))

        assert len(results) == 1
        assert results[0][0] == 8


class TestSearchWithConstraints:
    """Tests for search with feature constraints."""

    def test_search_word_value(self, loaded_api):
        """Search with feature constraint should filter results."""
        S = loaded_api.S

        results = list(S.search("word word=hello"))

        assert len(results) == 1
        assert results[0][0] == 1

    def test_search_word_multiple_values(self, loaded_api):
        """Search should handle regex in constraints."""
        S = loaded_api.S

        # Search for words starting with 'h'
        results = list(S.search("word word~^h"))

        # Should find 'hello'
        assert len(results) >= 1
        nodes = [r[0] for r in results]
        assert 1 in nodes


class TestSearchEmbedding:
    """Tests for search with embedding relations."""

    def test_search_phrase_word(self, loaded_api):
        """Search for phrase containing word."""
        S = loaded_api.S

        # Search query must start at column 0 (no leading whitespace)
        query = "phrase\n  word"

        results = list(S.search(query))

        # Should find phrase-word pairs
        assert len(results) > 0

        # Each result should be (phrase, word) tuple
        for phrase, word in results:
            assert loaded_api.F.otype.v(phrase) == "phrase"
            assert loaded_api.F.otype.v(word) == "word"

    def test_search_sentence_phrase(self, loaded_api):
        """Search for sentence containing phrase."""
        S = loaded_api.S

        # Search query must start at column 0 (no leading whitespace)
        query = "sentence\n  phrase"

        results = list(S.search(query))

        # Should find sentence-phrase pairs
        assert len(results) == 2  # One sentence, two phrases


class TestSearchLimit:
    """Tests for search with limit parameter."""

    def test_search_with_limit(self, loaded_api):
        """Search with limit should restrict results."""
        S = loaded_api.S

        results = list(S.search("word", limit=2))

        assert len(results) == 2

    def test_search_limit_one(self, loaded_api):
        """Search with limit=1 should return single result."""
        S = loaded_api.S

        results = list(S.search("word", limit=1))

        assert len(results) == 1


class TestSearchStudyFetch:
    """Tests for study() and fetch() workflow."""

    def test_study_then_fetch(self, loaded_api):
        """study() followed by fetch() should work."""
        S = loaded_api.S

        S.study("word", here=True)
        results = list(S.fetch())

        assert len(results) == 5

    def test_study_with_limit_fetch(self, loaded_api):
        """fetch() with limit should restrict results."""
        S = loaded_api.S

        S.study("word", here=True)
        results = list(S.fetch(limit=3))

        assert len(results) == 3


class TestSearchEmptyResults:
    """Tests for queries with no matches."""

    def test_search_impossible_constraint(self, loaded_api):
        """Search with impossible constraint should return empty."""
        S = loaded_api.S

        results = list(S.search("word word=nonexistent"))

        assert len(results) == 0

    def test_search_unknown_type(self, loaded_api):
        """Search for unknown type should return empty."""
        S = loaded_api.S

        results = list(S.search("unknowntype"))

        assert len(results) == 0


class TestSearchNamedNodes:
    """Tests for named node binding in queries."""

    def test_named_node_single(self, loaded_api):
        """Named node should bind results to variable."""
        S = loaded_api.S

        results = list(S.search("w:word"))

        # Should find all 5 words
        assert len(results) == 5

    def test_named_nodes_multiple(self, loaded_api):
        """Multiple named nodes should create pairs."""
        S = loaded_api.S

        # Two word variables at same level (siblings)
        query = "p:phrase\n  w1:word\n  w2:word\nw1 # w2"
        results = list(S.search(query))

        # Should find pairs of different words in same phrase
        assert len(results) > 0
        for p, w1, w2 in results:
            assert w1 != w2  # Different nodes

    def test_named_node_with_constraint(self, loaded_api):
        """Named nodes should work with feature constraints."""
        S = loaded_api.S

        results = list(S.search("w:word word=hello"))

        assert len(results) == 1
        assert results[0][0] == 1  # Node 1 is "hello"


class TestSearchFeatureConstraints:
    """Tests for various feature constraint operators."""

    def test_inequality_not_equal(self, loaded_api):
        """Feature#value should match nodes where feature != value."""
        S = loaded_api.S

        results = list(S.search("word pos#noun"))

        # Should find words that are NOT nouns
        # Nouns are nodes 3 (world) and 5 (morning)
        nodes = [r[0] for r in results]
        assert 3 not in nodes
        assert 5 not in nodes
        assert len(nodes) == 3  # interjection, adjective, adjective

    def test_inequality_missing(self, loaded_api):
        """Feature# should match nodes where feature is missing."""
        S = loaded_api.S

        # Phrases and sentences don't have pos feature
        results = list(S.search("phrase pos#"))

        # All phrases should match (they have no pos)
        assert len(results) == 2

    def test_existence_has_value(self, loaded_api):
        """Feature* should match nodes that have the feature."""
        S = loaded_api.S

        results = list(S.search("word pos*"))

        # All words have pos feature
        assert len(results) == 5

    def test_numeric_less_than(self, loaded_api):
        """Feature<N should match nodes with feature < N."""
        S = loaded_api.S

        results = list(S.search("word number<3"))

        # number values: 1,2,3,1,2 - so <3 means 1,2,1,2
        assert len(results) == 4

    def test_numeric_greater_than(self, loaded_api):
        """Feature>N should match nodes with feature > N."""
        S = loaded_api.S

        results = list(S.search("word number>2"))

        # Only node 3 has number=3
        assert len(results) == 1

    def test_value_alternatives(self, loaded_api):
        """Feature=val1|val2 should match either value."""
        S = loaded_api.S

        results = list(S.search("word pos=noun|adjective"))

        # Nouns: 3, 5. Adjectives: 2, 4.
        assert len(results) == 4
        nodes = [r[0] for r in results]
        assert set(nodes) == {2, 3, 4, 5}

    def test_multiple_constraints(self, loaded_api):
        """Multiple constraints should all apply."""
        S = loaded_api.S

        results = list(S.search("word word=hello pos=interjection"))

        assert len(results) == 1
        assert results[0][0] == 1

    def test_regex_constraint(self, loaded_api):
        """Feature~regex should match regex pattern."""
        S = loaded_api.S

        # Words ending in 'ing'
        results = list(S.search("word word~ing$"))

        # "morning" ends in "ing"
        assert len(results) == 1
        assert results[0][0] == 5


class TestSearchQuantifiers:
    """Tests for quantifier blocks (/where/, /without/, etc.)."""

    def test_without_excludes(self, loaded_api):
        """Phrase /without/ word should find phrases NOT containing word."""
        S = loaded_api.S

        query = "phrase\n/without/\n  word word=hello\n/-/"
        results = list(S.search(query))

        # Phrase 7 doesn't contain "hello" (it has "good", "morning")
        assert len(results) == 1
        assert results[0][0] == 7

    def test_where_have(self, loaded_api):
        """/where/ followed by /have/ adds more conditions."""
        S = loaded_api.S

        query = "sentence\n/where/\n  word word=hello\n/have/\n  word word=world\n/-/"
        results = list(S.search(query))

        # Sentence contains both "hello" and "world"
        assert len(results) == 1

    def test_with_or_alternatives(self, loaded_api):
        """/with/ and /or/ should match alternative conditions."""
        S = loaded_api.S

        query = "phrase\n/with/\n  word word=hello\n/or/\n  word word=good\n/-/"
        results = list(S.search(query))

        # Phrase 6 has "hello", phrase 7 has "good"
        assert len(results) == 2

    def test_without_multiple(self, loaded_api):
        """Multiple /without/ conditions using /or/."""
        S = loaded_api.S

        # Find sentence that doesn't have the specific word pattern
        # Using /without/ to exclude sentences containing "nonexistent"
        query = "sentence\n/without/\n  word word=nonexistent\n/-/"
        results = list(S.search(query))

        # Sentence 8 should match (doesn't have "nonexistent")
        assert len(results) == 1
        assert results[0][0] == 8


class TestSearchRelations:
    """Tests for relation constraints between named nodes."""

    def test_ordering_before(self, loaded_api):
        """w1 < w2 should find w1 before w2 in canonical order."""
        S = loaded_api.S

        query = "w1:word\nw2:word\nw1 < w2\nw1 word=hello\nw2 word=world"
        results = list(S.search(query))

        # "hello" (1) is before "world" (3)
        assert len(results) == 1

    def test_ordering_after(self, loaded_api):
        """w1 > w2 should find w1 after w2."""
        S = loaded_api.S

        query = "w1:word\nw2:word\nw1 > w2\nw1 word=world\nw2 word=hello"
        results = list(S.search(query))

        # "world" (3) is after "hello" (1)
        assert len(results) == 1

    def test_not_equal(self, loaded_api):
        """w1 # w2 should find different nodes."""
        S = loaded_api.S

        query = "w1:word\nw2:word\nw1 # w2\nw1 pos=noun\nw2 pos=noun"
        results = list(S.search(query))

        # Two different nouns (3 and 5)
        assert len(results) == 2

    def test_embedding_left(self, loaded_api):
        """p [[ w should find phrase embedding word."""
        S = loaded_api.S

        query = "p:phrase\nw:word\np [[ w"
        results = list(S.search(query))

        # All phrase-word pairs where phrase contains word
        assert len(results) == 5  # 3 words in phrase 6, 2 in phrase 7

    def test_embedding_right(self, loaded_api):
        """w ]] p should find word embedded in phrase."""
        S = loaded_api.S

        query = "w:word\np:phrase\nw ]] p"
        results = list(S.search(query))

        # Same as above, different direction
        assert len(results) == 5

    def test_adjacency_immediate(self, loaded_api):
        """w1 <: w2 should find immediately adjacent nodes."""
        S = loaded_api.S

        query = "w1:word\nw2:word\nw1 <: w2"
        results = list(S.search(query))

        # Adjacent pairs: (1,2), (2,3), (3,4), (4,5)
        assert len(results) == 4


class TestSearchEdgeFeatures:
    """Tests for edge feature traversal in queries."""

    def test_edge_forward(self, loaded_api):
        """Node with -edge> should follow edge forward."""
        S = loaded_api.S

        # Use edge as a relation between named nodes
        query = "w:word\np:phrase\nw -parent> p"
        results = list(S.search(query))

        # All words have parent edges to phrases
        assert len(results) == 5

    def test_edge_backward(self, loaded_api):
        """Node with <edge- should follow edge backward."""
        S = loaded_api.S

        # Use edge as a relation between named nodes
        query = "p:phrase\nw:word\np <parent- w"
        results = list(S.search(query))

        # Phrases have incoming parent edges from words
        assert len(results) == 5

    def test_edge_with_constraint(self, loaded_api):
        """Edge traversal with node constraint."""
        S = loaded_api.S

        # Use edge as a relation with feature constraint
        query = "w:word word=hello\np:phrase\nw -parent> p"
        results = list(S.search(query))

        # Only "hello" with its parent
        assert len(results) == 1


class TestSearchComments:
    """Tests for comment handling in queries."""

    def test_comment_line_ignored(self, loaded_api):
        """Lines starting with % should be ignored."""
        S = loaded_api.S

        query = "% This is a comment\nword"
        results = list(S.search(query))

        assert len(results) == 5

    def test_comment_between_lines(self, loaded_api):
        """Comments between query lines should be ignored."""
        S = loaded_api.S

        query = "phrase\n% Find words in phrase\n  word"
        results = list(S.search(query))

        assert len(results) == 5


class TestSearchCustomSets:
    """Tests for custom sets parameter in search."""

    def test_custom_set_basic(self, loaded_api):
        """Search with custom set should use nodes from set."""
        S = loaded_api.S

        # Create a custom set with only nodes 1 and 3
        my_words = {1, 3}
        results = list(S.search("mywords", sets={"mywords": my_words}))

        assert len(results) == 2
        nodes = [r[0] for r in results]
        assert set(nodes) == {1, 3}

    def test_custom_set_with_constraint(self, loaded_api):
        """Custom set should work with feature constraints."""
        S = loaded_api.S
        F = loaded_api.F

        # Create set of all words
        all_words = set(F.otype.s("word"))
        results = list(S.search("w pos=noun", sets={"w": all_words}))

        # Should find nouns (nodes 3 and 5)
        assert len(results) == 2
        nodes = [r[0] for r in results]
        assert set(nodes) == {3, 5}

    def test_custom_set_empty(self, loaded_api):
        """Empty custom set should return no results."""
        S = loaded_api.S

        results = list(S.search("empty", sets={"empty": set()}))

        assert len(results) == 0

    def test_custom_set_single_node(self, loaded_api):
        """Custom set with single node should work."""
        S = loaded_api.S

        results = list(S.search("single", sets={"single": {1}}))

        assert len(results) == 1
        assert results[0][0] == 1

    def test_custom_set_in_relation(self, loaded_api):
        """Custom sets should work in relations."""
        S = loaded_api.S

        # Create sets for words and phrases
        my_words = {1, 2}
        my_phrases = {6}

        query = "w:mywords\np:myphrases\nw ]] p"
        results = list(S.search(
            query,
            sets={"mywords": my_words, "myphrases": my_phrases}
        ))

        # Words 1 and 2 are embedded in phrase 6
        assert len(results) == 2


class TestSearchGenericNodeType:
    """Tests for generic node type (.) that matches all nodes."""

    def test_generic_node_all(self, loaded_api):
        """Generic node type should match all nodes."""
        S = loaded_api.S

        results = list(S.search("."))

        # Should find all 8 nodes
        assert len(results) == 8

    def test_generic_node_with_feature(self, loaded_api):
        """Generic node type with feature constraint."""
        S = loaded_api.S

        # Only words have 'pos' feature with value 'noun'
        results = list(S.search(". pos=noun"))

        assert len(results) == 2
        nodes = [r[0] for r in results]
        assert set(nodes) == {3, 5}

    def test_generic_node_in_embedding(self, loaded_api):
        """Generic node in embedding relation."""
        S = loaded_api.S

        # Find any node that embeds words
        query = "p:.\nw:word\np [[ w"
        results = list(S.search(query))

        # Phrases and sentence embed words
        # Phrase 6 embeds 3 words, phrase 7 embeds 2, sentence 8 embeds 5
        assert len(results) >= 5
