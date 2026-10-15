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


def _ticks_while(work) -> tuple[int, float]:
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


def _assert_other_threads_ran(ticks: int, elapsed: float) -> None:
    # With the GIL held for the whole call the counter thread gets roughly one
    # tick; released, it ticks about once per millisecond.
    assert elapsed > 0.05, f"call too short ({elapsed:.3f}s) to observe the GIL"
    assert ticks >= elapsed * 1000 * 0.25, f"{ticks} ticks in {elapsed:.3f}s"


def test_compiling_load_releases_the_gil(tmp_path):
    corpus = _write_synthetic_corpus(tmp_path / "fresh")
    ticks, elapsed = _ticks_while(
        lambda: Fabric(locations=str(corpus), silent="deep").loadAll(silent="deep")
    )
    _assert_other_threads_ran(ticks, elapsed)


def test_compile_releases_the_gil(corpus_dir, tmp_path):
    fabric = Fabric(locations=str(corpus_dir), silent="deep")
    ticks, elapsed = _ticks_while(lambda: fabric.compile(str(tmp_path / "c.cfr")))
    _assert_other_threads_ran(ticks, elapsed)


def test_search_releases_the_gil(api):
    def search_repeatedly() -> None:
        deadline = time.perf_counter() + 0.3
        while time.perf_counter() < deadline:
            tuple(api.S.search(TEMPLATE))

    ticks, elapsed = _ticks_while(search_repeatedly)
    _assert_other_threads_ran(ticks, elapsed)


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
