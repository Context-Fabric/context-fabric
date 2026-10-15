# Context-Fabric 0.6.0rc1 baselines (2026-10-14)

Parity, speed and memory baselines for the Rust/PyO3 core ("CF Rust",
`context-fabric` 0.6.0rc1, branch `cf-rust-rc1`, commit `d2fa402`) against

* **TF**: Text-Fabric 13.0.19, the reference (the version the golden files and
  parity queries were generated with; installed from its git tag because 13.0.19
  is no longer on PyPI), and
* **CF Python**: `context-fabric` 0.5.8, the previous pure-Python core (PyPI).

Machine: Apple M5 Max, 18 cores, 128 GB, macOS 26.6.2; Python 3.13.12; rustc 1.96.0.
The machine was shared (1-minute load average 3-4 during the run); all checks ran
sequentially.

* Machine-readable results: [`rc1_baseline_2026-10-14.json`](rc1_baseline_2026-10-14.json)
* Raw outputs (logs, JUnit XML, per-engine JSON, oracle diffs) and every generated
  table: [`rc1_2026-10-14/`](rc1_2026-10-14/), full tables in
  [`rc1_2026-10-14/tables.md`](rc1_2026-10-14/tables.md)

## How to run

```sh
# from the repository root; one-time: build both virtual environments
libs/core/.venv/bin/python libs/benchmarks/scripts/rc_baselines/run_rc_baselines.py --setup --steps corpora
# every check, sequentially (~25 min); results go to /tmp/cf_rc1/runs/<UTC stamp>
libs/core/.venv/bin/python libs/benchmarks/scripts/rc_baselines/run_rc_baselines.py --results /tmp/cf_rc1/runs/final
# fold a results directory into this folder (JSON + rc1_<date>/ + tables.md)
libs/core/.venv/bin/python libs/benchmarks/scripts/rc_baselines/aggregate.py --results /tmp/cf_rc1/runs/final --date 2026-10-14
```

`--steps` selects a subset (`corpora, perf, perf_gate, queries, oracle, golden, rust,
pytest, microbench, bench_cli`). The runner (`libs/benchmarks/scripts/rc_baselines/`):

| File | What it does |
|---|---|
| `run_rc_baselines.py` | Sets up the envs (`libs/core/.venv`: CF Rust via `maturin develop --release` + TF 13.0.19; `libs/benchmarks/.venv-cfpy058`: CF Python 0.5.8 + TF), copies the corpora's `.tf` sources (no caches) to `/tmp/cf_rc1/corpora`, compiles a fresh explicit `.cfr` per corpus for the `openMapped`-based tools, runs every step and logs each command with exit code, duration and load average to `steps.jsonl`. |
| `engines.py` | One loader per engine (`tf`, `cf_python`, `cf_rust` = `loadAll`, `cf_rust_mapped` = `openMapped(<cache>)`), cache clearing for cold loads, RSS/USS/peak-RSS snapshots. |
| `perf_probe.py` | One engine per process: cold load (cache deleted first) or warm load, memory at each phase, the `perf_gate` probes reproduced exactly, steady-state per-call latency (warm-up, median of 5 passes of 50k calls; time-budgeted for millisecond APIs), full `N.walk`, workload queries. |
| `query_suite.py` | The 34 ETCBC parity queries: count, golden-tuple membership, wall time, and a SHA-1 of the sorted result set compared with TF's (full set parity). |
| `oracle_parity.py` | Public-API value checks against TF: plan (TF samples up to 200 nodes per node type), eval (any engine), compare (per API area, every failing check). |
| `aggregate.py` | Builds the JSON, `tables.md`, and the regression check against `cf_0.6.0_record.json`. |

Existing tooling run by the runner: `cargo fmt --check`, `cargo test --features python`,
pytest for `libs/core/tests/python`, `libs/core/tests/parity` (`CONTEXT_FABRIC_RUN_PARITY=1`),
`libs/benchmarks/tests`, `libs/mcp/tests`, `tests/golden/compare.py` (CF Rust and
CF Python 0.5.8), `cfabric_benchmarks.perf_gate` (TF, then CF asserted against it),
`bench_scalar_latency.py`, `bench_batch_apis.py`, and `cfabric-bench latency|memory`
on BHSA in both envs. The core test suites resolve corpora from `CFABRIC_CORPORA`
and the explicit BHSA cache from `CFABRIC_BHSA_CFR`.

