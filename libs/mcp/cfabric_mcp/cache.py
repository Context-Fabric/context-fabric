"""Search result caching for Context-Fabric MCP server.

Caches search results by (corpus, template) to avoid re-running expensive queries.
All return_types (results, count, statistics, passages) can draw from the same cache.
"""

from __future__ import annotations

import logging
import threading
import time
import uuid
from dataclasses import dataclass, field
from typing import Any, Callable

logger = logging.getLogger("cfabric_mcp.cache")


@dataclass
class CachedSearchResult:
    """A cached search result.

    Attributes:
        cursor_id: UUID for pagination/lookup
        corpus: The corpus name
        template: The search template
        results: Raw search results (list of node ID tuples)
        total_count: Number of matches before the cache retention limit
        created_at: Unix timestamp when cached
        expires_at: Unix timestamp when cache entry expires
        last_accessed: Unix timestamp of last access (for LRU)
    """

    cursor_id: str
    corpus: str
    template: str
    results: list[tuple[int, ...]]
    total_count: int
    created_at: float
    expires_at: float
    last_accessed: float = field(default_factory=time.time)

    @property
    def is_truncated(self) -> bool:
        """Whether the retained results are only a prefix of all matches."""
        return len(self.results) < self.total_count

    @property
    def is_expired(self) -> bool:
        """Check if this cache entry has expired."""
        return time.time() > self.expires_at

    def touch(self) -> None:
        """Update last accessed time."""
        self.last_accessed = time.time()


class SearchCache:
    """In-memory cache for search results.

    Caches by (corpus, template) key. Different return_types and parameters
    can reuse the same cached results.

    Thread-safe for concurrent access.
    """

    def __init__(
        self,
        default_ttl: int = 300,  # 5 minutes
        max_entries: int = 100,
        max_results_per_entry: int = 10000,
    ) -> None:
        """Initialize the search cache.

        Args:
            default_ttl: Default time-to-live in seconds
            max_entries: Maximum number of cached searches
            max_results_per_entry: Maximum results to cache per search
        """
        self.default_ttl = default_ttl
        self.max_entries = max_entries
        self.max_results_per_entry = max_results_per_entry

        # Cache storage: key -> CachedSearchResult
        self._cache: dict[tuple[str, str], CachedSearchResult] = {}

        # Also index by cursor_id for pagination
        self._by_cursor: dict[str, CachedSearchResult] = {}

        # Lock for thread safety
        self._lock = threading.RLock()

    def get_or_execute(
        self,
        corpus: str,
        template: str,
        search_fn: Callable[[], list[tuple[int, ...]]],
        ttl: int | None = None,
    ) -> CachedSearchResult:
        """Get cached result or execute search and cache it.

        Args:
            corpus: Corpus name
            template: Search template
            search_fn: Function to execute search if not cached
            ttl: Optional TTL override

        Returns:
            CachedSearchResult with the search results
        """
        key = (corpus, template.strip())
        ttl = ttl or self.default_ttl

        with self._lock:
            # Check cache
            cached = self._cache.get(key)
            if cached and not cached.is_expired:
                logger.debug(
                    "Cache hit for template: %s...",
                    template[:50].replace("\n", " ")
                )
                cached.touch()
                return cached

            # Cache miss or expired - execute search
            logger.debug(
                "Cache miss for template: %s...",
                template[:50].replace("\n", " ")
            )

            # Run the search
            results = search_fn()

            # Sort by node-tuples for consistent ordering
            results = sorted(results)
            total_count = len(results)

            # Limit results to max_results_per_entry
            if len(results) > self.max_results_per_entry:
                logger.warning(
                    "Truncating cache entry from %d to %d results",
                    len(results),
                    self.max_results_per_entry,
                )
                results = results[: self.max_results_per_entry]

            # Create cache entry
            now = time.time()
            cursor_id = str(uuid.uuid4())
            entry = CachedSearchResult(
                cursor_id=cursor_id,
                corpus=corpus,
                template=template,
                results=results,
                total_count=total_count,
                created_at=now,
                expires_at=now + ttl,
                last_accessed=now,
            )

            # Evict old entries if needed
            self._evict_if_needed()

            # Store in cache
            self._cache[key] = entry
            self._by_cursor[cursor_id] = entry

            return entry

    def get_by_cursor(self, cursor_id: str) -> CachedSearchResult | None:
        """Get cached result by cursor ID.

        Args:
            cursor_id: The cursor ID from a previous search

        Returns:
            CachedSearchResult if found and not expired, None otherwise
        """
        with self._lock:
            entry = self._by_cursor.get(cursor_id)
            if entry and not entry.is_expired:
                entry.touch()
                return entry
            return None

    def get_page(
        self,
        cursor_id: str,
        offset: int,
        limit: int,
    ) -> tuple[list[tuple[int, ...]], bool, int] | None:
        """Get a page of results from a cached search.

        Args:
            cursor_id: The cursor ID
            offset: Number of results to skip
            limit: Maximum results to return

        Returns:
            Tuple of (results, has_more, total_count) or None if not found
        """
        entry = self.get_by_cursor(cursor_id)
        if not entry:
            return None

        page = entry.results[offset : offset + limit]
        has_more = offset + limit < len(entry.results)

        return page, has_more, entry.total_count

    def _evict_if_needed(self) -> None:
        """Evict entries if cache is at capacity.

        Uses LRU eviction strategy.
        """
        # First, remove expired entries
        self.cleanup_expired()

        # If still at capacity, evict LRU entries
        while len(self._cache) >= self.max_entries:
            # Find least recently used entry
            lru_key = None
            lru_time = float("inf")

            for key, entry in self._cache.items():
                if entry.last_accessed < lru_time:
                    lru_time = entry.last_accessed
                    lru_key = key

            if lru_key:
                entry = self._cache.pop(lru_key)
                self._by_cursor.pop(entry.cursor_id, None)
                logger.debug("Evicted LRU cache entry: %s", lru_key[1][:30])

    def cleanup_expired(self) -> int:
        """Remove all expired cache entries.

        Returns:
            Number of entries removed
        """
        with self._lock:
            expired_keys = [
                key for key, entry in self._cache.items() if entry.is_expired
            ]

            for key in expired_keys:
                entry = self._cache.pop(key)
                self._by_cursor.pop(entry.cursor_id, None)

            if expired_keys:
                logger.debug("Cleaned up %d expired cache entries", len(expired_keys))

            return len(expired_keys)

    def clear(self) -> None:
        """Clear all cache entries."""
        with self._lock:
            self._cache.clear()
            self._by_cursor.clear()
            logger.debug("Cache cleared")

    def stats(self) -> dict[str, Any]:
        """Get cache statistics.

        Returns:
            Dictionary with cache statistics
        """
        with self._lock:
            now = time.time()
            entries = list(self._cache.values())

            return {
                "total_entries": len(entries),
                "max_entries": self.max_entries,
                "expired_entries": sum(1 for e in entries if e.is_expired),
                "total_results_cached": sum(len(e.results) for e in entries),
                "oldest_entry_age": (
                    max(now - e.created_at for e in entries) if entries else 0
                ),
            }


# Global cache instance
_cache: SearchCache | None = None


def get_cache() -> SearchCache:
    """Get the global search cache instance.

    Creates a new instance if none exists.
    """
    global _cache
    if _cache is None:
        _cache = SearchCache()
    return _cache


def reset_cache() -> None:
    """Reset the global cache instance.

    Useful for testing.
    """
    global _cache
    if _cache:
        _cache.clear()
    _cache = None
