#!/usr/bin/env python3
"""Compare Text-Fabric and Rust mapped BHSA subset timings."""

from __future__ import annotations

import argparse
import math
import os
import re
import subprocess
import sys
from pathlib import Path


LOAD_RE = re.compile(r"load_ms=(?P<load_ms>[0-9.]+)")
QUERY_RE = re.compile(
    r"ok query=(?P<query>\S+) results=(?P<results>\d+) elapsed_ms=(?P<elapsed_ms>[0-9.]+)"
)


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
    parser.add_argument("--min-query-geomean-speedup", type=float, default=7.0)
    parser.add_argument("--min-wins", type=int, default=95)
    parser.add_argument("--min-shared-queries", type=int, default=100)
    args = parser.parse_args()

    env = os.environ.copy()
    python_path = str(core_root)
    env["PYTHONPATH"] = (
        f"{python_path}:{env['PYTHONPATH']}" if env.get("PYTHONPATH") else python_path
    )

    text_fabric_output = run_command(
        [
            sys.executable,
            "scripts/text_fabric_bhsa_mapped_subset.py",
            str(args.tf_path),
            str(args.limit),
        ],
        cwd=core_root,
        env=env,
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

    text_fabric = parse_timings(text_fabric_output)
    rust = parse_timings(rust_output)
    shared = sorted(set(text_fabric.queries) & set(rust.queries))
    if len(shared) < args.min_shared_queries:
        raise RuntimeError(f"shared query count {len(shared)} is below {args.min_shared_queries}")

    speedups = [text_fabric.queries[query] / rust.queries[query] for query in shared]
    geomean = math.prod(speedups) ** (1 / len(speedups))
    wins = sum(speedup > 1.0 for speedup in speedups)

    print("text_fabric")
    print(text_fabric_output.rstrip())
    print()
    print("rust")
    print(rust_output.rstrip())
    print()
    print("speedups")
    print(
        f"load_speedup={text_fabric.load_ms / rust.load_ms:.2f}x "
        f"text_fabric_ms={text_fabric.load_ms:.3f} rust_ms={rust.load_ms:.3f}"
    )
    print(f"query_geomean_speedup={geomean:.2f}x wins={wins}/{len(shared)}")

    failures = []
    if geomean < args.min_query_geomean_speedup:
        failures.append(
            f"query geometric mean speedup {geomean:.2f}x is below "
            f"{args.min_query_geomean_speedup:.2f}x"
        )
    if wins < args.min_wins:
        failures.append(f"Rust wins {wins}/{len(shared)} is below {args.min_wins}")
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
    load_match = LOAD_RE.search(output)
    if load_match is None:
        raise RuntimeError(f"missing load_ms in output:\n{output}")
    queries = {
        match.group("query"): float(match.group("elapsed_ms"))
        for match in QUERY_RE.finditer(output)
    }
    return Timings(load_ms=float(load_match.group("load_ms")), queries=queries)


if __name__ == "__main__":
    raise SystemExit(main())
