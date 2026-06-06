"""
Simulation: Looping vs Vectorized Node Feature Lookup

This script compares the performance of the OLD looping-based approach
to the NEW vectorized numpy approach for filtering nodes by feature values.

The looping approach checks each node individually:
    for node in nodes:
        if feature.v(node) == value:
            result.add(node)

The vectorized approach uses numpy array operations:
    node_arr = np.asarray(nodes)
    values = feature_data.indices[node_arr - 1]
    result = node_arr[values == target_idx]

Results are written to simulation_results.json in this directory.

Note: This is a local simulation to measure algorithmic speedup, not
part of the full benchmarking suite (see cfabric_benchmarks package).
"""

from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Any

import numpy as np


def load_corpus():
    """Load the BHSA corpus using Context-Fabric."""
    from cfabric import Fabric

    # Path relative to benchmarks directory
    corpus_path = Path(__file__).parent.parent.parent / ".corpora" / "bhsa" / "tf"
    cf = Fabric(locations=str(corpus_path), silent="deep")
    return cf.loadAll(silent="deep")


def looping_filter(
    nodes: set[int],
    feature: Any,
    value: str
) -> set[int]:
    """
    OLD approach: Loop through each node and check feature value.

    This is the approach used in the committed version of _spinAtom().
    """
    result = set()
    for n in nodes:
        if feature.v(n) == value:
            result.add(n)
    return result


def vectorized_filter(
    nodes: set[int],
    feature_data: Any,  # StringPool or IntFeatureArray
    value: str
) -> set[int]:
    """
    NEW approach: Use vectorized numpy operations.

    This is the approach in the uncommitted version of _spinAtom().
    """
    result = feature_data.filter_by_value(list(nodes), value)
    return set(result)


def benchmark_filter(
    name: str,
    nodes: set[int],
    feature: Any,
    feature_data: Any,
    value: str,
    iterations: int = 10
) -> dict:
    """Run both approaches and measure performance."""

    # Warmup
    _ = looping_filter(nodes, feature, value)
    _ = vectorized_filter(nodes, feature_data, value)

    # Benchmark looping
    looping_times = []
    for _ in range(iterations):
        start = time.perf_counter()
        result_loop = looping_filter(nodes, feature, value)
        elapsed = (time.perf_counter() - start) * 1000  # ms
        looping_times.append(elapsed)

    # Benchmark vectorized
    vectorized_times = []
    for _ in range(iterations):
        start = time.perf_counter()
        result_vec = vectorized_filter(nodes, feature_data, value)
        elapsed = (time.perf_counter() - start) * 1000  # ms
        vectorized_times.append(elapsed)

    # Verify results match
    assert result_loop == result_vec, f"Results mismatch for {name}!"

    looping_mean = np.mean(looping_times)
    vectorized_mean = np.mean(vectorized_times)
    speedup = looping_mean / vectorized_mean if vectorized_mean > 0 else float('inf')

    return {
        "name": name,
        "value": value,
        "node_count": len(nodes),
        "result_count": len(result_loop),
        "looping_mean_ms": round(looping_mean, 2),
        "looping_std_ms": round(np.std(looping_times), 2),
        "vectorized_mean_ms": round(vectorized_mean, 2),
        "vectorized_std_ms": round(np.std(vectorized_times), 2),
        "speedup": round(speedup, 2),
        "iterations": iterations
    }


def main():
    print("Loading BHSA corpus...")
    api = load_corpus()

    # Get all word nodes (this is the largest node type)
    print("Getting word nodes...")
    all_words = set(api.F.otype.s("word"))
    print(f"  Total words: {len(all_words):,}")

    # Test cases: feature name, value, description
    test_cases = [
        ("sp", "verb", "Part of speech = verb"),
        ("sp", "subs", "Part of speech = noun (subs)"),
        ("sp", "art", "Part of speech = article"),
        ("sp", "prep", "Part of speech = preposition"),
        ("sp", "conj", "Part of speech = conjunction"),
        ("vt", "perf", "Verb tense = perfect"),
        ("vt", "impf", "Verb tense = imperfect"),
        ("gn", "m", "Gender = masculine"),
        ("gn", "f", "Gender = feminine"),
        ("nu", "sg", "Number = singular"),
        ("nu", "pl", "Number = plural"),
    ]

    results = []

    print("\nRunning benchmarks (10 iterations each)...\n")
    print(f"{'Test':<35} {'Loop (ms)':<12} {'Vec (ms)':<12} {'Speedup':<10} {'Results'}")
    print("-" * 85)

    for feature_name, value, description in test_cases:
        feature = api.Fs(feature_name)
        feature_data = feature._data if hasattr(feature, '_data') else None

        if feature_data is None:
            print(f"Skipping {feature_name}: no mmap backend available")
            continue

        result = benchmark_filter(
            name=f"{feature_name}={value}",
            nodes=all_words,
            feature=feature,
            feature_data=feature_data,
            value=value,
            iterations=10
        )
        results.append(result)

        print(f"{description:<35} {result['looping_mean_ms']:>8.2f} ms  "
              f"{result['vectorized_mean_ms']:>8.2f} ms  "
              f"{result['speedup']:>6.2f}x    "
              f"{result['result_count']:,}")

    # Summary statistics
    print("\n" + "=" * 85)
    speedups = [r["speedup"] for r in results]
    print(f"\nSummary:")
    print(f"  Average speedup: {np.mean(speedups):.2f}x")
    print(f"  Min speedup:     {np.min(speedups):.2f}x")
    print(f"  Max speedup:     {np.max(speedups):.2f}x")

    # Save results
    output_path = Path(__file__).parent / "simulation_results.json"
    with open(output_path, "w") as f:
        json.dump({
            "corpus": "BHSA",
            "total_words": len(all_words),
            "results": results,
            "summary": {
                "average_speedup": round(np.mean(speedups), 2),
                "min_speedup": round(np.min(speedups), 2),
                "max_speedup": round(np.max(speedups), 2),
            }
        }, f, indent=2)

    print(f"\nResults saved to: {output_path}")


if __name__ == "__main__":
    main()
