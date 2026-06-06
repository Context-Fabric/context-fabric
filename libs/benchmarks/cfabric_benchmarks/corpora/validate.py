#!/usr/bin/env python3
"""
Validate Text-Fabric corpora loading in both Text-Fabric and Context-Fabric.

Tests each corpus with:
1. Text-Fabric loading from .tf files
2. Context-Fabric loading from .tf files (which auto-compiles to .cfm)
3. Context-Fabric loading from .cfm cache

Also samples feature values from both .tf and .cfm loading paths to verify
data integrity through the compile/load cycle.

Tests each corpus one at a time to ensure clean memory state and accurate error attribution.

Usage:
    python benchmarks/validate_corpora.py
    python benchmarks/validate_corpora.py --corpus bhsa  # Test single corpus
"""

import argparse
import gc
import shutil
import sys
import traceback
from dataclasses import dataclass
from pathlib import Path
from typing import Any

# Add packages to path
sys.path.insert(0, str(Path(__file__).parent.parent / "packages"))


def clear_caches(tf_path: Path) -> None:
    """Clear Text-Fabric and Context-Fabric cache directories."""
    for cache_name in [".tf", ".cfm"]:
        cache_path = tf_path / cache_name
        if cache_path.exists():
            shutil.rmtree(cache_path)


@dataclass
class FeatureSamples:
    """Sampled feature values for validation."""

    # Dict of feature_name -> list of (node, value) tuples
    node_samples: dict[str, list[tuple[int, Any]]]
    edge_samples: dict[str, list[tuple[int, int, Any]]]
    # List of (node, text) tuples from T.text()
    text_samples: list[tuple[int, str]]


@dataclass
class CorpusStats:
    """Statistics from loading a corpus."""

    max_slot: int = 0
    max_node: int = 0
    node_types: int = 0
    node_features: int = 0
    edge_features: int = 0
    samples: FeatureSamples | None = None
    error: str | None = None


@dataclass
class ValidationResult:
    """Result of validating a single corpus."""

    corpus: str
    tf_stats: CorpusStats
    cf_stats: CorpusStats
    cf_mmap_stats: CorpusStats  # CF loading from .cfm cache

    @property
    def tf_ok(self) -> bool:
        return self.tf_stats.error is None

    @property
    def cf_ok(self) -> bool:
        return self.cf_stats.error is None

    @property
    def cf_mmap_ok(self) -> bool:
        return self.cf_mmap_stats.error is None

    @property
    def stats_match(self) -> bool:
        if not self.tf_ok or not self.cf_ok:
            return False
        return (
            self.tf_stats.max_slot == self.cf_stats.max_slot
            and self.tf_stats.max_node == self.cf_stats.max_node
        )

    @property
    def mmap_stats_match(self) -> bool:
        """Check that .cfm loading produces same stats as .tf loading."""
        if not self.cf_ok or not self.cf_mmap_ok:
            return False
        return (
            self.cf_stats.max_slot == self.cf_mmap_stats.max_slot
            and self.cf_stats.max_node == self.cf_mmap_stats.max_node
            and self.cf_stats.node_features == self.cf_mmap_stats.node_features
            and self.cf_stats.edge_features == self.cf_mmap_stats.edge_features
        )

    @property
    def samples_match(self) -> bool:
        """Check that feature value samples match between .tf and .cfm loading."""
        if not self.cf_ok or not self.cf_mmap_ok:
            return False
        tf_samples = self.cf_stats.samples
        cfm_samples = self.cf_mmap_stats.samples
        if tf_samples is None or cfm_samples is None:
            return True  # No samples to compare

        # Compare node feature samples
        for feat, tf_vals in tf_samples.node_samples.items():
            cfm_vals = cfm_samples.node_samples.get(feat, [])
            if tf_vals != cfm_vals:
                return False

        # Compare edge feature samples
        for feat, tf_vals in tf_samples.edge_samples.items():
            cfm_vals = cfm_samples.edge_samples.get(feat, [])
            if tf_vals != cfm_vals:
                return False

        # Compare T.text() samples
        if tf_samples.text_samples != cfm_samples.text_samples:
            return False

        return True

    def get_sample_mismatches(self) -> list[str]:
        """Get list of features with mismatched samples."""
        mismatches = []
        tf_samples = self.cf_stats.samples
        cfm_samples = self.cf_mmap_stats.samples
        if tf_samples is None or cfm_samples is None:
            return mismatches

        for feat, tf_vals in tf_samples.node_samples.items():
            cfm_vals = cfm_samples.node_samples.get(feat, [])
            if tf_vals != cfm_vals:
                mismatches.append(f"F.{feat}")

        for feat, tf_vals in tf_samples.edge_samples.items():
            cfm_vals = cfm_samples.edge_samples.get(feat, [])
            if tf_vals != cfm_vals:
                mismatches.append(f"E.{feat}")

        if tf_samples.text_samples != cfm_samples.text_samples:
            mismatches.append("T.text()")
            # Add detailed text mismatch info
            mismatch_count = 0
            for (tf_node, tf_text), (cfm_node, cfm_text) in zip(
                tf_samples.text_samples, cfm_samples.text_samples
            ):
                if tf_node != cfm_node or tf_text != cfm_text:
                    tf_preview = repr(tf_text[:50]) if tf_text else repr(tf_text)
                    cfm_preview = repr(cfm_text[:50]) if cfm_text else repr(cfm_text)
                    mismatches.append(f"  node {tf_node}: .tf={tf_preview} vs .cfm={cfm_preview}")
                    mismatch_count += 1
                    if mismatch_count >= 5:
                        mismatches.append("  ... (more mismatches)")
                        break

        return mismatches