## Headline (BHSA)

| | TF 13.0.19 | CF Python 0.5.8 | CF Rust 0.6.0rc1 |
|---|---|---|---|
| Cold load (engine cache rebuilt), s | 35.3 | 67.1 | 28.7 |
| Warm load, s | 4.00 | 0.43 | 0.003 |
| RSS after warm load, MB | 6,170 | 290 | 33 |
| RSS / peak RSS after workload, MB | 6,169 / 6,309 | 421 / 559 | 250 / 270 |
| 34-query parity (count + full result set) | reference | 34/34 fresh cache, 33/34 reused cache | 34/34 |
| 34-query wall time, s | 12.9 | 11.5 fresh / 27.6 reused | 2.4 (process per query), 2.2 (in-process) |
| Oracle API value checks | reference | 355,975/355,975 fresh, 353,075 reused | 355,974/355,975 |
| `cfabric-bench latency`, 100 curated queries: total, geomean speedup over TF | 16.6 s | 14.8 s, 1.15x | 1.35 s, 15.4x (slower on 0 queries) |
| `cfabric-bench memory` RSS MB: single / spawn ×4 / fork ×4 | 6,275 / 25,123 / 6,424 | 388 / 1,560 / 524 | 143 / 642 / 260 |

## Parity

### CF Rust 0.6.0rc1

| Check | Result |
|---|---|
| Golden probes (TF truth) | bhsa 13/13, n1904 10/10, banks 11/11 |
| 34-query suite (`query_suite.py`) | 34/34 counts, golden tuples present, result sets identical to TF |
| `tests/parity` pytest (34 queries, 2 order cases, TF oracle on 3 corpora, strict API surface, golden probes, curated BHSA queries) | 52/52 passed |
| Oracle value checks (`oracle_parity.py`) | bhsa 355,974/355,975; n1904 147,307/147,308; banks 2,369/2,370 |
| `cargo fmt --check`, `cargo test --features python` | pass; 101 passed, 0 failed |
| pytest `libs/core/tests/python` (includes stubtest) | 253 passed, 8 skipped (quran corpus not present) |
| pytest `libs/mcp/tests` / `libs/benchmarks/tests` | 166/166 / 56/56 |

The one failing oracle check per corpus is `F.otype.freqList`: CF Rust provides a
frequency list for `otype`, where TF's `otype` has no `freqList` (raises
`AttributeError`). It is an addition, not a wrong value.

### CF Python 0.5.8 depends on its cache state

CF Python 0.5.8 gives different answers depending on whether its `.cfm` cache was
just built in the same process ("fresh") or is reused by a later process ("reused",
the normal case for users). Proven by evaluating the same oracle plan twice on each
corpus, deleting `.cfm` before the fresh run:

| Corpus | Fresh `.cfm` build | Reused `.cfm` |
|---|---|---|
| bhsa | 355,975/355,975 | 353,075/355,975 |
| n1904 | 147,308/147,308 | 145,986/147,308 |
| banks | 2,370/2,370 | 2,350/2,370 |
| 34 queries | 34/34 | 33/34 (`q_knear_after` wrong count) |

Reused-cache failures by API area: **E** `E.b` raises `AttributeError: 'tuple' object
has no attribute 'tolist'` (bhsa `mother`, `functional_parent`,
`distributional_parent`; n1904 `parent`, `subjref`); **F** bhsa `dist` returns
`None` for `-1` values and its `freqList` differs; **T** banks
`T.headingFromNode`, `T.top`, `T.structure` return `None` (structure lost). The
golden comparison for CF Python (run on the reused cache) fails the same six banks
structure probes.

## Speed and memory

BHSA below; N1904 tables are in `rc1_2026-10-14/tables.md`. "Cold" deletes the
engine's cache so the load rebuilds it (`.tf/`, `.cfm/`, `.cfr/`); the operating
system file cache stays warm (flushing it needs `sudo purge`, not available
unattended). USS is `psutil`'s unique set size; on macOS it counts compressed
private memory, which is why CF Python's USS exceeds its RSS.

