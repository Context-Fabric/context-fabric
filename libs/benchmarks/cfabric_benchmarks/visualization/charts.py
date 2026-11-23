"""Chart generation for benchmark results.

Uses dark mode styling for reusable visualizations.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

import matplotlib.pyplot as plt
import numpy as np
import seaborn as sns

from cfabric_benchmarks.models.latency import LatencyBenchmarkResult
from cfabric_benchmarks.models.memory import MemoryBenchmarkResult
from cfabric_benchmarks.models.progressive import ProgressiveLoadResult


# Color scheme
COLORS = {
    "tf": "#ff6b6b",  # Red for Text-Fabric
    "cf": "#4ecdc4",  # Teal for Context-Fabric
}


def setup_dark_style() -> None:
    """Set up dark mode matplotlib style."""
    plt.style.use("dark_background")
    sns.set_theme(
        style="darkgrid",
        rc={
            "axes.facecolor": "#1a1a2e",
            "figure.facecolor": "#0f0f1a",
            "grid.color": "#2a2a4a",
            "text.color": "#e0e0e0",
            "axes.labelcolor": "#e0e0e0",
            "xtick.color": "#e0e0e0",
            "ytick.color": "#e0e0e0",
        },
    )


def create_memory_comparison_chart(
    result: MemoryBenchmarkResult,
    output_path: Path,
    spawn_result: MemoryBenchmarkResult | None = None,
    fork_result: MemoryBenchmarkResult | None = None,
) -> None:
    """Create 2x2 memory comparison chart.

    Args:
        result: Memory benchmark result (single mode)
        output_path: Path to save chart
        spawn_result: Optional spawn mode result
        fork_result: Optional fork mode result
    """
    setup_dark_style()

    fig, axes = plt.subplots(2, 2, figsize=(14, 10))
    axes = axes.flatten()

    corpus_name = result.corpus_stats.name
    fig.suptitle(
        f"Context-Fabric vs Text-Fabric Performance\n"
        f"{corpus_name}: {result.corpus_stats.max_node:,} nodes, "
        f"{result.corpus_stats.node_features} features",
        fontsize=18,
        fontweight="bold",
        color="white",
        y=0.98,
    )

    colors = [COLORS["tf"], COLORS["cf"]]

    # 1. Load Time Comparison
    ax1 = axes[0]
    if result.tf_load_time_stats and result.cf_load_time_stats:
        times = [result.tf_load_time_stats.mean, result.cf_load_time_stats.mean]
        errors = [result.tf_load_time_stats.std, result.cf_load_time_stats.std]
        bars1 = ax1.bar(
            ["Text-Fabric", "Context-Fabric"],
            times,
            yerr=errors,
            color=colors,
            edgecolor="white",
            linewidth=2,
            capsize=5,
        )
        ax1.set_ylabel("Time (seconds)", fontsize=14)
        ax1.set_title("Cache Load Time", fontsize=16, fontweight="bold", pad=15)
        ax1.tick_params(axis="both", labelsize=13)
        ax1.set_ylim(0, max(times) * 1.35)
        for bar, val in zip(bars1, times):
            ax1.text(
                bar.get_x() + bar.get_width() / 2,
                bar.get_height() + max(times) * 0.05,
                f"{val:.2f}s",
                ha="center",
                va="bottom",
                fontsize=15,
                fontweight="bold",
            )
        # Calculate speedup ratio (TF/CF)
        # If > 1, CF is faster; if < 1, TF is faster
        if times[1] > 0 and times[0] > 0:
            if times[0] > times[1]:
                # CF is faster
                speedup = times[0] / times[1]
                speedup_label = f"{speedup:.1f}x faster"
            else:
                # TF is faster (CF is slower)
                speedup = times[1] / times[0]
                speedup_label = f"{speedup:.1f}x slower"
        else:
            speedup_label = "N/A"
        ax1.text(
            0.5,
            0.92,
            speedup_label,
            transform=ax1.transAxes,
            ha="center",
            va="top",
            fontsize=16,
            color=COLORS["cf"],
            fontweight="bold",
            bbox=dict(
                boxstyle="round",
                facecolor="#1a1a2e",
                edgecolor=COLORS["cf"],
                alpha=0.8,
            ),
        )
    else:
        ax1.text(0.5, 0.5, "No load time data", ha="center", va="center", fontsize=14)
        ax1.set_title("Cache Load Time", fontsize=16, fontweight="bold", pad=15)

    # 2. Memory Usage Comparison
    ax2 = axes[1]
    if result.tf_memory_stats and result.cf_memory_stats:
        memory = [result.tf_memory_stats.mean, result.cf_memory_stats.mean]
        errors = [result.tf_memory_stats.std, result.cf_memory_stats.std]
        bars2 = ax2.bar(
            ["Text-Fabric", "Context-Fabric"],
            memory,
            yerr=errors,
            color=colors,
            edgecolor="white",
            linewidth=2,
            capsize=5,
        )
        ax2.set_ylabel("Memory (MB)", fontsize=14)
        ax2.set_title("Memory Usage", fontsize=16, fontweight="bold", pad=15)
        ax2.tick_params(axis="both", labelsize=13)
        ax2.set_ylim(0, max(memory) * 1.35)
        for bar, val in zip(bars2, memory):
            label = f"{val:.0f} MB" if val < 1000 else f"{val/1024:.1f} GB"
            ax2.text(
                bar.get_x() + bar.get_width() / 2,
                bar.get_height() + max(memory) * 0.05,
                label,
                ha="center",
                va="bottom",
                fontsize=15,
                fontweight="bold",
            )
        reduction = (1 - memory[1] / memory[0]) * 100 if memory[0] > 0 else 0
        ax2.text(
            0.5,
            0.92,
            f"{reduction:.0f}% reduction",
            transform=ax2.transAxes,
            ha="center",
            va="top",
            fontsize=16,
            color=COLORS["cf"],
            fontweight="bold",
            bbox=dict(
                boxstyle="round",
                facecolor="#1a1a2e",
                edgecolor=COLORS["cf"],
                alpha=0.8,
            ),
        )
    else:
        ax2.text(0.5, 0.5, "No memory data", ha="center", va="center", fontsize=14)
        ax2.set_title("Memory Usage", fontsize=16, fontweight="bold", pad=15)

    # 3. Spawn Workers (if available)
    ax3 = axes[2]
    if result.tf_spawn_stats and result.cf_spawn_stats:
        # Calculate per-worker from total
        num_workers = 4  # Default
        for m in result.measurements:
            if m.num_workers:
                num_workers = m.num_workers
                break
        par_mem = [
            result.tf_spawn_stats.mean / num_workers,
            result.cf_spawn_stats.mean / num_workers,
        ]
        bars3 = ax3.bar(
            ["Text-Fabric", "Context-Fabric"],
            par_mem,
            color=colors,
            edgecolor="white",
            linewidth=2,
        )
        ax3.set_ylabel("Memory per Worker (MB)", fontsize=14)
        ax3.set_title(
            f"Spawn Workers ({num_workers}w, cold start)",
            fontsize=16,
            fontweight="bold",
            pad=15,
        )
        ax3.tick_params(axis="both", labelsize=13)
        ax3.set_ylim(0, max(par_mem) * 1.35)
        for bar, val in zip(bars3, par_mem):
            label = f"{val:.0f} MB" if val < 1000 else f"{val/1024:.1f} GB"
            ax3.text(
                bar.get_x() + bar.get_width() / 2,
                bar.get_height() + max(par_mem) * 0.05,
                label,
                ha="center",
                va="bottom",
                fontsize=15,
                fontweight="bold",
            )
        if par_mem[1] > 0:
            ratio = par_mem[0] / par_mem[1]
            ax3.text(
                0.5,
                0.92,
                f"{ratio:.1f}x less",
                transform=ax3.transAxes,
                ha="center",
                va="top",
                fontsize=16,
                color=COLORS["cf"],
                fontweight="bold",
                bbox=dict(
                    boxstyle="round",
                    facecolor="#1a1a2e",
                    edgecolor=COLORS["cf"],
                    alpha=0.8,
                ),
            )
    else:
        ax3.text(0.5, 0.5, "No spawn data", ha="center", va="center", fontsize=14)
        ax3.set_title("Spawn Workers (cold start)", fontsize=16, fontweight="bold", pad=15)

    # 4. Fork Workers (if available)
    ax4 = axes[3]
    if result.tf_fork_stats and result.cf_fork_stats:
        num_workers = 4
        for m in result.measurements:
            if m.num_workers:
                num_workers = m.num_workers
                break
        api_mem = [
            result.tf_fork_stats.mean / num_workers,
            result.cf_fork_stats.mean / num_workers,
        ]
        bars4 = ax4.bar(
            ["Text-Fabric", "Context-Fabric"],
            api_mem,
            color=colors,
            edgecolor="white",
            linewidth=2,
        )
        ax4.set_ylabel("Memory per Worker (MB)", fontsize=14)
        ax4.set_title(
            f"Fork Workers ({num_workers}w, pre-loaded API)",
            fontsize=16,
            fontweight="bold",
            pad=15,
        )
        ax4.tick_params(axis="both", labelsize=13)
        ax4.set_ylim(0, max(api_mem) * 1.35)
        for bar, val in zip(bars4, api_mem):
            label = f"{val:.0f} MB" if val < 1000 else f"{val/1024:.1f} GB"
            ax4.text(
                bar.get_x() + bar.get_width() / 2,
                bar.get_height() + max(api_mem) * 0.05,
                label,
                ha="center",
                va="bottom",
                fontsize=15,
                fontweight="bold",
            )
        if api_mem[1] > 0:
            ratio = api_mem[0] / api_mem[1]
            ax4.text(
                0.5,
                0.92,
                f"{ratio:.1f}x less",
                transform=ax4.transAxes,
                ha="center",
                va="top",
                fontsize=16,
                color=COLORS["cf"],
                fontweight="bold",
                bbox=dict(
                    boxstyle="round",
                    facecolor="#1a1a2e",
                    edgecolor=COLORS["cf"],
                    alpha=0.8,
                ),
            )
    else:
        ax4.text(0.5, 0.5, "No fork data", ha="center", va="center", fontsize=14)
        ax4.set_title("Fork Workers (pre-loaded API)", fontsize=16, fontweight="bold", pad=15)

    plt.tight_layout(rect=[0, 0, 1, 0.95])
    output_path.parent.mkdir(parents=True, exist_ok=True)
    plt.savefig(
        output_path,
        dpi=300,
        bbox_inches="tight",
        facecolor="#0f0f1a",
        edgecolor="none",
    )
    plt.close()


def create_progressive_scaling_chart(
    result: ProgressiveLoadResult,
    output_path: Path,
) -> None:
    """Create progressive scaling line chart.

    Args:
        result: Progressive load result
        output_path: Path to save chart
    """
    setup_dark_style()

    fig, ax = plt.subplots(figsize=(12, 6))

    x = list(range(1, result.max_corpora + 1))

    # Plot TF and CF lines
    if result.tf_memory_by_step:
        ax.plot(
            x,
            result.tf_memory_by_step,
            marker="^",
            color=COLORS["tf"],
            linewidth=2,
            markersize=10,
            label="Text-Fabric",
        )
    if result.cf_memory_by_step:
        ax.plot(
            x,
            result.cf_memory_by_step,
            marker="o",
            color=COLORS["cf"],
            linewidth=2,
            markersize=10,
            label="Context-Fabric",
        )

    ax.set_xlabel("Corpus Added at Each Step (cumulative)", fontsize=14)
    ax.set_ylabel("Total Memory (MB)", fontsize=14)
    ax.set_title(
        "Memory During Progressive Corpus Loading",
        fontsize=16,
        fontweight="bold",
    )
    ax.set_xticks(x)
    ax.set_xticklabels(result.corpora_order, rotation=45, ha="right")
    ax.legend(fontsize=12, loc="upper left")
    ax.tick_params(axis="both", labelsize=12)

    plt.tight_layout()
    output_path.parent.mkdir(parents=True, exist_ok=True)
    plt.savefig(
        output_path,
        dpi=300,
        bbox_inches="tight",
        facecolor="#0f0f1a",
        edgecolor="none",
    )
    plt.close()


def create_latency_distribution_chart(
    result: LatencyBenchmarkResult,
    output_path: Path,
) -> None:
    """Create latency distribution box plot.

    Args:
        result: Latency benchmark result
        output_path: Path to save chart
    """
    setup_dark_style()

    fig, ax = plt.subplots(figsize=(12, 6))

    # Prepare data by category
    categories = ["lexical", "structural", "quantified", "complex"]
    data = []
    positions = []
    colors = []

    for i, cat in enumerate(categories):
        tf_times = [
            m.execution_time_ms
            for m in result.measurements
            if m.implementation == "TF"
            and m.success
            and any(p.id == m.query_id and p.category == cat for p in result.queries)
        ]
        cf_times = [
            m.execution_time_ms
            for m in result.measurements
            if m.implementation == "CF"
            and m.success
            and any(p.id == m.query_id and p.category == cat for p in result.queries)
        ]

        if tf_times:
            data.append(tf_times)
            positions.append(i * 3)
            colors.append(COLORS["tf"])
        if cf_times:
            data.append(cf_times)
            positions.append(i * 3 + 1)
            colors.append(COLORS["cf"])

    if data:
        bp = ax.boxplot(
            data,
            positions=positions,
            patch_artist=True,
            widths=0.7,
        )

        for patch, color in zip(bp["boxes"], colors):
            patch.set_facecolor(color)
            patch.set_alpha(0.7)

        # Custom x-axis labels
        ax.set_xticks([i * 3 + 0.5 for i in range(len(categories))])
        ax.set_xticklabels([c.capitalize() for c in categories])

        # Legend
        from matplotlib.patches import Patch

        legend_elements = [
            Patch(facecolor=COLORS["tf"], alpha=0.7, label="Text-Fabric"),
            Patch(facecolor=COLORS["cf"], alpha=0.7, label="Context-Fabric"),
        ]
        ax.legend(handles=legend_elements, fontsize=12, loc="upper right")

    ax.set_ylabel("Execution Time (ms)", fontsize=14)
    ax.set_title(
        f"Query Latency by Pattern Category\n{result.corpus}",
        fontsize=16,
        fontweight="bold",
    )
    ax.tick_params(axis="both", labelsize=12)

    # Use log scale if range is large
    if data:
        all_times = [t for d in data for t in d]
        if max(all_times) / min(all_times) > 100:
            ax.set_yscale("log")

    plt.tight_layout()
    output_path.parent.mkdir(parents=True, exist_ok=True)
    plt.savefig(
        output_path,
        dpi=300,
        bbox_inches="tight",
        facecolor="#0f0f1a",
        edgecolor="none",
    )
    plt.close()


def create_latency_percentiles_chart(
    result: LatencyBenchmarkResult,
    output_path: Path,
) -> None:
    """Create latency percentiles bar chart.

    Args:
        result: Latency benchmark result
        output_path: Path to save chart
    """
    setup_dark_style()

    fig, ax = plt.subplots(figsize=(10, 6))

    percentiles = ["p50", "p95", "p99"]
    x = np.arange(len(percentiles))
    width = 0.35

    tf_values = []
    cf_values = []

    if result.tf_overall_stats and result.cf_overall_stats:
        tf_values = [
            result.tf_overall_stats.p50,
            result.tf_overall_stats.p95,
            result.tf_overall_stats.p99,
        ]
        cf_values = [
            result.cf_overall_stats.p50,
            result.cf_overall_stats.p95,
            result.cf_overall_stats.p99,
        ]

    if tf_values and cf_values:
        ax.bar(x - width / 2, tf_values, width, label="Text-Fabric", color=COLORS["tf"])
        ax.bar(x + width / 2, cf_values, width, label="Context-Fabric", color=COLORS["cf"])

        ax.set_xlabel("Percentile", fontsize=14)
        ax.set_ylabel("Latency (ms)", fontsize=14)
        ax.set_title(
            f"Query Latency Percentiles\n{result.corpus}",
            fontsize=16,
            fontweight="bold",
        )
        ax.set_xticks(x)
        ax.set_xticklabels(percentiles)
        ax.legend(fontsize=12, loc="upper left")

        # Add value labels
        for i, (tf, cf) in enumerate(zip(tf_values, cf_values)):
            ax.text(
                i - width / 2,
                tf + max(tf_values) * 0.02,
                f"{tf:.1f}",
                ha="center",
                fontsize=10,
            )
            ax.text(
                i + width / 2,
                cf + max(cf_values) * 0.02,
                f"{cf:.1f}",
                ha="center",
                fontsize=10,
            )
    else:
        ax.text(0.5, 0.5, "No latency data", ha="center", va="center", fontsize=14)

    ax.tick_params(axis="both", labelsize=12)

    plt.tight_layout()
    output_path.parent.mkdir(parents=True, exist_ok=True)
    plt.savefig(
        output_path,
        dpi=300,
        bbox_inches="tight",
        facecolor="#0f0f1a",
        edgecolor="none",
    )
    plt.close()


def create_multi_corpus_memory_chart(
    results: list[MemoryBenchmarkResult],
    output_path: Path,
) -> None:
    """Create horizontal bar chart comparing memory across corpora.

    Args:
        results: List of memory benchmark results
        output_path: Path to save chart
    """
    setup_dark_style()

    # Sort by TF memory (largest first)
    results = sorted(
        results,
        key=lambda r: r.tf_memory_stats.mean if r.tf_memory_stats else 0,
        reverse=True,
    )

    fig, ax = plt.subplots(figsize=(12, max(6, len(results) * 0.8)))

    y = np.arange(len(results))
    height = 0.35

    tf_memory = [r.tf_memory_stats.mean if r.tf_memory_stats else 0 for r in results]
    cf_memory = [r.cf_memory_stats.mean if r.cf_memory_stats else 0 for r in results]
    corpus_names = [r.corpus for r in results]

    ax.barh(y - height / 2, tf_memory, height, label="Text-Fabric", color=COLORS["tf"])
    ax.barh(y + height / 2, cf_memory, height, label="Context-Fabric", color=COLORS["cf"])

    ax.set_xlabel("Memory (MB)", fontsize=14)
    ax.set_ylabel("Corpus", fontsize=14)
    ax.set_title("Memory Usage by Corpus", fontsize=16, fontweight="bold")
    ax.set_yticks(y)
    ax.set_yticklabels(corpus_names)
    ax.legend(fontsize=12, loc="upper right")
    ax.tick_params(axis="both", labelsize=12)

    plt.tight_layout()
    output_path.parent.mkdir(parents=True, exist_ok=True)
    plt.savefig(
        output_path,
        dpi=300,
        bbox_inches="tight",
        facecolor="#0f0f1a",
        edgecolor="none",
    )
    plt.close()
