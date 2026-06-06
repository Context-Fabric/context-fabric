"""Command-line interface for cfabric benchmarks."""

from __future__ import annotations

import json
from datetime import datetime
from pathlib import Path
from typing import Literal

import click

from cfabric_benchmarks.models.config import (
    BenchmarkConfig,
    CorpusConfig,
    discover_corpora,
    get_corpora_by_size,
)
from cfabric_benchmarks.output.csv_writer import (
    write_cross_corpus_summary_csv,
    write_latency_measurements_csv,
    write_latency_statistics_csv,
    write_memory_measurements_csv,
    write_memory_summary_csv,
    write_progressive_steps_csv,
)
from cfabric_benchmarks.output.metadata import collect_environment


def get_default_corpora_dir() -> Path:
    """Get the default corpora directory."""
    package_dir = Path(__file__).parent.parent
    return package_dir / "corpora"


def create_output_dir(base_dir: Path) -> Path:
    """Create timestamped output directory."""
    timestamp = datetime.now().strftime("%Y-%m-%d_%H%M%S")
    output_dir = base_dir / timestamp
    output_dir.mkdir(parents=True, exist_ok=True)
    return output_dir


@click.group()
@click.version_option(version="0.1.0", prog_name="cfabric-bench")
def cli() -> None:
    """Context-Fabric benchmarking tool.

    Run performance benchmarks comparing Text-Fabric and Context-Fabric.
    """
    pass


