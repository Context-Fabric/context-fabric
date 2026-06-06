#!/usr/bin/env python3
"""Compare mapped Python wheel timings against the raw Rust mapped binary."""

from __future__ import annotations

import argparse
import math
import os
import re
import subprocess
import sys
import time
from pathlib import Path

from cfabric import Fabric
from scripts.python_bhsa_mapped_subset import QUERIES


LOAD_RE = re.compile(r"load_ms=(?P<load_ms>[0-9.]+)")
QUERY_RE = re.compile(r"ok query=(?P<query>\S+) results=(?P<results>\d+) elapsed_ms=(?P<elapsed_ms>[0-9.]+)")
FEATURES = (
    "otype",
    "oslots",
    "sp",
    "vt",
    "vs",
    "gn",
    "nu",
    "ps",
    "language",
    "function",
    "typ",
    "kind",
    "domain",
)


def main() -> int:
    repo_root = Path(__file__).resolve().parents[3]
    core_root = repo_root / "libs/core"
    parser = argparse.ArgumentParser()
    parser.add_argument("--tf-path", type=Path, default=repo_root / "libs/benchmarks/.corpora/bhsa/tf")
    parser.add_argument("--cache-path", type=Path, default=core_root / "target/bhsa-mapped-curated.cfr")
    parser.add_argument("--limit", type=int, default=5)
    parser.add_argument("--max-load-overhead-ratio", type=float, default=1.2)
    parser.add_argument("--max-query-geomean-overhead-ratio", type=float, default=1.3)
    parser.add_argument("--min-shared-queries", type=int, default=100)
    args = parser.parse_args()

    if not args.cache_path.exists():
        Fabric(locations=str(args.tf_path), silent="deep").compile(
            str(args.cache_path),
            features=FEATURES,
        )

    wheel = run_wheel(args.cache_path, args.limit)
    rust = run_rust(core_root, args.tf_path, args.cache_path, args.limit)

    print("wheel")
    print(f"loaded mapped_features={len(FEATURES)} load_ms={wheel.load_ms:.3f}")
    for query, elapsed_ms in wheel.queries.items():
        print(f"ok query={query} elapsed_ms={elapsed_ms:.3f}")
    print()
    print("rust")
    print(rust.raw.rstrip())
    print()
    print("overhead")

    shared = sorted(set(wheel.queries) & set(rust.queries))
    if len(shared) < args.min_shared_queries:
        raise RuntimeError(f"shared query count {len(shared)} is below {args.min_shared_queries}")

    load_overhead = wheel.load_ms / rust.load_ms
    ratios = [wheel.queries[query] / rust.queries[query] for query in shared if rust.queries[query] > 0]
    query_geomean = math.prod(ratios) ** (1 / len(ratios))
    print(f"load_overhead={load_overhead:.3f}x wheel_ms={wheel.load_ms:.3f} rust_ms={rust.load_ms:.3f}")
    print(f"query_geomean_overhead={query_geomean:.3f}x queries={len(shared)}")

    failures = []
    if load_overhead > args.max_load_overhead_ratio:
        failures.append(
            f"load overhead {load_overhead:.3f} exceeds {args.max_load_overhead_ratio:.3f}"
        )
    if query_geomean > args.max_query_geomean_overhead_ratio:
        failures.append(
            f"query geomean overhead {query_geomean:.3f} exceeds "
            f"{args.max_query_geomean_overhead_ratio:.3f}"
        )
    if failures:
        raise RuntimeError("; ".join(failures))
    return 0


class Timings:
    def __init__(self, load_ms: float, queries: dict[str, float], raw: str = "") -> None:
        self.load_ms = load_ms
        self.queries = queries
        self.raw = raw


def run_wheel(cache_path: Path, limit: int) -> Timings:
    start = time.perf_counter()
    mapped = Fabric(silent="deep").openMapped(str(cache_path))
    load_ms = (time.perf_counter() - start) * 1000
    timings = {}
    for query_id, template in QUERIES:
        start = time.perf_counter()
        mapped.search(template, limit=limit)
        timings[query_id] = (time.perf_counter() - start) * 1000
    return Timings(load_ms, timings)


def run_rust(core_root: Path, tf_path: Path, cache_path: Path, limit: int) -> Timings:
    completed = subprocess.run(
        [
            "cargo",
            "run",
            "--release",
            "--quiet",
            "--bin",
            "cf_rust_validate_bhsa_mapped_curated",
            "--",
            str(tf_path),
            str(cache_path),
            str(limit),
        ],
        cwd=core_root,
        env=os.environ.copy(),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=True,
    )
    load_match = LOAD_RE.search(completed.stdout)
    if load_match is None:
        raise RuntimeError(f"missing load_ms in Rust output:\\n{completed.stdout}")
    queries = {
        match.group("query"): float(match.group("elapsed_ms"))
        for match in QUERY_RE.finditer(completed.stdout)
    }
    return Timings(float(load_match.group("load_ms")), queries, completed.stdout)


if __name__ == "__main__":
    raise SystemExit(main())
