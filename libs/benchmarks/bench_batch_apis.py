"""Batch-API benchmark for context-fabric (latency workstream W4).

Measures the additive batch methods added in W4 against (a) the equivalent
context-fabric *per-call* Python loop and (b) the text-fabric per-call Python
loop, for 100k-node batches:

  * ``F.<feat>.vs(nodes)``            vs ``[F.<feat>.v(n) for n in nodes]``
  * ``F.<feat>.vs_array(nodes)``      (int feature -> numpy int64; no TF analogue)
  * ``L.u_many(nodes)`` / ``d_many``  vs ``[L.u(n) for n in nodes]``
  * ``T.text_many(nodes)``           vs ``[T.text(n) for n in nodes]``

Goal: the batch path is >=10x faster than the CF per-call loop on 100k nodes.

Run from ``/tmp`` so the source tree does not shadow the installed extension::

    source .venv/bin/activate
    cd /tmp && python /Users/cody/github/context-fabric/libs/benchmarks/bench_batch_apis.py

The BHSA cache is compiled fresh on first run (~60s) to ``/tmp/cf_bhsa_w4.cfr``::

    Fabric(locations='.../bhsa/tf/2021').compile('/tmp/cf_bhsa_w4.cfr')

text-fabric comparison: by default the per-call TF latencies are taken from
``baselines/bhsa_tf.json`` (measured once, see that file) and extrapolated to the
batch size (``tf_loop_s = per_call_us * 1e-6 * N``); pass ``--live-tf`` to load
text-fabric in-process and time the real Python loops instead.
"""

from __future__ import annotations

import argparse
import json
import os
import random
import statistics
import time
from pathlib import Path

CACHE = "/tmp/cf_bhsa_w4.cfr"
TF_LOCATION = "/Users/cody/github/etcbc/bhsa/tf/2021"
BASELINE = Path(__file__).resolve().parent / "baselines" / "bhsa_tf.json"
N = 100_000
PASSES = 5


def measure(fn, *, passes: int = PASSES) -> float:
    """Median wall-clock seconds for a zero-arg callable over ``passes`` runs."""
    times = []
    for _ in range(passes):
        start = time.perf_counter()
        fn()
        times.append(time.perf_counter() - start)
    return statistics.median(times)


def ensure_cache() -> None:
    if os.path.exists(CACHE):
        return
    from cfabric import Fabric

    print(f"compiling {CACHE} from {TF_LOCATION} ...")
    t0 = time.perf_counter()
    Fabric(locations=TF_LOCATION, silent="deep").compile(CACHE)
    print(f"  compiled in {time.perf_counter() - t0:.1f}s")


