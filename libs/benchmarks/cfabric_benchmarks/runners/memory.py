"""Memory benchmark runner.

Measures memory consumption for Text-Fabric and Context-Fabric across
single-process, spawn, and fork modes.
"""

from __future__ import annotations

import gc
import multiprocessing as mp
import os
import shutil
import time
from pathlib import Path
from typing import Any, Literal

from cfabric_benchmarks.analysis.statistics import compute_summary
from cfabric_benchmarks.models.config import BenchmarkConfig, CorpusConfig
from cfabric_benchmarks.models.memory import (
    CorpusStats,
    MemoryBenchmarkResult,
    MemoryMeasurement,
)
from cfabric_benchmarks.runners.base import (
    BaseBenchmarkRunner,
    get_corpus_name,
    get_corpus_stats,
)
from cfabric_benchmarks.runners.isolation import (
    WorkerPool,
    get_dir_size_mb,
    get_memory_mb,
    get_total_memory_mb,
    run_isolated,
)


class MemoryBenchmarkRunner(BaseBenchmarkRunner[MemoryBenchmarkResult]):
    """Runner for memory benchmarks.

    Measures memory usage in single-process, spawn, and fork modes
    for both Text-Fabric and Context-Fabric.
    """

    def name(self) -> str:
        return "MemoryBenchmark"

    def run(
        self,
        corpus: CorpusConfig,
        modes: list[Literal["single", "spawn", "fork"]] | None = None,
    ) -> MemoryBenchmarkResult:
        """Run memory benchmark for a corpus.

        Args:
            corpus: Corpus configuration
            modes: Modes to test (default: all enabled in config)

        Returns:
            MemoryBenchmarkResult with all measurements
        """
        modes = modes or []
        if not modes:
            modes.append("single")
            if self.config.measure_spawn:
                modes.append("spawn")
            if self.config.measure_fork:
                modes.append("fork")

        measurements: list[MemoryMeasurement] = []
        corpus_stats: CorpusStats | None = None

        source = str(corpus.tf_path)
        total_runs = self.config.warmup_runs + self.config.memory_runs

        self.log(f"Running {total_runs} runs for {corpus.name}")

        for run_id in range(1, total_runs + 1):
            is_warmup = run_id <= self.config.warmup_runs
            actual_run_id = run_id - self.config.warmup_runs if not is_warmup else 0

            if is_warmup:
                self.log(f"  Warmup run {run_id}/{self.config.warmup_runs}")
            else:
                self.log(f"  Run {actual_run_id}/{self.config.memory_runs}")

            # Single-process measurements
            if "single" in modes:
                for impl in ["TF", "CF"]:
                    result = self._measure_single(source, impl, run_id)
                    if result:
                        if not is_warmup:
                            result = MemoryMeasurement(
                                run_id=actual_run_id,
                                corpus=corpus.name,
                                implementation=result.implementation,
                                mode=result.mode,
                                compile_time_s=result.compile_time_s,
                                load_time_s=result.load_time_s,
                                rss_before_mb=result.rss_before_mb,
                                rss_after_mb=result.rss_after_mb,
                                cache_size_mb=result.cache_size_mb,
                            )
                            measurements.append(result)

                        # Get corpus stats from first TF run
                        if impl == "TF" and corpus_stats is None:
                            corpus_stats = self._get_corpus_stats_isolated(source)

            # Spawn mode measurements
            if "spawn" in modes and not is_warmup:
                for impl in ["TF", "CF"]:
                    result = self._measure_spawn(source, impl, actual_run_id, corpus.name)
                    if result:
                        measurements.append(result)

            # Fork mode measurements
            if "fork" in modes and not is_warmup:
                for impl in ["TF", "CF"]:
                    result = self._measure_fork(source, impl, actual_run_id, corpus.name)
                    if result:
                        measurements.append(result)

        # Ensure we have corpus stats
        if corpus_stats is None:
            corpus_stats = CorpusStats(
                name=corpus.name,
                max_slot=0,
                max_node=0,
                node_types=0,
                node_features=0,
                edge_features=0,
            )

        # Compute statistics
        result = MemoryBenchmarkResult(
            corpus=corpus.name,
            corpus_stats=corpus_stats,
            measurements=measurements,
        )

        # Add statistical summaries
        result = self._compute_statistics(result)

        return result

    def _measure_single(
        self,
        source: str,
        impl: Literal["TF", "CF"],
        run_id: int,
    ) -> MemoryMeasurement | None:
        """Measure single-process memory usage.

        Args:
            source: Path to corpus
            impl: Implementation to test
            run_id: Run identifier

        Returns:
            MemoryMeasurement or None if failed
        """
        self.log(f"    [{impl}] Single-process measurement...")

        if impl == "TF":
            result = run_isolated(_load_tf_and_measure, args=(source,))
        else:
            result = run_isolated(_load_cf_and_measure, args=(source,))

        if result.success and result.result:
            if impl == "TF":
                memory_mb, load_time, cache_size = result.result
                compile_time = None
            else:
                # CF returns compile_time as 4th element
                memory_mb, load_time, cache_size, compile_time = result.result
                compile_time = compile_time if compile_time > 0 else None
            return MemoryMeasurement(
                run_id=run_id,
                corpus=get_corpus_name(source),
                implementation=impl,
                mode="single",
                compile_time_s=compile_time,
                load_time_s=load_time,
                rss_before_mb=0,
                rss_after_mb=memory_mb,
                cache_size_mb=cache_size,
            )

        self.log(f"    [{impl}] Failed: {result.error}")
        return None

    def _measure_spawn(
        self,
        source: str,
        impl: Literal["TF", "CF"],
        run_id: int,
        corpus_name: str,
    ) -> MemoryMeasurement | None:
        """Measure spawn mode memory usage.

        Args:
            source: Path to corpus
            impl: Implementation to test
            run_id: Run identifier
            corpus_name: Name of the corpus

        Returns:
            MemoryMeasurement or None if failed
        """
        self.log(f"    [{impl}] Spawn mode ({self.config.num_workers} workers)...")

        try:
            worker_fn = _tf_spawn_worker if impl == "TF" else _cf_spawn_worker

            with WorkerPool(self.config.num_workers, context="spawn") as pool:
                start = time.perf_counter()
                pool.start_workers(worker_fn, worker_args=(source,))

                if not pool.wait_for_ready(timeout=300):
                    self.log(f"    [{impl}] Workers failed to start")
                    return None

                pool.signal_start()
                results = pool.get_results(timeout=120)

                load_time = time.perf_counter() - start
                total_memory = pool.get_total_memory_mb()

            per_worker = total_memory / self.config.num_workers

            return MemoryMeasurement(
                run_id=run_id,
                corpus=corpus_name,
                implementation=impl,
                mode="spawn",
                load_time_s=load_time,
                rss_before_mb=0,
                rss_after_mb=total_memory,
                num_workers=self.config.num_workers,
                total_rss_mb=total_memory,
                per_worker_rss_mb=per_worker,
            )

        except Exception as e:
            self.log(f"    [{impl}] Spawn failed: {e}")
            return None

    def _measure_fork(
        self,
        source: str,
        impl: Literal["TF", "CF"],
        run_id: int,
        corpus_name: str,
    ) -> MemoryMeasurement | None:
        """Measure fork mode memory usage.

        Args:
            source: Path to corpus
            impl: Implementation to test
            run_id: Run identifier
            corpus_name: Name of the corpus

        Returns:
            MemoryMeasurement or None if failed
        """
        self.log(f"    [{impl}] Fork mode ({self.config.num_workers} workers)...")

        try:
            runner_fn = _run_tf_fork_scenario if impl == "TF" else _run_cf_fork_scenario

            result = run_isolated(
                runner_fn,
                args=(source, self.config.num_workers),
                timeout=300,
            )

            if result.success and result.result:
                main_rss, workers_rss = result.result
                total_rss = main_rss + workers_rss
                per_worker = total_rss / self.config.num_workers

                return MemoryMeasurement(
                    run_id=run_id,
                    corpus=corpus_name,
                    implementation=impl,
                    mode="fork",
                    load_time_s=0,  # Not measured in fork mode
                    rss_before_mb=0,
                    rss_after_mb=total_rss,
                    num_workers=self.config.num_workers,
                    total_rss_mb=total_rss,
                    per_worker_rss_mb=per_worker,
                )

            self.log(f"    [{impl}] Fork failed: {result.error}")
            return None

        except Exception as e:
            self.log(f"    [{impl}] Fork failed: {e}")
            return None

    def _get_corpus_stats_isolated(self, source: str) -> CorpusStats:
        """Get corpus stats in isolated subprocess.

        Args:
            source: Path to corpus

        Returns:
            CorpusStats object
        """
        result = run_isolated(_get_tf_corpus_stats, args=(source,))
        if result.success and result.result:
            stats = result.result
            return CorpusStats(
                name=get_corpus_name(source),
                max_slot=stats["max_slot"],
                max_node=stats["max_node"],
                node_types=stats["node_types"],
                node_features=stats["node_features"],
                edge_features=stats["edge_features"],
            )

        return CorpusStats(
            name=get_corpus_name(source),
            max_slot=0,
            max_node=0,
            node_types=0,
            node_features=0,
            edge_features=0,
        )

    def _compute_statistics(
        self,
        result: MemoryBenchmarkResult,
    ) -> MemoryBenchmarkResult:
        """Compute statistical summaries for the results.

        Args:
            result: Result with raw measurements

        Returns:
            Result with computed statistics
        """
        # Single mode stats
        tf_single = [
            m.memory_used_mb
            for m in result.measurements
            if m.implementation == "TF" and m.mode == "single"
        ]
        cf_single = [
            m.memory_used_mb
            for m in result.measurements
            if m.implementation == "CF" and m.mode == "single"
        ]

        if tf_single:
            result.tf_memory_stats = compute_summary(tf_single, "tf_memory", "MB")
        if cf_single:
            result.cf_memory_stats = compute_summary(cf_single, "cf_memory", "MB")

        # Load time stats
        tf_load = [
            m.load_time_s
            for m in result.measurements
            if m.implementation == "TF" and m.mode == "single"
        ]
        cf_load = [
            m.load_time_s
            for m in result.measurements
            if m.implementation == "CF" and m.mode == "single"
        ]

        if tf_load:
            result.tf_load_time_stats = compute_summary(tf_load, "tf_load_time", "s")
        if cf_load:
            result.cf_load_time_stats = compute_summary(cf_load, "cf_load_time", "s")

        # Spawn mode stats
        tf_spawn = [
            m.total_rss_mb
            for m in result.measurements
            if m.implementation == "TF" and m.mode == "spawn" and m.total_rss_mb
        ]
        cf_spawn = [
            m.total_rss_mb
            for m in result.measurements
            if m.implementation == "CF" and m.mode == "spawn" and m.total_rss_mb
        ]

        if tf_spawn:
            result.tf_spawn_stats = compute_summary(tf_spawn, "tf_spawn", "MB")
        if cf_spawn:
            result.cf_spawn_stats = compute_summary(cf_spawn, "cf_spawn", "MB")

        # Fork mode stats
        tf_fork = [
            m.total_rss_mb
            for m in result.measurements
            if m.implementation == "TF" and m.mode == "fork" and m.total_rss_mb
        ]
        cf_fork = [
            m.total_rss_mb
            for m in result.measurements
            if m.implementation == "CF" and m.mode == "fork" and m.total_rss_mb
        ]

        if tf_fork:
            result.tf_fork_stats = compute_summary(tf_fork, "tf_fork", "MB")
        if cf_fork:
            result.cf_fork_stats = compute_summary(cf_fork, "cf_fork", "MB")

        return result


