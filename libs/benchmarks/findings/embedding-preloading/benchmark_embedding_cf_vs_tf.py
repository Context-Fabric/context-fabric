"""
Benchmark: CF with preload vs TF for embedding queries.

Tests 20 embedding patterns to see if CF can match TF performance.

Provenance
----------
Written: 2026-01-08 (after CF v0.5.0 added embedding preloading)
Purpose: Validate that CF with preloading matches TF performance for embedding queries
Context: CF's mmap-based CSR arrays were slower than TF for embedding-heavy queries
         due to page fault overhead. This script tested the preloading solution.

Results informed the decision to make preloading the default behavior in CF v0.5.0.
See embedding_preloading.md for full analysis.

Usage
-----
    python benchmark_embedding_cf_vs_tf.py

Requires BHSA corpus at: .corpora/bhsa/tf (relative to libs/benchmarks/)
"""

import time
from pathlib import Path

# Embedding query patterns using [[ and ]] relations
EMBEDDING_PATTERNS = [
    # Basic embedding: clause contains word
    ("clause [[ word", "Clauses containing words"),
    ("clause [[ word sp=verb", "Clauses containing verbs"),
    ("clause [[ word sp=subs", "Clauses containing nouns"),
    ("clause [[ word sp=prep", "Clauses containing prepositions"),

    # Phrase embeddings
    ("phrase [[ word", "Phrases containing words"),
    ("phrase [[ word sp=verb", "Phrases containing verbs"),
    ("phrase function=Pred [[ word", "Predicate phrases containing words"),
    ("phrase function=Subj [[ word", "Subject phrases containing words"),

    # Word embedded in clause
    ("word ]] clause", "Words in clauses"),
    ("word sp=verb ]] clause", "Verbs in clauses"),
    ("word sp=subs ]] clause", "Nouns in clauses"),

    # Multi-level embedding
    ("sentence [[ clause", "Sentences containing clauses"),
    ("sentence [[ phrase", "Sentences containing phrases"),
    ("verse [[ word", "Verses containing words"),
    ("chapter [[ clause", "Chapters containing clauses"),

    # Combined patterns
    ("clause [[ phrase function=Pred", "Clauses containing predicate phrases"),
    ("clause [[ phrase function=Subj", "Clauses containing subject phrases"),
    ("sentence [[ word sp=verb", "Sentences containing verbs"),

    # Nested embedding
    ("clause\n  phrase\n    word sp=verb", "Clause > phrase > verb"),
    ("sentence\n  clause\n    word", "Sentence > clause > word"),
]


def run_cf_benchmark(corpus_path: Path, preload: bool = False):
    """Run benchmark with Context-Fabric."""
    from cfabric import Fabric

    print(f"Loading CF corpus (preload={preload})...")
    TF = Fabric(locations=str(corpus_path), silent='deep')
    api = TF.loadAll(silent='deep')

    if preload:
        print("Preloading embedding structures...")
        api.C.levUp.preload()
        api.C.levDown.preload()
        mem = (api.C.levUp.data.memory_usage_bytes() +
               api.C.levDown.data.memory_usage_bytes()) / 1024 / 1024
        print(f"  Preloaded {mem:.1f}MB into RAM")

    results = {}
    total_time = 0

    print("\nRunning CF queries...")
    for pattern, desc in EMBEDDING_PATTERNS:
        try:
            # Warmup
            _ = api.S.search(pattern, limit=1)

            # Timed run
            t0 = time.perf_counter()
            result = api.S.search(pattern)
            elapsed = (time.perf_counter() - t0) * 1000

            count = len(result) if hasattr(result, '__len__') else sum(1 for _ in result)
            results[pattern] = (elapsed, count)
            total_time += elapsed
            print(f"  {desc[:40]:<40} {elapsed:>8.1f}ms  ({count:>7,} results)")
        except Exception as e:
            results[pattern] = (None, str(e))
            print(f"  {desc[:40]:<40} ERROR: {e}")

    print(f"\nTotal CF time: {total_time:.0f}ms")
    return results, total_time


def run_tf_benchmark(corpus_path: Path):
    """Run benchmark with Text-Fabric."""
    try:
        from tf.fabric import Fabric
    except ImportError:
        print("Text-Fabric not installed, skipping TF benchmark")
        return None, None

    print("Loading TF corpus...")
    TF = Fabric(locations=str(corpus_path), silent='deep')
    api = TF.loadAll(silent='deep')

    results = {}
    total_time = 0

    print("\nRunning TF queries...")
    for pattern, desc in EMBEDDING_PATTERNS:
        try:
            # Warmup
            _ = api.S.search(pattern, limit=1)

            # Timed run
            t0 = time.perf_counter()
            result = api.S.search(pattern)
            elapsed = (time.perf_counter() - t0) * 1000

            count = len(result) if hasattr(result, '__len__') else sum(1 for _ in result)
            results[pattern] = (elapsed, count)
            total_time += elapsed
            print(f"  {desc[:40]:<40} {elapsed:>8.1f}ms  ({count:>7,} results)")
        except Exception as e:
            results[pattern] = (None, str(e))
            print(f"  {desc[:40]:<40} ERROR: {e}")

    print(f"\nTotal TF time: {total_time:.0f}ms")
    return results, total_time


def main():
    corpus_path = Path(__file__).parent.parent.parent / ".corpora/bhsa/tf"

    if not corpus_path.exists():
        print(f"Corpus not found at {corpus_path}")
        return

    print("=" * 70)
    print("CF vs TF Embedding Query Benchmark")
    print("=" * 70)

    # CF without preload
    print("\n" + "-" * 70)
    print("CF (mmap only, no preload)")
    print("-" * 70)
    cf_results_mmap, cf_time_mmap = run_cf_benchmark(corpus_path, preload=False)

    # CF with preload
    print("\n" + "-" * 70)
    print("CF (with embedding preload)")
    print("-" * 70)
    cf_results_preload, cf_time_preload = run_cf_benchmark(corpus_path, preload=True)

    # TF
    print("\n" + "-" * 70)
    print("TF (Text-Fabric)")
    print("-" * 70)
    tf_results, tf_time = run_tf_benchmark(corpus_path)

    # Summary
    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)
    print(f"\nTotal embedding query time across {len(EMBEDDING_PATTERNS)} patterns:")
    print(f"  CF (mmap):    {cf_time_mmap:>8.0f}ms")
    print(f"  CF (preload): {cf_time_preload:>8.0f}ms")
    if tf_time:
        print(f"  TF:           {tf_time:>8.0f}ms")

        print(f"\nSpeedup from preload: {cf_time_mmap/cf_time_preload:.1f}x")
        print(f"CF (preload) vs TF:   {tf_time/cf_time_preload:.2f}x {'faster' if cf_time_preload < tf_time else 'slower'}")
    else:
        print(f"\nSpeedup from preload: {cf_time_mmap/cf_time_preload:.1f}x")


if __name__ == "__main__":
    main()
