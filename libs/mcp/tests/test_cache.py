"""Unit tests for search result caching."""

import time
import threading
from unittest.mock import MagicMock

import pytest

from cfabric_mcp.cache import SearchCache, CachedSearchResult, get_cache, reset_cache


class TestCachedSearchResult:
    """Tests for CachedSearchResult dataclass."""

    def test_is_expired_false_when_fresh(self):
        """Should not be expired when within TTL."""
        result = CachedSearchResult(
            cursor_id="test-id",
            corpus="test",
            template="word",
            results=[(1,), (2,)],
            total_count=2,
            created_at=time.time(),
            expires_at=time.time() + 300,  # 5 minutes from now
        )
        assert result.is_expired is False

    def test_is_expired_true_when_past_ttl(self):
        """Should be expired when past TTL."""
        result = CachedSearchResult(
            cursor_id="test-id",
            corpus="test",
            template="word",
            results=[(1,), (2,)],
            total_count=2,
            created_at=time.time() - 400,
            expires_at=time.time() - 100,  # Expired 100 seconds ago
        )
        assert result.is_expired is True

    def test_touch_updates_last_accessed(self):
        """touch() should update last_accessed timestamp."""
        result = CachedSearchResult(
            cursor_id="test-id",
            corpus="test",
            template="word",
            results=[(1,)],
            total_count=1,
            created_at=time.time() - 100,
            expires_at=time.time() + 200,
            last_accessed=time.time() - 100,
        )
        old_accessed = result.last_accessed
        time.sleep(0.01)
        result.touch()
        assert result.last_accessed > old_accessed


