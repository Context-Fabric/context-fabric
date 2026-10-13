"""Integration tests for cancelling searches and search timeouts."""

import pytest
from cfabric.search import CancelToken, SearchCancelled, SearchTimeout

TEMPLATE = """
phrase
  word
"""


class TestSearchCancellation:
    def test_uncancelled_token_gives_all_results(self, loaded_api):
        results = loaded_api.S.search(TEMPLATE, cancel=CancelToken())
        assert len(list(results)) == 5

    def test_cancelled_before_search(self, loaded_api):
        token = CancelToken()
        token.cancel()
        with pytest.raises(SearchCancelled):
            loaded_api.S.search(TEMPLATE, cancel=token)

    def test_timeout_shorthand(self, loaded_api):
        with pytest.raises(SearchTimeout):
            loaded_api.S.search(TEMPLATE, timeout=0)

    def test_cancel_takes_precedence_over_timeout(self, loaded_api):
        results = loaded_api.S.search(TEMPLATE, cancel=CancelToken(), timeout=0)
        assert len(list(results)) == 5

    def test_cancel_while_iterating(self, loaded_api):
        token = CancelToken()
        results = loaded_api.S.search(TEMPLATE, cancel=token)
        delivered = []
        with pytest.raises(SearchCancelled):
            for result in results:
                delivered.append(result)
                token.cancel()
        assert 1 <= len(delivered) < 5

    def test_cancel_with_limit(self, loaded_api):
        token = CancelToken()
        token.cancel()
        with pytest.raises(SearchCancelled):
            loaded_api.S.search(TEMPLATE, limit=2, cancel=token)

    def test_cancel_shallow_search(self, loaded_api):
        token = CancelToken()
        token.cancel()
        with pytest.raises(SearchCancelled):
            loaded_api.S.search(TEMPLATE, shallow=True, cancel=token)

    def test_cancel_reaches_quantifiers(self, loaded_api):
        template = """
phrase
/without/
  word word=nothing
/-/
"""
        assert len(list(loaded_api.S.search(template))) == 2
        token = CancelToken()
        token.cancel()
        with pytest.raises(SearchCancelled):
            loaded_api.S.search(template, cancel=token)

    def test_study_with_cancel(self, loaded_api):
        token = CancelToken()
        token.cancel()
        with pytest.raises(SearchCancelled):
            loaded_api.S.study(TEMPLATE, cancel=token)

    def test_study_then_fetch_is_stopped_by_token(self, loaded_api):
        token = CancelToken()
        loaded_api.S.study(TEMPLATE, cancel=token)
        token.cancel()
        with pytest.raises(SearchCancelled):
            list(loaded_api.S.fetch())
