"""Unit tests for cfabric.search.cancel."""

import threading
import time

import pytest
from cfabric.search.cancel import CancelToken, SearchCancelled, SearchTimeout


class TestCancelToken:
    def test_fresh_token_does_not_raise(self):
        token = CancelToken()
        token.check()
        assert not token.cancelled
        assert not token.expired

    def test_cancel_raises_search_cancelled(self):
        token = CancelToken()
        token.cancel()
        assert token.cancelled
        with pytest.raises(SearchCancelled):
            token.check()

    def test_cancel_is_not_a_timeout(self):
        token = CancelToken(timeout=60)
        token.cancel()
        with pytest.raises(SearchCancelled) as info:
            token.check()
        assert not isinstance(info.value, SearchTimeout)

    def test_zero_timeout_raises_search_timeout(self):
        token = CancelToken(timeout=0)
        time.sleep(0.001)
        assert token.expired
        with pytest.raises(SearchTimeout):
            token.check()

    def test_timeout_is_a_cancellation(self):
        assert issubclass(SearchTimeout, SearchCancelled)

    def test_negative_timeout_counts_as_zero(self):
        token = CancelToken(timeout=-5)
        time.sleep(0.001)
        with pytest.raises(SearchTimeout):
            token.check()

    def test_cancel_from_other_thread(self):
        token = CancelToken()
        thread = threading.Thread(target=token.cancel)
        thread.start()
        thread.join()
        with pytest.raises(SearchCancelled):
            token.check()