class TestSearchCache:
    """Tests for SearchCache class."""

    def test_get_or_execute_caches_result(self):
        """Should cache result on first call."""
        cache = SearchCache()
        call_count = 0

        def search_fn():
            nonlocal call_count
            call_count += 1
            return [(1,), (2,), (3,)]

        # First call - executes search
        result1 = cache.get_or_execute("corpus", "word", search_fn)
        assert call_count == 1
        assert len(result1.results) == 3

        # Second call - uses cache
        result2 = cache.get_or_execute("corpus", "word", search_fn)
        assert call_count == 1  # Not incremented
        assert result2.cursor_id == result1.cursor_id

    def test_get_or_execute_respects_ttl(self):
        """Should re-execute after TTL expires."""
        cache = SearchCache(default_ttl=0)  # Immediate expiration
        call_count = 0

        def search_fn():
            nonlocal call_count
            call_count += 1
            return [(1,)]

        # First call
        cache.get_or_execute("corpus", "word", search_fn)
        assert call_count == 1

        # Wait for expiration
        time.sleep(0.01)

        # Second call - should re-execute due to expiration
        cache.get_or_execute("corpus", "word", search_fn)
        assert call_count == 2

    def test_get_or_execute_different_templates(self):
        """Different templates should have separate cache entries."""
        cache = SearchCache()

        result1 = cache.get_or_execute("corpus", "word", lambda: [(1,)])
        result2 = cache.get_or_execute("corpus", "phrase", lambda: [(2,)])

        assert result1.cursor_id != result2.cursor_id
        assert result1.results == [(1,)]
        assert result2.results == [(2,)]

    def test_get_or_execute_different_corpora(self):
        """Different corpora should have separate cache entries."""
        cache = SearchCache()

        result1 = cache.get_or_execute("corpus1", "word", lambda: [(1,)])
        result2 = cache.get_or_execute("corpus2", "word", lambda: [(2,)])

        assert result1.cursor_id != result2.cursor_id

    def test_get_or_execute_strips_template(self):
        """Should normalize template by stripping whitespace."""
        cache = SearchCache()

        result1 = cache.get_or_execute("corpus", "word", lambda: [(1,)])
        result2 = cache.get_or_execute("corpus", "  word  ", lambda: [(2,)])

        # Should use same cache entry
        assert result1.cursor_id == result2.cursor_id

    def test_get_by_cursor_returns_cached(self):
        """Should retrieve cached result by cursor ID."""
        cache = SearchCache()
        cached = cache.get_or_execute("corpus", "word", lambda: [(1,), (2,)])

        result = cache.get_by_cursor(cached.cursor_id)
        assert result is not None
        assert result.cursor_id == cached.cursor_id

    def test_get_by_cursor_returns_none_for_invalid(self):
        """Should return None for invalid cursor ID."""
        cache = SearchCache()
        result = cache.get_by_cursor("nonexistent-id")
        assert result is None

    def test_get_by_cursor_returns_none_for_expired(self):
        """Should return None for expired cursor."""
        cache = SearchCache(default_ttl=0)
        cached = cache.get_or_execute("corpus", "word", lambda: [(1,)])

        time.sleep(0.01)
        result = cache.get_by_cursor(cached.cursor_id)
        assert result is None

    def test_get_page_returns_slice(self):
        """Should return correct page of results."""
        cache = SearchCache()
        cached = cache.get_or_execute("corpus", "word", lambda: [(1,), (2,), (3,), (4,), (5,)])

        page, has_more, total = cache.get_page(cached.cursor_id, offset=1, limit=2)

        assert page == [(2,), (3,)]
        assert has_more is True
        assert total == 5

    def test_get_page_last_page(self):
        """Should report has_more=False on last page."""
        cache = SearchCache()
        cached = cache.get_or_execute("corpus", "word", lambda: [(1,), (2,), (3,)])

        page, has_more, total = cache.get_page(cached.cursor_id, offset=2, limit=10)

        assert page == [(3,)]
        assert has_more is False
        assert total == 3

    def test_get_page_returns_none_for_invalid_cursor(self):
        """Should return None for invalid cursor."""
        cache = SearchCache()
        result = cache.get_page("nonexistent", offset=0, limit=10)
        assert result is None

    def test_max_entries_eviction(self):
        """Should evict LRU entries when at max capacity."""
        cache = SearchCache(max_entries=2)

        # Add 3 entries to cache with max_entries=2
        cache.get_or_execute("c", "t1", lambda: [(1,)])
        time.sleep(0.01)
        cache.get_or_execute("c", "t2", lambda: [(2,)])
        time.sleep(0.01)
        cache.get_or_execute("c", "t3", lambda: [(3,)])

        # Only 2 entries should remain
        stats = cache.stats()
        assert stats["total_entries"] == 2

    def test_max_results_per_entry_truncation(self):
        """Should truncate results exceeding max_results_per_entry."""
        cache = SearchCache(max_results_per_entry=3)

        cached = cache.get_or_execute(
            "corpus", "word",
            lambda: [(1,), (2,), (3,), (4,), (5,)]
        )

        assert len(cached.results) == 3

    def test_cleanup_expired_removes_old_entries(self):
        """cleanup_expired should remove expired entries."""
        cache = SearchCache(default_ttl=1)  # 1 second TTL

        cache.get_or_execute("c", "t1", lambda: [(1,)])
        cache.get_or_execute("c", "t2", lambda: [(2,)])

        # Both entries exist before expiration
        assert cache.stats()["total_entries"] == 2

        # Wait for expiration
        time.sleep(1.1)

        removed = cache.cleanup_expired()
        assert removed == 2
        assert cache.stats()["total_entries"] == 0

    def test_clear_removes_all_entries(self):
        """clear() should remove all entries."""
        cache = SearchCache()

        cache.get_or_execute("c", "t1", lambda: [(1,)])
        cache.get_or_execute("c", "t2", lambda: [(2,)])

        cache.clear()

        assert cache.stats()["total_entries"] == 0

    def test_stats_returns_correct_info(self):
        """stats() should return accurate statistics."""
        cache = SearchCache(max_entries=100)

        cache.get_or_execute("c", "t1", lambda: [(1,), (2,)])
        cache.get_or_execute("c", "t2", lambda: [(3,)])

        stats = cache.stats()

        assert stats["total_entries"] == 2
        assert stats["max_entries"] == 100
        assert stats["total_results_cached"] == 3
        assert stats["expired_entries"] == 0

    def test_thread_safety(self):
        """Cache should be thread-safe for concurrent access."""
        cache = SearchCache()
        results = []
        errors = []

        def worker(template_id):
            try:
                for _ in range(10):
                    cached = cache.get_or_execute(
                        "corpus",
                        f"template_{template_id}",
                        lambda: [(template_id,)]
                    )
                    results.append(cached.cursor_id)
            except Exception as e:
                errors.append(e)

        threads = [threading.Thread(target=worker, args=(i,)) for i in range(5)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()

        assert len(errors) == 0
        assert len(results) == 50


class TestGlobalCache:
    """Tests for global cache functions."""

    def test_get_cache_returns_singleton(self):
        """get_cache should return the same instance."""
        reset_cache()
        cache1 = get_cache()
        cache2 = get_cache()
        assert cache1 is cache2

    def test_reset_cache_clears_and_resets(self):
        """reset_cache should clear and reset the global cache."""
        cache1 = get_cache()
        cache1.get_or_execute("c", "t", lambda: [(1,)])

        reset_cache()

        cache2 = get_cache()
        assert cache1 is not cache2
        assert cache2.stats()["total_entries"] == 0
