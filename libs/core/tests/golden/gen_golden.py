#!/usr/bin/env python3
"""Generate the golden files from Text-Fabric (the reference implementation)."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path

from probes import CORPORA, GOLDEN_DIR, corpus_path, iter_probes, normalize, run_probe


def generate(name: str, output_dir: Path) -> Path:
    from tf.fabric import Fabric

    tf_path = corpus_path(name)
    if not (tf_path / "otype.tf").exists():
        raise FileNotFoundError(f"missing corpus: {tf_path}")

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
    parser.add_argument("corpus", nargs="*", default=list(CORPORA))
    parser.add_argument("--output-dir", default=os.environ.get("CONTEXT_FABRIC_GOLDEN_OUTPUT_DIR"))
    args = parser.parse_args()
    output_dir = Path(args.output_dir) if args.output_dir else GOLDEN_DIR

    for name in args.corpus:
        output = generate(name, output_dir)
        print(f"wrote {output}")


if __name__ == "__main__":
    main()
