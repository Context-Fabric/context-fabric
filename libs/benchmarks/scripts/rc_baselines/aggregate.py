#!/usr/bin/env python3
"""Fold a ``run_rc_baselines.py`` results directory into committed baselines.

Writes, under ``libs/benchmarks/baselines/``:

* ``rc1_baseline_<date>.json``: every parity, memory and speed figure plus
  regressions against the 0.6.0 record (``cf_0.6.0_record.json``).
* ``rc1_<date>/``: compact raw outputs (logs, per-engine JSON, oracle diffs,
  JUnit XML) and ``tables.md`` (the markdown tables used in RC1_BASELINES.md).

Usage::

    python aggregate.py --results /tmp/cf_rc1/runs/main [--date 2026-10-14]
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import json
import re
import shutil
import statistics
import xml.etree.ElementTree as ET
from collections import defaultdict
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
BASELINES = HERE.parents[1] / "baselines"
JUNE_RECORD = BASELINES / "cf_0.6.0_record.json"
JUNE_TF = BASELINES / "bhsa_tf.json"

# A probe regresses when it is this much slower than the 0.6.0 record and the
# absolute change exceeds the noise floor for its unit.
REGRESSION_RATIO = 1.25
NOISE_FLOOR = {"us": 0.1, "s": 0.005, "mb": 10.0}

ENGINE_LABEL = {
    "tf": "TF 13.0.19",
    "cf_python": "CF Python 0.5.8 (reused .cfm)",
    "cf_python_cold": "CF Python 0.5.8 (fresh .cfm build)",
    "cf_rust": "CF Rust 0.6.0rc1 (loadAll)",
    "cf_rust_mapped": "CF Rust 0.6.0rc1 (openMapped)",
    "cf_rust_inprocess": "CF Rust 0.6.0rc1 (loadAll, in-process)",
}


def read_json(path: Path) -> Any:
    return json.loads(path.read_text()) if path.exists() else None


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


# --------------------------------------------------------------------------- parity


def parse_cargo(results: Path) -> dict[str, Any]:
    out: dict[str, Any] = {}
    fmt_log = results / "rust" / "cargo_fmt.log"
    if fmt_log.exists():
        text = fmt_log.read_text()
        files = sorted({m.group(1) for m in re.finditer(r"^Diff in (\S+?):\d+:", text, re.M)})
        out["fmt_check"] = {
            "ok": not files,
            "hunks": len(re.findall(r"^Diff in ", text, re.M)),
            "files": [f.split("/libs/core/")[-1] for f in files],
        }
    test_log = results / "rust" / "cargo_test.log"
    if test_log.exists():
        text = test_log.read_text()
        totals = defaultdict(int)
        for match in re.finditer(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored", text):
            totals["passed"] += int(match.group(2))
            totals["failed"] += int(match.group(3))
            totals["ignored"] += int(match.group(4))
        failed = re.findall(r"^test (\S+) \.\.\. FAILED", text, re.M)
        ignored = re.findall(r"^test (\S+) \.\.\. ignored", text, re.M)
        out["test"] = {**totals, "failed_tests": failed, "ignored_tests": ignored}
    return out


def parse_junit(path: Path) -> dict[str, Any]:
    root = ET.parse(path).getroot()
    suites = [root] if root.tag == "testsuite" else list(root)
    totals = {"tests": 0, "failures": 0, "errors": 0, "skipped": 0}
    failing, skipped = [], []
    for suite in suites:
        for key in totals:
            totals[key] += int(suite.get(key, 0))
        for case in suite.iter("testcase"):
            name = f"{case.get('classname')}::{case.get('name')}"
            for tag in ("failure", "error"):
                node = case.find(tag)
                if node is not None:
                    message = (node.get("message") or node.text or "").strip().splitlines()
                    failing.append({"test": name, "kind": tag, "message": (message[0] if message else "")[:300]})
            skip = case.find("skipped")
            if skip is not None:
                skipped.append({"test": name, "reason": (skip.get("message") or "")[:200]})
    totals["passed"] = totals["tests"] - totals["failures"] - totals["errors"] - totals["skipped"]
    return {**totals, "failing": failing, "skipped_detail": skipped}


def parse_pytest(results: Path) -> dict[str, Any]:
    out = {}
    for xml in sorted((results / "pytest").glob("*.xml")):
        out[xml.stem] = parse_junit(xml)
    return out


def parse_golden(results: Path) -> dict[str, Any]:
    out: dict[str, Any] = {}
    for log in sorted((results / "golden").glob("*.log")):
        engine = log.stem
        counts: dict[str, dict[str, dict[str, int]]] = defaultdict(lambda: defaultdict(lambda: {"pass": 0, "fail": 0}))
        failures = []
        lines = log.read_text().splitlines()
        for index, line in enumerate(lines):
            match = re.match(r"^(PASS|FAIL) (\w+) (\w+):(\S+?)(:.*)?$", line)
            if not match:
                continue
            status, mode, corpus, probe = match.group(1), match.group(2), match.group(3), match.group(4)
            counts[corpus][mode]["pass" if status == "PASS" else "fail"] += 1
            if status == "FAIL":
                detail = [ln.strip() for ln in lines[index + 1:index + 3] if ln.startswith("  ")]
                failures.append({"corpus": corpus, "mode": mode, "probe": probe,
                                 "detail": [d[:400] for d in detail] or [match.group(5) or ""]})
        skips = [ln for ln in lines if ln.startswith("SKIP")]
        out[engine] = {"counts": {c: dict(m) for c, m in counts.items()}, "failures": failures, "skips": skips}
    return out


def parse_queries(results: Path) -> dict[str, Any]:
    out: dict[str, Any] = {}
    records = {p.stem: read_jsonl(p) for p in sorted((results / "queries").glob("*.jsonl"))}
    tf_hash = {r["id"]: r.get("result_sha1") for r in records.get("tf", []) if "id" in r}
    for engine, rows in records.items():
        meta = {k: v for r in rows for k, v in r.items() if k.startswith("_")}
        queries = [r for r in rows if "id" in r]
        for row in queries:
            row["set_matches_tf"] = (row.get("result_sha1") is not None
                                     and row.get("result_sha1") == tf_hash.get(row["id"]))
        out[engine] = {
            "engine_version": meta.get("_engine_version"),
            "load_s": meta.get("_load_s"),
            "mode": meta.get("_mode"),
            "total": len(queries),
            "count_and_sample_ok": sum(r.get("status") == "ok" for r in queries),
            "result_set_equal_to_tf": sum(r["set_matches_tf"] for r in queries),
            "wall_s_completed": round(sum(r.get("seconds") or 0 for r in queries), 3),
            "not_ok": [{k: r.get(k) for k in ("id", "status", "count", "tf_count", "error")}
                       for r in queries if r.get("status") != "ok"],
            "per_query": {r["id"]: {k: r.get(k) for k in ("status", "seconds", "count", "set_matches_tf")}
                          for r in queries},
        }
    return out


def parse_oracle(results: Path) -> dict[str, Any]:
    out: dict[str, Any] = defaultdict(dict)
    for cmp_file in sorted((results / "oracle").glob("*/cmp_*.json")):
        data = read_json(cmp_file)
        corpus = cmp_file.parent.name
        engine = cmp_file.stem.removeprefix("cmp_")
        out[corpus][engine] = {
            "total": data["total"],
            "passed": data["passed"],
            "failed": data["failed"],
            "by_area": data["by_area"],
            "failing_groups": [{k: g[k] for k in ("area", "check", "failed", "total", "categories")}
                               for g in data["failing_groups"]],
        }
    return dict(out)


# --------------------------------------------------------------------------- perf


def parse_perf(results: Path) -> dict[str, Any]:
    out: dict[str, Any] = defaultdict(dict)
    for corpus_dir in sorted((results / "perf").glob("*")):
        for engine in ("tf", "cf_python", "cf_rust", "cf_rust_mapped"):
            cold = read_json(corpus_dir / f"{engine}_cold.json")
            warms = [read_json(p) for p in sorted(corpus_dir.glob(f"{engine}_warm*.json"))]
            full = read_json(corpus_dir / f"{engine}_full.json")
            if not (cold or warms or full):
                continue
            entry: dict[str, Any] = {"engine_version": (full or cold or warms[0])["engine_version"]}
            if cold:
                entry["cold_load_s"] = cold["load_s"]
                entry["cold_peak_rss_mb"] = cold["memory"]["after_load"]["peak_rss_mb"]
                entry["cold_cache_cleared"] = cold.get("cache_cleared")
            if warms:
                loads = [w["load_s"] for w in warms]
                entry["warm_load_s_median"] = round(statistics.median(loads), 4)
                entry["warm_load_s_runs"] = loads
                entry["warm_after_load_memory"] = warms[-1]["memory"]["after_load"]
            if full:
                entry["full_load_s"] = full["load_s"]
                entry["memory"] = full["memory"]
                entry["gate"] = full["gate"]
                entry["steady_us"] = full["steady_us"]
                entry["queries"] = full["queries"]
            out[corpus_dir.name][engine] = entry
    return dict(out)


def parse_perf_gate(results: Path) -> dict[str, Any]:
    out = {}
    for engine in ("tf", "cf"):
        data = read_json(results / "perf_gate" / f"{engine}.json")
        if data:
            out[engine] = data
    log = results / "perf_gate" / "cf.log"
    if log.exists():
        out["cf_assert"] = [ln.strip() for ln in log.read_text().splitlines()
                            if "FAIL" in ln or "PERF GATE" in ln]
    return out


def parse_microbench(results: Path) -> dict[str, Any]:
    out = {}
    patterns = [r"(load_overhead)=([\d.]+)x", r"(query_geomean_overhead)=([\d.]+)x",
                r"(query_geomean_speedup)=([\d.]+)x", r"(load_speedup)=([\d.]+)x",
                r"(loaded \S+) load_ms=([\d.]+)", r"(wins)=(\d+)"]
    for log in sorted((results / "microbench").glob("*.log")):
        text = log.read_text()
        parsed = {}
        for pattern in patterns:
            for match in re.finditer(pattern, text):
                parsed[match.group(1)] = float(match.group(2))
        ok_queries = re.findall(r"ok query=(\S+) results=(\d+) elapsed_ms=([\d.]+)", text)
        if ok_queries:
            parsed["queries_ok"] = len(ok_queries)
            parsed["queries_total_ms"] = round(sum(float(q[2]) for q in ok_queries), 1)
        parsed["fail_lines"] = [ln for ln in text.splitlines() if re.search(r"fail|Error|error", ln)][:20]
        parsed["tail"] = text.splitlines()[-15:]
        out[log.stem] = parsed
    return out


def _summarise_bench_memory(raw_csv: Path) -> dict[str, Any]:
    rows = list(csv.DictReader(raw_csv.open()))
    groups: dict[tuple[str, str], list[dict[str, str]]] = defaultdict(list)
    for row in rows:
        groups[(row["implementation"], row["mode"])].append(row)

    def mean(values: list[str]) -> float | None:
        numbers = [float(v) for v in values if v not in ("", None)]
        return round(statistics.mean(numbers), 3) if numbers else None

    return {
        f"{impl}/{mode}": {
            "runs": len(group),
            "load_time_s": mean([r["load_time_s"] for r in group]),
            "rss_after_mb": mean([r["rss_after_mb"] for r in group]),
            "total_rss_mb": mean([r["total_rss_mb"] for r in group]),
            "per_worker_rss_mb": mean([r["per_worker_rss_mb"] for r in group]),
        }
        for (impl, mode), group in sorted(groups.items())
    }


def _summarise_bench_latency(stats_csv: Path) -> dict[str, Any]:
    by_query: dict[str, dict[str, float]] = defaultdict(dict)
    errors: dict[str, int] = defaultdict(int)
    for row in csv.DictReader(stats_csv.open()):
        if row["mean_ms"]:
            by_query[row["query_id"]][row["implementation"]] = float(row["mean_ms"])
        errors[row["implementation"]] += int(row.get("error_count") or 0)
    shared = [q for q, v in by_query.items() if "TF" in v and "CF" in v and v["CF"] > 0]
    ratios = [by_query[q]["TF"] / by_query[q]["CF"] for q in shared]
    geomean = statistics.geometric_mean(ratios) if ratios else None
    return {
        "queries": len(shared),
        "tf_total_ms": round(sum(by_query[q]["TF"] for q in shared), 1),
        "cf_total_ms": round(sum(by_query[q]["CF"] for q in shared), 1),
        "geomean_speedup_tf_over_cf": round(geomean, 2) if geomean else None,
        "cf_slower_queries": sorted(q for q in shared if by_query[q]["CF"] > by_query[q]["TF"]),
        "error_count": dict(errors),
    }


def parse_bench_cli(results: Path) -> dict[str, Any]:
    """Summarise ``cfabric-bench`` output; each env compares TF with its own ``cfabric``."""
    out: dict[str, Any] = {}
    for env_dir in sorted((results / "bench_cli").glob("*")):
        entry: dict[str, Any] = {}
        memory_csv = sorted(env_dir.glob("memory/*/memory_raw_bhsa.csv"))
        if memory_csv:
            entry["memory"] = _summarise_bench_memory(memory_csv[-1])
        latency_csv = sorted(env_dir.glob("latency/*/latency_statistics.csv"))
        if latency_csv:
            entry["latency"] = _summarise_bench_latency(latency_csv[-1])
        for kind in ("latency", "memory"):
            log = env_dir / f"{kind}.log"
            if log.exists() and kind not in entry:
                entry[f"{kind}_log_tail"] = log.read_text().splitlines()[-25:]
        out[env_dir.name] = entry
    return out


# --------------------------------------------------------------------------- regressions


def regressions_vs_june(perf: dict[str, Any], gate: dict[str, Any]) -> list[dict[str, Any]]:
    june = read_json(JUNE_RECORD) or {}
    rows: list[dict[str, Any]] = []

    def check(source: str, metric: str, old: float | None, new: float | None, unit: str) -> None:
        if old is None or new is None:
            return
        ratio = new / old if old else float("inf")
        flagged = ratio > REGRESSION_RATIO and (new - old) > NOISE_FLOOR[unit]
        rows.append({"source": source, "metric": metric, "june": old, "now": new, "unit": unit,
                     "ratio": round(ratio, 2), "regression": flagged})

    mapped = perf.get("bhsa", {}).get("cf_rust_mapped", {})
    for source, run in (("perf_probe cf_rust_mapped", mapped), ("perf_gate cf", gate.get("cf", {}))):
        if not run:
            continue
        gate_probes = run.get("gate", run)
        recorded_probes = [name for name, value in june.items()
                           if isinstance(value, dict) and "per_call_us" in value]
        for name in recorded_probes:
            check(source, f"gate {name} per_call", june[name]["per_call_us"],
                  (gate_probes.get(name) or {}).get("per_call_us"), "us")
        check(source, "F.sp.freqList", june.get("F.sp.freqList_s"), gate_probes.get("F.sp.freqList_s"), "s")
        check(source, "F.sp.v full sweep", june.get("F.sp.v_full_sweep_s"), gate_probes.get("F.sp.v_full_sweep_s"), "s")
        check(source, "load", june.get("load_s"), run.get("full_load_s", run.get("load_s")), "s")
        rss_load = run.get("memory", {}).get("after_load", {}).get("peak_rss_mb", run.get("rss_after_load_mb"))
        rss_final = run.get("memory", {}).get("final", {}).get("peak_rss_mb", run.get("rss_final_mb"))
        check(source, "peak RSS after load", june.get("rss_after_load_mb"), rss_load, "mb")
        check(source, "peak RSS final", june.get("rss_final_mb"), rss_final, "mb")
        for qid, q in (june.get("queries") or {}).items():
            check(source, f"query {qid}", q["s"], (run.get("queries", {}).get(qid) or {}).get("s"), "s")
    steady = mapped.get("steady_us", {})
    for name, old in (june.get("steady_state_us") or {}).items():
        check("perf_probe cf_rust_mapped", f"steady {name}", old, steady.get(name), "us")
    return rows


# --------------------------------------------------------------------------- tables


def fmt(value: Any, digits: int = 2) -> str:
    if value is None:
        return "–"
    if isinstance(value, float):
        return f"{value:,.{digits}f}"
    return str(value)


def render_tables(summary: dict[str, Any]) -> str:
    lines: list[str] = []
    perf = summary["perf"]
    engines = ("tf", "cf_python", "cf_rust", "cf_rust_mapped")
    for corpus, by_engine in perf.items():
        present = [e for e in engines if e in by_engine]
        header = "| Metric | " + " | ".join(ENGINE_LABEL[e] for e in present) + " |"
        sep = "|---" * (len(present) + 1) + "|"
        lines += [f"### {corpus.upper()}: load and memory", "", header, sep]

        def row(label: str, getter: Any, digits: int = 2) -> None:
            cells = []
            for e in present:
                try:
                    cells.append(fmt(getter(by_engine[e]), digits))
                except (KeyError, TypeError):
                    cells.append("–")
            lines.append(f"| {label} | " + " | ".join(cells) + " |")

        row("Cold load, s (cache rebuilt)", lambda d: d.get("cold_load_s"))
        row("Warm load, s (median of 3)", lambda d: d.get("warm_load_s_median"), 3)
        row("Peak RSS during cold load, MB", lambda d: d.get("cold_peak_rss_mb"), 0)
        row("RSS after warm load, MB", lambda d: d["memory"]["after_load"]["rss_mb"], 0)
        row("USS after warm load, MB", lambda d: d["memory"]["after_load"]["uss_mb"], 0)
        row("RSS after workload, MB", lambda d: d["memory"]["final"]["rss_mb"], 0)
        row("USS after workload, MB", lambda d: d["memory"]["final"]["uss_mb"], 0)
        row("Peak RSS after workload, MB", lambda d: d["memory"]["final"]["peak_rss_mb"], 0)
        lines += ["", f"### {corpus.upper()}: steady-state per-call latency (µs, median of 5 × 50k calls)", "",
                  header, sep]
        steady_keys = max((list(d["steady_us"]) for d in by_engine.values() if "steady_us" in d),
                          key=len, default=[])
        for key in steady_keys:
            if key == "N.walk_nodes" or key.startswith("_"):
                continue
            digits = 4 if key.endswith("_s") else 3
            label = "N.walk full pass, s" if key == "N.walk_full_s" else key
            row(label, lambda d, k=key: d["steady_us"][k], digits)
        lines += ["", f"### {corpus.upper()}: perf_gate-style first-call probes (µs/call) and workload", "",
                  header, sep]
        gate_keys = next((list(d["gate"]) for d in by_engine.values() if "gate" in d), [])
        for key in gate_keys:
            if key.startswith("_"):
                continue
            if key.endswith("_s"):
                row(f"{key}, s", lambda d, k=key: d["gate"][k], 4)
            else:
                row(f"{key} (n={by_engine[present[0]]['gate'][key]['n']})",
                    lambda d, k=key: d["gate"][k]["per_call_us"])
        query_keys = next((list(d["queries"]) for d in by_engine.values() if "queries" in d), [])
        for key in query_keys:
            row(f"query {key}, s", lambda d, k=key: d["queries"][k]["s"], 4)
        lines.append("")

    queries = summary["parity"]["query_suite"]
    if queries:
        lines += ["### BHSA: 34-query suite", "",
                  "| Engine | count + sample OK | result set == TF | wall time, s (completed queries) | not OK |",
                  "|---|---|---|---|---|"]
        for engine, data in queries.items():
            bad = ", ".join(f"{r['id']} ({r['status']})" for r in data["not_ok"]) or "–"
            lines.append(f"| {ENGINE_LABEL.get(engine, engine)} | {data['count_and_sample_ok']}/{data['total']} | "
                         f"{data['result_set_equal_to_tf']}/{data['total']} | {fmt(data['wall_s_completed'])} | {bad} |")
        lines.append("")
        ids = list(next(iter(queries.values()))["per_query"])
        present = list(queries)
        lines += ["| Query | " + " | ".join(ENGINE_LABEL.get(e, e) for e in present) + " |",
                  "|---" * (len(present) + 1) + "|"]
        for qid in ids:
            cells = []
            for e in present:
                q = queries[e]["per_query"].get(qid, {})
                mark = "" if q.get("status") == "ok" else f" ({q.get('status')})"
                cells.append(f"{fmt(q.get('seconds'), 3)}{mark}")
            lines.append(f"| {qid} | " + " | ".join(cells) + " |")
        lines.append("")

    oracle = summary["parity"]["oracle"]
    if oracle:
        lines += ["### Oracle API value checks vs TF 13.0.19", "",
                  "| Corpus | Engine | Passed / total | Failed | Failing areas |", "|---|---|---|---|---|"]
        for corpus, by_engine in oracle.items():
            for engine, data in by_engine.items():
                if engine == "tf":
                    continue
                areas = ", ".join(f"{a} {c.get('passed', 0)}/{c['total']}" for a, c in data["by_area"].items()
                                  if c.get("passed", 0) != c["total"]) or "–"
                lines.append(f"| {corpus} | {ENGINE_LABEL.get(engine, engine)} | {data['passed']:,}/{data['total']:,} | "
                             f"{data['failed']:,} | {areas} |")
        lines.append("")

    golden = summary["parity"]["golden"]
    if golden:
        lines += ["### Golden probes (TF truth)", "", "| Engine | Corpus | Mode | Pass | Fail |", "|---|---|---|---|---|"]
        for engine, data in golden.items():
            for corpus, modes in data["counts"].items():
                for mode, c in modes.items():
                    label = "CF Python 0.5.8" if engine == "cf_python" else "CF Rust 0.6.0rc1"
                    lines.append(f"| {label} | {corpus} | {mode} | {c['pass']} | {c['fail']} |")
        lines.append("")

    regressions = [r for r in summary["regressions_vs_june"] if r["regression"]]
    lines += ["### Regressions vs the 0.6.0 record (cf_0.6.0_record.json)", ""]
    if regressions:
        lines += ["| Source | Metric | Record | Now | Ratio |", "|---|---|---|---|---|"]
        for r in regressions:
            lines.append(f"| {r['source']} | {r['metric']} | {fmt(r['june'], 4)} {r['unit']} | "
                         f"{fmt(r['now'], 4)} {r['unit']} | {r['ratio']}x |")
    else:
        lines.append("None above the threshold.")
    lines.append("")
    return "\n".join(lines)


# --------------------------------------------------------------------------- main


COMPACT_GLOBS = [
    "environment.json", "steps.jsonl", "rust/*.log", "pytest/*.xml", "pytest/*.log", "golden/*.log",
    "queries/*.jsonl", "oracle/*/cmp_*.json", "perf/*/*.json", "perf_gate/*", "microbench/*.log",
    "bench_cli/*/*.log", "bench_cli/*/*/**/*.csv", "bench_cli/*/*/**/*.json",
]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--results", type=Path, required=True)
    parser.add_argument("--date", default=dt.date.today().isoformat())
    args = parser.parse_args(argv)
    results = args.results

    perf = parse_perf(results)
    gate = parse_perf_gate(results)
    summary = {
        "date": args.date,
        "results_dir": str(results),
        "environment": read_json(results / "environment.json"),
        "steps": read_jsonl(results / "steps.jsonl"),
        "parity": {
            "cargo": parse_cargo(results),
            "pytest": parse_pytest(results),
            "golden": parse_golden(results),
            "query_suite": parse_queries(results),
            "oracle": parse_oracle(results),
        },
        "perf": perf,
        "perf_gate": gate,
        "microbench": parse_microbench(results),
        "bench_cli": parse_bench_cli(results),
        "june_record": {"cf": read_json(JUNE_RECORD), "tf": read_json(JUNE_TF)},
    }
    summary["regressions_vs_june"] = regressions_vs_june(perf, gate)

    raw_dir = BASELINES / f"rc1_{args.date}"
    raw_dir.mkdir(parents=True, exist_ok=True)
    for pattern in COMPACT_GLOBS:
        for path in results.glob(pattern):
            if path.is_file():
                target = raw_dir / path.relative_to(results)
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(path, target)
    (raw_dir / "tables.md").write_text(render_tables(summary))
    out = BASELINES / f"rc1_baseline_{args.date}.json"
    out.write_text(json.dumps(summary, indent=1, ensure_ascii=False) + "\n")
    print(f"wrote {out} and {raw_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
