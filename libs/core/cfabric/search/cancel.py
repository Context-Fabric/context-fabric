"""
# Search cancellation

Cooperative cancellation and timeouts for searches.

A `CancelToken` is passed to `cfabric.search.search.Search.search` (or `study`).
The search engine checks the token at safe points: between the phases of
studying a template, while spinning edges, and before each candidate of the
outermost loop of result stitching. When the token has been cancelled, or its
deadline has passed, the search raises `SearchCancelled` (or `SearchTimeout`).

Checks are cheap, but they are not instantaneous: a single very expensive step
between two checks will run to completion. Callers that need a hard guarantee
should run searches in a separate process and terminate it.
"""

from __future__ import annotations

import threading
import time


class SearchCancelled(Exception):
    """Raised when a search is stopped through its `CancelToken`."""


class SearchTimeout(SearchCancelled):
    """Raised when a search exceeds the timeout of its `CancelToken`."""


class CancelToken:
    """A thread-safe handle to stop a running search.

    Parameters
    ----------
    timeout: float, optional None
        Number of seconds after creation of the token after which the search
        is stopped. `None` means no timeout.

    Examples
    --------
    >>> token = CancelToken(timeout=10)
    >>> results = api.S.search(template, cancel=token)

    From another thread:

    >>> token.cancel()
    """

    __slots__ = ("_event", "_deadline", "timeout")

    def __init__(self, timeout: float | None = None) -> None:
        self._event = threading.Event()
        self.timeout = timeout
        self._deadline = (
            None if timeout is None else time.monotonic() + max(0.0, timeout)
        )

    def cancel(self) -> None:
        """Request the search to stop at its next check."""
        self._event.set()

    @property
    def cancelled(self) -> bool:
        """Whether `cancel()` has been called."""
        return self._event.is_set()

    @property
    def expired(self) -> bool:
        """Whether the timeout has passed."""
        return self._deadline is not None and time.monotonic() > self._deadline

    def check(self) -> None:
        """Raise if the search should stop.

        Raises
        ------
        SearchCancelled
            If `cancel()` has been called.
        SearchTimeout
            If the timeout has passed.
        """
        if self._event.is_set():
            raise SearchCancelled("search cancelled")
        if self._deadline is not None and time.monotonic() > self._deadline:
            raise SearchTimeout(f"search exceeded timeout of {self.timeout} seconds")
