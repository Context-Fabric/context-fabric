#!/usr/bin/env python3
"""
Performance Comparison: Text-Fabric vs Context-Fabric

This benchmark compares loading performance and memory consumption between:
- Text-Fabric (TF): Original implementation using pickle/gzip caching
- Context-Fabric (CF): New implementation using memory-mapped numpy arrays

Usage:
    python benchmarks/compare_performance.py [--source PATH] [--output DIR]

Requirements:
    pip install text-fabric seaborn pandas matplotlib
"""

import argparse
import gc
import os
import shutil
import sys
import time
from dataclasses import dataclass
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd
import psutil
import seaborn as sns

# Add packages to path
sys.path.insert(0, str(Path(__file__).parent.parent / 'packages'))

# Default BHSA source path
DEFAULT_SOURCE = '/Users/cody/github/etcbc/bhsa/tf/2021'


@dataclass
class BenchmarkResult:
    """Store benchmark results for one implementation."""
    name: str
    compile_time: float = 0.0
    load_time: float = 0.0
    memory_before: float = 0.0
    memory_after: float = 0.0
    cache_size: float = 0.0

    @property
    def memory_used(self) -> float:
        return self.memory_after - self.memory_before


def get_memory_mb() -> float:
    """Get current process memory usage in MB."""
    return psutil.Process(os.getpid()).memory_info().rss / 1024 / 1024


def get_dir_size_mb(path: Path) -> float:
    """Get total size of directory in MB."""
    if not path.exists():
        return 0.0
    total = sum(f.stat().st_size for f in path.rglob('*') if f.is_file())
    return total / 1024 / 1024


def clear_caches(source: str) -> None:
    """Remove TF and CF cache directories."""
    for cache_name in ['.tf', '.cfm']:
        cache_path = Path(source) / cache_name
        if cache_path.exists():
            shutil.rmtree(cache_path)
            print(f"  Removed: {cache_path}")


def benchmark_text_fabric(source: str) -> BenchmarkResult:
    """Benchmark Text-Fabric loading."""
    from tf.fabric import Fabric as TFFabric

    result = BenchmarkResult("Text-Fabric")
    tf_cache = Path(source) / '.tf'

    # Measure compile (first load)
    gc.collect()
    result.memory_before = get_memory_mb()
    print("  Compiling (first load)...")

    start = time.perf_counter()
    tf = TFFabric(locations=source, silent='deep')
    api = tf.loadAll(silent='deep')
    result.compile_time = time.perf_counter() - start

    result.memory_after = get_memory_mb()
    result.cache_size = get_dir_size_mb(tf_cache)

    # Verify
    print(f"    Max node: {api.F.otype.maxNode:,}")

    # Cleanup
    del tf, api
    gc.collect()

    # Measure reload (from cache)
    print("  Loading from cache...")
    start = time.perf_counter()
    tf2 = TFFabric(locations=source, silent='deep')
    api2 = tf2.loadAll(silent='deep')
    result.load_time = time.perf_counter() - start

    del tf2, api2
    gc.collect()

    return result


def benchmark_context_fabric(source: str) -> BenchmarkResult:
    """Benchmark Context-Fabric loading."""
    from cfabric.core.fabric import Fabric as CFFabric

    result = BenchmarkResult("Context-Fabric")
    cf_cache = Path(source) / '.cfm'

    # Clear CF cache for fresh compile
    if cf_cache.exists():
        shutil.rmtree(cf_cache)

    # Measure compile (first load)
    gc.collect()
    result.memory_before = get_memory_mb()
    print("  Compiling (first load)...")

    start = time.perf_counter()
    tf = CFFabric(locations=source, silent='deep')
    api = tf.load('')
    result.compile_time = time.perf_counter() - start

    result.memory_after = get_memory_mb()
    result.cache_size = get_dir_size_mb(cf_cache)

    # Verify
    print(f"    Max node: {api.F.otype.maxNode:,}")

    # Cleanup
    del tf, api
    gc.collect()

    # Measure reload (from cache)
    print("  Loading from cache...")
    start = time.perf_counter()
    tf2 = CFFabric(locations=source, silent='deep')
    api2 = tf2.load('')
    result.load_time = time.perf_counter() - start

    del tf2, api2
    gc.collect()

    return result


