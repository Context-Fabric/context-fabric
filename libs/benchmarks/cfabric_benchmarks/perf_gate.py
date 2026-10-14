"""Repeatable TF-vs-CF performance gate for BHSA.

Measures, for one engine per process: corpus load time and memory, first-call
per-call probes over small node sets (these include one-time index builds),
steady-state per-call latency (warm-up, then the best of several passes), a
full ``N.walk`` pass, and the gate queries (the eight original benchmark queries
plus the four that once ran slower than Text-Fabric).

Usage
-----
Record a Context-Fabric (mapped) run::

    python -m cfabric_benchmarks.perf_gate --engine cf --out /tmp/cf.json

Record a Text-Fabric run (slow; loads the full 6 GB in-memory model), e.g. to
refresh ``baselines/bhsa_tf.json`` on the machine that runs the gate::

    python -m cfabric_benchmarks.perf_gate --engine tf --out baselines/bhsa_tf.json

Assert a CF run against ``baselines/targets.json`` and the recorded TF baseline
(does not reload TF)::

    python -m cfabric_benchmarks.perf_gate --engine cf --assert --strict

``--assert`` reports every violation; only ``--strict`` makes violations exit
non-zero. The targets combine absolute ceilings (machine independent, catch
order-of-magnitude regressions such as a per-call corpus scan) with ratios to
the recorded TF steady state and query times (catch "slower than TF").

Run Python from outside the repository root; importing ``cfabric`` from the repo
root breaks.  The CLI does not chdir, so invoke it from another directory.
"""

from __future__ import annotations

import argparse
import json
import random
import resource
import sys
import time
from pathlib import Path
from typing import Any, Callable

# Repo paths (perf_gate.py -> cfabric_benchmarks -> benchmarks -> libs -> repo)
_BENCHMARKS_DIR = Path(__file__).resolve().parents[1]
_REPO_ROOT = _BENCHMARKS_DIR.parents[1]
DEFAULT_BHSA_TF = _BENCHMARKS_DIR / ".corpora" / "bhsa" / "tf"
DEFAULT_CACHE = Path("/tmp/cf_bhsa.cfr")
DEFAULT_BASELINE = _BENCHMARKS_DIR / "baselines" / "bhsa_tf.json"
DEFAULT_TARGETS = _BENCHMARKS_DIR / "baselines" / "targets.json"
PARITY_QUERIES_JSON = _REPO_ROOT / "libs" / "core" / "tests" / "parity" / "parity_queries.json"

# BHSA node-id intervals the probes draw from (phrases, verses).
_PHRASES = (651573, 904775)
_VERSES = (1414389, 1437601)

# First-call probe sizes mirror the original bench so numbers stay comparable.
_N_WORD_CALLS = 10000
_N_LOCALITY = 30
_N_EDGE = 500
_N_SECTION = 200
_N_VERSE = 100

# Steady state: warm-up pass, then the best of several passes of many calls
# (machine load only ever adds time, so the best pass is the most repeatable
# statistic on a shared machine). Calls slower than _SLOW_CALL_S get a
# time-budgeted pass so a regression to milliseconds per call stays measurable
# instead of running for minutes.
_STEADY_CALLS = 50_000
_STEADY_PASSES = 5
_SLOW_CALL_S = 20e-6
_SLOW_PASS_BUDGET_S = 1.0
_MIN_SLOW_CALLS = 100
_WALK_PASSES = 3

