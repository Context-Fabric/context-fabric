#!/usr/bin/env python3
"""Compare Python cfabric and Rust mapped BHSA owned memory usage."""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path


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
        "--max-rust-uss-ratio",
        type=float,
        default=1.0,
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
    if python.uss_bytes == 0 or rust.uss_bytes == 0:
        raise RuntimeError(
            "USS measurement returned zero; this OS may deny child process "
            "memory_full_info().uss access"
        )
    ratio = rust.uss_bytes / python.uss_bytes
    print(
        f"python_peak_uss_mb={python.uss_bytes / (1024 * 1024):.2f} "
        f"rust_peak_uss_mb={rust.uss_bytes / (1024 * 1024):.2f} "
        f"rust_vs_python_uss_ratio={ratio:.3f}"
    )
    if ratio > args.max_rust_uss_ratio:
        raise RuntimeError(
            f"Rust peak USS ratio {ratio:.3f} exceeds "
            f"{args.max_rust_uss_ratio:.3f}"
        )
    return 0


class Measurement:
    def __init__(self, output: str, time_output: str, uss_bytes: int) -> None:
        self.output = output
        self.time_output = time_output
        self.uss_bytes = uss_bytes


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
    try:
        import psutil
    except ImportError as error:
        raise RuntimeError("psutil is required for USS memory measurement") from error

    with tempfile.TemporaryFile(mode="w+") as stdout_file, tempfile.TemporaryFile(
        mode="w+"
    ) as stderr_file:
        process = psutil.Popen(
            command,
            cwd=cwd,
            env=env,
            text=True,
            stdout=stdout_file,
            stderr=stderr_file,
        )
        peak_uss = 0
        while True:
            peak_uss = max(peak_uss, process_tree_uss(process))
            if process.poll() is not None:
                break
            time.sleep(0.02)
        stdout_file.seek(0)
        stderr_file.seek(0)
        stdout = stdout_file.read()
        stderr = stderr_file.read()
    if process.returncode != 0:
        raise subprocess.CalledProcessError(process.returncode, command, stdout, stderr)
    return Measurement(
        output=stdout,
        time_output=f"peak_uss_bytes={peak_uss}\n{stderr}",
        uss_bytes=peak_uss,
    )


def process_tree_uss(process: object) -> int:
    try:
        processes = [process, *process.children(recursive=True)]
    except Exception:
        processes = [process]
    total = 0
    for item in processes:
        try:
            total += item.memory_full_info().uss
        except Exception:
            continue
    return total


if __name__ == "__main__":
    raise SystemExit(main())
