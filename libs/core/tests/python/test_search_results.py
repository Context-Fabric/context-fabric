"""The results of ``S.search`` / ``S.fetch`` / ``S.count`` as in Text-Fabric
(``tf/search/searchexe.py::fetch`` / ``count``): a lazy iterator without a
limit, a tuple with one, a set with ``shallow``; the fail limit of
``4 * maxNode`` results with its cut-off warning; and Ctrl-C stopping a
running search.

On ``mini_corpus`` ``maxNode`` is 8, so the fail limit is 32 results.
"""

from __future__ import annotations

import warnings
from collections.abc import Iterator

import pytest

from conftest import WatchedProcess, write_synthetic_corpus

FAIL_LIMIT = 32
CUT_OFF = r"^cut off at 32 results\. There are more \.\.\.$"
PAIRS = "a:.\nb:.\na # b"
"""Every ordered pair of distinct nodes: 8 * 7 = 56 results, over the fail limit."""


def test_without_a_limit_results_are_a_lazy_iterator(loaded_api):
    results = loaded_api.S.search("word")
    assert isinstance(results, Iterator)
    assert type(results).__name__ == "SearchResultIterator"
    assert next(results) == (1,)
    assert list(results) == [(2,), (3,), (4,), (5,)]
    # Read once, like Text-Fabric's generator.
    assert list(results) == []


@pytest.mark.filterwarnings("ignore:cut off")
@pytest.mark.parametrize(
    ("limit", "shallow", "kind"),
    [
        (None, False, Iterator),
        (3, False, tuple),
        (0, False, tuple),
        (-1, False, tuple),
        (None, True, set),
        (2, True, set),
        (None, 2, set),
        (1, 2, set),
    ],
)
def test_return_type_follows_limit_and_shallow(loaded_api, limit, shallow, kind):
    assert isinstance(loaded_api.S.search(PAIRS, limit=limit, shallow=shallow), kind)


def test_shallow_sets_are_complete_whatever_the_limit(loaded_api):
    # TF returns the whole set for `shallow`, ignoring `limit`.
    assert loaded_api.S.search(PAIRS, shallow=True, limit=1) == set(range(1, 9))
    assert len(loaded_api.S.search(PAIRS, shallow=2, limit=1)) == 56


def test_unlimited_iteration_stops_at_the_fail_limit_with_a_warning(loaded_api):
    with pytest.warns(UserWarning, match=CUT_OFF):
        rows = list(loaded_api.S.search(PAIRS))
    assert len(rows) == FAIL_LIMIT
    loaded_api.S.study(PAIRS)
    with pytest.warns(UserWarning, match=CUT_OFF):
        assert len(list(loaded_api.S.fetch())) == FAIL_LIMIT


@pytest.mark.parametrize("limit", [0, -1])
def test_a_falsy_limit_returns_a_tuple_up_to_the_fail_limit(loaded_api, limit):
    with pytest.warns(UserWarning, match=CUT_OFF):
        rows = loaded_api.S.search(PAIRS, limit=limit)
    assert isinstance(rows, tuple) and len(rows) == FAIL_LIMIT


def test_a_positive_limit_caps_quietly(loaded_api):
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        assert len(loaded_api.S.search(PAIRS, limit=40)) == 40
        assert len(loaded_api.S.search(PAIRS, limit=100)) == 56
        assert list(loaded_api.S.search("word")) == [(1,), (2,), (3,), (4,), (5,)]


def test_count_applies_the_fail_limit(loaded_api):
    loaded_api.S.study(PAIRS)
    with pytest.warns(UserWarning, match=CUT_OFF):
        assert loaded_api.S.count() == FAIL_LIMIT
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        assert loaded_api.S.count(limit=40) == 40
        assert loaded_api.S.count(limit=100) == 56


def test_lazy_results_keep_memory_bounded(corpus_tf_dir):
    """The first rows of a cross product of N1904's 137,779 words arrive at
    once, without producing the rest."""
    corpus = str(corpus_tf_dir("n1904"))
    # Compiling the cache takes far more memory than searching; do it first.
    WatchedProcess(LOAD_SCRIPT, corpus, rss_limit_mb=4096, timeout=600).finish()
    child = WatchedProcess(LAZY_SCRIPT, corpus, rss_limit_mb=2048, timeout=120)
    stdout, _ = child.finish()
    rows, seconds, peak_mb = stdout.split()
    assert int(rows) == 10
    assert float(seconds) < 5
    assert float(peak_mb) < 500


LOAD_SCRIPT = """
import sys
from cfabric import Fabric
Fabric(locations=sys.argv[1], silent="deep").loadAll(silent="deep")
"""


LAZY_SCRIPT = """
import itertools, sys, time
from cfabric import Fabric
from process_memory import peak_resident_mb
api = Fabric(locations=sys.argv[1], silent="deep").loadAll(silent="deep")
start = time.perf_counter()
rows = list(itertools.islice(api.S.search("x:word\\ny:word\\nx # y"), 10))
seconds = time.perf_counter() - start
print(len(rows), seconds, peak_resident_mb())
"""


INTERRUPT_SCRIPT = """
import _thread, sys, threading, time
from cfabric import Fabric
api = Fabric(locations=sys.argv[1], silent="deep").loadAll(silent="deep")
mode, delay = sys.argv[2], float(sys.argv[3])
# Two contradictory conditions: no results, a cross product of work.
template = "a:word\\nb:word\\na .letter=letter. b\\na .letter#letter. b"
searching = threading.Event()
pressed = []

def press_ctrl_c():
    # What Ctrl-C does on every platform: SIGINT's handler becomes pending
    # for the main thread, which the engine polls while it searches.
    pressed.append((time.monotonic(), searching.is_set()))
    _thread.interrupt_main()

timer = threading.Timer(delay, press_ctrl_c)
try:
    searching.set()
    timer.start()
    if mode == "iterate":
        for _ in api.S.search(template):
            pass
    elif mode == "limit":
        api.S.search(template, limit=10)
    else:
        api.S.study(template)
        api.S.count()
    searching.clear()
    timer.cancel()
    print("finished", flush=True)
except KeyboardInterrupt:
    pressed_at, was_searching = pressed[0]
    print("interrupted", was_searching, time.monotonic() - pressed_at, flush=True)
"""


@pytest.fixture(scope="module")
def interrupt_corpus(tmp_path_factory):
    corpus = write_synthetic_corpus(tmp_path_factory.mktemp("interrupt") / "corpus", 30_000)
    # Compile the cache once, outside the timed runs.
    from cfabric import Fabric

    Fabric(locations=str(corpus), silent="deep").loadAll(silent="deep")
    return corpus


CTRL_C_AFTER_SECONDS = 1.0
"""How long the search runs before the child presses Ctrl-C (it takes several
seconds uninterrupted)."""


@pytest.mark.parametrize("mode", ["iterate", "limit", "count"])
def test_ctrl_c_interrupts_a_running_search(interrupt_corpus, mode):
    child = WatchedProcess(
        INTERRUPT_SCRIPT,
        str(interrupt_corpus),
        mode,
        str(CTRL_C_AFTER_SECONDS),
        rss_limit_mb=1024,
        timeout=60,
    )
    stdout, stderr = child.finish()
    outcome, *details = stdout.split()
    assert outcome == "interrupted", f"{stdout}\n{stderr}"
    was_searching, seconds = details
    assert was_searching == "True", "the search ended before it could be interrupted"
    assert float(seconds) < 2
    assert child.process.returncode == 0