def create_results_table(tf_result: BenchmarkResult, cf_result: BenchmarkResult) -> pd.DataFrame:
    """Create results comparison table."""
    data = {
        'Metric': [
            'Compile Time (s)',
            'Load Time (s)',
            'Memory Used (MB)',
            'Cache Size (MB)'
        ],
        'Text-Fabric': [
            tf_result.compile_time,
            tf_result.load_time,
            tf_result.memory_used,
            tf_result.cache_size
        ],
        'Context-Fabric': [
            cf_result.compile_time,
            cf_result.load_time,
            cf_result.memory_used,
            cf_result.cache_size
        ]
    }

    df = pd.DataFrame(data)

    # Calculate improvement
    improvements = []
    for i, metric in enumerate(df['Metric']):
        tf_val = df.iloc[i]['Text-Fabric']
        cf_val = df.iloc[i]['Context-Fabric']

        if 'Time' in metric:
            # For time, higher ratio = CF is faster
            ratio = tf_val / cf_val if cf_val > 0 else 0
            if ratio > 1:
                improvements.append(f"{ratio:.2f}x faster")
            else:
                improvements.append(f"{1/ratio:.2f}x slower")
        elif 'Memory' in metric:
            # For memory, show reduction percentage
            reduction = (1 - cf_val / tf_val) * 100 if tf_val > 0 else 0
            if reduction > 0:
                improvements.append(f"{reduction:.1f}% less")
            else:
                improvements.append(f"{-reduction:.1f}% more")
        else:
            # Cache size
            ratio = cf_val / tf_val if tf_val > 0 else 0
            if ratio > 1:
                improvements.append(f"{ratio:.1f}x larger")
            else:
                improvements.append(f"{1/ratio:.1f}x smaller")

    df['CF Improvement'] = improvements
    return df


def create_charts(tf_result: BenchmarkResult, cf_result: BenchmarkResult, output_dir: Path) -> None:
    """Create performance comparison charts with dark mode."""
    # Set dark mode style
    plt.style.use('dark_background')
    sns.set_theme(style="darkgrid", rc={
        "axes.facecolor": "#1a1a2e",
        "figure.facecolor": "#0f0f1a",
        "grid.color": "#2a2a4a",
        "text.color": "#e0e0e0",
        "axes.labelcolor": "#e0e0e0",
        "xtick.color": "#e0e0e0",
        "ytick.color": "#e0e0e0"
    })

    # Color palette
    colors = ['#ff6b6b', '#4ecdc4']  # Red for TF, Teal for CF

    # Create figure with subplots
    fig, axes = plt.subplots(2, 2, figsize=(14, 10))
    fig.suptitle('Context-Fabric vs Text-Fabric Performance', fontsize=16, fontweight='bold', color='white')

    # 1. Load Time Comparison
    ax1 = axes[0, 0]
    times = [tf_result.load_time, cf_result.load_time]
    bars1 = ax1.bar(['Text-Fabric', 'Context-Fabric'], times, color=colors, edgecolor='white', linewidth=1.5)
    ax1.set_ylabel('Time (seconds)', fontsize=11)
    ax1.set_title('Cache Load Time', fontsize=13, fontweight='bold')
    for bar, val in zip(bars1, times):
        ax1.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.1,
                f'{val:.2f}s', ha='center', va='bottom', fontsize=11, fontweight='bold')
    speedup = tf_result.load_time / cf_result.load_time
    ax1.text(0.5, 0.95, f'{speedup:.1f}x faster', transform=ax1.transAxes,
             ha='center', va='top', fontsize=12, color='#4ecdc4', fontweight='bold')

    # 2. Memory Usage Comparison
    ax2 = axes[0, 1]
    memory = [tf_result.memory_used, cf_result.memory_used]
    bars2 = ax2.bar(['Text-Fabric', 'Context-Fabric'], memory, color=colors, edgecolor='white', linewidth=1.5)
    ax2.set_ylabel('Memory (MB)', fontsize=11)
    ax2.set_title('Memory Usage', fontsize=13, fontweight='bold')
    for bar, val in zip(bars2, memory):
        label = f'{val:.0f} MB' if val < 1000 else f'{val/1024:.1f} GB'
        ax2.text(bar.get_x() + bar.get_width()/2, bar.get_height() + max(memory)*0.02,
                label, ha='center', va='bottom', fontsize=11, fontweight='bold')
    reduction = (1 - cf_result.memory_used / tf_result.memory_used) * 100
    ax2.text(0.5, 0.95, f'{reduction:.0f}% reduction', transform=ax2.transAxes,
             ha='center', va='top', fontsize=12, color='#4ecdc4', fontweight='bold')

    # 3. Compile Time Comparison
    ax3 = axes[1, 0]
    compile_times = [tf_result.compile_time, cf_result.compile_time]
    bars3 = ax3.bar(['Text-Fabric', 'Context-Fabric'], compile_times, color=colors, edgecolor='white', linewidth=1.5)
    ax3.set_ylabel('Time (seconds)', fontsize=11)
    ax3.set_title('Initial Compile Time', fontsize=13, fontweight='bold')
    for bar, val in zip(bars3, compile_times):
        ax3.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 1,
                f'{val:.1f}s', ha='center', va='bottom', fontsize=11, fontweight='bold')

    # 4. Cache Size Comparison
    ax4 = axes[1, 1]
    cache_sizes = [tf_result.cache_size, cf_result.cache_size]
    bars4 = ax4.bar(['Text-Fabric', 'Context-Fabric'], cache_sizes, color=colors, edgecolor='white', linewidth=1.5)
    ax4.set_ylabel('Size (MB)', fontsize=11)
    ax4.set_title('Cache Size on Disk', fontsize=13, fontweight='bold')
    for bar, val in zip(bars4, cache_sizes):
        ax4.text(bar.get_x() + bar.get_width()/2, bar.get_height() + max(cache_sizes)*0.02,
                f'{val:.0f} MB', ha='center', va='bottom', fontsize=11, fontweight='bold')

    plt.tight_layout()
    plt.savefig(output_dir / 'performance_comparison.png', dpi=150, bbox_inches='tight',
                facecolor='#0f0f1a', edgecolor='none')
    plt.close()

    # Create summary bar chart
    fig2, ax = plt.subplots(figsize=(10, 6))
    fig2.patch.set_facecolor('#0f0f1a')

    metrics = ['Load\nTime', 'Memory\nUsage']
    tf_normalized = [1.0, 1.0]  # TF as baseline
    cf_normalized = [
        cf_result.load_time / tf_result.load_time,
        cf_result.memory_used / tf_result.memory_used
    ]

    x = np.arange(len(metrics))
    width = 0.35

    bars_tf = ax.bar(x - width/2, tf_normalized, width, label='Text-Fabric', color='#ff6b6b', edgecolor='white')
    bars_cf = ax.bar(x + width/2, cf_normalized, width, label='Context-Fabric', color='#4ecdc4', edgecolor='white')

    ax.set_ylabel('Relative to Text-Fabric (lower is better)', fontsize=11)
    ax.set_title('Context-Fabric Performance (normalized)', fontsize=14, fontweight='bold', color='white')
    ax.set_xticks(x)
    ax.set_xticklabels(metrics, fontsize=12)
    ax.legend(loc='upper right', fontsize=11)
    ax.axhline(y=1.0, color='#666666', linestyle='--', alpha=0.7)

    # Add value labels
    for bar in bars_cf:
        height = bar.get_height()
        ax.text(bar.get_x() + bar.get_width()/2, height + 0.02,
               f'{height:.2f}x', ha='center', va='bottom', fontsize=11, fontweight='bold', color='#4ecdc4')

    plt.tight_layout()
    plt.savefig(output_dir / 'performance_normalized.png', dpi=150, bbox_inches='tight',
                facecolor='#0f0f1a', edgecolor='none')
    plt.close()

    print(f"\nCharts saved to {output_dir}/")