### BHSA: load and memory

| Metric | TF 13.0.19 | CF Python 0.5.8 | CF Rust (loadAll) | CF Rust (openMapped) |
|---|---|---|---|---|
| Cold load, s (cache rebuilt) | 35.27 | 67.08 | 28.73 | – |
| Warm load, s (median of 3) | 4.002 | 0.427 | 0.003 | – |
| Peak RSS during cold load, MB | 6,157 | 8,649 | 3,080 | – |
| RSS after warm load, MB | 6,170 | 290 | 33 | 32 |
| USS after warm load, MB | 6,173 | 960 | 16 | 16 |
| RSS after workload, MB | 6,169 | 421 | 250 | 251 |
| USS after workload, MB | 6,146 | 1,002 | 96 | 98 |
| Peak RSS after workload, MB | 6,309 | 559 | 270 | 272 |

### BHSA: steady-state per-call latency (µs, median of 5 × 50k calls)

| API | TF 13.0.19 | CF Python 0.5.8 | CF Rust (loadAll) | CF Rust (openMapped) |
|---|---|---|---|---|
| F.sp.v | 0.056 | 0.183 | 0.029 | 0.028 |
| F.g_word.v | 0.069 | 0.208 | 0.038 | 0.037 |
| **L.u** | 0.111 | 0.820 | **0.131** | **0.129** |
| L.d | 0.569 | 3.269 | 0.136 | 0.134 |
| L.n | 0.237 | 1.576 | 0.109 | 0.102 |
| L.p | 0.222 | 1.593 | 0.094 | 0.093 |
| L.i | 0.039 | 0.037 | 0.021 | 0.021 |
| E.mother.f | 0.035 | 0.349 | 0.032 | 0.032 |
| E.mother.t | 0.043 | 0.389 | 0.030 | 0.030 |
| E.mother.b | 0.067 | error | 0.040 | 0.040 |
| T.sectionFromNode | 5.335 | 13.752 | 0.450 | 0.445 |
| T.text (verse) | 8.818 | 31.315 | 1.558 | 1.524 |
| T.nodeFromSection | 0.223 | 0.263 | 0.202 | 0.201 |
| **N.walk full pass, s** | 0.0199 | 0.1563 | **0.0298** | **0.0285** |

CF Rust is slower than TF only on `L.u` (1.2x) and a full `N.walk` (1.5x).

### BHSA: first-call probes (`perf_gate` style)

These time the first `n` calls after load, so they include the one-time building
of lazy indexes. CF Rust is slower than TF on `L.u` (2.4 vs 0.6 µs, n=30),
`E.mother.f` (1.3 vs 0.2 µs, n=500) and `T.nodeFromSection` (61 vs 0.9 µs, n=100:
the first call does a one-time setup of tens of milliseconds; later calls take under 1 µs). Every
workload query is 3-60x faster than TF. Full table in `rc1_2026-10-14/tables.md`.

### 34-query suite (BHSA, seconds)

CF Rust is faster than TF on all 34 queries (in-process total 2.2 s vs 12.9 s);
`q_regex_gloss_space` is the closest (0.003 vs 0.004). Per-query times for every
engine are in `rc1_2026-10-14/tables.md`.

### Other benchmarks (BHSA)

| Tool | Result |
|---|---|
| `bench_scalar_latency.py` | `F.v` 0.036-0.058 µs per call (call floor 0.023 µs); 1.4 MB RSS growth over 36M calls |
| `bench_batch_apis.py` (100k nodes) | over the CF per-call loop: `F.sp.vs` 2.6x, `F.number.vs` 2.3x, `vs_array` 3.5x, `L.d_many` 1.1x, `T.text_many` 1.1x, `L.u_many` 0.9x |
| `perf_gate --assert --strict` | all targets met; final RSS 255 MB |

## Regressions against the 0.6.0 record

None above the threshold (1.25x slower and above a noise floor) against
`cf_0.6.0_record.json`.

## Notes

* Text-Fabric 13.0.19 is installed from `git+https://github.com/annotation/text-fabric@v13.0.19`
  (PyPI only offers 13.1.x now).
* The runner deletes and recompiles its explicit `.cfr` caches on every run, because
  this release refuses caches written in an older `.cfr` format.
