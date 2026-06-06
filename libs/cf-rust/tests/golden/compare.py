#!/usr/bin/env python3
"""Compare cf_rust dump output against committed Python golden masters."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[4]
PROBE_DIR = ROOT / "libs" / "cf-rust" / "tests" / "golden" / "probes"
GOLDEN_DIR = ROOT / "libs" / "cf-rust" / "tests" / "golden" / "golden"
CORPORA_DIR = ROOT / "libs" / "benchmarks" / ".corpora"
CF_RUST_DIR = ROOT / "libs" / "cf-rust"


def load_json_lines(text: str) -> list[dict[str, Any]]:
    return [json.loads(line) for line in text.splitlines() if line.strip()]


def run_dump(tf_path: Path, probes: Path, mapped: bool) -> subprocess.CompletedProcess[str]:
    command = [
        "cargo",
        "run",
        "--release",
        "--quiet",
        "--bin",
        "cf_rust_dump",
        "--",
    ]
    if mapped:
        command.append("--mapped")
    command.extend([str(tf_path), str(probes)])
    return subprocess.run(
        command,
        cwd=CF_RUST_DIR,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )


def compare_mode(name: str, mode: str, expected_by_id: dict[str, Any], tf_path: Path, probes: Path) -> bool:
    completed = run_dump(tf_path, probes, mapped=(mode == "mapped"))
    if completed.returncode != 0:
        print(completed.stderr, file=sys.stderr)
        return False

    actual = load_json_lines(completed.stdout)
    actual_by_id = {record["id"]: record["result"] for record in actual}

    ok = True
    for probe_id, expected_result in expected_by_id.items():
        actual_result = actual_by_id.get(probe_id)
        if actual_result == expected_result:
            print(f"PASS {mode} {name}:{probe_id}")
        else:
            ok = False
            print(f"FAIL {mode} {name}:{probe_id}")
            print(f"  expected: {json.dumps(expected_result, ensure_ascii=False)}")
            print(f"  actual:   {json.dumps(actual_result, ensure_ascii=False)}")

    for probe_id in sorted(set(actual_by_id) - set(expected_by_id)):
        ok = False
        print(f"FAIL {mode} {name}:{probe_id}: unexpected probe result")

    return ok


def compare(name: str) -> bool:
    tf_path = CORPORA_DIR / name / "tf"
    probes = PROBE_DIR / f"{name}.jsonl"
    golden = GOLDEN_DIR / f"{name}.json"

    if not (tf_path / "otype.tf").exists():
        print(f"SKIP {name}: missing corpus at {tf_path}")
        return True
    if not golden.exists():
        print(f"SKIP {name}: missing golden file at {golden}")
        return True

    expected = json.loads(golden.read_text())
    expected_by_id = {record["id"]: record["result"] for record in expected}
    materialized_ok = compare_mode(name, "materialized", expected_by_id, tf_path, probes)
    mapped_ok = compare_mode(name, "mapped", expected_by_id, tf_path, probes)
    return materialized_ok and mapped_ok


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("corpus", nargs="*", default=["bhsa", "n1904", "banks"])
    args = parser.parse_args()

    ok = True
    for name in args.corpus:
        ok = compare(name) and ok
    raise SystemExit(0 if ok else 1)


if __name__ == "__main__":
    main()