# Gate queries: the eight original benchmark queries plus the four that ran
# slower than TF in the 0.6.0rc1 baseline. Templates come from the parity
# suite when present there, else from these fallbacks.
_QUERY_FALLBACKS: dict[str, str] = {
    "q_phrase_without_verb": "phrase\n/without/\n  word sp=verb\n/-/",
    "q_regex_gcons": "word g_cons~^MLK",
    "q_regex_typ": "phrase typ~^(NP|PP)$\n  word sp=subs",
    "q_regex_gloss_space": "word gloss~give birth",
    "q_feat_exists_qere": "word qere",
    "q_feat_empty_qere": "word qere#",
    "q_multivalue_neg": "word vt#perf|impf|wayq sp=verb vs=nif",
    "q_slot_equal": "clause\n  == sentence",
    "q_mother_basic": "\nclause\n-mother> clause\n",
    "q_mother_reverse": "\nclause\n<mother- clause\n",
    "q_mother_named_distinct_verse": (
        "\nv1:verse\n    c1:clause\nv2:verse\n    c2:clause\n\nc1 -mother> c2\nv1 # v2\n"
    ),
    "q_slot_overlap_parent": (
        "\nsentence\n/with/\nv1:verse\n&& ..\nv2:verse\n&& ..\nv1 < v2\n/-/\n"
    ),
}


def _peak_rss_mb() -> float:
    # ru_maxrss is bytes on macOS, kilobytes on Linux.
    maxrss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    if sys.platform == "darwin":
        return maxrss / (1024 * 1024)
    return maxrss / 1024


def _load_engine(engine: str, bhsa_tf: Path, cache: Path) -> Any:
    if engine == "tf":
        from tf.fabric import Fabric  # type: ignore[import-untyped]

        return Fabric(locations=str(bhsa_tf), silent="deep").loadAll(silent="deep")
    if engine == "cf":
        from cfabric import Fabric

        return Fabric(locations=str(bhsa_tf)).openMapped(str(cache))
    raise ValueError(f"unknown engine {engine!r}")


def _query_templates() -> dict[str, str]:
    templates = dict(_QUERY_FALLBACKS)
    if PARITY_QUERIES_JSON.exists():
        with open(PARITY_QUERIES_JSON) as handle:
            by_id = {q["id"]: q["template"] for q in json.load(handle)["queries"]}
        for qid in templates:
            if qid in by_id:
                templates[qid] = by_id[qid]
    return templates


def steady_per_call_us(fn: Callable[[Any], Any], items: list[Any]) -> float:
    """Best-pass per-call microseconds after a warm-up pass over ``items``."""
    warmup = items[: min(len(items), 200)]
    start = time.perf_counter()
    for item in warmup:
        fn(item)
    estimate = (time.perf_counter() - start) / len(warmup)
    n_calls = _STEADY_CALLS
    if estimate > _SLOW_CALL_S:
        n_calls = max(_MIN_SLOW_CALLS, int(_SLOW_PASS_BUDGET_S / estimate))
    else:
        for item in items:
            fn(item)
    calls = [items[i % len(items)] for i in range(n_calls)]
    times = []
    for _ in range(_STEADY_PASSES):
        start = time.perf_counter()
        for item in calls:
            fn(item)
        times.append(time.perf_counter() - start)
    return round(min(times) / len(calls) * 1e6, 3)


