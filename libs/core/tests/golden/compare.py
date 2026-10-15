#!/usr/bin/env python3
"""Compare cfabric's answers to the golden probes with Text-Fabric's."""

from __future__ import annotations

import argparse
import json
from typing import Any

from probes import CORPORA, GOLDEN_DIR, corpus_path, iter_probes, normalize, run_probe


def compare(name: str) -> bool:
    from cfabric import Fabric

    tf_path = corpus_path(name)
    golden = GOLDEN_DIR / f"{name}.json"
    if not (tf_path / "otype.tf").exists():
        print(f"SKIP {name}: missing corpus at {tf_path}")
        return True

    expected: dict[str, Any] = {
        record["id"]: record["result"] for record in json.loads(golden.read_text())
    }
    api = Fabric(locations=str(tf_path), silent="deep").loadAll(silent="deep")
    actual = {probe["id"]: normalize(run_probe(api, probe)) for probe in iter_probes(name)}

    ok = True
    for probe_id, expected_result in expected.items():
        actual_result = actual.get(probe_id)
        if actual_result == expected_result:
            print(f"PASS {name}:{probe_id}")
        else:
            ok = False
            print(f"FAIL {name}:{probe_id}")
            print(f"  expected: {json.dumps(expected_result, ensure_ascii=False)}")
            print(f"  actual:   {json.dumps(actual_result, ensure_ascii=False)}")
    for probe_id in sorted(set(actual) - set(expected)):
        ok = False
        print(f"FAIL {name}:{probe_id}: probe has no golden result")
    return ok


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("corpus", nargs="*", default=list(CORPORA))
    args = parser.parse_args()
    results = [compare(name) for name in args.corpus]
    raise SystemExit(0 if all(results) else 1)


if __name__ == "__main__":
    main()