def nodes_of_type(F, otype: str, max_node: int) -> list[int]:
    return [n for n in range(1, max_node + 1) if F.otype.v(n) == otype]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--live-tf", action="store_true", help="time real TF loops")
    args = parser.parse_args()

    from cfabric._core import Fabric

    ensure_cache()
    baseline = json.loads(BASELINE.read_text())

    t0 = time.perf_counter()
    corpus = Fabric().openMapped(CACHE)
    F, L, T = corpus.F, corpus.L, corpus.T
    max_node = F.otype.maxNode
    max_slot = F.otype.maxSlot
    print(f"loaded {CACHE} in {time.perf_counter() - t0:.2f}s  "
          f"maxSlot={max_slot} maxNode={max_node}")

    rng = random.Random(20260610)
    words = [rng.randint(1, max_slot) for _ in range(N)]
    # Container nodes for d_many (slots have no embeddees). Sample from clauses.
    clauses = nodes_of_type(F, "clause", max_node)
    clause_sample = [rng.choice(clauses) for _ in range(N)] if clauses else words
    # Verses for text_many at the section grain.
    verses = nodes_of_type(F, "verse", max_node)
    verse_sample = [rng.choice(verses) for _ in range(min(N, 20_000))] if verses else words

    rows: list[tuple[str, int, float, float, float | None]] = []

    def bench(label, n, batch_fn, loop_fn, tf_per_call_us=None, tf_loop_fn=None):
        batch_s = measure(batch_fn)
        loop_s = measure(loop_fn)
        if tf_loop_fn is not None:
            tf_s: float | None = measure(tf_loop_fn)
        elif tf_per_call_us is not None:
            tf_s = tf_per_call_us * 1e-6 * n
        else:
            tf_s = None
        rows.append((label, n, batch_s, loop_s, tf_s))

    # The CF per-call loop is written idiomatically (`[F.x.v(n) for n in ...]`),
    # i.e. exactly the loop the batch API replaces and exactly how the TF loop is
    # written, so the comparison is apples-to-apples.

    # ---- F.<feat>.vs (string feature: sp) ----
    bench("F.sp.vs (str)", N,
          lambda: F.sp.vs(words),
          lambda: [F.sp.v(n) for n in words],
          tf_per_call_us=baseline.get("F.sp.v", {}).get("per_call_us"))

    # ---- F.<feat>.vs (int feature: number) ----
    bench("F.number.vs (int)", N,
          lambda: F.number.vs(words),
          lambda: [F.number.v(n) for n in words])

    # ---- F.<feat>.vs_array (int feature -> numpy) ----
    bench("F.number.vs_array", N,
          lambda: F.number.vs_array(words),
          lambda: [F.number.v(n) for n in words])

    # ---- L.u_many ----
    bench("L.u_many", N,
          lambda: L.u_many(words),
          lambda: [L.u(n) for n in words],
          tf_per_call_us=baseline.get("L.u", {}).get("per_call_us"))

    # ---- L.d_many (on clauses) ----
    bench("L.d_many (clauses)", len(clause_sample),
          lambda: L.d_many(clause_sample),
          lambda: [L.d(n) for n in clause_sample],
          tf_per_call_us=baseline.get("L.d", {}).get("per_call_us"))

    # ---- T.text_many (words) ----
    bench("T.text_many (words)", N,
          lambda: T.text_many(words),
          lambda: [T.text(n) for n in words])

    # ---- T.text_many (verses) ----
    bench("T.text_many (verses)", len(verse_sample),
          lambda: T.text_many(verse_sample),
          lambda: [T.text(n) for n in verse_sample],
          tf_per_call_us=baseline.get("T.text.verse", {}).get("per_call_us"))

    # Optional: real TF loops (override the extrapolated estimates).
    if args.live_tf:
        rows = _with_live_tf(rows, words, clause_sample, verse_sample)

    _print_table(rows)
    return 0


def _with_live_tf(rows, words, clause_sample, verse_sample):
    try:
        from tf.fabric import Fabric as TFabric
    except Exception as exc:  # pragma: no cover - optional path
        print(f"--live-tf requested but text-fabric unavailable: {exc}")
        return rows
    print("loading text-fabric (live) ...")
    TF = TFabric(locations=[TF_LOCATION], silent="deep")
    api = TF.load("otype sp number", silent="deep")
    Ft, Lt, Tt = api.F, api.L, api.T
    spv, numv, u, d, text = Ft.sp.v, Ft.number.v, Lt.u, Lt.d, Tt.text
    live = {
        "F.sp.vs (str)": lambda: [spv(n) for n in words],
        "L.u_many": lambda: [u(n) for n in words],
        "L.d_many (clauses)": lambda: [d(n) for n in clause_sample],
        "T.text_many (verses)": lambda: [text(n) for n in verse_sample],
    }
    out = []
    for label, n, batch_s, loop_s, tf_s in rows:
        if label in live:
            tf_s = measure(live[label])
        out.append((label, n, batch_s, loop_s, tf_s))
    return out


def _print_table(rows):
    print()
    header = f"{'api':<24}{'N':>8}{'batch ms':>11}{'cf loop ms':>12}" \
             f"{'tf loop ms':>12}{'vs cf':>9}{'vs tf':>9}"
    print(header)
    print("-" * len(header))
    for label, n, batch_s, loop_s, tf_s in rows:
        cf_ratio = loop_s / batch_s if batch_s else float("nan")
        if tf_s is not None:
            tf_ratio = f"{tf_s / batch_s:>8.1f}x"
            tf_ms = f"{tf_s * 1e3:>11.2f}"
        else:
            tf_ratio = f"{'-':>9}"
            tf_ms = f"{'-':>12}"
        print(f"{label:<24}{n:>8}{batch_s * 1e3:>11.2f}{loop_s * 1e3:>12.2f}"
              f"{tf_ms}{cf_ratio:>8.1f}x{tf_ratio}")
    print()
    print("vs cf = CF per-call loop / CF batch  (target >= 10x)")
    print("vs tf = TF per-call loop / CF batch  (tf loop extrapolated from "
          "baselines/bhsa_tf.json unless --live-tf)")


if __name__ == "__main__":
    raise SystemExit(main())