def run_bench(engine: str, bhsa_tf: Path, cache: Path) -> dict[str, Any]:
    """Execute the full benchmark for one engine and return a results dict."""
    res: dict[str, Any] = {"engine": engine}

    res["rss_before_load_mb"] = round(_peak_rss_mb(), 1)
    t0 = time.time()
    api = _load_engine(engine, bhsa_tf, cache)
    res["load_s"] = round(time.time() - t0, 3)
    res["rss_after_load_mb"] = round(_peak_rss_mb(), 1)
    res["rss_load_delta_mb"] = round(res["rss_after_load_mb"] - res["rss_before_load_mb"], 1)

    F, E, T, L, N, S = api.F, api.E, api.T, api.L, api.N, api.S
    max_slot = F.otype.maxSlot
    rng = random.Random(42)
    words = [rng.randint(1, max_slot) for _ in range(_N_WORD_CALLS)]
    phrases = [rng.randint(*_PHRASES) for _ in range(2000)]
    verses = [rng.randint(*_VERSES) for _ in range(1000)]

    def first_call(name: str, fn: Callable[[Any], Any], items: list[Any]) -> None:
        start = time.time()
        for item in items:
            fn(item)
        elapsed = time.time() - start
        res[name] = {
            "total_s": round(elapsed, 4),
            "per_call_us": round(elapsed / len(items) * 1e6, 2),
            "n": len(items),
        }

    first_call("F.sp.v", F.sp.v, words)
    first_call("F.g_word.v", F.g_word.v, words)
    first_call("L.u", L.u, words[:_N_LOCALITY])
    first_call("L.d", L.d, phrases[:_N_LOCALITY])
    first_call("L.n", L.n, words[:_N_LOCALITY])
    first_call("L.p", L.p, words[:_N_LOCALITY])
    first_call("L.i", L.i, phrases[:_N_LOCALITY])
    first_call("E.mother.f", E.mother.f, phrases[:_N_EDGE])
    first_call("E.mother.t", E.mother.t, phrases[:_N_EDGE])
    first_call("E.mother.b", E.mother.b, phrases[:_N_EDGE])
    first_call("T.sectionFromNode", T.sectionFromNode, words[:_N_SECTION])
    first_call("T.text.verse", T.text, verses[:_N_VERSE])
    sections = [tuple(T.sectionFromNode(verse)) for verse in verses]
    first_call("T.nodeFromSection", T.nodeFromSection, sections[:_N_VERSE])

    start = time.time()
    F.sp.freqList()
    res["F.sp.freqList_s"] = round(time.time() - start, 4)

    start = time.time()
    value = F.sp.v
    for node in range(1, max_slot + 1):
        value(node)
    res["F.sp.v_full_sweep_s"] = round(time.time() - start, 3)

    steady_probes: list[tuple[str, Callable[[Any], Any], list[Any]]] = [
        ("F.sp.v", F.sp.v, words),
        ("F.g_word.v", F.g_word.v, words),
        ("L.u", L.u, words),
        ("L.d", L.d, phrases),
        ("L.n", L.n, words),
        ("L.p", L.p, words),
        ("L.i", L.i, phrases),
        ("E.mother.f", E.mother.f, phrases),
        ("E.mother.t", E.mother.t, phrases),
        ("E.mother.b", E.mother.b, phrases),
        ("T.sectionFromNode", T.sectionFromNode, words),
        ("T.text.verse", T.text, verses),
        ("T.nodeFromSection", T.nodeFromSection, sections),
    ]
    res["steady_state_us"] = {
        name: steady_per_call_us(fn, items) for name, fn, items in steady_probes
    }

    walk_times = []
    for _ in range(_WALK_PASSES):
        start = time.perf_counter()
        for _node in N.walk():
            pass
        walk_times.append(time.perf_counter() - start)
    res["N.walk_full_s"] = round(min(walk_times), 4)

    qres: dict[str, dict[str, Any]] = {}
    for qid, template in _query_templates().items():
        start = time.time()
        rows = list(S.search(template))
        qres[qid] = {"s": round(time.time() - start, 4), "count": len(rows)}
    res["queries"] = qres
    res["rss_final_mb"] = round(_peak_rss_mb(), 1)
    return res


