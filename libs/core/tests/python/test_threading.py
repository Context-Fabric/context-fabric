"""Long engine calls release the GIL; corpora are safe across threads and fork.

Loading (which may compile the cache) and searching run in Rust for seconds on
real corpora. Holding the GIL meanwhile freezes every other Python thread, e.g.
an MCP server's event loop, so these calls must release it.
"""

from __future__ import annotations

import multiprocessing
import sys
import threading
import time
from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import pytest

from cfabric import Fabric
from conftest import write_synthetic_corpus

WORDS = 300_000
WORDS_PER_SENTENCE = 10
TEMPLATE = "sentence\n  word letter=a"


def _write_synthetic_corpus(directory: Path) -> Path:
    """A corpus large enough that compiling and searching take measurable time."""
    return write_synthetic_corpus(directory, WORDS, WORDS_PER_SENTENCE)


@pytest.fixture(scope="module")
def corpus_dir(tmp_path_factory) -> Path:
    return _write_synthetic_corpus(tmp_path_factory.mktemp("threading") / "synthetic")


@pytest.fixture(scope="module")
def api(corpus_dir):
    return Fabric(locations=str(corpus_dir), silent="deep").loadAll(silent="deep")


def _ticks_while(work: Callable[[], object]) -> tuple[int, float]:
    """Count how often a pure-Python thread runs while ``work()`` executes."""
    ticks = 0
    done = threading.Event()

    def count() -> None:
        nonlocal ticks
        while not done.is_set():
            ticks += 1
            time.sleep(0.001)

    counter = threading.Thread(target=count)
    counter.start()
    started = time.perf_counter()
    try:
        work()
    finally:
        elapsed = time.perf_counter() - started
        done.set()
        counter.join()
    return ticks, elapsed


CALIBRATION_SECONDS = 0.2
"""How long the main thread sleeps (without the GIL) to measure the counter's
free-running tick rate."""
RELEASED_FRACTION = 0.05
"""Share of the free-running rate the counter must keep during the call. A
1 ms sleep oversleeps on loaded or timer-coalescing machines (macOS CI runners,
x86_64 under Rosetta, tick a few hundred times a second), and the engine's
work competes for the CPU, so the bar is relative to the rate measured on the
same machine, and low."""
MIN_EXPECTED_TICKS = 100
"""Ticks a free-running counter must be expected to make during the call, so
that the bar (at least 5 ticks) stays above the one or two ticks a call that
holds the GIL leaves the counter."""


def _assert_releases_the_gil(work: Callable[[], object]) -> None:
    """Assert that a pure-Python thread keeps running while ``work()`` runs."""
    idle_ticks, idle_elapsed = _ticks_while(lambda: time.sleep(CALIBRATION_SECONDS))
    free_rate = idle_ticks / idle_elapsed
    ticks, elapsed = _ticks_while(work)
    expected = free_rate * elapsed
    assert expected >= MIN_EXPECTED_TICKS, (
        f"call too short ({elapsed:.3f}s at {free_rate:.0f} ticks/s) to observe the GIL"
    )
    assert ticks >= expected * RELEASED_FRACTION, (
        f"{ticks} ticks in {elapsed:.3f}s; free-running rate {free_rate:.0f} ticks/s"
    )


def test_compiling_load_releases_the_gil(tmp_path):
    corpus = _write_synthetic_corpus(tmp_path / "fresh")
    _assert_releases_the_gil(
        lambda: Fabric(locations=str(corpus), silent="deep").loadAll(silent="deep")
    )


def test_compile_releases_the_gil(corpus_dir, tmp_path):
    fabric = Fabric(locations=str(corpus_dir), silent="deep")
    _assert_releases_the_gil(lambda: fabric.compile(str(tmp_path / "c.cfr")))


NO_RESULTS = "a:word\nb:word\na .letter=letter. b\na .letter#letter. b"
"""Two contradictory conditions: no results after comparing every pair of
words, so the first step of the result iterator is one long engine call."""