@cli.command()
@click.option(
    "--memory-runs",
    default=5,
    help="Number of runs for memory benchmark",
    show_default=True,
)
@click.option(
    "--latency-runs",
    default=5,
    help="Number of runs for latency benchmark",
    show_default=True,
)
@click.option(
    "--progressive-runs",
    default=5,
    help="Number of runs for progressive benchmark",
    show_default=True,
)
@click.option(
    "--warmup",
    default=1,
    help="Number of warmup runs (excluded from statistics)",
    show_default=True,
)
@click.option(
    "--workers",
    default=4,
    help="Number of workers for memory spawn/fork tests",
    show_default=True,
)
@click.option(
    "--corpora-dir",
    type=click.Path(exists=True, path_type=Path),
    default=None,
    help="Directory containing corpora (default: package .corpora/)",
)
@click.option(
    "--output-dir",
    type=click.Path(path_type=Path),
    default=Path("./benchmark_results"),
    help="Output directory for results",
    show_default=True,
)
@click.option(
    "--queries",
    default=50,
    help="Number of search patterns to generate",
    show_default=True,
)
@click.option(
    "--iterations",
    default=10,
    help="Iterations per query for latency benchmark",
    show_default=True,
)
@click.option(
    "--max-corpora",
    default=10,
    help="Maximum corpora for progressive loading",
    show_default=True,
)
@click.option("--no-pdf", is_flag=True, help="Skip PDF chart generation")
def full(
    memory_runs: int,
    latency_runs: int,
    progressive_runs: int,
    warmup: int,
    workers: int,
    corpora_dir: Path | None,
    output_dir: Path,
    queries: int,
    iterations: int,
    max_corpora: int,
    no_pdf: bool,
) -> None:
    """Run full benchmark suite (memory, latency, progressive)."""
    from cfabric_benchmarks.runners.latency import LatencyBenchmarkRunner
    from cfabric_benchmarks.runners.memory import MemoryBenchmarkRunner
    from cfabric_benchmarks.runners.progressive import ProgressiveLoadRunner
    from cfabric_benchmarks.visualization.reports import (
        generate_full_report,
        save_individual_charts,
    )

    corpora_dir = corpora_dir or get_default_corpora_dir()
    output_dir = create_output_dir(output_dir)

    click.echo(f"Running full benchmark suite")
    click.echo(f"  Memory runs: {memory_runs}, Latency runs: {latency_runs}, Progressive runs: {progressive_runs}")
    click.echo(f"  Warmup: {warmup}, Workers: {workers}")
    click.echo(f"  Corpora: {corpora_dir}")
    click.echo(f"  Output: {output_dir}")

    config = BenchmarkConfig(
        memory_runs=memory_runs,
        latency_runs=latency_runs,
        progressive_runs=progressive_runs,
        warmup_runs=warmup,
        num_workers=workers,
        corpora_dir=corpora_dir,
        output_dir=output_dir,
        num_queries=queries,
        latency_iterations=iterations,
        max_corpora=max_corpora,
        generate_pdf=not no_pdf,
    )

    # Save config
    with open(output_dir / "config.json", "w") as f:
        json.dump(config.model_dump(mode="json"), f, indent=2, default=str)

    # Collect environment metadata
    click.echo("\nCollecting environment metadata...")
    env_meta = collect_environment()
    with open(output_dir / "environment.json", "w") as f:
        json.dump(env_meta.model_dump(mode="json"), f, indent=2)

    # Discover corpora
    corpora = discover_corpora(corpora_dir)
    if not corpora:
        click.echo(f"No corpora found in {corpora_dir}", err=True)
        return

    click.echo(f"\nDiscovered {len(corpora)} corpora: {', '.join(c.name for c in corpora)}")

    # Memory benchmarks
    click.echo("\n" + "=" * 60)
    click.echo("MEMORY BENCHMARKS")
    click.echo("=" * 60)

    memory_dir = output_dir / "memory"
    memory_dir.mkdir(exist_ok=True)

    memory_runner = MemoryBenchmarkRunner(config)
    memory_results = []

    for corpus in corpora:
        click.echo(f"\nBenchmarking {corpus.name}...")
        result = memory_runner.run(corpus)
        memory_results.append(result)

        # Write per-corpus CSV
        write_memory_measurements_csv(
            result.measurements,
            memory_dir / f"raw_{corpus.name}.csv",
        )

    # Write summary CSV
    write_memory_summary_csv(memory_results, memory_dir / "summary.csv")
    write_cross_corpus_summary_csv(memory_results, memory_dir / "cross_corpus_summary.csv")

    # Progressive loading
    click.echo("\n" + "=" * 60)
    click.echo("PROGRESSIVE LOADING")
    click.echo("=" * 60)

    progressive_dir = output_dir / "progressive"
    progressive_dir.mkdir(exist_ok=True)

    progressive_runner = ProgressiveLoadRunner(config)
    progressive_result = progressive_runner.run(corpora, max_corpora=max_corpora)

    write_progressive_steps_csv(
        progressive_result.steps,
        progressive_dir / "raw_steps.csv",
    )

    if progressive_result.tf_scaling or progressive_result.cf_scaling:
        scaling_data = {
            "tf": progressive_result.tf_scaling.model_dump() if progressive_result.tf_scaling else None,
            "cf": progressive_result.cf_scaling.model_dump() if progressive_result.cf_scaling else None,
        }
        with open(progressive_dir / "scaling_analysis.json", "w") as f:
            json.dump(scaling_data, f, indent=2)

    # Latency benchmarks
    click.echo("\n" + "=" * 60)
    click.echo("LATENCY BENCHMARKS")
    click.echo("=" * 60)

    latency_dir = output_dir / "latency"
    latency_dir.mkdir(exist_ok=True)

    # Find validation corpus (cuc or smallest available)
    sorted_corpora = get_corpora_by_size(corpora)
    validation_corpus = sorted_corpora[0] if sorted_corpora else None

    # Use largest corpus for latency test
    target_corpus = sorted_corpora[-1] if sorted_corpora else None

    if target_corpus:
        click.echo(f"\nRunning latency benchmark on {target_corpus.name}...")
        if validation_corpus and validation_corpus.name != target_corpus.name:
            click.echo(f"  Validating patterns on {validation_corpus.name} first")

        latency_runner = LatencyBenchmarkRunner(config)
        latency_result = latency_runner.run(
            target_corpus,
            validation_corpus=validation_corpus if validation_corpus.name != target_corpus.name else None,
        )

        # Save patterns
        patterns_data = [p.model_dump() for p in latency_result.queries]
        with open(latency_dir / "queries.json", "w") as f:
            json.dump(patterns_data, f, indent=2)

        write_latency_measurements_csv(
            latency_result.measurements,
            latency_dir / "raw_measurements.csv",
        )
        write_latency_statistics_csv(
            latency_result.statistics,
            latency_dir / "statistics.csv",
        )
    else:
        latency_result = None
        click.echo("No corpus available for latency benchmark")

    # Generate visualizations
    if not no_pdf:
        click.echo("\n" + "=" * 60)
        click.echo("GENERATING VISUALIZATIONS")
        click.echo("=" * 60)

        save_individual_charts(
            output_dir,
            memory_results=memory_results,
            latency_result=latency_result,
            progressive_result=progressive_result,
        )

        generate_full_report(
            output_dir / "report.pdf",
            memory_results=memory_results,
            latency_result=latency_result,
            progressive_result=progressive_result,
        )

    click.echo("\n" + "=" * 60)
    click.echo(f"COMPLETE - Results saved to {output_dir}")
    click.echo("=" * 60)