def assert_targets(
    cf: dict[str, Any],
    targets: dict[str, Any],
    baseline: dict[str, Any],
) -> list[str]:
    """Return human-readable target violations of a CF run (empty == all pass).

    ``targets`` keys (all optional):

    * ``per_call_us``: first-call probe -> ceiling (µs/call).
    * ``steady_state_us``: steady-state probe -> absolute ceiling (µs/call).
    * ``steady_state_vs_tf``: ``{"max_ratio": r, "overrides": {probe: r}}``;
      every steady-state probe (and ``N.walk_full_s``) must stay within ``r``
      times the TF baseline's value. A probe without a TF value fails.
    * ``rss_load_delta_mb`` / ``rss_final_mb``: memory ceilings (MB).
    * ``queries_le_tf`` with ``query_max_ratio_vs_tf`` (default 1.0): every gate
      query must run within that ratio of the TF baseline's wall time.
    """
    failures: list[str] = []

    for name, limit in targets.get("per_call_us", {}).items():
        probe = cf.get(name)
        if not isinstance(probe, dict) or "per_call_us" not in probe:
            failures.append(f"{name}: missing from cf run")
            continue
        actual = probe["per_call_us"]
        if actual > limit:
            failures.append(f"{name}: {actual}µs/call > target {limit}µs/call")

    steady = cf.get("steady_state_us", {})
    for name, limit in targets.get("steady_state_us", {}).items():
        actual = steady.get(name)
        if actual is None:
            failures.append(f"steady {name}: missing from cf run")
        elif actual > limit:
            failures.append(f"steady {name}: {actual}µs/call > ceiling {limit}µs/call")

    vs_tf = targets.get("steady_state_vs_tf")
    if vs_tf is not None:
        overrides = vs_tf.get("overrides", {})
        tf_steady = dict(baseline.get("steady_state_us", {}))
        cf_steady = dict(steady)
        if "N.walk_full_s" in cf:
            cf_steady["N.walk_full_s"] = cf["N.walk_full_s"]
        if "N.walk_full_s" in baseline:
            tf_steady["N.walk_full_s"] = baseline["N.walk_full_s"]
        for name, actual in cf_steady.items():
            ratio_limit = overrides.get(name, vs_tf["max_ratio"])
            reference = tf_steady.get(name)
            if reference is None:
                failures.append(f"steady {name}: no TF baseline value to compare with")
            elif actual > reference * ratio_limit:
                failures.append(
                    f"steady {name}: cf {actual} > {ratio_limit} x tf {reference}"
                )

    for key in ("rss_load_delta_mb", "rss_final_mb"):
        limit = targets.get(key)
        if limit is None:
            continue
        actual = cf.get(key)
        if actual is None:
            failures.append(f"{key}: missing from cf run")
        elif actual > limit:
            failures.append(f"{key}: {actual}MB > target {limit}MB")

    if targets.get("queries_le_tf"):
        ratio_limit = targets.get("query_max_ratio_vs_tf", 1.0)
        tf_queries = baseline.get("queries", {})
        for qid, cf_q in cf.get("queries", {}).items():
            tf_q = tf_queries.get(qid)
            if tf_q is None:
                failures.append(f"query {qid}: no TF baseline time to compare with")
            elif cf_q["s"] > tf_q["s"] * ratio_limit:
                failures.append(
                    f"query {qid}: cf {cf_q['s']}s > {ratio_limit} x tf {tf_q['s']}s"
                )

    return failures


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="cfabric_benchmarks.perf_gate",
        description="TF-vs-CF performance gate for BHSA.",
    )
    parser.add_argument("--engine", choices=["cf", "tf"], required=True)
    parser.add_argument("--out", type=Path, help="write results JSON to this path")
    parser.add_argument(
        "--bhsa-tf", type=Path, default=DEFAULT_BHSA_TF, help="BHSA TF source dir"
    )
    parser.add_argument(
        "--cache", type=Path, default=DEFAULT_CACHE, help="compiled .cfr cache (cf)"
    )
    parser.add_argument(
        "--assert",
        dest="do_assert",
        action="store_true",
        help="compare a cf run against targets + TF baseline",
    )
    parser.add_argument("--targets", type=Path, default=DEFAULT_TARGETS)
    parser.add_argument("--baseline", type=Path, default=DEFAULT_BASELINE)
    parser.add_argument(
        "--strict",
        action="store_true",
        help="exit non-zero when --assert finds violations",
    )
    args = parser.parse_args(argv)

    res = run_bench(args.engine, args.bhsa_tf, args.cache)
    text = json.dumps(res, indent=1)
    print(text)
    if args.out:
        args.out.write_text(text + "\n")

    if args.do_assert:
        if args.engine != "cf":
            print("\n--assert only applies to --engine cf", file=sys.stderr)
            return 2
        with open(args.targets) as handle:
            targets = json.load(handle)
        with open(args.baseline) as handle:
            baseline = json.load(handle)
        failures = assert_targets(res, targets, baseline)
        if failures:
            print(f"\nPERF GATE: {len(failures)} target violation(s):")
            for failure in failures:
                print(f"  FAIL {failure}")
            if args.strict:
                return 1
            print("(non-strict: not failing the build)")
        else:
            print("\nPERF GATE: all targets met")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
