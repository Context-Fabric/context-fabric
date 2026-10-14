#!/usr/bin/env python3
"""Load time, memory and per-call latency probe for one engine on one corpus.

One invocation measures one engine in a fresh process and writes a JSON record:

* ``load``: wall time of the full-corpus load. With ``--cold`` the engine cache
  is deleted first, so the time includes building the cache from ``.tf`` files.
* ``memory``: RSS / USS / peak RSS before load, after load, after the per-call
  probes and after the query workload.
* ``gate``: the ``cfabric_benchmarks.perf_gate`` micro-probes reproduced exactly
  (same seed, node sets, call counts and ``time.time`` clock), so the numbers
  are directly comparable with ``baselines/cf_0.6.0_record.json`` and
  ``baselines/bhsa_tf.json``. These include first-call costs over tiny ``n``.
* ``steady``: the same APIs after a warm-up pass, median per-call time of
  several passes of many calls (comparable with the June record's
  ``steady_state_us``), plus ``L.n``/``L.p``/``L.i``, edge ``t``/``b``,
  ``T.nodeFromSection`` and a full ``N.walk``.
* ``queries``: the workload search templates (BHSA: the eight ``perf_gate``
  queries) with wall time and result count.

With ``--load-only`` only the load and memory sections are produced.

Usage::

    cd /tmp && python perf_probe.py --engine cf_rust --corpus bhsa \
        --tf-path /tmp/cf_rc1/corpora/bhsa/tf --out bhsa_cf_rust.json
"""

from __future__ import annotations

import argparse
import json
import platform
import random
import statistics
import sys
import time
from pathlib import Path
from typing import Any, Callable

sys.path.insert(0, str(Path(__file__).resolve().parent))

import engines  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[4]
PARITY_QUERIES_JSON = REPO_ROOT / "libs" / "core" / "tests" / "parity" / "parity_queries.json"

# perf_gate probe sizes (kept identical so gate numbers stay comparable).
GATE_N_WORD = 10_000
GATE_N_LU = 30
GATE_N_LD = 30
GATE_N_EDGE = 500
GATE_N_SECTION = 200
GATE_N_VERSE = 100

STEADY_CALLS = 50_000
STEADY_PASSES = 5
SLOW_CALL_S = 20e-6
SLOW_PASS_BUDGET_S = 1.0
MIN_SLOW_CALLS = 100
WALK_PASSES = 3

# Corpus profiles: which node types / features stand in for the BHSA probes.
PROFILES: dict[str, dict[str, Any]] = {
    "bhsa": {
        "mid_type": "phrase",
        "small_feature": "sp",
        "large_feature": "g_word",
        "edge_feature": "mother",
        # The perf_gate queries, templates from parity_queries.json.
        "query_ids": [
            "q_phrase_without_verb",
            "q_regex_gcons",
            "q_regex_typ",
            "q_regex_gloss_space",
            "q_feat_exists_qere",
            "q_feat_empty_qere",
            "q_multivalue_neg",
            "q_slot_equal",
            "q_mother_basic",
            "q_mother_reverse",
            "q_mother_named_distinct_verse",
            "q_slot_overlap_parent",
        ],
    },
    "n1904": {
        "mid_type": "phrase",
        "small_feature": "sp",
        "large_feature": "unicode",
        "edge_feature": "parent",
        "queries": {
            "n_word_verb": "word sp=verb",
            "n_clause_phrase_word": "clause\n  phrase\n    word sp=subs",
            "n_regex_lemma": "word lemma~^λεγ",
            "n_sentence_without_verb": "sentence\n/without/\n  word sp=verb\n/-/",
        },
    },
}


def profile_queries(profile: dict[str, Any]) -> dict[str, str]:
    if "queries" in profile:
        return dict(profile["queries"])
    by_id = {q["id"]: q["template"] for q in json.loads(PARITY_QUERIES_JSON.read_text())["queries"]}
    return {qid: by_id[qid] for qid in profile["query_ids"]}


def type_interval(api: Any, node_type: str) -> tuple[int, int]:
    """First and last node of ``node_type`` (all engines keep types contiguous)."""
    for row in api.C.levels.data:
        if row[0] == node_type:
            return int(row[2]), int(row[3])
    raise KeyError(node_type)


