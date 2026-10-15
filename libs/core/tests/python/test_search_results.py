"""The results of ``S.search`` / ``S.fetch`` / ``S.count`` as in Text-Fabric
(``tf/search/searchexe.py::fetch`` / ``count``): a lazy iterator without a
limit, a tuple with one, a set with ``shallow``; the fail limit of
``4 * maxNode`` results with its cut-off warning; and Ctrl-C stopping a
running search.

On ``mini_corpus`` ``maxNode`` is 8, so the fail limit is 32 results.
"""

from __future__ import annotations

import signal
import time
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
    child = WatchedProcess(
        LAZY_SCRIPT, str(corpus_tf_dir("n1904")), rss_limit_mb=2048, timeout=120
    )
    stdout, _ = child.finish()
    rows, seconds, peak_mb = stdout.split()
    assert int(rows) == 10
    assert float(seconds) < 5
    assert float(peak_mb) < 500


LAZY_SCRIPT = """
import itertools, resource, sys, time
from cfabric import Fabric
api = Fabric(locations=sys.argv[1], silent="deep").loadAll(silent="deep")
start = time.perf_counter()
rows = list(itertools.islice(api.S.search("x:word\\ny:word\\nx # y"), 10))
seconds = time.perf_counter() - start
peak = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
peak_mb = peak / 2**20 if sys.platform == "darwin" else peak / 2**10
print(len(rows), seconds, peak_mb)
"""


INTERRUPT_SCRIPT = """
import sys
from cfabric import Fabric
api = Fabric(locations=sys.argv[1], silent="deep").loadAll(silent="deep")
# Two contradictory conditions: no results, a cross product of work.
template = "a:word\\nb:word\\na .letter=letter. b\\na .letter#letter. b"
print("ready", flush=True)
try:
    if sys.argv[2] == "iterate":
        for _ in api.S.search(template):
            pass
    elif sys.argv[2] == "limit":
        api.S.search(template, limit=10)
    else:
        api.S.study(template)
        api.S.count()
    print("finished", flush=True)
except KeyboardInterrupt:
    print("interrupted", flush=True)
"""


@pytest.fixture(scope="module")
def interrupt_corpus(tmp_path_factory):
    corpus = write_synthetic_corpus(tmp_path_factory.mktemp("interrupt") / "corpus", 30_000)
    # Compile the cache once, outside the timed runs.
    from cfabric import Fabric

    Fabric(locations=str(corpus), silent="deep").loadAll(silent="deep")
    return corpus


@pytest.mark.parametrize("mode", ["iterate", "limit", "count"])
def test_ctrl_c_interrupts_a_running_search(interrupt_corpus, mode):
    child = WatchedProcess(
        INTERRUPT_SCRIPT, str(interrupt_corpus), mode, rss_limit_mb=1024, timeout=60
    )
    assert child.readline() == "ready"
    time.sleep(1)
    assert child.process.poll() is None, "the search ended before it could be interrupted"
    child.process.send_signal(signal.SIGINT)
    sent = time.monotonic()
    assert child.readline() == "interrupted"
    assert time.monotonic() - sent < 2
    child.finish()
    assert child.process.returncode == 0
