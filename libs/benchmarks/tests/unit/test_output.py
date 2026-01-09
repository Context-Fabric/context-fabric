"""Tests for output functions (CSV, metadata)."""

from __future__ import annotations

import csv
import json
from pathlib import Path

import pytest

from cfabric_benchmarks.models.latency import (
    LatencyStatistics,
    QueryMeasurement,
)
from cfabric_benchmarks.models.memory import MemoryMeasurement
from cfabric_benchmarks.models.progressive import ProgressiveLoadStep
from cfabric_benchmarks.output.csv_writer import (
    write_latency_measurements_csv,
    write_latency_statistics_csv,
    write_memory_measurements_csv,
    write_progressive_steps_csv,
)
from cfabric_benchmarks.output.metadata import collect_environment


class TestMemoryMeasurementsCSV:
    """Tests for memory measurements CSV output."""

    def test_write_measurements(self, tmp_path: Path) -> None:
        """Test writing memory measurements to CSV."""
        measurements = [
            MemoryMeasurement(
                corpus="bhsa",
                implementation="TF",
                mode="single",
                run_id=1,
                rss_before_mb=100.0,
                rss_after_mb=600.0,
                load_time_s=2.5,
            ),
            MemoryMeasurement(
                corpus="bhsa",
                implementation="CF",
                mode="single",
                run_id=1,
                rss_before_mb=100.0,
                rss_after_mb=400.0,
                load_time_s=1.5,
            ),
        ]

        output_file = tmp_path / "memory.csv"
        write_memory_measurements_csv(measurements, output_file)

        assert output_file.exists()

        with open(output_file) as f:
            reader = csv.DictReader(f)
            rows = list(reader)

        assert len(rows) == 2
        assert rows[0]["corpus"] == "bhsa"
        assert rows[0]["implementation"] == "TF"
        assert float(rows[0]["rss_after_mb"]) == 600.0

    def test_empty_measurements(self, tmp_path: Path) -> None:
        """Test writing empty measurements list."""
        output_file = tmp_path / "empty.csv"
        write_memory_measurements_csv([], output_file)

        assert output_file.exists()
        with open(output_file) as f:
            content = f.read()
        # Should have header but no data rows
        assert "corpus" in content or content.strip() == ""


class TestLatencyMeasurementsCSV:
    """Tests for latency measurements CSV output."""

    def test_write_measurements(self, tmp_path: Path) -> None:
        """Test writing latency measurements to CSV."""
        measurements = [
            QueryMeasurement(
                query_id="lex_001",
                implementation="TF",
                run_id=1,
                iteration=1,
                execution_time_ms=15.5,
                result_count=1000,
                success=True,
            ),
            QueryMeasurement(
                query_id="lex_001",
                implementation="CF",
                run_id=1,
                iteration=1,
                execution_time_ms=10.2,
                result_count=1000,
                success=True,
            ),
        ]

        output_file = tmp_path / "latency.csv"
        write_latency_measurements_csv(measurements, output_file)

        assert output_file.exists()

        with open(output_file) as f:
            reader = csv.DictReader(f)
            rows = list(reader)

        assert len(rows) == 2
        assert rows[0]["query_id"] == "lex_001"


class TestLatencyStatisticsCSV:
    """Tests for latency statistics CSV output."""

    def test_write_statistics(self, tmp_path: Path) -> None:
        """Test writing latency statistics to CSV."""
        stats = [
            LatencyStatistics(
                query_id="lex_001",
                implementation="TF",
                category="lexical",
                mean_ms=15.0,
                std_ms=2.0,
                min_ms=12.0,
                max_ms=20.0,
                p50_ms=14.5,
                p95_ms=19.0,
                p99_ms=19.8,
                sample_count=10,
                error_count=0,
            ),
        ]

        output_file = tmp_path / "stats.csv"
        write_latency_statistics_csv(stats, output_file)

        assert output_file.exists()

        with open(output_file) as f:
            reader = csv.DictReader(f)
            rows = list(reader)

        assert len(rows) == 1
        assert rows[0]["query_id"] == "lex_001"
        assert float(rows[0]["mean_ms"]) == 15.0


class TestProgressiveStepsCSV:
    """Tests for progressive loading steps CSV output."""

    def test_write_steps(self, tmp_path: Path) -> None:
        """Test writing progressive loading steps to CSV."""
        steps = [
            ProgressiveLoadStep(
                step=1,
                corpus_added="cuc",
                corpora_loaded=["cuc"],
                implementation="CF",
                run_id=1,
                total_rss_mb=100.0,
                incremental_rss_mb=100.0,
                cumulative_load_time_s=0.5,
                step_load_time_s=0.5,
            ),
            ProgressiveLoadStep(
                step=2,
                corpus_added="syrnt",
                corpora_loaded=["cuc", "syrnt"],
                implementation="CF",
                run_id=1,
                total_rss_mb=200.0,
                incremental_rss_mb=100.0,
                cumulative_load_time_s=1.5,
                step_load_time_s=1.0,
            ),
        ]

        output_file = tmp_path / "progressive.csv"
        write_progressive_steps_csv(steps, output_file)

        assert output_file.exists()

        with open(output_file) as f:
            reader = csv.DictReader(f)
            rows = list(reader)

        assert len(rows) == 2
        assert rows[0]["corpus_added"] == "cuc"
        assert float(rows[1]["total_rss_mb"]) == 200.0


class TestEnvironmentMetadata:
    """Tests for environment metadata collection."""

    def test_collect_metadata(self) -> None:
        """Test collecting environment metadata."""
        env = collect_environment()

        # Check hardware info is populated
        assert env.hardware.cpu_cores > 0
        assert env.hardware.cpu_threads > 0
        assert env.hardware.ram_total_gb > 0

        # Check software info
        assert env.software.python_version is not None
        assert len(env.software.python_version) > 0

        # Check timestamp
        assert env.timestamp is not None

    def test_metadata_serialization(self) -> None:
        """Test that metadata can be serialized to JSON."""
        env = collect_environment()
        json_data = env.model_dump(mode="json")

        # Should be serializable
        json_str = json.dumps(json_data)
        assert len(json_str) > 0

        # Should be deserializable
        parsed = json.loads(json_str)
        assert "hardware" in parsed
        assert "software" in parsed
