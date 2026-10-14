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

WORDS = 300_000
WORDS_PER_SENTENCE = 10
TEMPLATE = "sentence\n  word letter=a"


def _write_synthetic_corpus(directory: Path) -> Path:
    """A corpus large enough that compiling and searching take measurable time."""
    directory.mkdir(parents=True)
    sentences = WORDS // WORDS_PER_SENTENCE
    header = "@{kind}\n@valueType=str\n@writtenBy=test\n\n"
    (directory / "otype.tf").write_text(
        header.format(kind="node")
        + f"1-{WORDS}\tword\n{WORDS + 1}-{WORDS + sentences}\tsentence\n"
    )
    slots = (
        f"{start}-{start + WORDS_PER_SENTENCE - 1}"
        for start in range(1, WORDS + 1, WORDS_PER_SENTENCE)
    )
    (directory / "oslots.tf").write_text(
        header.format(kind="edge") + f"{WORDS + 1}\t" + "\n".join(slots) + "\n"
    )
    letters = "abcdefghij"
    (directory / "letter.tf").write_text(
        header.format(kind="node")
        + "\n".join(letters[n % len(letters)] for n in range(WORDS))
        + "\n"
    )
    (directory / "otext.tf").write_text("@config\n@fmt:text-orig-full={letter} \n")
    return directory


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
            api.S.search(TEMPLATE)

    ticks, elapsed = _ticks_while(search_repeatedly)
    _assert_other_threads_ran(ticks, elapsed)


def test_concurrent_searches_match_serial_results(api):
    templates = [TEMPLATE, "word letter=b", "sentence\n  word letter=c\n  word letter=d"]
    expected = {template: api.S.search(template) for template in templates}
    with ThreadPoolExecutor(max_workers=8) as pool:
        futures = [(t, pool.submit(api.S.search, t)) for t in templates * 8]
        for template, future in futures:
            assert future.result() == expected[template]


def _search_in_child(queue) -> None:
    queue.put(len(_FORKED_API.S.search(TEMPLATE)))


@pytest.mark.skipif(sys.platform == "win32", reason="fork is POSIX-only")
def test_mapped_corpus_is_usable_after_fork(api):
    global _FORKED_API
    _FORKED_API = api
    expected = len(api.S.search(TEMPLATE))
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
print(len(api.S.search(sys.argv[2])))
"""


def test_processes_compiling_the_same_cache_concurrently_all_succeed(tmp_path):
    """Each process publishes a complete cache atomically; none sees a partial one."""
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