# Worker functions (must be at module level for pickling)


def _load_tf_and_measure(source: str) -> tuple[float, float, float]:
    """Load TF corpus and measure memory."""
    from tf.fabric import Fabric as TFFabric

    start = time.perf_counter()
    tf = TFFabric(locations=source, silent="deep")
    api = tf.loadAll(silent="deep")
    load_time = time.perf_counter() - start

    gc.collect()
    memory = get_memory_mb()

    cache_size = get_dir_size_mb(Path(source) / ".tf")

    return memory, load_time, cache_size


def _load_cf_and_measure(source: str) -> tuple[float, float, float, float]:
    """Load CF corpus and measure memory.

    Returns:
        Tuple of (memory_mb, load_time_s, cache_size_mb, compile_time_s)
    """
    from cfabric import Fabric as CFFabric

    compile_time = 0.0
    cache_path = Path(source) / ".cfm"

    # Check if we need to compile first
    if not cache_path.exists():
        # Compile and discard to get compile time
        compile_start = time.perf_counter()
        cf_compile = CFFabric(locations=source, silent="deep")
        cf_compile.loadAll(silent="deep")
        compile_time = time.perf_counter() - compile_start
        del cf_compile
        gc.collect()

    # Now measure load from cache
    # Embedding structures are preloaded automatically by default
    start = time.perf_counter()
    cf = CFFabric(locations=source, silent="deep")
    api = cf.loadAll(silent="deep")
    load_time = time.perf_counter() - start

    gc.collect()
    memory = get_memory_mb()

    cache_size = get_dir_size_mb(cache_path)

    return memory, load_time, cache_size, compile_time


