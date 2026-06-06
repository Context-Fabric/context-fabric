"""Progressive corpus loading benchmark runner.

Measures memory scaling as multiple corpora are loaded sequentially.
"""

from __future__ import annotations

import gc
import time
from pathlib import Path
from typing import Any, Literal

from cfabric_benchmarks.analysis.statistics import aggregate_runs, linear_regression
from cfabric_benchmarks.models.config import BenchmarkConfig, CorpusConfig, get_corpora_by_size
from cfabric_benchmarks.models.progressive import (
    ProgressiveLoadResult,
    ProgressiveLoadStep,
    ScalingAnalysis,
)
from cfabric_benchmarks.runners.base import BaseBenchmarkRunner
from cfabric_benchmarks.runners.isolation import get_memory_mb, run_isolated


class ProgressiveLoadRunner(BaseBenchmarkRunner[ProgressiveLoadResult]):
    """Runner for progressive corpus loading benchmarks.

    Loads corpora one at a time and measures cumulative memory usage
    to analyze scaling characteristics.
    """

    def name(self) -> str:
        return "ProgressiveLoad"

    def run(
        self,
        corpora: list[CorpusConfig],
        max_corpora: int | None = None,
    ) -> ProgressiveLoadResult:
        """Run progressive loading benchmark.

        Args:
            corpora: List of corpora to load (will be sorted by size)
            max_corpora: Maximum number of corpora to load

        Returns:
            ProgressiveLoadResult with all measurements
        """
        # Sort by size
        sorted_corpora = get_corpora_by_size(corpora)
        max_corpora = max_corpora or self.config.max_corpora
        sorted_corpora = sorted_corpora[:max_corpora]

        if not sorted_corpora:
            self.log("No corpora to test!")
            return ProgressiveLoadResult(
                max_corpora=0,
                corpora_order=[],
                num_runs=0,
                steps=[],
            )

        corpora_order = [c.name for c in sorted_corpora]
        self.log(f"Testing progressive loading of {len(sorted_corpora)} corpora")
        self.log(f"  Order: {' -> '.join(corpora_order)}")

        steps: list[ProgressiveLoadStep] = []
        total_runs = self.config.warmup_runs + self.config.progressive_runs

        for run_id in range(1, total_runs + 1):
            is_warmup = run_id <= self.config.warmup_runs
            actual_run_id = run_id - self.config.warmup_runs if not is_warmup else 0

            if is_warmup:
                self.log(f"  Warmup run {run_id}/{self.config.warmup_runs}")
            else:
                self.log(f"  Run {actual_run_id}/{self.config.progressive_runs}")

            # Run progressive loading for each implementation
            for impl in ["TF", "CF"]:
                impl_steps = self._run_progressive_load(
                    sorted_corpora, impl, actual_run_id
                )
                if not is_warmup:
                    steps.extend(impl_steps)

        # Compute scaling analysis
        self.log("Computing scaling analysis...")
        tf_scaling = self._compute_scaling(steps, "TF", len(sorted_corpora))
        cf_scaling = self._compute_scaling(steps, "CF", len(sorted_corpora))

        # Compute average memory by step
        tf_memory_by_step, _ = aggregate_runs(
            [
                [s.total_rss_mb for s in steps if s.implementation == "TF" and s.run_id == r]
                for r in range(1, self.config.progressive_runs + 1)
            ]
        )
        cf_memory_by_step, _ = aggregate_runs(
            [
                [s.total_rss_mb for s in steps if s.implementation == "CF" and s.run_id == r]
                for r in range(1, self.config.progressive_runs + 1)
            ]
        )

        return ProgressiveLoadResult(
            max_corpora=len(sorted_corpora),
            corpora_order=corpora_order,
            num_runs=self.config.progressive_runs,
            steps=steps,
            tf_scaling=tf_scaling,
            cf_scaling=cf_scaling,
            tf_memory_by_step=tf_memory_by_step if tf_memory_by_step else None,
            cf_memory_by_step=cf_memory_by_step if cf_memory_by_step else None,
        )

    def _run_progressive_load(
        self,
        corpora: list[CorpusConfig],
        impl: Literal["TF", "CF"],
        run_id: int,
    ) -> list[ProgressiveLoadStep]:
        """Run progressive loading for one implementation.

        Args:
            corpora: Corpora to load in order
            impl: Implementation to use
            run_id: Run identifier

        Returns:
            List of ProgressiveLoadStep for each corpus added
        """
        # Run in isolated subprocess to get clean measurements
        loader_fn = _tf_progressive_loader if impl == "TF" else _cf_progressive_loader
        corpus_paths = [str(c.tf_path) for c in corpora]
        corpus_names = [c.name for c in corpora]

        result = run_isolated(
            loader_fn,
            args=(corpus_paths, corpus_names),
            timeout=600,  # 10 minutes for large corpora
        )

        if not result.success or not result.result:
            self.log(f"    [{impl}] Progressive load failed: {result.error}")
            return []

        raw_steps = result.result
        steps = []
        prev_memory = 0.0
        prev_time = 0.0

        for i, (memory, load_time, loaded_corpora) in enumerate(raw_steps, start=1):
            steps.append(
                ProgressiveLoadStep(
                    step=i,
                    corpus_added=corpus_names[i - 1],
                    corpora_loaded=loaded_corpora,
                    implementation=impl,
                    run_id=run_id,
                    total_rss_mb=memory,
                    incremental_rss_mb=memory - prev_memory,
                    cumulative_load_time_s=load_time,
                    step_load_time_s=load_time - prev_time,
                )
            )
            prev_memory = memory
            prev_time = load_time

        return steps

    def _compute_scaling(
        self,
        steps: list[ProgressiveLoadStep],
        impl: Literal["TF", "CF"],
        num_corpora: int,
    ) -> ScalingAnalysis | None:
        """Compute linear scaling analysis.

        Args:
            steps: All progressive load steps
            impl: Implementation to analyze
            num_corpora: Number of corpora

        Returns:
            ScalingAnalysis or None if insufficient data
        """
        # Get average memory at each step
        impl_steps = [s for s in steps if s.implementation == impl]
        if not impl_steps:
            return None

        # Group by step and average
        step_memories: dict[int, list[float]] = {}
        for s in impl_steps:
            if s.step not in step_memories:
                step_memories[s.step] = []
            step_memories[s.step].append(s.total_rss_mb)

        if not step_memories:
            return None

        x = list(range(1, num_corpora + 1))
        y = [
            sum(step_memories.get(i, [0])) / len(step_memories.get(i, [1]))
            for i in x
        ]

        # Linear regression
        slope, intercept, r_squared = linear_regression(x, y)

        return ScalingAnalysis(
            implementation=impl,
            slope_mb_per_corpus=slope,
            intercept_mb=intercept,
            r_squared=r_squared,
            predicted_10_corpora_mb=slope * 10 + intercept,
            predicted_50_corpora_mb=slope * 50 + intercept,
        )


