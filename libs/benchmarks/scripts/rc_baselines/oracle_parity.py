#!/usr/bin/env python3
"""Count public-API value checks of an engine against the Text-Fabric oracle.

Three steps, each in its own process so every engine can live in its own
virtual environment:

``plan``     (Text-Fabric) sample nodes of every node type and enumerate the
             checks: every node feature value, edge ``f``/``t``/``b``, locality
             ``u``/``d``/``n``/``p``/``i``, ``T.text`` in every format, section
             and heading lookups, ``nodeFromSection`` round trips, frequency
             lists, ``N.walk``/``sortNodes``, and corpus metadata.
``eval``     (any engine) evaluate the plan and write one value per check.
``compare``  diff an engine's values against the Text-Fabric values, counting
             passes per API area and listing every failing check.

Comparison semantics follow ``libs/core/tests/parity/test_tf_oracle_parity.py``:
``L.d``, ``E.b`` and frequency lists compare as multisets, ``L.i`` as a set,
everything else exactly. Separate ``order`` checks additionally compare
``L.d`` and ``N.walk`` in exact order. Large values are stored as SHA-1 digests.

Usage::

    python oracle_parity.py plan    --tf-path T --out plan.json [--per-type 200]
    python oracle_parity.py eval    --engine cf_rust --tf-path T --plan plan.json --out cf.json
    python oracle_parity.py compare --plan plan.json --expected tf.json --actual cf.json --out cmp.json
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import time
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Callable

sys.path.insert(0, str(Path(__file__).resolve().parent))

import engines  # noqa: E402

MAX_INLINE = 200
FAILURES_PER_GROUP = 25

# kind -> comparison canonicalisation: exact, multiset (sorted), set (dedup + sorted)
KIND_CANON = {
    "L.d": "multiset",
    "E.b": "multiset",
    "L.i": "set",
    "F.freqList": "multiset",
    "E.freqList": "multiset",
    "N.walk": "multiset",
    "Fall": "set",
    "Eall": "set",
    "T.formats": "set",
    "N.otypeRank": "exact",
}


# --------------------------------------------------------------------------- plan


def sample_nodes(api: Any, per_type: int) -> list[int]:
    """Evenly spaced nodes of every type, always including first/middle/last."""
    nodes: set[int] = set()
    for row in api.C.levels.data:
        first, last = int(row[2]), int(row[3])
        if first > last:
            continue
        count = last - first + 1
        if count <= per_type:
            nodes.update(range(first, last + 1))
            continue
        step = (last - first) / (per_type - 1)
        nodes.update(first + round(i * step) for i in range(per_type))
        nodes.update({first, first + (last - first) // 2, last})
    return sorted(nodes)


def build_plan(api: Any, per_type: int) -> dict[str, Any]:
    T = api.T
    sample = sample_nodes(api, per_type)
    section_types = set(T.sectionTypes or ())
    otype = api.F.otype
    section_nodes = [n for n in sample if otype.v(n) in section_types]
    sections = []
    for node in section_nodes:
        section = T.sectionFromNode(node)
        if section:
            sections.append([*section])
    groups: list[dict[str, Any]] = []

    def add(area: str, kind: str, args: str | None = None, **extra: Any) -> None:
        groups.append({"area": area, "kind": kind, "args": args, **extra})

    for kind in ("Fall", "Eall", "C.levels", "N.otypeRank", "T.formats", "T.sectionTypes"):
        add("meta", kind)
    node_features = sorted(set(api.Fall(warp=False)) | {"otype"})
    for feature in node_features:
        add("F", "F.v", "sample", feature=feature)
        add("F", "F.freqList", feature=feature)
    for node_type in [row[0] for row in api.C.levels.data]:
        add("F", "F.otype.s", node_type=node_type)
    add("E", "E.oslots.s", "sample")
    for feature in sorted(api.Eall(warp=False)):
        for kind in ("E.f", "E.t", "E.b"):
            add("E", kind, "sample", feature=feature)
        add("E", "E.freqList", feature=feature)
    for kind in ("L.u", "L.d", "L.n", "L.p", "L.i"):
        add("L", kind, "sample")
    for fmt in sorted(T.formats):
        add("T", "T.text", "sample", fmt=fmt)
    add("T", "T.sectionFromNode", "sample")
    add("T", "T.headingFromNode", "sample")
    add("T", "T.nodeFromSection", "sections")
    add("T", "T.top")
    add("T", "T.structure")
    add("N", "N.walk")
    add("N", "N.sortNodes")
    add("order", "L.d.ordered", "sample")
    add("order", "N.walk.ordered")
    return {"per_type": per_type, "samples": {"sample": sample, "sections": sections}, "groups": groups}


# --------------------------------------------------------------------------- eval


def _call_factory(api: Any, group: dict[str, Any]) -> Callable[..., Any]:
    kind = group["kind"]
    F, E, L, T, N, C = api.F, api.E, api.L, api.T, api.N, api.C
    if kind == "Fall":
        return lambda: api.Fall(warp=False)
    if kind == "Eall":
        return lambda: api.Eall(warp=False)
    if kind == "C.levels":
        return lambda: C.levels.data
    if kind == "N.otypeRank":
        return lambda: dict(N.otypeRank)
    if kind == "T.formats":
        return lambda: list(T.formats)
    if kind == "T.sectionTypes":
        return lambda: list(T.sectionTypes)
    if kind == "F.v":
        feature = api.Fs(group["feature"], warn=False)
        return feature.v
    if kind == "F.freqList":
        return lambda: api.Fs(group["feature"], warn=False).freqList()
    if kind == "F.otype.s":
        return lambda: F.otype.s(group["node_type"])
    if kind == "E.oslots.s":
        return E.oslots.s
    if kind in ("E.f", "E.t", "E.b"):
        feature = api.Es(group["feature"], warn=False)
        return getattr(feature, kind[-1])
    if kind == "E.freqList":
        return lambda: api.Es(group["feature"], warn=False).freqList()
    if kind in ("L.u", "L.d", "L.n", "L.p", "L.i"):
        return getattr(L, kind[-1])
    if kind == "L.d.ordered":
        return L.d
    if kind == "T.text":
        fmt = group["fmt"]
        return lambda node: T.text(node, fmt=fmt)
    if kind == "T.sectionFromNode":
        return T.sectionFromNode
    if kind == "T.headingFromNode":
        return T.headingFromNode
    if kind == "T.nodeFromSection":
        return lambda section: T.nodeFromSection(tuple(section))
    if kind == "T.top":
        return T.top
    if kind == "T.structure":
        return lambda: T.structure()
    if kind in ("N.walk", "N.walk.ordered"):
        return lambda: list(N.walk())
    if kind == "N.sortNodes":
        return None  # needs the sample; handled in evaluate()
    raise ValueError(f"unknown kind {kind}")


def canonical(kind: str, value: Any) -> Any:
    mode = KIND_CANON.get(kind, "exact")
    if mode != "exact" and isinstance(value, list):
        items = value if mode == "multiset" else list({json.dumps(v, sort_keys=True): v for v in value}.values())
        return sorted(items, key=lambda v: json.dumps(v, sort_keys=True, ensure_ascii=False))
    return value


def encode(value: Any) -> Any:
    text = json.dumps(value, sort_keys=True, ensure_ascii=False)
    if len(text) <= MAX_INLINE:
        return value
    return f"sha1:{hashlib.sha1(text.encode()).hexdigest()}:len={len(text)}"


def evaluate(api: Any, plan: dict[str, Any]) -> list[list[Any]]:
    results: list[list[Any]] = []
    for group in plan["groups"]:
        kind = group["kind"]
        try:
            fn = _call_factory(api, group)
        except Exception as error:  # noqa: BLE001 - a missing API is a failing check
            fn_error: Any = {"__error__": f"{type(error).__name__}"}
            fn = None
        else:
            fn_error = None

        def run(call: Callable[[], Any]) -> Any:
            try:
                return encode(canonical(kind, engines.normalize(call())))
            except Exception as error:  # noqa: BLE001 - errors are compared like values
                return {"__error__": type(error).__name__}

        if kind == "N.sortNodes":
            sample = list(reversed(plan["samples"]["sample"]))
            results.append([run(lambda: api.N.sortNodes(sample))])
            continue
        if group["args"] is None:
            results.append([fn_error if fn is None else run(fn)])
            continue
        items = plan["samples"][group["args"]]
        if fn is None:
            results.append([fn_error] * len(items))
            continue
        results.append([run(lambda item=item: fn(item)) for item in items])
    return results


# --------------------------------------------------------------------------- compare


def group_label(group: dict[str, Any]) -> str:
    label = group["kind"]
    for key in ("feature", "fmt", "node_type"):
        if key in group:
            label += f"[{group[key]}]"
    return label


def failure_category(expected: Any, actual: Any) -> str:
    """Classify a failing check.

    ``engine_error``: the engine raised where Text-Fabric returned a value.
    ``tf_error``: Text-Fabric raised where the engine returned a value (an API
    the engine offers beyond Text-Fabric; still counted as a failed check).
    ``value``: both returned values that differ (or both raised differently).
    """
    exp_err = isinstance(expected, dict) and "__error__" in expected
    act_err = isinstance(actual, dict) and "__error__" in actual
    if act_err and not exp_err:
        return "engine_error"
    if exp_err and not act_err:
        return "tf_error"
    return "value"


def compare(plan: dict[str, Any], expected: dict[str, Any], actual: dict[str, Any]) -> dict[str, Any]:
    by_area: dict[str, Counter[str]] = defaultdict(Counter)
    failing_groups = []
    for group, exp_values, act_values in zip(plan["groups"], expected["values"], actual["values"]):
        items = plan["samples"][group["args"]] if group["args"] else [None]
        failures = []
        for item, exp, act in zip(items, exp_values, act_values):
            if exp == act:
                by_area[group["area"]]["passed"] += 1
            else:
                failures.append({"arg": item, "expected": exp, "actual": act})
            by_area[group["area"]]["total"] += 1
        if failures:
            categories = Counter(failure_category(f["expected"], f["actual"]) for f in failures)
            failing_groups.append(
                {
                    "area": group["area"],
                    "check": group_label(group),
                    "categories": dict(categories),
                    "failed": len(failures),
                    "total": len(items),
                    "examples": failures[:FAILURES_PER_GROUP],
                }
            )
    total = sum(c["total"] for c in by_area.values())
    passed = sum(c["passed"] for c in by_area.values())
    return {
        "engine": actual["engine"],
        "engine_version": actual["engine_version"],
        "oracle_version": expected["engine_version"],
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "by_area": {area: dict(counts) for area, counts in sorted(by_area.items())},
        "failing_groups": failing_groups,
    }


# --------------------------------------------------------------------------- CLI


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    p_plan = sub.add_parser("plan")
    p_plan.add_argument("--tf-path", type=Path, required=True)
    p_plan.add_argument("--per-type", type=int, default=200)
    p_plan.add_argument("--out", type=Path, required=True)
    p_eval = sub.add_parser("eval")
    p_eval.add_argument("--engine", choices=engines.ENGINES, required=True)
    p_eval.add_argument("--tf-path", type=Path, required=True)
    p_eval.add_argument("--mapped-cache", type=Path)
    p_eval.add_argument("--plan", type=Path, required=True)
    p_eval.add_argument("--clear-cache", action="store_true",
                        help="delete the engine cache first (cold load rebuilds it)")
    p_eval.add_argument("--out", type=Path, required=True)
    p_cmp = sub.add_parser("compare")
    p_cmp.add_argument("--plan", type=Path, required=True)
    p_cmp.add_argument("--expected", type=Path, required=True)
    p_cmp.add_argument("--actual", type=Path, required=True)
    p_cmp.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)

    if args.command == "plan":
        engines.check_engine_identity("tf")
        api = engines.load("tf", args.tf_path)
        plan = build_plan(api, args.per_type)
        plan["oracle_version"] = engines.engine_version("tf")
        args.out.write_text(json.dumps(plan))
        checks = sum(len(plan["samples"][g["args"]]) if g["args"] else 1 for g in plan["groups"])
        print(json.dumps({"groups": len(plan["groups"]), "sample_nodes": len(plan["samples"]["sample"]), "checks": checks}))
        return 0

    if args.command == "eval":
        engines.check_engine_identity(args.engine)
        plan = json.loads(args.plan.read_text())
        if args.clear_cache:
            engines.clear_cache(args.engine, args.tf_path, args.mapped_cache)
        start = time.perf_counter()
        api = engines.load(args.engine, args.tf_path, args.mapped_cache)
        values = evaluate(api, plan)
        args.out.write_text(
            json.dumps(
                {
                    "engine": args.engine,
                    "engine_version": engines.engine_version(args.engine),
                    "seconds": round(time.perf_counter() - start, 1),
                    "values": values,
                },
                ensure_ascii=False,
            )
        )
        print(json.dumps({"engine": args.engine, "seconds": round(time.perf_counter() - start, 1)}))
        return 0

    plan = json.loads(args.plan.read_text())
    result = compare(plan, json.loads(args.expected.read_text()), json.loads(args.actual.read_text()))
    args.out.write_text(json.dumps(result, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps({k: result[k] for k in ("engine", "total", "passed", "failed")}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
