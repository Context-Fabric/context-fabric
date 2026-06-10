"""Scalar-latency micro-benchmark for context-fabric ``F.<feat>.v(n)``.

Measures per-call latency of node-feature value lookups against the text-fabric
baseline (``libs/benchmarks/baselines/bhsa_tf.json``) to validate the
scalar-latency fast paths (pool-id string interning, handle-owned dispatch,
positional-only signature) added to ``bindings/features.rs``.

Run from ``/tmp`` so the local source tree does not shadow the installed
``cfabric`` extension::

    source .venv/bin/activate
    cd /tmp && python /Users/cody/github/context-fabric/libs/benchmarks/bench_scalar_latency.py

Methodology mirrors ``perf_gate.py`` (capture the bound method once, loop over a
seeded random list of nodes, ``per_call = elapsed / n``) so the numbers are
directly comparable to the TF baseline, but uses >=1M calls per pass and reports
the median across several passes. The pyo3 call floor is measured with a no-op
method (``F.<feat>._noop``) and subtracted to expose pure binding work.
"""

from __future__ import annotations

import json
import os
import random
import statistics
import sys
import time
from pathlib import Path

CACHE = "/tmp/cf_bhsa_v3_full.cfr"
BASELINE = Path(__file__).resolve().parent / "baselines" / "bhsa_tf.json"
N_CALLS = 1_000_000
PASSES = 9


def rss_mb() -> float:
    """Resident set size of this process in MB (portable-ish via psutil/ps)."""
    try:
        import resource

        usage = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
        # macOS reports bytes, Linux reports KiB.
        return usage / (1024 * 1024) if sys.platform == "darwin" else usage / 1024
    except Exception:
        return float("nan")


def measure(fn, nodes: list[int]) -> float:
    """Median per-call seconds over PASSES passes of len(nodes) calls each."""
    times: list[float] = []
    for _ in range(PASSES):
        start = time.perf_counter()
        for node in nodes:
            fn(node)
        times.append(time.perf_counter() - start)
    return statistics.median(times) / len(nodes)


def main() -> int:
    from cfabric._core import Fabric

    baseline = json.loads(BASELINE.read_text())

    t0 = time.perf_counter()
    corpus = Fabric().openMapped(CACHE)
    F = corpus.F
    load_s = time.perf_counter() - t0
    max_slot = F.otype.maxSlot
    print(f"loaded {CACHE} in {load_s:.2f}s  maxSlot={max_slot}")

    rng = random.Random(42)
    nodes = [rng.randint(1, max_slot) for _ in range(N_CALLS)]

    targets = [
        ("F.sp.v", "sp", "str ~14 distinct"),
        ("F.lex.v", "lex", "str (lexemes)"),
        ("F.g_word.v", "g_word", "str ~400k distinct"),
        ("F.number.v", "number", "int"),
    ]

    # Distinct value counts (informational; bounds the intern table size).
    distinct = {}
    for _, attr, _desc in targets:
        try:
            distinct[attr] = len(getattr(F, attr).freqList())
        except Exception:
            distinct[attr] = None

    # Warm pass: fault in mmap pages + populate intern caches before timing and
    # before the RSS snapshot, so the post-benchmark RSS delta reflects intern
    # growth during timing rather than first-touch page faults.
    for _, attr, _desc in targets:
        handle = getattr(F, attr)
        v = handle.v
        for node in nodes[:200_000]:
            v(node)
    # pyo3 call floor.
    noop = getattr(F, "sp")._noop
    noop_per_call = measure(noop, nodes)

    rss_warm = rss_mb()

    print()
    header = (
        f"{'bench':<12} {'distinct':>9} {'before µs':>10} {'after µs':>9} "
        f"{'CF-noop µs':>11} {'TF µs':>7} {'verdict':>9}"
    )
    print(header)
    print("-" * len(header))

    results = {}
    for label, attr, _desc in targets:
        handle = getattr(F, attr)  # stable handle (intern cache persists)
        v = handle.v
        legacy = handle._v_legacy
        # correctness: optimized path agrees with the legacy path and a fresh call
        assert handle.v(nodes[0]) == handle._v_legacy(nodes[0])
        assert handle.v(nodes[0]) == getattr(F, attr).v(nodes[0])
        before = measure(legacy, nodes) * 1e6
        per_call = measure(v, nodes)
        raw_us = per_call * 1e6
        adj_us = (per_call - noop_per_call) * 1e6
        tf = baseline.get(label, {})
        tf_us = tf.get("per_call_us")
        verdict = ""
        if tf_us is not None:
            verdict = "FASTER" if raw_us < tf_us else "slower"
        results[label] = {"before_us": before, "raw_us": raw_us, "tf_us": tf_us}
        tf_str = f"{tf_us:.2f}" if tf_us is not None else "  -  "
        dn = distinct.get(attr)
        dn_str = str(dn) if dn is not None else "?"
        print(
            f"{label:<12} {dn_str:>9} {before:>10.3f} {raw_us:>9.3f} "
            f"{adj_us:>11.3f} {tf_str:>7} {verdict:>9}"
        )

    print()
    print(f"pyo3 _noop floor: {noop_per_call * 1e6:.3f} µs/call")

    rss_after = rss_mb()
    print()
    print(f"RSS after warm+fill:     {rss_warm:.1f} MB")
    print(f"RSS after {PASSES * len(targets)}M timed calls: {rss_after:.1f} MB")
    print(f"RSS delta (intern during timing): {rss_after - rss_warm:.1f} MB  (must be < 50)")

    # Interning identity check: repeated equal values reuse the same object.
    sp = getattr(F, "sp")
    a = sp.v(nodes[0])
    same_val_node = next(n for n in nodes[1:] if sp.v(n) == a)
    b = sp.v(same_val_node)
    print()
    print(
        f"intern identity: F.sp.v reuse for equal value '{a}' -> "
        f"{'same object (interned)' if a is b else 'distinct objects'}"
    )

    # Rough intern-table memory estimate (bounded by min(distinct, 65536)).
    cap = 65_536
    print()
    print("intern-table bound per feature (entries):")
    for _, attr, _desc in targets:
        dn = distinct.get(attr)
        if dn is None:
            continue
        held = min(dn, cap)
        est_mb = held * 90 / (1024 * 1024)  # ~90B/entry (key+ptr+map+str obj)
        print(f"  {attr:<10} held<={held:<6} ~{est_mb:.2f} MB")

    print()
    ok = all(
        r["tf_us"] is None or r["raw_us"] < r["tf_us"] for r in results.values()
    )
    print("ALL TARGETS BEAT TF" if ok else "some targets slower than TF (see table)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
