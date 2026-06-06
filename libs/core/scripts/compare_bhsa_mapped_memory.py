#!/usr/bin/env python3
"""Compare Python cfabric and Rust mapped BHSA peak memory usage."""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path


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
    parser.add_argument(
        "--cache-path",
        type=Path,
        default=core_root / "target/bhsa-mapped-curated.cfr",
    )
    parser.add_argument("--limit", type=int, default=5)
    parser.add_argument(
        "--max-rust-rss-ratio",
        type=float,
        default=1.0,
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
        [
            "cargo",
            "build",
            "--release",
            "--quiet",
            "--bin",
            "cf_rust_validate_bhsa_mapped_curated",
        ],
        cwd=core_root,
        env=os.environ.copy(),
    )

    python = measure_command(
        [
            sys.executable,
            "scripts/python_bhsa_mapped_subset.py",
            str(args.tf_path),
            str(args.limit),
        ],
        cwd=core_root,
        env=python_env,
    )
    rust = measure_command(
        [
            str(core_root / "target/release/cf_rust_validate_bhsa_mapped_curated"),
            str(args.tf_path),
            str(args.cache_path),
            str(args.limit),
        ],
        cwd=core_root,
        env=os.environ.copy(),
    )

    print("python")
    print(python.output.rstrip())
    print(python.time_output.rstrip())
    print()
    print("rust")
    print(rust.output.rstrip())
    print(rust.time_output.rstrip())
    print()
    print("memory")
    ratio = rust.rss_bytes / python.rss_bytes
    print(
        f"python_peak_rss_mb={python.rss_bytes / (1024 * 1024):.2f} "
        f"rust_peak_rss_mb={rust.rss_bytes / (1024 * 1024):.2f} "
        f"rust_vs_python_rss_ratio={ratio:.3f}"
    )
    if ratio > args.max_rust_rss_ratio:
        raise RuntimeError(
            f"Rust peak RSS ratio {ratio:.3f} exceeds "
            f"{args.max_rust_rss_ratio:.3f}"
        )
    return 0


class Measurement:
    def __init__(self, output: str, time_output: str, rss_bytes: int) -> None:
        self.output = output
        self.time_output = time_output
        self.rss_bytes = rss_bytes


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
    time_command = ["/usr/bin/time", "-l", *command]
    completed = subprocess.run(
        time_command,
        cwd=cwd,
        env=env,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    rss_bytes = parse_time_rss_bytes(completed.stderr)
    return Measurement(
        output=completed.stdout,
        time_output=completed.stderr,
        rss_bytes=rss_bytes,
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
