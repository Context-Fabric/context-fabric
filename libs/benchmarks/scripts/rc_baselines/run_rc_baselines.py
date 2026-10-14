#!/usr/bin/env python3
"""Reproducible RC baseline runner: parity, performance and memory for TF / CF Python / CF Rust.

Runs every check sequentially (clean timings) and writes machine-readable output
to ``--results`` (default ``/tmp/cf_rc1/runs/<UTC stamp>``). ``aggregate.py``
then folds a results directory into ``libs/benchmarks/baselines/``.

Engines and their virtual environments (created by ``--setup``):

* CF Rust 0.6.0rc1 + Text-Fabric 13.0.19: ``libs/core/.venv`` (maturin build of
  this worktree, plus ``cfabric-benchmarks`` and ``cfabric-mcp`` editable).
* CF Python 0.5.8 (+ Text-Fabric 13.0.19 for the ``cfabric-bench`` comparisons):
  ``libs/benchmarks/.venv-cfpy058``.

Text-Fabric 13.0.19 (the version the golden files and parity queries were
generated with) is no longer on PyPI, so it is installed from its git tag.

Steps (``--steps`` takes a comma list; default runs all, timing-sensitive steps first):

``corpora``     copy the ``.tf`` sources (no caches) into ``<scratch>/corpora``.
``rust``        ``cargo fmt --check``, ``cargo test --features python``, the
                ``cf_rust_parity`` (34 queries, raw Rust) and
                ``cf_rust_validate_corpora`` binaries.
``pytest``      core ``tests/python``, core ``tests/parity`` (query + TF oracle,
                ``CONTEXT_FABRIC_RUN_PARITY=1``), benchmarks, mcp, repo ``tests``.
``golden``      ``tests/golden/compare.py`` for CF Rust (materialized, mapped,
                python) and for CF Python 0.5.8 (python mode).
``queries``     the 34 parity queries on TF, CF Python and CF Rust (``query_suite.py``).
``oracle``      public-API value checks vs TF on every corpus (``oracle_parity.py``).
``perf``        cold/warm load, memory and per-call latency per engine (``perf_probe.py``).
``perf_gate``   ``cfabric_benchmarks.perf_gate`` for TF and CF Rust (June-record tool).
``legacy``      existing scripts: ``compare_bhsa_mapped_wheel_overhead.py``,
                ``compare_bhsa_tf_mapped_subset.py``, the mapped-subset timings on
                CF Python 0.5.8, ``bench_scalar_latency.py``, ``bench_batch_apis.py``.
``bench_cli``   ``cfabric-bench latency`` and ``memory`` on BHSA in both envs.

Usage::

    python3 libs/benchmarks/scripts/rc_baselines/run_rc_baselines.py --setup
    python3 libs/benchmarks/scripts/rc_baselines/run_rc_baselines.py --steps perf,perf_gate
    python3 libs/benchmarks/scripts/rc_baselines/aggregate.py --results /tmp/cf_rc1/runs/<stamp>
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import platform
import shutil
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
CORE = REPO / "libs" / "core"
BENCH = REPO / "libs" / "benchmarks"
SOURCE_CORPORA = BENCH / ".corpora"
RUST_VENV = CORE / ".venv"
CFPY_VENV = BENCH / ".venv-cfpy058"
TF_REQUIREMENT = "text-fabric @ git+https://github.com/annotation/text-fabric@v13.0.19"
CFPY_REQUIREMENT = "context-fabric==0.5.8"
PYENV_PYTHON = "3.13.12"
BENCH_DEPS = ["pydantic>=2.0", "click>=8.0", "numpy>=1.26", "pandas>=2.0", "matplotlib>=3.8",
              "seaborn>=0.13", "psutil>=5.9", "scipy>=1.11", "pytest", "pytest-cov"]
MCP_DEPS = ["mcp>=1.0,<2", "uvicorn>=0.32.0", "pytest-asyncio", "requests>=2.32"]

CORPORA = ("bhsa", "n1904", "banks")
PERF_CORPORA = ("bhsa", "n1904")  # banks (~100 nodes) is too small for timing
PERF_ENGINES = ("tf", "cf_python", "cf_rust")
WARM_REPEATS = 3
# BHSA cache paths hard-coded by tests/python and the bench_* scripts.
FIXED_BHSA_CACHE_PATHS = ("/tmp/cf_bhsa.cfr", "/tmp/cf_bhsa_v3_full.cfr", "/tmp/cf_bhsa_w4.cfr")
# Timing-sensitive steps run first, before the long test suites heat the machine.
ALL_STEPS = ("corpora", "perf", "perf_gate", "queries", "oracle", "golden", "rust", "pytest",
             "legacy", "bench_cli")


class Runner:
    """Executes steps, logging every command and its exit status to ``steps.jsonl``."""

    def __init__(self, results: Path, scratch: Path) -> None:
        self.results = results
        self.scratch = scratch
        self.corpora = scratch / "corpora"
        self.bhsa_cfr = self.cfr_path("bhsa")
        self.rust_py = RUST_VENV / "bin" / "python"
        self.cfpy_py = CFPY_VENV / "bin" / "python"
        results.mkdir(parents=True, exist_ok=True)

    # ------------------------------------------------------------------ helpers
    def run(self, name: str, command: list[str], *, log: Path, cwd: Path = Path("/tmp"),
            env: dict[str, str] | None = None, timeout: float | None = None) -> int:
        log.parent.mkdir(parents=True, exist_ok=True)
        full_env = {**os.environ, **(env or {})}
        load_avg = os.getloadavg()
        start = time.time()
        print(f"[{dt.datetime.now():%H:%M:%S}] {name}: {' '.join(command)}", flush=True)
        with log.open("w") as handle:
            try:
                code = subprocess.run(command, cwd=cwd, env=full_env, stdout=handle,
                                      stderr=subprocess.STDOUT, timeout=timeout).returncode
            except subprocess.TimeoutExpired:
                code = -999
        seconds = round(time.time() - start, 1)
        entry = {"step": name, "command": command, "cwd": str(cwd), "exit": code,
                 "seconds": seconds, "load_avg_at_start": [round(x, 2) for x in load_avg],
                 "log": str(log.relative_to(self.results))}
        with (self.results / "steps.jsonl").open("a") as handle:
            handle.write(json.dumps(entry) + "\n")
        print(f"    exit={code} seconds={seconds}", flush=True)
        return code

    def tf_path(self, corpus: str) -> Path:
        return self.corpora / corpus / "tf"

    def cfr_path(self, corpus: str) -> Path:
        """Explicit compiled cache used by openMapped-based tools (outside the corpus dir)."""
        return self.scratch / f"cf_{corpus}.cfr"

    def ensure_cfr(self, corpus: str) -> Path:
        cache = self.cfr_path(corpus)
        if not cache.exists():
            self.run(f"compile {corpus} cfr", [str(self.rust_py), "-c",
                     "import sys; from cfabric import Fabric; "
                     "Fabric(locations=sys.argv[1]).compile(sys.argv[2])",
                     str(self.tf_path(corpus)), str(cache)],
                     log=self.results / "setup" / f"compile_{corpus}_cfr.log")
        return cache

    def ensure_bhsa_cfr(self) -> None:
        self.ensure_cfr("bhsa")

    def link_fixed_bhsa_caches(self) -> None:
        """Point the hard-coded BHSA cache paths of tests and bench scripts at the scratch cache."""
        self.ensure_bhsa_cfr()
        for fixed in FIXED_BHSA_CACHE_PATHS:
            link = Path(fixed)
            if link.is_symlink() or link.exists():
                link.unlink()
            link.symlink_to(self.bhsa_cfr)

    # ------------------------------------------------------------------ steps
    def step_corpora(self) -> None:
        for corpus in CORPORA:
            target = self.tf_path(corpus)
            target.mkdir(parents=True, exist_ok=True)
            self.run(f"copy {corpus}", ["rsync", "-a", "--exclude", ".tf", "--exclude", ".cfm",
                     "--exclude", ".cfr", f"{SOURCE_CORPORA / corpus / 'tf'}/", f"{target}/"],
                     log=self.results / "setup" / f"copy_{corpus}.log")

    def step_rust(self) -> None:
        out = self.results / "rust"
        env = {"PYO3_PYTHON": str(self.rust_py)}
        self.run("cargo fmt --check", ["cargo", "fmt", "--check"], cwd=CORE, log=out / "cargo_fmt.log")
        self.run("cargo test", ["cargo", "test", "--features", "python"], cwd=CORE, env=env,
                 log=out / "cargo_test.log")
        self.ensure_bhsa_cfr()
        self.run("cf_rust_parity", ["cargo", "run", "--release", "--quiet", "--bin", "cf_rust_parity",
                 "--", str(self.bhsa_cfr), str(CORE / "tests/parity/parity_queries.json"), "", "300"],
                 cwd=CORE, log=out / "cf_rust_parity.log")
        self.run("cf_rust_validate_corpora", ["cargo", "run", "--release", "--quiet", "--bin",
                 "cf_rust_validate_corpora", "--", str(self.corpora)], cwd=CORE,
                 log=out / "cf_rust_validate_corpora.log")

    def step_pytest(self) -> None:
        out = self.results / "pytest"
        self.link_fixed_bhsa_caches()
        suites = {
            "core_python": (CORE, ["tests/python"], {}),
            "core_parity": (CORE, ["tests/parity"], {
                "CONTEXT_FABRIC_RUN_PARITY": "1", "CFABRIC_BHSA_CFR": str(self.bhsa_cfr),
                "CFABRIC_BHSA_TF": str(self.tf_path("bhsa")), "CFQ_TIMEOUT": "300"}),
            "benchmarks": (BENCH, ["tests"], {}),
            "mcp": (REPO / "libs" / "mcp", ["tests"], {}),
            "repo_tests": (REPO, ["tests"], {}),
        }
        for name, (cwd, paths, env) in suites.items():
            self.run(f"pytest {name}", [str(self.rust_py), "-m", "pytest", *paths, "-q", "-rA",
                     "-p", "no:cacheprovider", f"--junitxml={out / (name + '.xml')}"],
                     cwd=cwd, env=env, log=out / f"{name}.log")

    def step_golden(self) -> None:
        out = self.results / "golden"
        script = str(CORE / "tests/golden/compare.py")
        self.run("golden cf_rust", [str(self.rust_py), script, *CORPORA], cwd=REPO,
                 log=out / "cf_rust.log")
        self.run("golden cf_python", [str(self.cfpy_py), script, *CORPORA, "--mode", "python"],
                 cwd=REPO, log=out / "cf_python.log")

    def step_queries(self) -> None:
        out = self.results / "queries"
        self.ensure_bhsa_cfr()
        # CF Python 0.5.8 answers differently after a fresh .cfm build ("cold") than
        # when it loads an existing .cfm ("warm"), so both states are measured.
        runs = (("tf", "tf", self.rust_py, "auto", False),
                ("cf_python_cold", "cf_python", self.cfpy_py, "auto", True),
                ("cf_python", "cf_python", self.cfpy_py, "auto", False),
                ("cf_rust", "cf_rust", self.rust_py, "isolated", False),
                ("cf_rust_inprocess", "cf_rust", self.rust_py, "in-process", False))
        for label, engine, python, mode, cold in runs:
            self.run(f"query suite {label}", [str(python), str(HERE / "query_suite.py"),
                     "--engine", engine, "--tf-path", str(self.tf_path("bhsa")), "--mode", mode,
                     "--timeout", "300", "--out", str(out / f"{label}.jsonl"),
                     *(["--clear-cache"] if cold else [])],
                     log=out / f"{label}.log")

    def step_oracle(self) -> None:
        script = str(HERE / "oracle_parity.py")
        for corpus in CORPORA:
            out = self.results / "oracle" / corpus
            tf_path = str(self.tf_path(corpus))
            plan = out / "plan.json"
            self.run(f"oracle plan {corpus}", [str(self.rust_py), script, "plan", "--tf-path", tf_path,
                     "--out", str(plan)], log=out / "plan.log")
            # CF Python 0.5.8 is evaluated after a fresh .cfm build and on a reused .cfm.
            evals = (("tf", "tf", self.rust_py, False), ("cf_rust", "cf_rust", self.rust_py, False),
                     ("cf_python_cold", "cf_python", self.cfpy_py, True),
                     ("cf_python", "cf_python", self.cfpy_py, False))
            for label, engine, python, cold in evals:
                self.run(f"oracle eval {corpus} {label}", [str(python), script, "eval", "--engine",
                         engine, "--tf-path", tf_path, "--plan", str(plan),
                         "--out", str(out / f"values_{label}.json"),
                         *(["--clear-cache"] if cold else [])], log=out / f"eval_{label}.log")
            for engine in ("tf", "cf_rust", "cf_python_cold", "cf_python"):
                self.run(f"oracle compare {corpus} {engine}", [str(self.rust_py), script, "compare",
                         "--plan", str(plan), "--expected", str(out / "values_tf.json"),
                         "--actual", str(out / f"values_{engine}.json"),
                         "--out", str(out / f"cmp_{engine}.json")], log=out / f"cmp_{engine}.log")

    def step_perf(self) -> None:
        script = str(HERE / "perf_probe.py")
        for corpus in PERF_CORPORA:
            out = self.results / "perf" / corpus
            tf_path = str(self.tf_path(corpus))
            for engine in PERF_ENGINES:
                python = str(self.cfpy_py if engine == "cf_python" else self.rust_py)
                base = [python, script, "--engine", engine, "--corpus", corpus, "--tf-path", tf_path]
                self.run(f"perf {corpus} {engine} cold", [*base, "--cold", "--load-only",
                         "--out", str(out / f"{engine}_cold.json")], log=out / f"{engine}_cold.log")
                for rep in range(1, WARM_REPEATS + 1):
                    self.run(f"perf {corpus} {engine} warm{rep}", [*base, "--load-only",
                             "--out", str(out / f"{engine}_warm{rep}.json")],
                             log=out / f"{engine}_warm{rep}.log")
                self.run(f"perf {corpus} {engine} full", [*base, "--out",
                         str(out / f"{engine}_full.json")], log=out / f"{engine}_full.log")
            cache = self.ensure_cfr(corpus)
            base = [str(self.rust_py), script, "--engine", "cf_rust_mapped", "--corpus", corpus,
                    "--tf-path", tf_path, "--mapped-cache", str(cache)]
            self.run(f"perf {corpus} cf_rust_mapped full", [*base, "--out",
                     str(out / "cf_rust_mapped_full.json")], log=out / "cf_rust_mapped_full.log")

    def step_perf_gate(self) -> None:
        out = self.results / "perf_gate"
        self.ensure_bhsa_cfr()
        base = [str(self.rust_py), "-m", "cfabric_benchmarks.perf_gate", "--bhsa-tf",
                str(self.tf_path("bhsa"))]
        self.run("perf_gate tf", [*base, "--engine", "tf", "--out", str(out / "tf.json")],
                 log=out / "tf.log")
        self.run("perf_gate cf", [*base, "--engine", "cf", "--cache", str(self.bhsa_cfr), "--assert",
                 "--out", str(out / "cf.json")], log=out / "cf.log")

    def step_legacy(self) -> None:
        out = self.results / "legacy"
        self.ensure_bhsa_cfr()
        bhsa = str(self.tf_path("bhsa"))
        curated_cache = self.scratch / "bhsa-mapped-curated.cfr"
        scripts = CORE / "scripts"
        self.run("wheel overhead", [str(self.rust_py), str(scripts / "compare_bhsa_mapped_wheel_overhead.py"),
                 "--tf-path", bhsa, "--cache-path", str(curated_cache), "--limit", "5"],
                 cwd=CORE, log=out / "compare_bhsa_mapped_wheel_overhead.log")
        self.run("tf vs mapped subset", [str(self.rust_py), str(scripts / "compare_bhsa_tf_mapped_subset.py"),
                 "--tf-path", bhsa, "--cache-path", str(curated_cache), "--limit", "5"],
                 cwd=CORE, log=out / "compare_bhsa_tf_mapped_subset.log")
        # compare_bhsa_mapped_subset.py / compare_bhsa_mapped_memory.py put libs/core on
        # PYTHONPATH, which now resolves to the Rust overlay; run their Python half
        # directly on CF Python 0.5.8 instead (the Rust half is the curated binary).
        self.run("mapped subset cf_python", [str(self.cfpy_py), str(scripts / "python_bhsa_mapped_subset.py"),
                 bhsa, "5"], log=out / "python_bhsa_mapped_subset_cf_python.log")
        self.run("mapped subset raw rust", ["cargo", "run", "--release", "--quiet", "--bin",
                 "cf_rust_validate_bhsa_mapped_curated", "--", bhsa, str(curated_cache), "5"],
                 cwd=CORE, log=out / "cf_rust_validate_bhsa_mapped_curated.log")
        self.link_fixed_bhsa_caches()
        self.run("bench_scalar_latency", [str(self.rust_py), str(BENCH / "bench_scalar_latency.py")],
                 log=out / "bench_scalar_latency.log")
        self.run("bench_batch_apis", [str(self.rust_py), str(BENCH / "bench_batch_apis.py")],
                 log=out / "bench_batch_apis.log")

    def step_bench_cli(self) -> None:
        for label, venv in (("cf_rust", RUST_VENV), ("cf_python", CFPY_VENV)):
            out = self.results / "bench_cli" / label
            cli = str(venv / "bin" / "cfabric-bench")
            self.run(f"cfabric-bench latency {label}", [cli, "latency", "--corpus", "bhsa",
                     "--corpora-dir", str(self.corpora), "--queries", "104", "--iterations", "3",
                     "--runs", "1", "--output-dir", str(out / "latency")], log=out / "latency.log")
            self.run(f"cfabric-bench memory {label}", [cli, "memory", "-c", "bhsa", "--corpora-dir",
                     str(self.corpora), "--runs", "3", "--warmup", "1", "--workers", "4",
                     "--output-dir", str(out / "memory")], log=out / "memory.log")


def setup_envs() -> None:
    """Create/refresh both virtual environments (idempotent)."""
    def sh(*command: str, cwd: Path = REPO) -> None:
        print("+", " ".join(command), flush=True)
        subprocess.run(list(command), cwd=cwd, check=True,
                       env={**os.environ, "PYENV_VERSION": PYENV_PYTHON})

    for venv in (RUST_VENV, CFPY_VENV):
        if not (venv / "bin" / "python").exists():
            sh("python", "-m", "venv", str(venv))
        sh(str(venv / "bin" / "pip"), "install", "-q", "--upgrade", "pip")
    rust_pip = str(RUST_VENV / "bin" / "pip")
    sh(rust_pip, "install", "-q", "maturin>=1.8,<2", "pyyaml", TF_REQUIREMENT, *BENCH_DEPS, *MCP_DEPS)
    sh(str(RUST_VENV / "bin" / "maturin"), "develop", "--release", cwd=CORE)
    sh(rust_pip, "install", "-q", "--no-deps", "-e", str(BENCH), "-e", str(REPO / "libs" / "mcp"))
    cfpy_pip = str(CFPY_VENV / "bin" / "pip")
    sh(cfpy_pip, "install", "-q", CFPY_REQUIREMENT, TF_REQUIREMENT, *BENCH_DEPS)
    sh(cfpy_pip, "install", "-q", "--no-deps", "-e", str(BENCH))


def record_environment(runner: Runner) -> None:
    def capture(command: list[str], cwd: Path = REPO) -> str:
        try:
            return subprocess.run(command, cwd=cwd, capture_output=True, text=True).stdout.strip()
        except OSError as error:
            return f"unavailable: {error}"

    probe = ("import platform, cfabric, tf.parameters as p; "
             "print(platform.python_version(), cfabric.__version__, p.VERSION)")
    env = {
        "timestamp_utc": dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds"),
        "git_branch": capture(["git", "rev-parse", "--abbrev-ref", "HEAD"]),
        "git_commit": capture(["git", "rev-parse", "HEAD"]),
        "git_dirty": bool(capture(["git", "status", "--porcelain"])),
        "machine": {
            "cpu": capture(["sysctl", "-n", "machdep.cpu.brand_string"]),
            "cores": os.cpu_count(),
            "memory_gb": round(int(capture(["sysctl", "-n", "hw.memsize"]) or 0) / 2**30),
            "os": f"{platform.system()} {platform.mac_ver()[0]} {platform.machine()}",
        },
        "rustc": capture(["rustc", "--version"]),
        "cargo": capture(["cargo", "--version"]),
        "rust_venv": {"path": str(RUST_VENV), "python/cfabric/tf": capture([str(runner.rust_py), "-c", probe], Path("/tmp"))},
        "cf_python_venv": {"path": str(CFPY_VENV), "python/cfabric/tf": capture([str(runner.cfpy_py), "-c", probe], Path("/tmp"))},
        "scratch_corpora": str(runner.corpora),
        "source_corpora": str(SOURCE_CORPORA.resolve()),
        "bhsa_checkout": capture(["cat", str(SOURCE_CORPORA / "bhsa" / "tf" / "__checkout__.txt")]),
        "load_avg_at_start": list(os.getloadavg()),
    }
    (runner.results / "environment.json").write_text(json.dumps(env, indent=1) + "\n")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    parser.add_argument("--results", type=Path, default=Path(f"/tmp/cf_rc1/runs/{stamp}"))
    parser.add_argument("--scratch", type=Path, default=Path("/tmp/cf_rc1"))
    parser.add_argument("--steps", default=",".join(ALL_STEPS))
    parser.add_argument("--setup", action="store_true", help="create/refresh the venvs first")
    args = parser.parse_args(argv)

    steps = [s.strip() for s in args.steps.split(",") if s.strip()]
    unknown = sorted(set(steps) - set(ALL_STEPS))
    if unknown:
        parser.error(f"unknown steps: {unknown}")
    if args.setup:
        setup_envs()
    runner = Runner(args.results.resolve(), args.scratch.resolve())
    if "corpora" in steps or not runner.corpora.exists():
        runner.step_corpora()
    record_environment(runner)
    for step in steps:
        if step != "corpora":
            getattr(runner, f"step_{step}")()
    print(f"results: {runner.results}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