def gate_probes(api: Any, profile: dict[str, Any]) -> tuple[dict[str, Any], dict[str, list[int]]]:
    """Replicate ``perf_gate.run_bench``'s per-call probes exactly."""
    F, E, T, L = api.F, api.E, api.T, api.L
    max_slot = F.otype.maxSlot
    mid_first, mid_last = type_interval(api, profile["mid_type"])
    verse_type = list(T.sectionTypes)[-1]
    verse_first, verse_last = type_interval(api, verse_type)
    rng = random.Random(42)
    words = [rng.randint(1, max_slot) for _ in range(GATE_N_WORD)]
    mids = [rng.randint(mid_first, mid_last) for _ in range(2000)]
    verses = [rng.randint(verse_first, verse_last) for _ in range(1000)]

    small = getattr(F, profile["small_feature"])
    large = getattr(F, profile["large_feature"])
    edge = getattr(E, profile["edge_feature"])
    out: dict[str, Any] = {
        "_intervals": {
            "max_slot": max_slot,
            profile["mid_type"]: [mid_first, mid_last],
            verse_type: [verse_first, verse_last],
        }
    }

    def bench(name: str, fn: Callable[[Any], Any], items: list[Any]) -> None:
        start = time.time()
        for item in items:
            fn(item)
        elapsed = time.time() - start
        out[name] = {
            "total_s": round(elapsed, 4),
            "per_call_us": round(elapsed / len(items) * 1e6, 2),
            "n": len(items),
        }

    sf, lf, ef = profile["small_feature"], profile["large_feature"], profile["edge_feature"]
    bench(f"F.{sf}.v", small.v, words)
    bench(f"F.{lf}.v", large.v, words)
    bench("L.u", L.u, words[:GATE_N_LU])
    bench("L.d", L.d, mids[:GATE_N_LD])
    bench(f"E.{ef}.f", edge.f, mids[:GATE_N_EDGE])
    bench("T.sectionFromNode", T.sectionFromNode, words[:GATE_N_SECTION])
    bench("T.text.verse", T.text, verses[:GATE_N_VERSE])
    sections = [tuple(T.sectionFromNode(v)) for v in verses[:GATE_N_VERSE]]
    bench("T.nodeFromSection", T.nodeFromSection, sections)

    start = time.time()
    small.freqList()
    out[f"F.{sf}.freqList_s"] = round(time.time() - start, 4)

    start = time.time()
    value = small.v
    for node in range(1, max_slot + 1):
        value(node)
    out[f"F.{sf}.v_full_sweep_s"] = round(time.time() - start, 3)
    return out, {"words": words, "mids": mids, "verses": verses}


def steady_per_call_us(fn: Callable[[Any], Any], items: list[Any]) -> float:
    """Median per-call microseconds after a warm-up pass over ``items``.

    Fast APIs get ``STEADY_CALLS`` calls per pass (as in the June record). APIs
    slower than ``SLOW_CALL_S`` per call get a time-budgeted pass of at least
    ``MIN_SLOW_CALLS`` calls so that millisecond-scale calls stay measurable.
    """
    warmup = items[: min(len(items), 200)]
    start = time.perf_counter()
    for item in warmup:
        fn(item)
    estimate = (time.perf_counter() - start) / len(warmup)
    n_calls = STEADY_CALLS
    if estimate > SLOW_CALL_S:
        n_calls = max(MIN_SLOW_CALLS, int(SLOW_PASS_BUDGET_S / estimate))
    else:
        for item in items:
            fn(item)
    calls = [items[i % len(items)] for i in range(n_calls)]
    times = []
    for _ in range(STEADY_PASSES):
        start = time.perf_counter()
        for item in calls:
            fn(item)
        times.append(time.perf_counter() - start)
    return round(statistics.median(times) / len(calls) * 1e6, 3)


