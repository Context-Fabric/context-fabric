"""Tests for Pydantic models."""

from __future__ import annotations

from pathlib import Path

import pytest

from cfabric_benchmarks.models.config import (
    CORPUS_SIZE_ORDER,
    BenchmarkConfig,
    CorpusConfig,
    get_corpora_by_size,
)
from cfabric_benchmarks.models.latency import (
    LatencyStatistics,
    QueryMeasurement,
    SearchQuery,
    ValidationReport,
)
from cfabric_benchmarks.models.memory import (
    CorpusStats,
    MemoryBenchmarkResult,
    MemoryMeasurement,
)
from cfabric_benchmarks.models.progressive import (
    ProgressiveLoadResult,
    ProgressiveLoadStep,
    ScalingAnalysis,
)
from cfabric_benchmarks.models.statistics import ComparisonResult, StatisticalSummary


class TestBenchmarkConfig:
    """Tests for BenchmarkConfig model."""

    def test_default_values(self, tmp_path: Path) -> None:
        """Test default configuration values."""
        config = BenchmarkConfig(
            corpora_dir=tmp_path / "corpora",
            output_dir=tmp_path / "output",
        )
        assert config.memory_runs == 5
        assert config.warmup_runs == 1
        assert config.num_workers == 4
        assert config.num_queries == 50
        assert config.latency_iterations == 10
        assert config.validation_corpus == "cuc"
        assert config.generate_pdf is True

    def test_custom_values(self, tmp_path: Path) -> None:
        """Test custom configuration values."""
        config = BenchmarkConfig(
            memory_runs=10,
            warmup_runs=2,
            corpora_dir=tmp_path / "corpora",
            output_dir=tmp_path / "output",
            num_queries=100,
            generate_pdf=False,
        )
        assert config.memory_runs == 10
        assert config.warmup_runs == 2
        assert config.num_queries == 100
        assert config.generate_pdf is False


class TestCorpusConfig:
    """Tests for CorpusConfig model."""

    def test_creation(self, tmp_path: Path) -> None:
        """Test corpus configuration creation."""
        corpus_dir = tmp_path / "bhsa"
        corpus_dir.mkdir()
        tf_path = corpus_dir / "tf"
        tf_path.mkdir()
        config = CorpusConfig(name="bhsa", path=corpus_dir, tf_path=tf_path)
        assert config.name == "bhsa"
        assert config.path == corpus_dir
        assert config.tf_path == tf_path

    def test_size_ordering(self, tmp_path: Path) -> None:
        """Test corpus size ordering."""
        # Create directories
        for name in ["bhsa", "cuc", "lxx"]:
            (tmp_path / name).mkdir()
            (tmp_path / name / "tf").mkdir()

        corpora = [
            CorpusConfig(name="bhsa", path=tmp_path / "bhsa", tf_path=tmp_path / "bhsa" / "tf"),
            CorpusConfig(name="cuc", path=tmp_path / "cuc", tf_path=tmp_path / "cuc" / "tf"),
            CorpusConfig(name="lxx", path=tmp_path / "lxx", tf_path=tmp_path / "lxx" / "tf"),
        ]
        sorted_corpora = get_corpora_by_size(corpora)
        names = [c.name for c in sorted_corpora]
        # cuc should come first (smallest), bhsa last (largest)
        assert names.index("cuc") < names.index("lxx")
        assert names.index("lxx") < names.index("bhsa")


class TestSearchQuery:
    """Tests for SearchQuery model."""

    def test_creation(self) -> None:
        """Test query creation."""
        query = SearchQuery(
            id="lex_001",
            category="lexical",
            template="word sp=verb",
            description="Find all verbs",
            expected_complexity="low",
        )
        assert query.id == "lex_001"
        assert query.category == "lexical"
        assert query.validated is False

    def test_validation_status(self) -> None:
        """Test query validation status."""
        query = SearchQuery(
            id="lex_001",
            category="lexical",
            template="word sp=verb",
            description="Find all verbs",
            expected_complexity="low",
            validated=True,
        )
        assert query.validated is True