@cli.command()
@click.option("--corpus", "-c", multiple=True, help="Corpus name(s) to benchmark")
@click.option("--runs", default=5, help="Number of benchmark runs", show_default=True)
@click.option("--warmup", default=1, help="Number of warmup runs", show_default=True)
@click.option("--workers", default=4, help="Number of workers for multi-process tests", show_default=True)
@click.option(
    "--corpora-dir",
    type=click.Path(exists=True, path_type=Path),
    default=None,
    help="Directory containing corpora",
)
@click.option(
    "--output-dir",
    type=click.Path(path_type=Path),
    default=Path("./benchmark_results"),
    help="Output directory",
    show_default=True,
)
def memory(
    corpus: tuple[str, ...],
    runs: int,
    warmup: int,
    workers: int,
    corpora_dir: Path | None,
    output_dir: Path,
) -> None:
    """Run memory benchmarks only."""
    from cfabric_benchmarks.runners.memory import MemoryBenchmarkRunner
    from cfabric_benchmarks.visualization.charts import create_memory_comparison_chart

    corpora_dir = corpora_dir or get_default_corpora_dir()
    output_dir = create_output_dir(output_dir)

    config = BenchmarkConfig(
        memory_runs=runs,
        warmup_runs=warmup,
        corpora_dir=corpora_dir,
        output_dir=output_dir,
        num_workers=workers,
    )

    # Discover corpora
    all_corpora = discover_corpora(corpora_dir)
    if corpus:
        corpora_list = [c for c in all_corpora if c.name in corpus]
        if not corpora_list:
            click.echo(f"No matching corpora found. Available: {', '.join(c.name for c in all_corpora)}", err=True)
            return
    else:
        corpora_list = all_corpora

    click.echo(f"Running memory benchmark for: {', '.join(c.name for c in corpora_list)}")

    runner = MemoryBenchmarkRunner(config)
    results = []

    for c in corpora_list:
        click.echo(f"\nBenchmarking {c.name}...")
        result = runner.run(c)
        results.append(result)

        # Write raw data
        write_memory_measurements_csv(
            result.measurements,
            output_dir / f"memory_raw_{c.name}.csv",
        )

        # Create chart
        create_memory_comparison_chart(result, output_dir / f"fig_memory_{c.name}.pdf")
        create_memory_comparison_chart(result, output_dir / f"fig_memory_{c.name}.png")

    # Write summary
    write_memory_summary_csv(results, output_dir / "memory_summary.csv")
    write_cross_corpus_summary_csv(results, output_dir / "cross_corpus_summary.csv")

    click.echo(f"\nResults saved to {output_dir}")


