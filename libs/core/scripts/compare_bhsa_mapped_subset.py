#!/usr/bin/env python3
"""Compare Python cfabric and Rust mapped BHSA subset timings."""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path


LOAD_RE = re.compile(r"load_ms=(?P<load_ms>[0-9.]+)")
QUERY_RE = re.compile(r"ok query=(?P<query>\S+) results=(?P<results>\d+) elapsed_ms=(?P<elapsed_ms>[0-9.]+)")


def main() -> int:
    repo_root = Path(__file__).resolve().parents[3]
    core_root = repo_root / "libs/core"
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--tf-path",
        type=Path,
        default=repo_root / "libs/benchmarks/.corpora/bhsa/tf",
    )
    parser.add_argument(
        "--cache-path",
        type=Path,
        default=core_root / "target/bhsa-mapped-curated.cfr",
    )
    parser.add_argument("--limit", type=int, default=5)
    parser.add_argument("--min-load-speedup", type=float, default=2.0)
    parser.add_argument("--min-query-geomean-speedup", type=float, default=2.0)
    parser.add_argument("--min-shared-queries", type=int, default=104)
    parser.add_argument(
        "--allow-slower-query",
        action="store_true",
        help="Do not fail if an individual shared query is slower in Rust.",
    )
    args = parser.parse_args()

    python_env = os.environ.copy()
    python_path = str(repo_root / "libs/core")
    if python_env.get("PYTHONPATH"):
        python_env["PYTHONPATH"] = f"{python_path}:{python_env['PYTHONPATH']}"
    else:
        python_env["PYTHONPATH"] = python_path

    python_output = run_command(
        [
            sys.executable,
            "scripts/python_bhsa_mapped_subset.py",
            str(args.tf_path),
            str(args.limit),
        ],
        cwd=core_root,
        env=python_env,
    )
    rust_output = run_command(
        [
            "cargo",
            "run",
            "--release",
            "--quiet",
            "--bin",
            "cf_rust_validate_bhsa_mapped_curated",
            "--",
            str(args.tf_path),
            str(args.cache_path),
            str(args.limit),
        ],
        cwd=core_root,
        env=os.environ.copy(),
    )

    python = parse_timings(python_output)
    rust = parse_timings(rust_output)
    print("python")
    print(python_output.rstrip())
    print()
    print("rust")
    print(rust_output.rstrip())
    print()
    print("speedups")
    load_speedup = python.load_ms / rust.load_ms
    print(
        f"load_speedup={load_speedup:.2f}x "
        f"python_ms={python.load_ms:.3f} rust_ms={rust.load_ms:.3f}"
    )

    common_queries = sorted(set(python.queries) & set(rust.queries))
    python_only_queries = sorted(set(python.queries) - set(rust.queries))
    rust_only_queries = sorted(set(rust.queries) - set(python.queries))
    slower_queries: list[tuple[str, float, float, float]] = []
    for query in common_queries:
        python_ms = python.queries[query]
        rust_ms = rust.queries[query]
        speedup = python_ms / rust_ms
        if speedup < 1.0:
            slower_queries.append((query, speedup, python_ms, rust_ms))
        print(
            f"query={query} speedup={speedup:.2f}x "
            f"python_ms={python_ms:.3f} rust_ms={rust_ms:.3f}"
        )

    if not common_queries:
        raise RuntimeError("no shared query timings found")
    geometric = 1.0
    for query in common_queries:
        geometric *= python.queries[query] / rust.queries[query]
    geometric **= 1.0 / len(common_queries)
    print(f"query_geomean_speedup={geometric:.2f}x queries={len(common_queries)}")
    failures = []
    if load_speedup < args.min_load_speedup:
        failures.append(
            f"load speedup {load_speedup:.2f}x is below "
            f"{args.min_load_speedup:.2f}x"
        )
    if geometric < args.min_query_geomean_speedup:
        failures.append(
            f"query geometric mean speedup {geometric:.2f}x is below "
            f"{args.min_query_geomean_speedup:.2f}x"
        )
    if len(common_queries) < args.min_shared_queries:
        failures.append(
            f"shared query count {len(common_queries)} is below "
            f"{args.min_shared_queries}"
        )
    if python_only_queries:
        failures.append(
            "queries present only in Python output: " + ", ".join(python_only_queries)
        )
    if rust_only_queries:
        failures.append(
            "queries present only in Rust output: " + ", ".join(rust_only_queries)
        )
    if slower_queries and not args.allow_slower_query:
        failures.append(
            "Rust was slower for individual queries: "
            + ", ".join(
                f"{query} ({speedup:.2f}x, python_ms={python_ms:.3f}, "
                f"rust_ms={rust_ms:.3f})"
                for query, speedup, python_ms, rust_ms in slower_queries
            )
        )
    if failures:
        raise RuntimeError("; ".join(failures))
    return 0


class Timings:
    def __init__(self, load_ms: float, queries: dict[str, float]) -> None:
        self.load_ms = load_ms
        self.queries = queries


def run_command(command: list[str], cwd: Path, env: dict[str, str]) -> str:
    completed = subprocess.run(
        command,
        cwd=cwd,
        env=env,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    return completed.stdout


def parse_timings(output: str) -> Timings:
    load_ms: float | None = None
    queries: dict[str, float] = {}
    for line in output.splitlines():
        if load_ms is None:
            load_match = LOAD_RE.search(line)
            if load_match:
                load_ms = float(load_match.group("load_ms"))
        query_match = QUERY_RE.search(line)
        if query_match:
            queries[query_match.group("query")] = float(query_match.group("elapsed_ms"))
    if load_ms is None:
        raise RuntimeError(f"missing load_ms in output:\n{output}")
    return Timings(load_ms=load_ms, queries=queries)


if __name__ == "__main__":
    raise SystemExit(main())
