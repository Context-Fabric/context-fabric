#!/usr/bin/env python3
"""Generate Python golden-master outputs for cf_rust public API probes."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[4]
PROBE_DIR = ROOT / "libs" / "cf-rust" / "tests" / "golden" / "probes"
GOLDEN_DIR = ROOT / "libs" / "cf-rust" / "tests" / "golden" / "golden"
CORPORA_DIR = ROOT / "libs" / "benchmarks" / ".corpora"


def load_cfabric() -> Any:
    sys.path.insert(0, str(ROOT / "libs" / "core"))
    from cfabric import Fabric

    return Fabric


def corpus_path(name: str) -> Path:
    return CORPORA_DIR / name / "tf"


def iter_probes(name: str) -> list[dict[str, Any]]:
    path = PROBE_DIR / f"{name}.jsonl"
    return [
        json.loads(line)
        for line in path.read_text().splitlines()
        if line.strip()
    ]


def run_probe(api: Any, probe: dict[str, Any]) -> Any:
    kind = probe["kind"]
    args = probe.get("args", {})

    if kind == "text":
        kwargs = {"fmt": args.get("format")}
        if "descend" in args:
            kwargs["descend"] = args["descend"]
        return api.T.text(args["node"], **kwargs)
    if kind == "section_from_node":
        return api.T.sectionFromNode(args["node"], lang=args.get("lang", ""))
    if kind == "node_from_section":
        return api.T.nodeFromSection(tuple(args["section"]), lang=args.get("lang", ""))
    if kind == "heading_from_node":
        return api.T.headingFromNode(args["node"])
    if kind == "node_from_heading":
        return api.T.nodeFromHeading(tuple(tuple(item) for item in args["heading"]))
    if kind == "structure":
        return api.T.structure(args.get("node"))
    if kind == "top":
        return api.T.top()
    if kind == "structure_pretty":
        return api.T.structurePretty(args.get("node"), fullHeading=args.get("full_heading", False))
    if kind == "book_name":
        return api.T.bookName(args["node"], lang=args.get("lang", ""))
    if kind == "book_node":
        return api.T.bookNode(args["name"], lang=args.get("lang", ""))
    if kind == "locality_up":
        return api.L.u(args["node"], otype=args.get("node_type"))
    if kind == "search":
        return api.S.search(args["template"], limit=args.get("limit"))

    raise ValueError(f"unsupported probe kind {kind!r}")


def normalize(value: Any) -> Any:
    try:
        import numpy as np

        if isinstance(value, np.integer):
            return int(value)
    except Exception:
        pass
    if isinstance(value, int):
        return int(value)
    if isinstance(value, tuple):
        return [normalize(item) for item in value]
    if isinstance(value, list):
        return [normalize(item) for item in value]
    if isinstance(value, dict):
        return {key: normalize(item) for key, item in value.items()}
    return value


def generate(name: str, output_dir: Path) -> Path:
    tf_path = corpus_path(name)
    if not (tf_path / "otype.tf").exists():
        raise FileNotFoundError(f"missing corpus: {tf_path}")

    Fabric = load_cfabric()
    api = Fabric(locations=str(tf_path), silent="deep").loadAll(silent="deep")
    records = [
        {"id": probe["id"], "result": normalize(run_probe(api, probe))}
        for probe in iter_probes(name)
    ]

    output_dir.mkdir(parents=True, exist_ok=True)
    output = output_dir / f"{name}.json"
    output.write_text(json.dumps(records, ensure_ascii=False, indent=2) + "\n")
    return output


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("corpus", nargs="*", default=["bhsa", "n1904", "banks"])
    parser.add_argument("--output-dir", default=os.environ.get("CF_RUST_GOLDEN_OUTPUT_DIR"))
    args = parser.parse_args()
    output_dir = Path(args.output_dir) if args.output_dir else GOLDEN_DIR

    for name in args.corpus:
        output = generate(name, output_dir)
        print(f"wrote {output}")


if __name__ == "__main__":
    main()