@cli.command()
@click.option("--corpus", "-c", required=True, help="Corpus name to benchmark")
@click.option("--queries", default=50, help="Number of patterns to generate", show_default=True)
@click.option("--iterations", default=10, help="Iterations per pattern", show_default=True)
@click.option("--runs", default=5, help="Number of benchmark runs", show_default=True)
@click.option("--validation-corpus", help="Corpus for pattern validation (default: smallest available)")
@click.option(
    "--corpora-dir",
    type=click.Path(exists=True, path_type=Path),
    default=None,
    help="Directory containing corpora",
)
@click.option(
    "--output-dir",
    type=click.Path(path_type=Path),
    default=Path("./benchmark_results"),
    help="Output directory",
    show_default=True,
)
def latency(
    corpus: str,
    queries: int,
    iterations: int,
    runs: int,
    validation_corpus: str | None,
    corpora_dir: Path | None,
    output_dir: Path,
) -> None:
    """Run latency benchmarks only."""
    from cfabric_benchmarks.queries.curated import get_bhsa_queries
    from cfabric_benchmarks.runners.latency import LatencyBenchmarkRunner
    from cfabric_benchmarks.visualization.charts import (
        create_latency_distribution_chart,
        create_latency_percentiles_chart,
    )

    corpora_dir = corpora_dir or get_default_corpora_dir()
    output_dir = create_output_dir(output_dir)

    config = BenchmarkConfig(
        latency_runs=runs,
        warmup_runs=1,
        corpora_dir=corpora_dir,
        output_dir=output_dir,
        num_queries=queries,
        latency_iterations=iterations,
    )

    # Find corpora
    all_corpora = discover_corpora(corpora_dir)
    target = next((c for c in all_corpora if c.name == corpus), None)
    if not target:
        click.echo(f"Corpus '{corpus}' not found. Available: {', '.join(c.name for c in all_corpora)}", err=True)
        return

    # Only BHSA is supported (has curated patterns)
    if corpus != "bhsa":
        click.echo(f"Latency benchmark only supports BHSA corpus (has curated patterns)", err=True)
        return

    curated_queries = get_bhsa_queries()
    if queries < len(curated_queries):
        curated_queries = curated_queries[:queries]
    click.echo(f"Using {len(curated_queries)} curated BHSA queries")

    click.echo(f"Running latency benchmark on {corpus}")

    runner = LatencyBenchmarkRunner(config)
    result = runner.run(target, queries=curated_queries)

    # Save results
    patterns_data = [p.model_dump() for p in result.queries]
    with open(output_dir / "queries.json", "w") as f:
        json.dump(patterns_data, f, indent=2)

    write_latency_measurements_csv(result.measurements, output_dir / "latency_raw.csv")
    write_latency_statistics_csv(result.statistics, output_dir / "latency_statistics.csv")

    # Create charts
    create_latency_distribution_chart(result, output_dir / "fig_latency_distribution.pdf")
    create_latency_distribution_chart(result, output_dir / "fig_latency_distribution.png")
    create_latency_percentiles_chart(result, output_dir / "fig_latency_percentiles.pdf")
    create_latency_percentiles_chart(result, output_dir / "fig_latency_percentiles.png")

    click.echo(f"\nResults saved to {output_dir}")