# Worker functions (must be at module level for pickling)


def _tf_progressive_loader(
    corpus_paths: list[str],
    corpus_names: list[str],
) -> list[tuple[float, float, list[str]]]:
    """Load TF corpora progressively and measure memory."""
    from tf.fabric import Fabric as TFFabric

    results = []
    loaded_apis = []
    loaded_names = []
    cumulative_time = 0.0

    for i, (path, name) in enumerate(zip(corpus_paths, corpus_names)):
        start = time.perf_counter()
        tf = TFFabric(locations=path, silent="deep")
        api = tf.loadAll(silent="deep")
        load_time = time.perf_counter() - start
        cumulative_time += load_time

        loaded_apis.append(api)
        loaded_names.append(name)

        gc.collect()
        memory = get_memory_mb()

        results.append((memory, cumulative_time, list(loaded_names)))

    return results


def _cf_progressive_loader(
    corpus_paths: list[str],
    corpus_names: list[str],
) -> list[tuple[float, float, list[str]]]:
    """Load CF corpora progressively and measure memory."""
    from cfabric import Fabric as CFFabric

    results = []
    loaded_apis = []
    loaded_names = []
    cumulative_time = 0.0

    for i, (path, name) in enumerate(zip(corpus_paths, corpus_names)):
        start = time.perf_counter()
        cf = CFFabric(locations=path, silent="deep")
        api = cf.loadAll(silent="deep")
        # Embedding structures are preloaded automatically by default
        load_time = time.perf_counter() - start
        cumulative_time += load_time

        loaded_apis.append(api)
        loaded_names.append(name)

        gc.collect()
        memory = get_memory_mb()

        results.append((memory, cumulative_time, list(loaded_names)))

    return results