def load_with_text_fabric(tf_path: Path) -> CorpusStats:
    """Load corpus with Text-Fabric and return stats."""
    stats = CorpusStats()
    try:
        from tf.fabric import Fabric

        tf = Fabric(locations=str(tf_path), silent="deep")
        api = tf.loadAll(silent="deep")

        stats.max_slot = api.F.otype.maxSlot
        stats.max_node = api.F.otype.maxNode
        stats.node_types = len(api.F.otype.all)
        stats.node_features = len([f for f in dir(api.F) if not f.startswith("_")])
        stats.edge_features = len([f for f in dir(api.E) if not f.startswith("_")])

        del tf, api
        gc.collect()

    except Exception as e:
        stats.error = f"{type(e).__name__}: {e}"
        traceback.print_exc()

    return stats


def sample_feature_values(api, sample_size: int = 100) -> FeatureSamples:
    """Sample feature values from loaded API for validation.

    Samples nodes at regular intervals across the corpus to get representative coverage.
    """
    node_samples: dict[str, list[tuple[int, Any]]] = {}
    edge_samples: dict[str, list[tuple[int, int, Any]]] = {}
    text_samples: list[tuple[int, str]] = []

    max_slot = api.F.otype.maxSlot
    max_node = api.F.otype.maxNode

    # Sample nodes at regular intervals (slots + non-slots)
    all_nodes = list(range(1, max_node + 1))
    step = max(1, len(all_nodes) // sample_size)
    sample_nodes = all_nodes[::step][:sample_size]

    # Get node feature names (excluding special ones)
    node_features = [
        f for f in dir(api.F)
        if not f.startswith("_") and f not in ("otype", "oslots")
    ]

    # Sample up to 5 node features
    features_to_sample = node_features[:5]

    for feat_name in features_to_sample:
        feat = getattr(api.F, feat_name, None)
        if feat is None:
            continue
        samples = []
        for node in sample_nodes:
            try:
                val = feat.v(node)
                # Convert numpy types to Python types for comparison
                if hasattr(val, "item"):
                    val = val.item()
                samples.append((int(node), val))
            except Exception:
                pass
        node_samples[feat_name] = samples

    # Get edge feature names
    edge_features = [
        f for f in dir(api.E)
        if not f.startswith("_") and f not in ("oslots",)
    ]

    # Sample up to 3 edge features
    edge_features_to_sample = edge_features[:3]

    for feat_name in edge_features_to_sample:
        feat = getattr(api.E, feat_name, None)
        if feat is None:
            continue
        samples = []
        # Sample edges from our sample nodes
        for node in sample_nodes[:20]:  # Limit edge sampling
            try:
                targets = feat.f(node)
                if targets:
                    for target in list(targets)[:3]:  # Limit targets per node
                        # Check if edge has values
                        val = feat.v(node, target) if hasattr(feat, "v") else None
                        if hasattr(val, "item"):
                            val = val.item()
                        samples.append((int(node), int(target), val))
            except Exception:
                pass
        edge_samples[feat_name] = samples

    # Sample T.text() for a subset of nodes (slots and non-slots)
    # Use fewer samples since T.text() can be slow for large nodes
    text_sample_nodes = sample_nodes[:50]
    for node in text_sample_nodes:
        try:
            text = api.T.text(node)
            text_samples.append((int(node), text))
        except Exception:
            pass

    return FeatureSamples(
        node_samples=node_samples,
        edge_samples=edge_samples,
        text_samples=text_samples,
    )


def load_with_context_fabric(tf_path: Path, collect_samples: bool = False) -> CorpusStats:
    """Load corpus with Context-Fabric and return stats."""
    stats = CorpusStats()
    try:
        from cfabric import Fabric

        tf = Fabric(locations=str(tf_path), silent="deep")
        api = tf.loadAll(silent="deep")

        stats.max_slot = api.F.otype.maxSlot
        stats.max_node = api.F.otype.maxNode
        stats.node_types = len(api.F.otype.all)
        stats.node_features = len([f for f in dir(api.F) if not f.startswith("_")])
        stats.edge_features = len([f for f in dir(api.E) if not f.startswith("_")])

        if collect_samples:
            stats.samples = sample_feature_values(api)

        del tf, api
        gc.collect()

    except Exception as e:
        stats.error = f"{type(e).__name__}: {e}"
        traceback.print_exc()

    return stats


def validate_corpus(corpus_name: str, corpus_dir: Path) -> ValidationResult:
    """Validate a single corpus with both TF and CF."""
    tf_path = corpus_dir / "tf"

    print(f"\n{'=' * 60}")
    print(f"Validating: {corpus_name}")
    print(f"Path: {tf_path}")
    print("=" * 60)

    # Clear caches for fresh validation
    clear_caches(tf_path)

    # Count TF files
    tf_files = list(tf_path.glob("*.tf"))
    print(f"TF files: {len(tf_files)}")

    # Test Text-Fabric
    print("\n[Text-Fabric] Loading...")
    tf_stats = load_with_text_fabric(tf_path)
    if tf_stats.error:
        print(f"  ERROR: {tf_stats.error}")
    else:
        print(f"  max_slot: {tf_stats.max_slot:,}")
        print(f"  max_node: {tf_stats.max_node:,}")
        print(f"  node_types: {tf_stats.node_types}")
        print(f"  features: {tf_stats.node_features} node, {tf_stats.edge_features} edge")

    # Test Context-Fabric (from .tf, which auto-compiles to .cfm)
    print("\n[Context-Fabric] Loading from .tf...")
    cf_stats = load_with_context_fabric(tf_path, collect_samples=True)
    if cf_stats.error:
        print(f"  ERROR: {cf_stats.error}")
    else:
        print(f"  max_slot: {cf_stats.max_slot:,}")
        print(f"  max_node: {cf_stats.max_node:,}")
        print(f"  node_types: {cf_stats.node_types}")
        print(f"  features: {cf_stats.node_features} node, {cf_stats.edge_features} edge")
        if cf_stats.samples:
            print(f"  sampled: {len(cf_stats.samples.node_samples)} node features, {len(cf_stats.samples.edge_samples)} edge features, {len(cf_stats.samples.text_samples)} T.text() calls")

    # Test Context-Fabric loading from .cfm cache
    print("\n[Context-Fabric] Loading from .cfm cache...")
    cf_mmap_stats = load_with_context_fabric(tf_path, collect_samples=True)
    if cf_mmap_stats.error:
        print(f"  ERROR: {cf_mmap_stats.error}")
    else:
        print(f"  max_slot: {cf_mmap_stats.max_slot:,}")
        print(f"  max_node: {cf_mmap_stats.max_node:,}")
        print(f"  node_types: {cf_mmap_stats.node_types}")
        print(f"  features: {cf_mmap_stats.node_features} node, {cf_mmap_stats.edge_features} edge")
        if cf_mmap_stats.samples:
            print(f"  sampled: {len(cf_mmap_stats.samples.node_samples)} node features, {len(cf_mmap_stats.samples.edge_samples)} edge features, {len(cf_mmap_stats.samples.text_samples)} T.text() calls")

    # Compare
    result = ValidationResult(
        corpus=corpus_name,
        tf_stats=tf_stats,
        cf_stats=cf_stats,
        cf_mmap_stats=cf_mmap_stats,
    )

    if result.tf_ok and result.cf_ok and result.cf_mmap_ok:
        if result.stats_match and result.mmap_stats_match and result.samples_match:
            print("\n[PASS] All implementations loaded successfully with matching stats and samples")
        elif not result.stats_match:
            print("\n[WARN] TF vs CF stats differ!")
            print(f"  TF max_slot={tf_stats.max_slot}, CF max_slot={cf_stats.max_slot}")
            print(f"  TF max_node={tf_stats.max_node}, CF max_node={cf_stats.max_node}")
        elif not result.mmap_stats_match:
            print("\n[WARN] CF .tf vs .cfm stats differ!")
            print(f"  .tf: {cf_stats.node_features} node, {cf_stats.edge_features} edge")
            print(f"  .cfm: {cf_mmap_stats.node_features} node, {cf_mmap_stats.edge_features} edge")
        elif not result.samples_match:
            print("\n[FAIL] CF .tf vs .cfm feature values differ!")
            mismatches = result.get_sample_mismatches()
            print(f"  Mismatched features: {', '.join(mismatches)}")
    elif not result.cf_mmap_ok:
        print("\n[FAIL] Context-Fabric .cfm loading FAILED")
    elif result.tf_ok and not result.cf_ok:
        print("\n[FAIL] Text-Fabric OK, Context-Fabric FAILED")
    elif not result.tf_ok and result.cf_ok:
        print("\n[FAIL] Text-Fabric FAILED, Context-Fabric OK")
    else:
        print("\n[FAIL] Both implementations FAILED")

    return result


def print_summary(results: list[ValidationResult]) -> None:
    """Print summary table of all results."""
    print("\n" + "=" * 90)
    print("VALIDATION SUMMARY")
    print("=" * 90)

    print(f"\n{'Corpus':<15} {'TF':<6} {'CF':<6} {'CFM':<6} {'Match':<6} {'Slots':>12} {'Nodes':>12}")
    print("-" * 90)

    passed = 0
    failed = 0

    for r in results:
        tf_status = "OK" if r.tf_ok else "FAIL"
        cf_status = "OK" if r.cf_ok else "FAIL"
        cfm_status = "OK" if r.cf_mmap_ok else "FAIL"
        match_status = "YES" if r.stats_match and r.mmap_stats_match and r.samples_match else "NO"

        slots = f"{r.tf_stats.max_slot:,}" if r.tf_ok else "-"
        nodes = f"{r.tf_stats.max_node:,}" if r.tf_ok else "-"

        print(f"{r.corpus:<15} {tf_status:<6} {cf_status:<6} {cfm_status:<6} {match_status:<6} {slots:>12} {nodes:>12}")

        if r.tf_ok and r.cf_ok and r.cf_mmap_ok and r.stats_match and r.mmap_stats_match and r.samples_match:
            passed += 1
        else:
            failed += 1

    print("-" * 90)
    print(f"Total: {passed} passed, {failed} failed out of {len(results)} corpora")

    if failed > 0:
        print("\nFailed corpora need investigation.")


def main():
    parser = argparse.ArgumentParser(description="Validate TF corpora loading")
    parser.add_argument("--corpus", help="Validate single corpus by name")
    args = parser.parse_args()

    # Corpora at package root (.corpora at libs/benchmarks/ level)
    script_dir = Path(__file__).parent
    corpora_dir = script_dir.parent.parent / ".corpora"

    # Find all corpora
    corpus_dirs = sorted(
        [d for d in corpora_dir.iterdir() if d.is_dir() and (d / "tf").exists()]
    )

    if args.corpus:
        # Single corpus mode
        corpus_dirs = [d for d in corpus_dirs if d.name == args.corpus]
        if not corpus_dirs:
            print(f"Corpus not found: {args.corpus}")
            sys.exit(1)

    print("=" * 60)
    print("Biblical Studies Corpora Validation")
    print("=" * 60)
    print(f"\nCorpora directory: {corpora_dir}")
    print(f"Corpora to validate: {len(corpus_dirs)}")

    results = []
    for corpus_dir in corpus_dirs:
        result = validate_corpus(corpus_dir.name, corpus_dir)
        results.append(result)
        gc.collect()

    print_summary(results)

    # Exit with error code if any failed
    if any(
        not (r.tf_ok and r.cf_ok and r.cf_mmap_ok and r.stats_match and r.mmap_stats_match and r.samples_match)
        for r in results
    ):
        sys.exit(1)


if __name__ == "__main__":
    main()