@cli.command()
@click.option("--max-corpora", default=10, help="Maximum number of corpora to load", show_default=True)
@click.option("--runs", default=5, help="Number of benchmark runs", show_default=True)
@click.option(
    "--corpora-dir",
    type=click.Path(exists=True, path_type=Path),
    default=None,
    help="Directory containing corpora",
)
@click.option(
    "--output-dir",
    type=click.Path(path_type=Path),
    default=Path("./benchmark_results"),
    help="Output directory",
    show_default=True,
)
def progressive(
    max_corpora: int,
    runs: int,
    corpora_dir: Path | None,
    output_dir: Path,
) -> None:
    """Run progressive loading benchmark."""
    from cfabric_benchmarks.runners.progressive import ProgressiveLoadRunner
    from cfabric_benchmarks.visualization.charts import create_progressive_scaling_chart

    corpora_dir = corpora_dir or get_default_corpora_dir()
    output_dir = create_output_dir(output_dir)

    config = BenchmarkConfig(
        progressive_runs=runs,
        warmup_runs=1,
        corpora_dir=corpora_dir,
        output_dir=output_dir,
        max_corpora=max_corpora,
    )

    corpora = discover_corpora(corpora_dir)
    if not corpora:
        click.echo(f"No corpora found in {corpora_dir}", err=True)
        return

    click.echo(f"Running progressive loading test with up to {max_corpora} corpora")

    runner = ProgressiveLoadRunner(config)
    result = runner.run(corpora, max_corpora=max_corpora)

    # Save results
    write_progressive_steps_csv(result.steps, output_dir / "progressive_steps.csv")

    if result.tf_scaling or result.cf_scaling:
        scaling_data = {
            "tf": result.tf_scaling.model_dump() if result.tf_scaling else None,
            "cf": result.cf_scaling.model_dump() if result.cf_scaling else None,
        }
        with open(output_dir / "scaling_analysis.json", "w") as f:
            json.dump(scaling_data, f, indent=2)

    # Create chart
    create_progressive_scaling_chart(result, output_dir / "fig_scaling_progressive.pdf")
    create_progressive_scaling_chart(result, output_dir / "fig_scaling_progressive.png")

    click.echo(f"\nResults saved to {output_dir}")


@cli.command("validate-patterns")
@click.option(
    "--corpora-dir",
    type=click.Path(exists=True, path_type=Path),
    default=None,
    help="Directory containing corpora",
)
@click.option(
    "--output-dir",
    type=click.Path(path_type=Path),
    default=Path("./benchmark_results"),
    help="Output directory",
    show_default=True,
)
def validate_patterns(
    corpora_dir: Path | None,
    output_dir: Path,
) -> None:
    """Validate curated BHSA patterns on the BHSA corpus."""
    from cfabric_benchmarks.generators.validator import validate_queries_on_corpus
    from cfabric_benchmarks.queries.curated import get_bhsa_queries
    from cfabric_benchmarks.runners.base import load_tf_api

    corpora_dir = corpora_dir or get_default_corpora_dir()
    output_dir = create_output_dir(output_dir)

    # Find BHSA corpus
    all_corpora = discover_corpora(corpora_dir)
    target = next((c for c in all_corpora if c.name == "bhsa"), None)
    if not target:
        click.echo(f"BHSA corpus not found. Available: {', '.join(c.name for c in all_corpora)}", err=True)
        return

    # Get curated patterns
    patterns = get_bhsa_queries()
    click.echo(f"Validating {len(patterns)} curated BHSA patterns...")

    # Load API
    api = load_tf_api(str(target.tf_path))

    # Validate
    report = validate_queries_on_corpus(patterns, api, corpus_name="bhsa")

    # Show results
    valid_count = sum(1 for p in report.queries if p.validated)
    click.echo(f"\nValidation Results:")
    click.echo(f"  Total: {report.total_queries}")
    click.echo(f"  Valid: {valid_count}")
    click.echo(f"  Invalid: {report.total_queries - valid_count}")

    # Show failures
    failures = [query for query in report.queries if not query.validated]
    if failures:
        click.echo(f"\nFailed patterns:")
        for failure in failures[:10]:  # Show first 10
            error = failure.validation_error or ""
            click.echo(f"  - {failure.id}: {error[:60]}...")

    # Save results
    patterns_data = [p.model_dump() for p in report.queries]
    with open(output_dir / "queries.json", "w") as f:
        json.dump(patterns_data, f, indent=2)

    report_data = {
        "corpus": report.validation_corpus,
        "total": report.total_queries,
        "valid": valid_count,
        "invalid": report.total_queries - valid_count,
        "failures": [failure.model_dump() for failure in failures],
    }
    with open(output_dir / "validation_report.json", "w") as f:
        json.dump(report_data, f, indent=2)

    click.echo(f"\nResults saved to {output_dir}")