def steady_probes(api: Any, profile: dict[str, Any], nodes: dict[str, list[int]]) -> dict[str, Any]:
    """Steady-state probes; an API that raises is recorded as an error string."""
    F, E, T, L, N = api.F, api.E, api.T, api.L, api.N
    words, mids, verses = nodes["words"], nodes["mids"], nodes["verses"]
    sections = [tuple(T.sectionFromNode(v)) for v in verses]
    sf, lf, ef = profile["small_feature"], profile["large_feature"], profile["edge_feature"]
    edge = getattr(E, ef)
    probes: list[tuple[str, Callable[[Any], Any], list[Any]]] = [
        (f"F.{sf}.v", getattr(F, sf).v, words),
        (f"F.{lf}.v", getattr(F, lf).v, words),
        ("L.u", L.u, words),
        ("L.d", L.d, mids),
        ("L.n", L.n, words),
        ("L.p", L.p, words),
        ("L.i", L.i, words),
        (f"E.{ef}.f", edge.f, mids),
        (f"E.{ef}.t", edge.t, mids),
        (f"E.{ef}.b", edge.b, mids),
        ("T.sectionFromNode", T.sectionFromNode, words),
        ("T.text.verse", T.text, verses),
        ("T.nodeFromSection", T.nodeFromSection, sections),
    ]
    out: dict[str, Any] = {}
    errors: dict[str, str] = {}
    for name, fn, items in probes:
        try:
            out[name] = steady_per_call_us(fn, items)
        except Exception as error:  # noqa: BLE001 - record and continue
            out[name] = None
            errors[name] = f"{type(error).__name__}: {error}"[:300]
    walk_times = []
    count = 0
    for _ in range(WALK_PASSES):
        start = time.perf_counter()
        count = 0
        for _node in N.walk():
            count += 1
        walk_times.append(time.perf_counter() - start)
    out["N.walk_full_s"] = round(statistics.median(walk_times), 4)
    out["N.walk_nodes"] = count
    if errors:
        out["_errors"] = errors
    return out


def query_workload(api: Any, queries: dict[str, str]) -> dict[str, Any]:
    out: dict[str, Any] = {}
    for qid, template in queries.items():
        start = time.time()
        try:
            rows = list(api.S.search(template))
            out[qid] = {"s": round(time.time() - start, 4), "count": len(rows)}
        except Exception as error:  # noqa: BLE001 - record and continue
            out[qid] = {"s": None, "count": None, "error": f"{type(error).__name__}: {error}"}
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--engine", choices=engines.ENGINES, required=True)
    parser.add_argument("--corpus", required=True, help="corpus name (selects probe profile)")
    parser.add_argument("--tf-path", type=Path, required=True)
    parser.add_argument("--mapped-cache", type=Path, help=".cfr file for cf_rust_mapped")
    parser.add_argument("--cold", action="store_true", help="delete the engine cache first")
    parser.add_argument("--load-only", action="store_true", help="skip probes and queries")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)

    engines.check_engine_identity(args.engine)
    record: dict[str, Any] = {
        "engine": args.engine,
        "engine_version": engines.engine_version(args.engine),
        "corpus": args.corpus,
        "tf_path": str(args.tf_path),
        "python": platform.python_version(),
        "mode": "cold" if args.cold else "warm",
        "memory": {"before_load": engines.memory_snapshot()},
    }
    if args.cold:
        record["cache_cleared"] = engines.clear_cache(args.engine, args.tf_path, args.mapped_cache)

    start = time.perf_counter()
    api = engines.load(args.engine, args.tf_path, args.mapped_cache)
    record["load_s"] = round(time.perf_counter() - start, 4)
    record["memory"]["after_load"] = engines.memory_snapshot()

    if not args.load_only:
        profile = PROFILES[args.corpus]
        gate, nodes = gate_probes(api, profile)
        record["gate"] = gate
        record["memory"]["after_gate_probes"] = engines.memory_snapshot()
        record["steady_us"] = steady_probes(api, profile, nodes)
        record["memory"]["after_steady_probes"] = engines.memory_snapshot()
        record["queries"] = query_workload(api, profile_queries(profile))
        record["memory"]["final"] = engines.memory_snapshot()

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps({k: record[k] for k in ("engine", "corpus", "mode", "load_s")}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
