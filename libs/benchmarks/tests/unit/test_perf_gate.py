"""Tests for the perf gate's target checks (no corpus needed)."""

from __future__ import annotations

import json
from typing import Any

import pytest

from cfabric_benchmarks.perf_gate import (
    DEFAULT_BASELINE,
    DEFAULT_TARGETS,
    _QUERY_FALLBACKS,
    assert_targets,
)

STEADY_PROBES = (
    "F.sp.v",
    "F.g_word.v",
    "L.u",
    "L.d",
    "L.n",
    "L.p",
    "L.i",
    "E.mother.f",
    "E.mother.t",
    "E.mother.b",
    "T.sectionFromNode",
    "T.text.verse",
    "T.nodeFromSection",
)


def _run(steady: dict[str, float], **extra: Any) -> dict[str, Any]:
    run: dict[str, Any] = {"steady_state_us": steady}
    run.update(extra)
    return run


class TestSteadyStateVersusTf:
    targets = {"steady_state_vs_tf": {"max_ratio": 1.25, "overrides": {"N.walk_full_s": 1.5}}}

    def test_within_ratio_passes(self) -> None:
        baseline = {"steady_state_us": {"L.n": 0.3}, "N.walk_full_s": 0.02}
        cf = _run({"L.n": 0.35}, **{"N.walk_full_s": 0.029})
        assert assert_targets(cf, self.targets, baseline) == []

    def test_per_call_scan_regression_is_caught(self) -> None:
        # The 0.6.0rc1 baseline: L.n rescanned all slots per call (21.6 ms).
        baseline = {"steady_state_us": {"L.n": 0.336}}
        failures = assert_targets(_run({"L.n": 21648.0}), self.targets, baseline)
        assert failures == ["steady L.n: cf 21648.0 > 1.25 x tf 0.336"]

    def test_override_applies_to_walk(self) -> None:
        baseline = {"steady_state_us": {}, "N.walk_full_s": 0.02}
        failures = assert_targets(_run({}, **{"N.walk_full_s": 0.031}), self.targets, baseline)
        assert failures == ["steady N.walk_full_s: cf 0.031 > 1.5 x tf 0.02"]

    def test_probe_without_tf_value_fails(self) -> None:
        failures = assert_targets(_run({"E.mother.t": 0.1}), self.targets, {})
        assert failures == ["steady E.mother.t: no TF baseline value to compare with"]


class TestAbsoluteCeilings:
    def test_steady_ceiling(self) -> None:
        targets = {"steady_state_us": {"E.mother.t": 2.0, "L.p": 2.0}}
        failures = assert_targets(_run({"E.mother.t": 519.0}), targets, {})
        assert failures == [
            "steady E.mother.t: 519.0µs/call > ceiling 2.0µs/call",
            "steady L.p: missing from cf run",
        ]

    def test_first_call_ceiling(self) -> None:
        targets = {"per_call_us": {"L.i": 50}}
        cf = {"L.i": {"per_call_us": 10538.0}}
        assert assert_targets(cf, targets, {}) == ["L.i: 10538.0µs/call > target 50µs/call"]

    def test_memory_ceilings(self) -> None:
        targets = {"rss_load_delta_mb": 20, "rss_final_mb": 250}
        failures = assert_targets({"rss_load_delta_mb": 13.0}, targets, {})
        assert failures == ["rss_final_mb: missing from cf run"]


class TestQueriesVersusTf:
    targets = {"queries_le_tf": True, "query_max_ratio_vs_tf": 1.1}

    def test_tolerance(self) -> None:
        baseline = {"queries": {"q_mother_basic": {"s": 0.042}}}
        assert assert_targets({"queries": {"q_mother_basic": {"s": 0.046}}}, self.targets, baseline) == []
        failures = assert_targets(
            {"queries": {"q_mother_basic": {"s": 0.0668}}}, self.targets, baseline
        )
        assert failures == ["query q_mother_basic: cf 0.0668s > 1.1 x tf 0.042s"]

    def test_query_without_tf_time_fails(self) -> None:
        failures = assert_targets({"queries": {"q_new": {"s": 0.01}}}, self.targets, {"queries": {}})
        assert failures == ["query q_new: no TF baseline time to compare with"]


@pytest.fixture(scope="module")
def targets() -> dict[str, Any]:
    return json.loads(DEFAULT_TARGETS.read_text())


@pytest.fixture(scope="module")
def baseline() -> dict[str, Any]:
    return json.loads(DEFAULT_BASELINE.read_text())


class TestCommittedGateFiles:
    """The committed targets and TF baseline cover every gated probe and query."""

    def test_targets_gate_every_steady_probe(self, targets: dict[str, Any]) -> None:
        assert set(targets["steady_state_us"]) == set(STEADY_PROBES)
        assert targets["steady_state_vs_tf"]["max_ratio"] >= 1.0
        for probe in ("L.n", "L.p", "L.i", "E.mother.t", "E.mother.b"):
            assert probe in targets["per_call_us"]
        assert targets["queries_le_tf"] is True

    def test_tf_baseline_has_steady_state_and_every_gate_query(
        self, baseline: dict[str, Any]
    ) -> None:
        assert baseline["engine"] == "tf"
        assert set(baseline["steady_state_us"]) == set(STEADY_PROBES)
        assert "N.walk_full_s" in baseline
        assert set(baseline["queries"]) == set(_QUERY_FALLBACKS)
        for slow_query in (
            "q_mother_basic",
            "q_mother_reverse",
            "q_mother_named_distinct_verse",
            "q_slot_overlap_parent",
        ):
            assert slow_query in baseline["queries"]