class TestQueryMeasurement:
    """Tests for QueryMeasurement model."""

    def test_successful_measurement(self) -> None:
        """Test successful query measurement."""
        m = QueryMeasurement(
            query_id="lex_001",
            implementation="TF",
            run_id=1,
            iteration=1,
            execution_time_ms=15.5,
            result_count=1000,
            success=True,
        )
        assert m.success is True
        assert m.error is None
        assert m.execution_time_ms == 15.5

    def test_failed_measurement(self) -> None:
        """Test failed query measurement."""
        m = QueryMeasurement(
            query_id="lex_001",
            implementation="CF",
            run_id=1,
            iteration=1,
            execution_time_ms=0.0,
            result_count=0,
            success=False,
            error="Invalid pattern syntax",
        )
        assert m.success is False
        assert m.error == "Invalid pattern syntax"


class TestStatisticalSummary:
    """Tests for StatisticalSummary model."""

    def test_creation(self) -> None:
        """Test statistical summary creation."""
        summary = StatisticalSummary(
            metric_name="latency",
            unit="ms",
            mean=15.5,
            median=14.2,
            std=3.2,
            variance=10.24,
            min=10.0,
            max=25.0,
            range=15.0,
            n=100,
            ci_lower=14.0,
            ci_upper=17.0,
            p25=12.0,
            p50=14.2,
            p75=18.0,
            p90=21.0,
            p95=22.0,
            p99=24.5,
        )
        assert summary.mean == 15.5
        assert summary.n == 100
        assert summary.ci_lower < summary.ci_upper
        assert summary.variance == pytest.approx(summary.std ** 2, rel=0.01)


class TestMemoryMeasurement:
    """Tests for MemoryMeasurement model."""

    def test_creation(self) -> None:
        """Test memory measurement creation."""
        m = MemoryMeasurement(
            corpus="bhsa",
            implementation="CF",
            mode="single",
            run_id=1,
            rss_before_mb=100.0,
            rss_after_mb=600.0,
            load_time_s=2.5,
        )
        assert m.corpus == "bhsa"
        assert m.implementation == "CF"
        assert m.rss_before_mb == 100.0
        assert m.rss_after_mb == 600.0
        assert m.memory_used_mb == 500.0  # computed field


class TestProgressiveLoadStep:
    """Tests for ProgressiveLoadStep model."""

    def test_creation(self) -> None:
        """Test progressive load step creation."""
        step = ProgressiveLoadStep(
            step=3,
            corpus_added="lxx",
            corpora_loaded=["cuc", "syrnt", "lxx"],
            implementation="CF",
            run_id=1,
            total_rss_mb=800.0,
            incremental_rss_mb=200.0,
            cumulative_load_time_s=5.0,
            step_load_time_s=2.0,
        )
        assert step.step == 3
        assert len(step.corpora_loaded) == 3
        assert step.incremental_rss_mb == 200.0


class TestScalingAnalysis:
    """Tests for ScalingAnalysis model."""

    def test_creation(self) -> None:
        """Test scaling analysis creation."""
        analysis = ScalingAnalysis(
            implementation="CF",
            slope_mb_per_corpus=50.0,
            intercept_mb=100.0,
            r_squared=0.98,
            predicted_10_corpora_mb=600.0,
            predicted_50_corpora_mb=2600.0,
        )
        assert analysis.slope_mb_per_corpus == 50.0
        assert analysis.r_squared == 0.98


class TestValidationReport:
    """Tests for ValidationReport model."""

    def test_creation(self) -> None:
        """Test validation report creation."""
        queries = [
            SearchQuery(
                id="lex_001",
                category="lexical",
                template="word sp=verb",
                description="Find verbs",
                expected_complexity="low",
                validated=True,
            ),
            SearchQuery(
                id="lex_002",
                category="lexical",
                template="word sp=noun",
                description="Find nouns",
                expected_complexity="low",
                validated=False,
                validation_error="Feature not found",
            ),
        ]
        report = ValidationReport(
            validation_corpus="cuc",
            total_queries=2,
            validated_count=1,
            failed_count=1,
            queries=queries,
        )
        assert report.total_queries == 2
        assert report.validated_count == 1
        assert report.failed_count == 1
        assert report.success_rate == 0.5