@pytest.fixture(scope="module")
def pairs_api(tmp_path_factory):
    """A corpus small enough that comparing all its word pairs takes about a
    second."""
    corpus = write_synthetic_corpus(tmp_path_factory.mktemp("pairs") / "corpus", 4_000)
    return Fabric(locations=str(corpus), silent="deep").loadAll(silent="deep")


def test_search_releases_the_gil(pairs_api):
    _assert_releases_the_gil(lambda: tuple(pairs_api.S.search(NO_RESULTS)))


def test_count_releases_the_gil(pairs_api):
    def count() -> None:
        pairs_api.S.study(NO_RESULTS)
        pairs_api.S.count()

    _assert_releases_the_gil(count)


def _hold_the_gil(seconds: float) -> None:
    """Hold the GIL for about ``seconds`` in one C call: ``sum`` over a
    ``range`` never lets other threads in, so it stands in for an engine call
    that forgets to release the GIL."""
    sample = 1_000_000
    started = time.perf_counter()
    sum(range(sample))
    per_item = (time.perf_counter() - started) / sample
    sum(range(int(seconds / per_item)))


def test_holding_the_gil_fails_the_check():
    """The check tells a call that holds the GIL from one that releases it."""
    with pytest.raises(AssertionError, match="ticks in"):
        _assert_releases_the_gil(lambda: _hold_the_gil(1.0))


def test_concurrent_searches_match_serial_results(api):
    templates = [TEMPLATE, "word letter=b", "sentence\n  word letter=c\n  word letter=d"]
    expected = {template: tuple(api.S.search(template)) for template in templates}
    with ThreadPoolExecutor(max_workers=8) as pool:
        futures = [
            (t, pool.submit(lambda t: tuple(api.S.search(t)), t)) for t in templates * 8
        ]
        for template, future in futures:
            assert future.result() == expected[template]


def _search_in_child(queue) -> None:
    queue.put(len(tuple(_FORKED_API.S.search(TEMPLATE))))


@pytest.mark.skipif(sys.platform == "win32", reason="fork is POSIX-only")
def test_mapped_corpus_is_usable_after_fork(api):
    global _FORKED_API
    _FORKED_API = api
    expected = len(tuple(api.S.search(TEMPLATE)))
    context = multiprocessing.get_context("fork")
    queue = context.Queue()
    child = context.Process(target=_search_in_child, args=(queue,))
    child.start()
    child.join(timeout=60)
    assert child.exitcode == 0
    assert queue.get(timeout=5) == expected


_FORKED_API = None


_CONCURRENT_LOAD = """
import sys
from cfabric import Fabric
api = Fabric(locations=sys.argv[1], silent="deep").loadAll(silent="deep")
print(len(tuple(api.S.search(sys.argv[2]))))
"""


def test_processes_compiling_the_same_cache_concurrently_all_succeed(tmp_path):
    """The first process to publish its cache wins and the others map it: none
    sees a partial cache, none replaces a cache another has mapped (which
    Windows refuses), and no staged file is left behind."""
    import subprocess

    corpus = _write_synthetic_corpus(tmp_path / "shared")
    processes = [
        subprocess.Popen(
            [sys.executable, "-c", _CONCURRENT_LOAD, str(corpus), TEMPLATE],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            cwd=tmp_path,
        )
        for _ in range(6)
    ]
    outputs = [process.communicate(timeout=300) for process in processes]
    for process, (stdout, stderr) in zip(processes, outputs):
        assert process.returncode == 0, stderr
    counts = {stdout.strip() for stdout, _ in outputs}
    assert len(counts) == 1 and int(counts.pop()) > 0
    leftovers = [path.name for path in (corpus / ".cfr").rglob("*.partial")]
    assert leftovers == []
    assert len(list((corpus / ".cfr").rglob("*.cfr"))) == 1
