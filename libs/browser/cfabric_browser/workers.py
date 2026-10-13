"""Searching in worker processes.

Searches run in separate processes so that a heavy query never blocks the
browser, several searches can run at once, and a runaway search can always be
stopped. Each worker opens the corpora itself; because corpora are memory
mapped, the workers share most of their memory with each other and with the
server process.

Protocol, per worker, over two one-way pipes:

*   server to worker: `("search", job_id, corpus, template, limit, timeout,
    sets)`, `("cancel", job_id)`, `("stop",)`
*   worker to server: `("ready",)`, `("batch", job_id, [tuples])`,
    `("done", job_id, count, truncated)`, `("failed", job_id, problems)`,
    `("stopped", job_id, reason, count)` with reason `timeout` or `cancelled`

Cancellation is cooperative (the search checks its `CancelToken`); a watchdog
kills and replaces workers that do not stop in time.
"""

from __future__ import annotations

import logging
import multiprocessing as mp
import queue
import threading
import time
import uuid
from collections import OrderedDict
from collections.abc import Callable
from dataclasses import dataclass, field
from multiprocessing.connection import Connection
from typing import Any

logger = logging.getLogger(__name__)

QUEUED = "queued"
RUNNING = "running"
DONE = "done"
FAILED = "failed"
TIMEOUT = "timeout"
CANCELLED = "cancelled"
FINISHED = frozenset({DONE, FAILED, TIMEOUT, CANCELLED})

BATCH_SIZE = 2000
BATCH_INTERVAL = 0.15
GRACE = 3.0


@dataclass
class SearchJob:
    """A search and its (growing) results, shared between threads."""

    id: str
    corpus: str
    template: str
    limit: int
    timeout: float
    state: str = QUEUED
    results: list[tuple[int, ...]] = field(default_factory=list)
    truncated: bool = False
    sorted: bool = False
    problems: list[dict[str, Any]] = field(default_factory=list)
    created: float = field(default_factory=time.monotonic)
    started: float | None = None
    finished: float | None = None
    version: int = 0
    sets: dict[str, set[int]] | None = field(default=None, repr=False)
    _cond: threading.Condition = field(default_factory=threading.Condition, repr=False)

    @property
    def count(self) -> int:
        return len(self.results)

    @property
    def elapsed(self) -> float:
        if self.started is None:
            return 0.0
        return (self.finished or time.monotonic()) - self.started

    def update(self, **changes: Any) -> None:
        with self._cond:
            for key, value in changes.items():
                setattr(self, key, value)
            self.version += 1
            self._cond.notify_all()

    def extend(self, batch: list[tuple[int, ...]]) -> None:
        with self._cond:
            self.results.extend(batch)
            self.version += 1
            self._cond.notify_all()

    def wait_change(self, version: int, timeout: float) -> int:
        """Block until the job changes after `version`, or the timeout passes."""
        with self._cond:
            if self.version == version and self.state not in FINISHED:
                self._cond.wait(timeout)
            return self.version

    def page(self, offset: int, limit: int) -> list[tuple[int, ...]]:
        with self._cond:
            return self.results[offset : offset + limit]

    def snapshot(self) -> dict[str, Any]:
        with self._cond:
            return {
                "id": self.id,
                "corpus": self.corpus,
                "template": self.template,
                "state": self.state,
                "count": len(self.results),
                "truncated": self.truncated,
                "sorted": self.sorted,
                "limit": self.limit,
                "timeout": self.timeout,
                "elapsed": round(self.elapsed, 3),
                "problems": list(self.problems),
                "version": self.version,
            }


# the worker process


