#!/usr/bin/env python3
"""Compare Python cfabric and Rust BHSA load-all time and owned memory."""

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
        default=1.5,
        help="Fail if wheel query time is more than this multiple of raw Rust query time.",
    )
    parser.add_argument(
        "--max-rust-uss-ratio",
        type=float,
        default=1.1,
        help="Fail if Rust peak USS is more than this fraction of Python peak USS.",
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

    python_output = run_command(
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
    rust_output = run_command(
        [
            str(core_root / "target/release/cf_rust_bhsa_load_all"),
            str(args.tf_path),
            str(args.limit),
        ],
        cwd=core_root,
        env=os.environ.copy(),
    )

    python_timing = parse_timing(python_output)
    rust_timing = parse_timing(rust_output)
    python_memory = parse_memory(python_output)

    print("python")
    print(python_output.rstrip())
    print()
    print("rust")
    print(rust_output.rstrip())
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
        f"python_baseline_uss_mb={python_memory.baseline_uss_mb:.2f} "
        f"python_first_open_uss_mb={python_memory.first_open_uss_mb:.2f} "
        f"python_final_uss_mb={python_memory.final_uss_mb:.2f} "
        f"python_repeated_open_growth_mb={python_memory.repeated_growth_mb:.2f}"
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
    if python_memory.first_open_uss_mb > 50:
        failures.append(
            f"Python first-open USS {python_memory.first_open_uss_mb:.2f} MB exceeds 50 MB"
        )
    if python_memory.final_uss_mb > 50:
        failures.append(f"Python final USS {python_memory.final_uss_mb:.2f} MB exceeds 50 MB")
    if python_memory.repeated_growth_mb > 5:
        failures.append(
            f"Python repeated-open USS growth {python_memory.repeated_growth_mb:.2f} MB exceeds 5 MB"
        )
    if failures:
        raise RuntimeError("; ".join(failures))
    return 0


PYTHON_LOAD_ALL = r"""
import sys
import time
import gc
import psutil
from cfabric import Fabric

tf_path = sys.argv[1]
limit = int(sys.argv[2])
process = psutil.Process()

def uss_mb():
    return process.memory_full_info().uss / (1024 * 1024)

baseline_uss_mb = uss_mb()
load_start = time.perf_counter()
fabric = Fabric(locations=tf_path, silent="deep")
api = fabric.loadAll(silent="deep")
if api is False:
    raise RuntimeError("failed to load BHSA Python cfabric API")
load_ms = (time.perf_counter() - load_start) * 1000

feature_count = len(api.Fall(warp=False)) + len(api.Eall(warp=False))
print(f"loaded python_features={feature_count} load_ms={load_ms:.3f}")

first_open_uss_mb = uss_mb()
apis = [api]
for _ in range(4):
    apis.append(Fabric(locations=tf_path, silent="deep").loadAll(silent="deep"))
    gc.collect()
final_uss_mb = uss_mb()
print(
    f"memory baseline_uss_mb={baseline_uss_mb:.2f} "
    f"first_open_uss_mb={first_open_uss_mb:.2f} "
    f"final_uss_mb={final_uss_mb:.2f} "
    f"repeated_growth_mb={final_uss_mb - first_open_uss_mb:.2f}"
)

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
    def __init__(self, output: str, time_output: str, uss_bytes: int) -> None:
        self.output = output
        self.time_output = time_output
        self.uss_bytes = uss_bytes


class Timing:
    def __init__(self, load_ms: float, query_ms: float) -> None:
        self.load_ms = load_ms
        self.query_ms = query_ms


class Memory:
    def __init__(
        self,
        baseline_uss_mb: float,
        first_open_uss_mb: float,
        final_uss_mb: float,
        repeated_growth_mb: float,
    ) -> None:
        self.baseline_uss_mb = baseline_uss_mb
        self.first_open_uss_mb = first_open_uss_mb
        self.final_uss_mb = final_uss_mb
        self.repeated_growth_mb = repeated_growth_mb


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


def parse_timing(output: str) -> Timing:
    load_match = LOAD_RE.search(output)
    query_match = QUERY_RE.search(output)
    if not load_match or not query_match:
        raise RuntimeError(f"missing timing fields in output:\n{output}")
    return Timing(
        load_ms=float(load_match.group("load_ms")),
        query_ms=float(query_match.group("elapsed_ms")),
    )


def parse_memory(output: str) -> Memory:
    memory_match = re.search(
        r"baseline_uss_mb=(?P<baseline>[0-9.]+) "
        r"first_open_uss_mb=(?P<first>[0-9.]+) "
        r"final_uss_mb=(?P<final>[0-9.]+) "
        r"repeated_growth_mb=(?P<growth>-?[0-9.]+)",
        output,
    )
    if not memory_match:
        raise RuntimeError(f"missing memory fields in output:\n{output}")
    return Memory(
        baseline_uss_mb=float(memory_match.group("baseline")),
        first_open_uss_mb=float(memory_match.group("first")),
        final_uss_mb=float(memory_match.group("final")),
        repeated_growth_mb=float(memory_match.group("growth")),
    )


if __name__ == "__main__":
    raise SystemExit(main())