@cli.command()
@click.argument("results_dir", type=click.Path(exists=True, path_type=Path))
@click.option("--format", "-f", type=click.Choice(["pdf", "png", "both"]), default="both", help="Output format")
@click.option(
    "--charts",
    "-c",
    default="progressive,latency,multicorpus",
    help="Chart types to generate (comma-separated): progressive, latency, multicorpus",
)
def visualize(results_dir: Path, format: Literal["pdf", "png", "both"], charts: str) -> None:
    """Generate visualizations from existing results.

    By default generates: progressive, latency, multicorpus charts.
    Per-corpus memory charts are not supported (require corpus stats not stored in CSV).
    """
    from cfabric_benchmarks.output.loaders import (
        load_latency_result,
        load_memory_results,
        load_progressive_result,
    )
    from cfabric_benchmarks.visualization.charts import (
        create_latency_distribution_chart,
        create_latency_percentiles_chart,
        create_multi_corpus_memory_chart,
        create_progressive_scaling_chart,
    )

    chart_types = {c.strip().lower() for c in charts.split(",")}
    click.echo(f"Loading results from {results_dir}...")

    created: list[Path] = []

    # Progressive chart
    if "progressive" in chart_types:
        progressive_result = load_progressive_result(results_dir)
        if progressive_result:
            click.echo("  Loaded progressive results")
            for ext in ["pdf", "png"] if format == "both" else [format]:
                path = results_dir / f"fig_scaling_progressive.{ext}"
                create_progressive_scaling_chart(progressive_result, path)
                created.append(path)

    # Multi-corpus memory chart
    if "multicorpus" in chart_types:
        memory_results = load_memory_results(results_dir)
        if len(memory_results) > 1:
            click.echo(f"  Loaded {len(memory_results)} memory results")
            for ext in ["pdf", "png"] if format == "both" else [format]:
                path = results_dir / f"fig_memory_multicorpus.{ext}"
                create_multi_corpus_memory_chart(memory_results, path)
                created.append(path)

    # Latency charts
    if "latency" in chart_types:
        latency_result = load_latency_result(results_dir)
        if latency_result:
            click.echo(f"  Loaded {len(latency_result.measurements)} latency measurements")
            for ext in ["pdf", "png"] if format == "both" else [format]:
                for chart_func, name in [
                    (create_latency_distribution_chart, "fig_latency_distribution"),
                    (create_latency_percentiles_chart, "fig_latency_percentiles"),
                ]:
                    path = results_dir / f"{name}.{ext}"
                    chart_func(latency_result, path)
                    created.append(path)

    click.echo(f"\nCreated {len(created)} chart files in {results_dir}")


@cli.command()
def environment() -> None:
    """Print test environment information."""
    env = collect_environment()

    click.echo("Test Environment")
    click.echo("=" * 50)

    click.echo(f"\nHardware:")
    click.echo(f"  CPU: {env.hardware.cpu_model}")
    click.echo(f"  Cores: {env.hardware.cpu_cores} physical, {env.hardware.cpu_threads} logical")
    click.echo(f"  Memory: {env.hardware.ram_total_gb:.1f} GB")
    click.echo(f"  Storage: {env.hardware.storage_type}")

    click.echo(f"\nSoftware:")
    click.echo(f"  OS: {env.os_name} {env.os_version} ({env.architecture})")
    click.echo(f"  Python: {env.software.python_version}")
    click.echo(f"  Text-Fabric: {env.software.text_fabric_version}")
    click.echo(f"  Context-Fabric: {env.software.context_fabric_version}")

    click.echo(f"\nTimestamp: {env.timestamp}")


if __name__ == "__main__":
    cli()
