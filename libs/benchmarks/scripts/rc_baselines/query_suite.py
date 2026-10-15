#!/usr/bin/env python3
"""Run the 34 ETCBC parity queries on one engine and record parity + wall time.

For every query in ``libs/core/tests/parity/parity_queries.json`` the suite
records the result count, whether the golden ``tf_first_results`` tuples are
present, the wall time of ``list(S.search(template))`` and a SHA-1 of the
sorted result tuples. Comparing that hash with the Text-Fabric run gives full
result-*set* parity (stronger than the count + sample check in
``tests/parity/test_query_parity.py``).

Per-query timeouts:

* ``tf`` / ``cf_python`` run in-process (one load) with ``SIGALRM``; both
  engines execute search in Python, so the alarm interrupts a runaway query.
* ``cf_rust`` / ``cf_rust_mapped`` run each query in a child process (native
  search code does not observe Python signals), mirroring the parity test.
  Each such query then runs in a fresh process against cold mmap pages, so
  ``--mode in-process`` is also offered for a like-for-like timing with the
  in-process engines (no hard timeout in that mode for native engines).

Results are appended as JSON lines so a crash keeps everything measured so far.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import signal
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))

import engines  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[4]
PARITY_QUERIES_JSON = REPO_ROOT / "libs" / "core" / "tests" / "parity" / "parity_queries.json"
IN_PROCESS_ENGINES = {"tf", "cf_python"}


class QueryTimeout(Exception):
    pass


def load_queries() -> list[dict[str, Any]]:
    return json.loads(PARITY_QUERIES_JSON.read_text())["queries"]


def run_one(api: Any, query: dict[str, Any]) -> dict[str, Any]:
    """Execute one query and summarise it against the golden values."""
    start = time.perf_counter()
    rows = [tuple(int(x) for x in row) for row in api.S.search(query["template"])]
    seconds = time.perf_counter() - start
    seen = set(rows)
    first = [tuple(int(x) for x in t) for t in query["tf_first_results"]]
    digest = hashlib.sha1(json.dumps(sorted(rows)).encode()).hexdigest()
    present = [t in seen for t in first]
    return {
        "id": query["id"],
        "seconds": round(seconds, 4),
        "count": len(rows),
        "tf_count": query["tf_count"],
        "count_ok": len(rows) == query["tf_count"],
        "first_results_present": all(present),
        "duplicates": len(rows) - len(seen),
        "result_sha1": digest,
        "status": "ok" if len(rows) == query["tf_count"] and all(present) else "wrong",
    }


def _alarm(_signum: int, _frame: Any) -> None:
    raise QueryTimeout()


def run_in_process(args: argparse.Namespace, queries: list[dict[str, Any]], sink: Any) -> None:
    start = time.perf_counter()
    api = engines.load(args.engine, args.tf_path, args.mapped_cache)
    load_s = time.perf_counter() - start
    sink.write(json.dumps({"_load_s": round(load_s, 3)}) + "\n")
    signal.signal(signal.SIGALRM, _alarm)
    for query in queries:
        signal.alarm(int(args.timeout))
        try:
            record = run_one(api, query)
        except QueryTimeout:
            record = {"id": query["id"], "status": "timeout", "seconds": None, "tf_count": query["tf_count"]}
        except Exception as error:  # noqa: BLE001 - record engine errors per query
            record = {
                "id": query["id"],
                "status": "error",
                "error": f"{type(error).__name__}: {error}"[:500],
                "tf_count": query["tf_count"],
            }
        finally:
            signal.alarm(0)
        sink.write(json.dumps(record) + "\n")
        sink.flush()
        print(f"{record['id']:<32} {record['status']:<8} {record.get('seconds')}", flush=True)


def run_isolated(args: argparse.Namespace, queries: list[dict[str, Any]], sink: Any) -> None:
    for query in queries:
        command = [
            sys.executable,
            str(Path(__file__).resolve()),
            "--engine",
            args.engine,
            "--tf-path",
            str(args.tf_path),
            "--single",
            query["id"],
        ]
        if args.mapped_cache:
            command += ["--mapped-cache", str(args.mapped_cache)]
        try:
            proc = subprocess.run(
                command, capture_output=True, text=True, timeout=args.timeout, cwd="/tmp"
            )
            lines = [ln for ln in proc.stdout.splitlines() if ln.startswith("{")]
            if proc.returncode == 0 and lines:
                record = json.loads(lines[-1])
            else:
                record = {
                    "id": query["id"],
                    "status": "error",
                    "error": f"exit {proc.returncode}: {proc.stderr[-500:]}",
                    "tf_count": query["tf_count"],
                }
        except subprocess.TimeoutExpired:
            record = {"id": query["id"], "status": "timeout", "seconds": None, "tf_count": query["tf_count"]}
        sink.write(json.dumps(record) + "\n")
        sink.flush()
        print(f"{record['id']:<32} {record['status']:<8} {record.get('seconds')}", flush=True)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--engine", choices=engines.ENGINES, required=True)
    parser.add_argument("--tf-path", type=Path, required=True)
    parser.add_argument("--mapped-cache", type=Path)
    parser.add_argument("--timeout", type=float, default=300.0, help="per-query seconds")
    parser.add_argument("--out", type=Path, help="JSON-lines output (suite mode)")
    parser.add_argument("--mode", choices=("auto", "in-process", "isolated"), default="auto",
                        help="auto: in-process for tf/cf_python, isolated for cf_rust*")
    parser.add_argument("--clear-cache", action="store_true",
                        help="delete the engine cache first (cold load rebuilds it)")
    parser.add_argument("--single", help=argparse.SUPPRESS)
    args = parser.parse_args(argv)

    engines.check_engine_identity(args.engine)
    queries = load_queries()

    if args.single:
        api = engines.load(args.engine, args.tf_path, args.mapped_cache)
        query = next(q for q in queries if q["id"] == args.single)
        print(json.dumps(run_one(api, query)))
        return 0

    if args.out is None:
        parser.error("--out is required in suite mode")
    if args.clear_cache:
        engines.clear_cache(args.engine, args.tf_path, args.mapped_cache)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w") as sink:
        sink.write(
            json.dumps({"_engine": args.engine, "_engine_version": engines.engine_version(args.engine)})
            + "\n"
        )
        in_process = args.mode == "in-process" or (
            args.mode == "auto" and args.engine in IN_PROCESS_ENGINES
        )
        sink.write(json.dumps({"_mode": "in-process" if in_process else "isolated"}) + "\n")
        if in_process:
            run_in_process(args, queries, sink)
        else:
            run_isolated(args, queries, sink)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