def _worker_main(
    specs: list[tuple[str, str]],
    commands: Connection,
    output: Connection,
    batchSize: int,
    batchInterval: float,
) -> None:
    """Entry point of a worker process."""
    from cfabric.registry import CorpusRegistry
    from cfabric.search.cancel import CancelToken, SearchCancelled, SearchTimeout
    from cfabric.search.searchexe import SearchExe

    from cfabric_browser.queries import (
        capture_engine_messages,
        problems_of,
        quantifier_problems,
    )

    registry = CorpusRegistry()
    for name, path in specs:
        registry.register(path, name)
        registry.get_api(name)
    output.send(("ready",))

    jobs: queue.Queue = queue.Queue()
    current: dict[str, Any] = {"id": None, "token": None}
    lock = threading.Lock()

    def listen() -> None:
        while True:
            try:
                msg = commands.recv()
            except (EOFError, OSError):
                jobs.put(None)
                return
            if msg[0] == "search":
                jobs.put(msg)
            elif msg[0] == "cancel":
                with lock:
                    if current["id"] == msg[1] and current["token"] is not None:
                        current["token"].cancel()
            elif msg[0] == "stop":
                jobs.put(None)
                return

    threading.Thread(target=listen, daemon=True).start()

    while True:
        msg = jobs.get()
        if msg is None:
            break
        (_, jobId, corpus, template, limit, timeout, sets) = msg
        token = CancelToken(timeout=timeout)
        with lock:
            current["id"] = jobId
            current["token"] = token

        count = 0
        batch: list[tuple[int, ...]] = []
        lastSend = time.monotonic()

        def flush(jobId: str) -> None:
            nonlocal batch, lastSend
            if batch:
                output.send(("batch", jobId, batch))
                batch = []
            lastSend = time.monotonic()

        try:
            api = registry.get_api(corpus)
            with capture_engine_messages() as messages:
                exe = SearchExe(
                    api,
                    template,
                    outerTemplate=template,
                    quKind=None,
                    offset=0,
                    sets=sets,
                    shallow=False,
                    silent="deep",
                    setInfo={},
                    cancel=token,
                )
                exe.study()
                if not exe.good:
                    output.send(("failed", jobId, problems_of(exe, messages)))
                    continue
                nested = quantifier_problems(messages)
                if nested:
                    output.send(("failed", jobId, nested))
                    continue
                truncated = False
                for result in exe.fetch(limit=None):
                    if count >= limit:
                        truncated = True
                        break
                    batch.append(tuple(int(n) for n in result))
                    count += 1
                    if (
                        len(batch) >= batchSize
                        or time.monotonic() - lastSend > batchInterval
                    ):
                        flush(jobId)
            flush(jobId)
            output.send(("done", jobId, count, truncated))
        except SearchTimeout:
            flush(jobId)
            output.send(("stopped", jobId, TIMEOUT, count))
        except SearchCancelled:
            flush(jobId)
            output.send(("stopped", jobId, CANCELLED, count))
        except Exception as e:  # noqa: BLE001 - report anything to the server
            flush(jobId)
            output.send(
                ("failed", jobId, [{"kind": "error", "line": None, "message": repr(e)}])
            )
        finally:
            with lock:
                current["id"] = None
                current["token"] = None


# the server side


class _Worker:
    def __init__(
        self,
        index: int,
        context: Any,
        specs: list[tuple[str, str]],
        batchSize: int,
        batchInterval: float,
    ) -> None:
        self.index = index
        (cmdRecv, self.commands) = context.Pipe(duplex=False)
        (self.output, outSend) = context.Pipe(duplex=False)
        self.process = context.Process(
            target=_worker_main,
            args=(specs, cmdRecv, outSend, batchSize, batchInterval),
            name=f"cfabric-search-{index}",
            daemon=True,
        )
        self.process.start()
        cmdRecv.close()
        outSend.close()
        self.ready = threading.Event()
        self.job: SearchJob | None = None
        self.dead = False

    def send(self, msg: tuple) -> bool:
        try:
            self.commands.send(msg)
            return True
        except (OSError, EOFError, BrokenPipeError):
            return False

    def kill(self) -> None:
        self.dead = True
        if self.process.is_alive():
            self.process.kill()
        self.process.join(timeout=5)
        for conn in (self.commands, self.output):
            try:
                conn.close()
            except OSError:
                pass


