"""Golden probes shared by the generator (Text-Fabric) and the comparison (cfabric).

A probe is one API call (``kind`` + ``args``) recorded in ``probes/<corpus>.jsonl``;
its golden result is the Text-Fabric answer stored in ``golden/<corpus>.json``.

Environment:
    CFABRIC_CORPORA: directory holding ``<name>/tf`` corpora
        (default: ``libs/benchmarks/.corpora`` in this repository).
"""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
PROBE_DIR = HERE / "probes"
GOLDEN_DIR = HERE / "golden"
REPO_CORPORA = HERE.parents[3] / "benchmarks" / ".corpora"
CORPORA = ("bhsa", "n1904", "banks")


def corpus_path(name: str) -> Path:
    """``<corpora>/<name>/tf``, with ``CFABRIC_CORPORA`` overriding the default."""
    return Path(os.environ.get("CFABRIC_CORPORA") or REPO_CORPORA) / name / "tf"


def iter_probes(name: str) -> list[dict[str, Any]]:
    path = PROBE_DIR / f"{name}.jsonl"
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


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
