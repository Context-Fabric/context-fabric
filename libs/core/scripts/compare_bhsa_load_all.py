#!/usr/bin/env python3
"""Compare Python cfabric and Rust BHSA load-all time and peak memory."""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path


LOAD_RE = re.compile(r"load_ms=(?P<load_ms>[0-9.]+)")
QUERY_RE = re.compile(r"ok query=(?P<query>\S+) results=(?P<results>\d+) elapsed_ms=(?P<elapsed_ms>[0-9.]+)")
MAC_RSS_RE = re.compile(r"(?P<rss>\d+)\s+maximum resident set size")
MAC_FOOTPRINT_RE = re.compile(r"(?P<rss>\d+)\s+peak memory footprint")
GNU_RSS_RE = re.compile(r"Maximum resident set size \(kbytes\):\s+(?P<rss>\d+)")


def main() -> int:
    repo_root = Path(__file__).resolve().parents[3]
    core_root = repo_root / "libs/core"

    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--tf-path",
        type=Path,
        default=repo_root / "libs/benchmarks/.corpora/bhsa/tf",
    )
    parser.add_argument("--limit", type=int, default=5)
    parser.add_argument(
        "--max-load-overhead-ratio",
        type=float,
        default=1.2,
        help="Fail if wheel load time is more than this multiple of raw Rust load time.",
    )
    parser.add_argument(
        "--max-query-overhead-ratio",
        type=float,
        default=1.3,
        help="Fail if wheel query time is more than this multiple of raw Rust query time.",
    )
    parser.add_argument(
        "--max-rust-rss-ratio",
        type=float,
        default=1.1,
        help="Fail if Rust peak RSS is more than this fraction of Python peak RSS.",
    )
    args = parser.parse_args()

    python_env = os.environ.copy()
    python_path = str(repo_root / "libs/core")
    python_env["PYTHONPATH"] = (
        f"{python_path}:{python_env['PYTHONPATH']}"
        if python_env.get("PYTHONPATH")
        else python_path
    )

    run_command(
        ["cargo", "build", "--release", "--quiet", "--bin", "cf_rust_bhsa_load_all"],
        cwd=core_root,
        env=os.environ.copy(),
    )

    python = measure_command(
        [
            sys.executable,
            "-c",
            PYTHON_LOAD_ALL,
            str(args.tf_path),
            str(args.limit),
        ],
        cwd=core_root,
        env=python_env,
    )
    rust = measure_command(
        [
            str(core_root / "target/release/cf_rust_bhsa_load_all"),
            str(args.tf_path),
            str(args.limit),
        ],
        cwd=core_root,
        env=os.environ.copy(),
    )

    python_timing = parse_timing(python.output)
    rust_timing = parse_timing(rust.output)
    rss_ratio = rust.rss_bytes / python.rss_bytes

    print("python")
    print(python.output.rstrip())
    print(python.time_output.rstrip())
    print()
    print("rust")
    print(rust.output.rstrip())
    print(rust.time_output.rstrip())
    print()
    print("comparison")
    print(
        f"load_speedup={python_timing.load_ms / rust_timing.load_ms:.2f}x "
        f"python_ms={python_timing.load_ms:.3f} rust_ms={rust_timing.load_ms:.3f}"
    )
    print(
        f"query_speedup={python_timing.query_ms / rust_timing.query_ms:.2f}x "
        f"python_ms={python_timing.query_ms:.3f} rust_ms={rust_timing.query_ms:.3f}"
    )
    print(
        f"python_peak_rss_mb={python.rss_bytes / (1024 * 1024):.2f} "
        f"rust_peak_rss_mb={rust.rss_bytes / (1024 * 1024):.2f} "
        f"rust_vs_python_rss_ratio={rss_ratio:.3f}"
    )
    load_overhead = python_timing.load_ms / rust_timing.load_ms
    query_overhead = python_timing.query_ms / rust_timing.query_ms
    failures = []
    if load_overhead > args.max_load_overhead_ratio:
        failures.append(
            f"wheel load overhead {load_overhead:.3f} exceeds "
            f"{args.max_load_overhead_ratio:.3f}"
        )
    if query_overhead > args.max_query_overhead_ratio:
        failures.append(
            f"wheel query overhead {query_overhead:.3f} exceeds "
            f"{args.max_query_overhead_ratio:.3f}"
        )
    if rss_ratio > args.max_rust_rss_ratio:
        failures.append(
            f"Rust peak RSS ratio {rss_ratio:.3f} exceeds "
            f"{args.max_rust_rss_ratio:.3f}"
        )
    if failures:
        raise RuntimeError("; ".join(failures))
    return 0


PYTHON_LOAD_ALL = r"""
import sys
import time
from cfabric import Fabric

tf_path = sys.argv[1]
limit = int(sys.argv[2])
load_start = time.perf_counter()
fabric = Fabric(locations=tf_path, silent="deep")
api = fabric.loadAll(silent="deep")
if api is False:
    raise RuntimeError("failed to load BHSA Python cfabric API")
load_ms = (time.perf_counter() - load_start) * 1000

feature_count = len(api.Fall(warp=False)) + len(api.Eall(warp=False))
print(f"loaded python_features={feature_count} load_ms={load_ms:.3f}")

query_start = time.perf_counter()
results = []
for result in api.S.search("word sp=verb", limit=limit):
    results.append(result)
    if len(results) >= limit:
        break
query_ms = (time.perf_counter() - query_start) * 1000
print(f"ok query=word_sp_verb results={len(results)} elapsed_ms={query_ms:.3f}")
"""


class Measurement:
    def __init__(self, output: str, time_output: str, rss_bytes: int) -> None:
        self.output = output
        self.time_output = time_output
        self.rss_bytes = rss_bytes


class Timing:
    def __init__(self, load_ms: float, query_ms: float) -> None:
        self.load_ms = load_ms
        self.query_ms = query_ms


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


def measure_command(command: list[str], cwd: Path, env: dict[str, str]) -> Measurement:
    completed = subprocess.run(
        ["/usr/bin/time", "-l", *command],
        cwd=cwd,
        env=env,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return Measurement(
        output=completed.stdout,
        time_output=completed.stderr,
        rss_bytes=parse_time_rss_bytes(completed.stderr),
    )


def parse_timing(output: str) -> Timing:
    load_match = LOAD_RE.search(output)
    query_match = QUERY_RE.search(output)
    if not load_match or not query_match:
        raise RuntimeError(f"missing timing fields in output:\n{output}")
    return Timing(
        load_ms=float(load_match.group("load_ms")),
        query_ms=float(query_match.group("elapsed_ms")),
    )


def parse_time_rss_bytes(output: str) -> int:
    if match := MAC_RSS_RE.search(output):
        return int(match.group("rss"))
    if match := MAC_FOOTPRINT_RE.search(output):
        return int(match.group("rss"))
    if match := GNU_RSS_RE.search(output):
        return int(match.group("rss")) * 1024
    raise RuntimeError(f"could not parse peak RSS from /usr/bin/time output:\n{output}")


if __name__ == "__main__":
    raise SystemExit(main())