class SearchPool:
    """A pool of worker processes that run searches.

    Parameters
    ----------
    specs: list of (name, path)
        The corpora the workers must be able to search.
    sort_key: callable
        Given a corpus name, the key function that puts result tuples in
        canonical order (`N.sortKeyTuple`), used when a search has finished.
    workers: int
        Number of worker processes, i.e. searches that can run at once.
    max_results: int
        Results kept per search; searches stop (truncated) beyond this.
    timeout: float
        Default time limit per search, in seconds.
    keep: int
        Number of finished searches kept for paging and export.
    """

    def __init__(
        self,
        specs: list[tuple[str, str]],
        sort_key: Callable[[str], Callable[[tuple[int, ...]], Any]] | None = None,
        workers: int = 2,
        max_results: int = 500_000,
        timeout: float = 60.0,
        keep: int = 100,
        batch_size: int = BATCH_SIZE,
        batch_interval: float = BATCH_INTERVAL,
    ) -> None:
        self.specs = list(specs)
        self.sort_key = sort_key
        self.max_results = max_results
        self.timeout = timeout
        self.keep = keep
        self.batch_size = batch_size
        self.batch_interval = batch_interval
        self._context = mp.get_context("spawn")
        self._lock = threading.RLock()
        self._workers: list[_Worker] = []
        self._queue: list[SearchJob] = []
        self._jobs: OrderedDict[str, SearchJob] = OrderedDict()
        self._cancelDeadline: dict[str, float] = {}
        self._closing = False
        self._size = max(1, workers)
        self._watchdog: threading.Thread | None = None

    # lifecycle

    def start(self, wait: bool = True, timeout: float = 600.0) -> None:
        """Start the workers; by default wait until they have loaded the corpora."""
        with self._lock:
            for i in range(self._size):
                self._workers.append(self._spawn(i))
        self._watchdog = threading.Thread(
            target=self._watch, name="cfabric-search-watchdog", daemon=True
        )
        self._watchdog.start()
        if wait:
            deadline = time.monotonic() + timeout
            for worker in list(self._workers):
                worker.ready.wait(max(0.0, deadline - time.monotonic()))

    def shutdown(self) -> None:
        with self._lock:
            self._closing = True
            workers = list(self._workers)
        for worker in workers:
            worker.send(("stop",))
        for worker in workers:
            worker.process.join(timeout=2)
            worker.kill()

    @property
    def ready(self) -> bool:
        return any(w.ready.is_set() for w in self._workers)

    def _spawn(self, index: int) -> _Worker:
        worker = _Worker(
            index, self._context, self.specs, self.batch_size, self.batch_interval
        )
        threading.Thread(
            target=self._read,
            args=(worker,),
            name=f"cfabric-search-reader-{index}",
            daemon=True,
        ).start()
        return worker

    def _replace(self, worker: _Worker) -> None:
        """Kill a worker and start a fresh one in its place."""
        worker.kill()
        with self._lock:
            if self._closing or worker not in self._workers:
                return
            i = self._workers.index(worker)
            self._workers[i] = self._spawn(worker.index)

    # jobs

    def submit(
        self,
        corpus: str,
        template: str,
        limit: int | None = None,
        timeout: float | None = None,
        sets: dict[str, set[int]] | None = None,
    ) -> SearchJob:
        """Queue a search; it starts as soon as a worker is free."""
        job = SearchJob(
            id=uuid.uuid4().hex[:12],
            corpus=corpus,
            template=template,
            limit=min(limit or self.max_results, self.max_results),
            timeout=min(timeout or self.timeout, self.timeout),
            sets=sets,
        )
        with self._lock:
            self._jobs[job.id] = job
            self._queue.append(job)
            self._evict()
        self._dispatch()
        return job

    def get(self, job_id: str) -> SearchJob | None:
        with self._lock:
            job = self._jobs.get(job_id)
            if job is not None:
                self._jobs.move_to_end(job_id)
            return job

    def cancel(self, job_id: str) -> bool:
        """Stop a search. Returns False if there is no such search."""
        with self._lock:
            job = self._jobs.get(job_id)
            if job is None:
                return False
            if job.state == QUEUED:
                self._queue.remove(job)
                job.update(state=CANCELLED, finished=time.monotonic())
                return True
            if job.state != RUNNING:
                return True
            worker = next((w for w in self._workers if w.job is job), None)
            self._cancelDeadline[job.id] = time.monotonic() + GRACE
        if worker is not None:
            worker.send(("cancel", job.id))
        return True

    def _evict(self) -> None:
        finished = [j for j in self._jobs.values() if j.state in FINISHED]
        excess = len(self._jobs) - self.keep
        for job in finished[: max(0, excess)]:
            del self._jobs[job.id]

    def _dispatch(self) -> None:
        with self._lock:
            while self._queue:
                worker = next(
                    (
                        w
                        for w in self._workers
                        if w.ready.is_set() and w.job is None and not w.dead
                    ),
                    None,
                )
                if worker is None:
                    return
                job = self._queue.pop(0)
                worker.job = job
                job.update(state=RUNNING, started=time.monotonic())
                sent = worker.send(
                    (
                        "search",
                        job.id,
                        job.corpus,
                        job.template,
                        job.limit,
                        job.timeout,
                        job.sets,
                    )
                )
                if not sent:
                    worker.job = None
                    job.update(state=QUEUED, started=None)
                    self._queue.insert(0, job)
                    return

    def _finish(self, worker: _Worker, job: SearchJob, **changes: Any) -> None:
        if changes.get("state") == DONE and self.sort_key is not None:
            try:
                key = self.sort_key(job.corpus)
                with job._cond:
                    job.results.sort(key=key)
                changes["sorted"] = True
            except Exception:
                logger.exception("Could not sort results of search %s", job.id)
        job.update(finished=time.monotonic(), **changes)
        with self._lock:
            if worker.job is job:
                worker.job = None
            self._cancelDeadline.pop(job.id, None)
        self._dispatch()

    def _read(self, worker: _Worker) -> None:
        """Receive the messages of one worker."""
        while True:
            try:
                msg = worker.output.recv()
            except (EOFError, OSError):
                break
            kind = msg[0]
            if kind == "ready":
                worker.ready.set()
                self._dispatch()
                continue
            job = worker.job
            if job is None or msg[1] != job.id:
                continue
            if kind == "batch":
                job.extend(msg[2])
            elif kind == "done":
                self._finish(worker, job, state=DONE, truncated=msg[3])
            elif kind == "failed":
                self._finish(worker, job, state=FAILED, problems=msg[2])
            elif kind == "stopped":
                self._finish(worker, job, state=msg[2])

        # the worker is gone
        job = worker.job
        if job is not None and job.state not in FINISHED:
            self._finish(
                worker,
                job,
                state=FAILED,
                problems=[
                    {
                        "kind": "error",
                        "line": None,
                        "message": "The search process stopped unexpectedly",
                    }
                ],
            )
        if not worker.dead and not self._closing:
            logger.warning("Search worker %d died; starting a new one", worker.index)
            self._replace(worker)

    def _watch(self) -> None:
        """Kill workers whose search ignores its timeout or a cancel request."""
        while not self._closing:
            time.sleep(0.5)
            now = time.monotonic()
            stuck: list[tuple[_Worker, SearchJob, str]] = []
            with self._lock:
                for worker in self._workers:
                    job = worker.job
                    if job is None or job.started is None:
                        continue
                    deadline = self._cancelDeadline.get(job.id)
                    if deadline is not None and now > deadline:
                        stuck.append((worker, job, CANCELLED))
                    elif now > job.started + job.timeout + GRACE:
                        stuck.append((worker, job, TIMEOUT))
            for worker, job, reason in stuck:
                logger.warning("Search %s did not stop; restarting its worker", job.id)
                self._finish(worker, job, state=reason)
                self._replace(worker)
