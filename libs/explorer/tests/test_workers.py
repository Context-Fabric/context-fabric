"""Tests for the search pool: the watchdog, and workers that can't load a corpus.

The watchdog tests use stand-ins for the worker processes, so that a search
can be "stuck" without a search engine that really hangs.
"""

import threading
import time
from pathlib import Path

from cfabric_browser.workers import (
    CANCELLED,
    DONE,
    FAILED,
    RUNNING,
    TIMEOUT,
    SearchPool,
)

FIXTURE = Path(__file__).resolve().parent / "fixtures" / "display_corpus"
DATA = FIXTURE / "tf" / "1.0"


class FakeWorker:
    """What the pool needs of a worker, without a process."""

    def __init__(self, index: int) -> None:
        self.index = index
        self.ready = threading.Event()
        self.ready.set()
        self.settled = threading.Event()
        self.settled.set()
        self.job = None
        self.dead = False
        self.sent: list[tuple] = []
        self.killed = False

    def send(self, msg: tuple) -> bool:
        self.sent.append(msg)
        return True

    def kill(self) -> None:
        self.dead = True
        self.killed = True


def fake_pool(workers: int = 1) -> tuple[SearchPool, list[FakeWorker]]:
    pool = SearchPool([], workers=workers, timeout=5)
    spawned: list[FakeWorker] = []

    def spawn(index: int) -> FakeWorker:
        worker = FakeWorker(index)
        spawned.append(worker)
        return worker

    pool._spawn = spawn  # type: ignore[method-assign]
    pool._workers = [spawn(i) for i in range(workers)]
    return pool, spawned


def test_stuck_search_ends_and_its_worker_is_replaced():
    pool, spawned = fake_pool()
    job = pool.submit("alpha", "word")
    old = spawned[0]
    assert old.job is job and job.state == RUNNING

    pool._stop_stuck(time.monotonic() + 100)

    assert job.state == TIMEOUT
    assert old.killed
    assert pool._workers == [spawned[1]]


def test_queued_search_does_not_go_to_the_worker_being_killed():
    pool, spawned = fake_pool()
    stuck = pool.submit("alpha", "word")
    waiting = pool.submit("alpha", "phrase")
    old = spawned[0]

    pool._stop_stuck(time.monotonic() + 100)

    assert stuck.state == TIMEOUT
    # the next search went to the new worker, not to the killed one
    assert not any(m[0] == "search" and m[1] == waiting.id for m in old.sent)
    new = spawned[1]
    assert new.job is waiting
    assert waiting.state == RUNNING


def test_watchdog_leaves_a_search_that_finished_meanwhile():
    pool, spawned = fake_pool()
    job = pool.submit("alpha", "word")
    worker = spawned[0]
    # the reader has started finishing it (sorting) when the watchdog looks
    with pool._lock:
        assert pool._claim(job)

    pool._stop_stuck(time.monotonic() + 100)

    assert not worker.killed
    assert pool._finish(worker, job, claimed=True, state=DONE)
    assert job.state == DONE
    assert worker.job is None


def test_a_job_is_finished_only_once():
    pool, spawned = fake_pool()
    job = pool.submit("alpha", "word")
    pool.cancel(job.id)
    assert pool._finish(spawned[0], job, state=DONE)
    assert not pool._finish(spawned[0], job, state=CANCELLED)
    assert job.state == DONE


def test_corpus_that_fails_to_load_fails_its_searches(tmp_path):
    empty = tmp_path / "empty"
    empty.mkdir()
    pool = SearchPool(
        [("alpha", str(DATA)), ("broken", str(empty))], workers=1, timeout=20
    )
    pool.start(wait=True, timeout=60)
    try:
        assert pool.ready
        broken = pool.submit("broken", "word")
        good = pool.submit("alpha", "word")
        deadline = time.monotonic() + 30
        while good.state not in (DONE, FAILED) and time.monotonic() < deadline:
            time.sleep(0.05)
        assert broken.state == FAILED
        assert broken.problems
        assert good.state == DONE
        assert good.count > 0
    finally:
        pool.shutdown()


def test_a_queued_search_is_cancelled_at_once():
    pool, spawned = fake_pool()
    running = pool.submit("alpha", "word")
    waiting = pool.submit("alpha", "phrase")
    assert pool.cancel(waiting.id)
    assert waiting.state == CANCELLED
    # it never reaches a worker, also not when the first one is done
    pool._finish(spawned[0], running, state=DONE)
    assert not any(m[0] == "search" and m[1] == waiting.id for m in spawned[0].sent)
    assert not pool.cancel("nonexistent")


def test_a_running_search_is_asked_to_stop_and_then_ended():
    pool, spawned = fake_pool()
    job = pool.submit("alpha", "word")
    assert pool.cancel(job.id)
    assert ("cancel", job.id) in spawned[0].sent
    # it doesn't stop within the grace period: its worker is ended
    pool._stop_stuck(time.monotonic() + 10)
    assert job.state == CANCELLED
    assert spawned[0].killed


def test_only_the_last_finished_searches_are_kept():
    pool, spawned = fake_pool()
    pool.keep = 3
    jobs = []
    for i in range(5):
        job = pool.submit("alpha", f"word n={i}")
        pool._finish(spawned[0], job, state=DONE)
        jobs.append(job)
    pool.submit("alpha", "phrase")  # adding one evicts the oldest finished
    kept = [j for j in jobs if pool.get(j.id) is not None]
    assert kept == jobs[-2:]
