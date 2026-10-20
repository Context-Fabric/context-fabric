"""
# Search cancellation

Cooperative cancellation and timeouts for searches.

The browser's search workers (`workers.py`) check a `CancelToken` between the
results they get from the core's `S.search`; when it has been cancelled, or
its deadline has passed, `check()` raises `SearchCancelled` (or
`SearchTimeout`). Neither core checks it while it is still matching, so a
search that doesn't get to its results in time is stopped by ending its
worker process (the pool's watchdog). Moved here from the core (it was only
on the browser's branches), so that the browser works with any core.
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

    __slots__ = ("_deadline", "_event", "timeout")

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