def _get_tf_corpus_stats(source: str) -> dict:
    """Get corpus stats via TF."""
    from tf.fabric import Fabric as TFFabric

    tf = TFFabric(locations=source, silent="deep")
    api = tf.loadAll(silent="deep")
    return get_corpus_stats(api)


def _tf_spawn_worker(ready_event, start_event, result_queue, source: str):
    """TF worker for spawn mode."""
    from tf.fabric import Fabric as TFFabric

    tf = TFFabric(locations=source, silent="deep")
    api = tf.loadAll(silent="deep")

    ready_event.set()
    start_event.wait()

    # Access features to exercise memory
    _exercise_features(api)

    gc.collect()
    mem = get_memory_mb()
    result_queue.put((os.getpid(), mem))
    time.sleep(2)


def _cf_spawn_worker(ready_event, start_event, result_queue, source: str):
    """CF worker for spawn mode.

    Embedding structures are preloaded automatically by default.
    """
    from cfabric import Fabric as CFFabric

    cf = CFFabric(locations=source, silent="deep")
    api = cf.loadAll(silent="deep")

    ready_event.set()
    start_event.wait()

    # Access features to exercise memory
    _exercise_features(api)

    gc.collect()
    mem = get_memory_mb()
    result_queue.put((os.getpid(), mem))
    time.sleep(2)


