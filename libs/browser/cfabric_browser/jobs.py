"""Work that runs in the background with progress for the interface to show:
building a translation, downloading a corpus.

A job has a key (a translation id, a corpus id); one job per key runs at a
time. Its status is a small dict that the API returns as it is::

    {"state": "running" | "done" | "failed", "step": ..., "done": n, "total": m, "error": None}
"""

from __future__ import annotations

import logging
import threading
from collections.abc import Callable
from typing import Any

logger = logging.getLogger(__name__)

# called by the work with a step, the amount done and the total
Progress = Callable[[str, int, int], None]


class BackgroundJobs:
    """Jobs in threads, with their progress and how they ended.

    `expected` are the exceptions whose message is meant for the user as it is
    (a source that isn't what was expected); an `OSError` is reported as a
    problem with writing or the network, and anything else as a failure.
    """

    def __init__(self, expected: tuple[type[Exception], ...] = ()) -> None:
        self.expected = expected
        self._lock = threading.Lock()
        self._status: dict[str, dict[str, Any]] = {}

    def status(self, key: str) -> dict[str, Any] | None:
        with self._lock:
            status = self._status.get(key)
            return dict(status) if status else None

    def run(
        self, key: str, work: Callable[[Progress], Any], first_step: str = "download"
    ) -> dict[str, Any]:
        """Start `work(progress)` in a thread, unless a job for `key` runs already."""
        with self._lock:
            current = self._status.get(key)
            if current and current["state"] == "running":
                return dict(current)
            status = {
                "state": "running",
                "step": first_step,
                "done": 0,
                "total": 0,
                "error": None,
            }
            self._status[key] = status
        thread = threading.Thread(target=self._run, args=(key, work), daemon=True)
        thread.start()
        return dict(status)

    def _progress(self, key: str, step: str, done: int, total: int) -> None:
        with self._lock:
            self._status[key].update(step=step, done=done, total=total)

    def _run(self, key: str, work: Callable[[Progress], Any]) -> None:
        try:
            work(lambda step, done, total: self._progress(key, step, done, total))
        except self.expected as e:
            self._finish(key, str(e))
        except OSError as e:
            self._finish(key, f"Could not write it: {e}")
        except Exception as e:
            logger.exception("The job %s failed", key)
            self._finish(key, f"It failed: {e}")
        else:
            self._finish(key, None)

    def _finish(self, key: str, error: str | None) -> None:
        with self._lock:
            self._status[key].update(state="failed" if error else "done", error=error)
