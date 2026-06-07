#!/usr/bin/env python3
"""Run the BHSA mapped-subset query timings against Text-Fabric."""

from __future__ import annotations

import argparse
import time
from pathlib import Path

from scripts.python_bhsa_mapped_subset import FEATURES, QUERIES


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "tf_path",
        nargs="?",
        type=Path,
        default=Path(__file__).resolve().parents[2] / "benchmarks/.corpora/bhsa/tf",
    )
    parser.add_argument("limit", nargs="?", type=int, default=5)
    args = parser.parse_args()

    from tf.fabric import Fabric

    load_start = time.perf_counter()
    fabric = Fabric(locations=str(args.tf_path), silent="deep")
    api = fabric.load(" ".join(FEATURES), silent="deep")
    if api is False:
        raise RuntimeError("failed to load BHSA Text-Fabric API")
    load_ms = (time.perf_counter() - load_start) * 1000
    print(f"loaded text_fabric_features={len(FEATURES)} load_ms={load_ms:.3f}")

    failures: list[str] = []
    for query_id, template in QUERIES:
        start = time.perf_counter()
        try:
            results = []
            for result in api.S.search(template):
                results.append(result)
                if len(results) >= args.limit:
                    break
            elapsed_ms = (time.perf_counter() - start) * 1000
            print(f"ok query={query_id} results={len(results)} elapsed_ms={elapsed_ms:.3f}")
        except Exception as error:  # noqa: BLE001 - script reports query-level failures.
            print(f"fail query={query_id} error={error}")
            failures.append(query_id)

    if failures:
        print(f"failed queries: {', '.join(failures)}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