def print_results(tf_result: BenchmarkResult, cf_result: BenchmarkResult) -> None:
    """Print results to console."""
    print("\n" + "=" * 70)
    print("PERFORMANCE COMPARISON RESULTS")
    print("=" * 70)

    print(f"\n{tf_result.name}:")
    print(f"  Compile time: {tf_result.compile_time:.2f}s")
    print(f"  Load time:    {tf_result.load_time:.2f}s")
    print(f"  Memory used:  {tf_result.memory_used:.1f} MB")
    print(f"  Cache size:   {tf_result.cache_size:.1f} MB")

    print(f"\n{cf_result.name}:")
    print(f"  Compile time: {cf_result.compile_time:.2f}s")
    print(f"  Load time:    {cf_result.load_time:.2f}s")
    print(f"  Memory used:  {cf_result.memory_used:.1f} MB")
    print(f"  Cache size:   {cf_result.cache_size:.1f} MB")

    print("\n" + "=" * 70)
    print("ANALYSIS")
    print("=" * 70)

    load_speedup = tf_result.load_time / cf_result.load_time
    memory_reduction = (1 - cf_result.memory_used / tf_result.memory_used) * 100

    print(f"\n  Load speedup:      {load_speedup:.2f}x faster")
    print(f"  Memory reduction:  {memory_reduction:.1f}%")
    print("=" * 70)


def main():
    parser = argparse.ArgumentParser(description='Benchmark Text-Fabric vs Context-Fabric')
    parser.add_argument('--source', default=DEFAULT_SOURCE, help='Path to TF source files')
    parser.add_argument('--output', default='benchmarks/results', help='Output directory for charts')
    parser.add_argument('--skip-clear', action='store_true', help='Skip clearing caches before benchmark')
    args = parser.parse_args()

    source = args.source
    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)

    print("=" * 70)
    print("Context-Fabric Performance Benchmark")
    print("=" * 70)
    print(f"\nSource: {source}")
    print(f"Output: {output_dir}")

    # Clear caches
    if not args.skip_clear:
        print("\nClearing caches...")
        clear_caches(source)

    # Benchmark Text-Fabric
    print("\n[1/2] Benchmarking Text-Fabric...")
    tf_result = benchmark_text_fabric(source)

    # Benchmark Context-Fabric
    print("\n[2/2] Benchmarking Context-Fabric...")
    cf_result = benchmark_context_fabric(source)

    # Print results
    print_results(tf_result, cf_result)

    # Create results table
    df = create_results_table(tf_result, cf_result)
    print("\nResults Table:")
    print(df.to_string(index=False))

    # Save CSV
    csv_path = output_dir / 'results.csv'
    df.to_csv(csv_path, index=False)
    print(f"\nResults saved to {csv_path}")

    # Create charts
    print("\nGenerating charts...")
    create_charts(tf_result, cf_result, output_dir)

    print("\nBenchmark complete!")


if __name__ == '__main__':
    main()