def _exercise_features(api: Any) -> int:
    """Access string features to exercise memory."""
    count = 0
    string_features = []

    for fname in [
        "g_word_utf8",
        "lex_utf8",
        "g_cons_utf8",
        "voc_lex_utf8",
        "g_lex_utf8",
        "lex",
        "g_word",
    ]:
        if hasattr(api.F, fname):
            string_features.append(getattr(api.F, fname))

    max_slot = min(api.F.otype.maxSlot, 50000)
    for n in range(1, max_slot + 1):
        for feat in string_features:
            val = feat.v(n)
            if val:
                count += 1

    return count


def _run_tf_fork_scenario(source: str, num_workers: int) -> tuple[float, float]:
    """Run TF fork scenario."""
    from tf.fabric import Fabric as TFFabric

    tf = TFFabric(locations=source, silent="deep")
    api = tf.loadAll(silent="deep")

    gc.collect()
    main_rss = get_memory_mb()

    ctx = mp.get_context("fork")
    worker_queue = ctx.Queue()

    def worker(api_ref, q):
        _exercise_features(api_ref)
        q.put(os.getpid())
        time.sleep(2)

    processes = []
    for _ in range(num_workers):
        p = ctx.Process(target=worker, args=(api, worker_queue))
        p.start()
        processes.append(p)

    for _ in range(num_workers):
        try:
            worker_queue.get(timeout=60)
        except Exception:
            pass

    pids = [p.pid for p in processes]
    workers_rss = get_total_memory_mb(pids)

    for p in processes:
        p.join(timeout=5)
        if p.is_alive():
            p.terminate()

    return main_rss, workers_rss


def _run_cf_fork_scenario(source: str, num_workers: int) -> tuple[float, float]:
    """Run CF fork scenario.

    Embedding structures are preloaded automatically by default.
    """
    from cfabric import Fabric as CFFabric

    cf = CFFabric(locations=source, silent="deep")
    api = cf.loadAll(silent="deep")

    gc.collect()
    main_rss = get_memory_mb()

    ctx = mp.get_context("fork")
    worker_queue = ctx.Queue()

    def worker(api_ref, q):
        _exercise_features(api_ref)
        q.put(os.getpid())
        time.sleep(2)

    processes = []
    for _ in range(num_workers):
        p = ctx.Process(target=worker, args=(api, worker_queue))
        p.start()
        processes.append(p)

    for _ in range(num_workers):
        try:
            worker_queue.get(timeout=60)
        except Exception:
            pass

    pids = [p.pid for p in processes]
    workers_rss = get_total_memory_mb(pids)

    for p in processes:
        p.join(timeout=5)
        if p.is_alive():
            p.terminate()

    return main_rss, workers_rss
