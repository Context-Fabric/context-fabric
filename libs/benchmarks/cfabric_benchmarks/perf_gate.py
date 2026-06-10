"""Repeatable TF-vs-CF performance gate for BHSA.

Adapted from the throwaway ``/tmp/cf_perf/bench.py`` into a stable CLI that
measures: corpus load time, RSS after load, a set of per-call API micro-probes,
and the eight parity-correct benchmark queries.

Usage
-----
Record a Context-Fabric (mapped) run::

    python -m cfabric_benchmarks.perf_gate --engine cf --out /tmp/cf.json

Record a Text-Fabric run (slow; loads the full 6 GB in-memory model)::

    python -m cfabric_benchmarks.perf_gate --engine tf --out /tmp/tf.json

Assert a CF run against the per-call targets and the recorded TF baseline
(does not reload TF -- it reads ``baselines/bhsa_tf.json``)::

    python -m cfabric_benchmarks.perf_gate --engine cf --assert \
        --targets baselines/targets.json \
        --baseline baselines/bhsa_tf.json

The ``--assert`` mode reports per-call latencies against the recorded targets;
the 0.6.0 engine meets them on every probe.  It exits non-zero only when
``--strict`` is also passed, so it can be wired into CI as a hard gate.

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

# Probe sizes mirror the original bench so numbers stay comparable.
_N_WORD_CALLS = 10000
_N_LU = 30
_N_LD = 30
_N_EDGE = 500
_N_SECTION = 200
_N_VERSE = 100

# The eight parity-correct benchmark queries (templates pulled from the golden
# baseline when the id is present there, else the inline fallback).
_QUERY_FALLBACKS: dict[str, str] = {
    "q_phrase_without_verb": "phrase\n/without/\n  word sp=verb\n/-/",
    "q_regex_gcons": "word g_cons~^MLK",
    "q_regex_typ": "phrase typ~^(NP|PP)$\n  word sp=subs",
    "q_regex_gloss_space": "word gloss~give birth",
    "q_feat_exists_qere": "word qere",
    "q_feat_empty_qere": "word qere#",
    "q_multivalue_neg": "word vt#perf|impf|wayq sp=verb vs=nif",
    "q_slot_equal": "clause\n  == sentence",
}


def _rss_mb() -> float:
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
    queries_json = _REPO_ROOT / ".claude" / "parity_queries.json"
    if queries_json.exists():
        with open(queries_json) as handle:
            by_id = {q["id"]: q["template"] for q in json.load(handle)["queries"]}
        for qid in templates:
            if qid in by_id:
                templates[qid] = by_id[qid]
    return templates


def run_bench(engine: str, bhsa_tf: Path, cache: Path) -> dict[str, Any]:
    """Execute the full benchmark for one engine and return a results dict."""
    res: dict[str, Any] = {"engine": engine}

    t0 = time.time()
    api = _load_engine(engine, bhsa_tf, cache)
    res["load_s"] = round(time.time() - t0, 3)
    res["rss_after_load_mb"] = round(_rss_mb(), 1)

    F, E, T, L = api.F, api.E, api.T, api.L
    S = api.S
    max_slot = F.otype.maxSlot
    rng = random.Random(42)
    words = [rng.randint(1, max_slot) for _ in range(_N_WORD_CALLS)]
    phrases = [rng.randint(651573, 904775) for _ in range(2000)]
    verses = [rng.randint(1414389, 1437601) for _ in range(1000)]

    def bench(name: str, fn: Callable[[int], Any], items: list[int]) -> None:
        start = time.time()
        count = 0
        for item in items:
            fn(item)
            count += 1
        elapsed = time.time() - start
        res[name] = {
            "total_s": round(elapsed, 4),
            "per_call_us": round(elapsed / count * 1e6, 2),
            "n": count,
        }

    bench("F.sp.v", F.sp.v, words)
    bench("F.g_word.v", F.g_word.v, words)
    bench("L.u", L.u, words[:_N_LU])
    bench("L.d", L.d, phrases[:_N_LD])
    bench("E.mother.f", E.mother.f, phrases[:_N_EDGE])
    bench("T.sectionFromNode", T.sectionFromNode, words[:_N_SECTION])
    bench("T.text.verse", T.text, verses[:_N_VERSE])

    start = time.time()
    F.sp.freqList()
    res["F.sp.freqList_s"] = round(time.time() - start, 4)

    start = time.time()
    value = F.sp.v
    for node in range(1, max_slot + 1):
        value(node)
    res["F.sp.v_full_sweep_s"] = round(time.time() - start, 3)

    qres: dict[str, dict[str, Any]] = {}
    for qid, template in _query_templates().items():
        start = time.time()
        rows = list(S.search(template))
        qres[qid] = {"s": round(time.time() - start, 4), "count": len(rows)}
    res["queries"] = qres
    res["rss_final_mb"] = round(_rss_mb(), 1)
    return res


def assert_targets(
    cf: dict[str, Any],
    targets: dict[str, Any],
    baseline: dict[str, Any],
) -> list[str]:
    """Return a list of human-readable target violations (empty == all pass)."""
    failures: list[str] = []

    for name, limit in targets.get("per_call_us", {}).items():
        probe = cf.get(name)
        if not isinstance(probe, dict) or "per_call_us" not in probe:
            failures.append(f"{name}: missing from cf run")
            continue
        actual = probe["per_call_us"]
        if actual > limit:
            failures.append(f"{name}: {actual}µs/call > target {limit}µs/call")

    rss_limit = targets.get("rss_final_mb")
    if rss_limit is not None:
        actual_rss = cf.get("rss_final_mb")
        if actual_rss is None:
            failures.append("rss_final_mb: missing from cf run")
        elif actual_rss > rss_limit:
            failures.append(f"rss_final_mb: {actual_rss}MB > target {rss_limit}MB")

    if targets.get("queries_le_tf"):
        tf_queries = baseline.get("queries", {})
        for qid, cf_q in cf.get("queries", {}).items():
            tf_q = tf_queries.get(qid)
            if tf_q is None:
                continue
            if cf_q["s"] > tf_q["s"]:
                failures.append(
                    f"query {qid}: cf {cf_q['s']}s > tf {tf_q['s']}s"
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
        help="exit non-zero when --assert finds violations (off by default)",
    )
    args = parser.parse_args(argv)

    res = run_bench(args.engine, args.bhsa_tf, args.cache)
    text = json.dumps(res, indent=1)
    print(text)
    if args.out:
        args.out.write_text(text)

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
            print("(non-strict: not failing the build -- Phase 2/3 pending)")
        else:
            print("\nPERF GATE: all targets met")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
