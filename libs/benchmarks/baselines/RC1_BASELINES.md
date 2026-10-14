# Context-Fabric 0.6.0rc1 baselines (2026-10-14)

Fresh parity, speed and memory baselines for the Rust/PyO3 core ("CF Rust",
`context-fabric` 0.6.0rc1, worktree commit `774e8d0`) against

* **TF**: Text-Fabric 13.0.19, the reference (the version the golden files and
  parity queries were generated with; installed from its git tag because 13.0.19
  is no longer on PyPI), and
* **CF Python**: `context-fabric` 0.5.8, the previous pure-Python core (PyPI).

Machine: Apple M5 Max, 18 cores, 128 GB, macOS 26.6.2; Python 3.13.12; rustc 1.96.0.
The machine was shared (a VM and other agents' jobs; 1-minute load average 3-8
during the runs); all checks ran sequentially.

* Machine-readable results: [`rc1_baseline_2026-10-14.json`](rc1_baseline_2026-10-14.json)
* Raw outputs (logs, JUnit XML, per-engine JSON, oracle diffs) and every generated
  table: [`rc1_2026-10-14/`](rc1_2026-10-14/), full tables in
  [`rc1_2026-10-14/tables.md`](rc1_2026-10-14/tables.md)

## How to run

```sh
# from the repository root; one-time: build both virtual environments
libs/core/.venv/bin/python libs/benchmarks/scripts/rc_baselines/run_rc_baselines.py --setup --steps corpora
# every check, sequentially (~45 min); results go to /tmp/cf_rc1/runs/<UTC stamp>
libs/core/.venv/bin/python libs/benchmarks/scripts/rc_baselines/run_rc_baselines.py --results /tmp/cf_rc1/runs/final
# fold a results directory into this folder (JSON + rc1_<date>/ + tables.md)
libs/core/.venv/bin/python libs/benchmarks/scripts/rc_baselines/aggregate.py --results /tmp/cf_rc1/runs/final --date 2026-10-14
```

`--steps` selects a subset (`corpora, perf, perf_gate, queries, oracle, golden, rust,
pytest, legacy, bench_cli`). The runner (`libs/benchmarks/scripts/rc_baselines/`):

| File | What it does |
|---|---|
| `run_rc_baselines.py` | Sets up the envs (`libs/core/.venv`: CF Rust via `maturin develop --release` + TF 13.0.19; `libs/benchmarks/.venv-cfpy058`: CF Python 0.5.8 + TF), copies the corpora's `.tf` sources (no caches) to `/tmp/cf_rc1/corpora`, runs every step and logs each command with exit code, duration and load average to `steps.jsonl`. |
| `engines.py` | One loader per engine (`tf`, `cf_python`, `cf_rust` = `loadAll`, `cf_rust_mapped` = `openMapped(<cache>)`), cache clearing for cold loads, RSS/USS/peak-RSS snapshots. |
| `perf_probe.py` | One engine per process: cold load (cache deleted first) or warm load, memory at each phase, the `perf_gate` probes reproduced exactly, steady-state per-call latency (warm-up, median of 5 passes of 50k calls; time-budgeted for millisecond APIs), full `N.walk`, workload queries. |
| `query_suite.py` | The 34 ETCBC parity queries: count, golden-tuple membership, wall time, and a SHA-1 of the sorted result set compared with TF's (full set parity). |
| `oracle_parity.py` | Public-API value checks against TF: plan (TF samples up to 200 nodes per node type), eval (any engine), compare (per API area, every failing check). |
| `aggregate.py` | Builds the JSON, `tables.md`, and the regression check against `cf_0.6.0_record.json`. |

Existing tooling run by the runner: `cargo fmt --check`, `cargo test --features python`,
the `cf_rust_parity` and `cf_rust_validate_corpora` binaries, pytest for
`libs/core/tests/python`, `libs/core/tests/parity` (`CONTEXT_FABRIC_RUN_PARITY=1`),
`libs/benchmarks/tests`, `libs/mcp/tests`, repo `tests/`, `tests/golden/compare.py`
(CF Rust in 3 modes, and CF Python 0.5.8 in python mode),
`cfabric_benchmarks.perf_gate` (TF and CF), `compare_bhsa_mapped_wheel_overhead.py`,
`compare_bhsa_tf_mapped_subset.py`, `python_bhsa_mapped_subset.py` on CF Python 0.5.8,
`bench_scalar_latency.py`, `bench_batch_apis.py`, and `cfabric-bench latency|memory`
on BHSA in both envs.

## Headline (BHSA)

| | TF 13.0.19 | CF Python 0.5.8 | CF Rust 0.6.0rc1 |
|---|---|---|---|
| Cold load (engine cache rebuilt), s | 38.2 | 72.4 | 34.5 |
| Warm load, s | 4.68 | 0.76 | 0.011 |
| RSS after warm load, MB | 6,169 | 290 | 38 |
| RSS / peak RSS after workload, MB | 6,153 / 6,309 | 408 / 559 | 261 / 289 |
| 34-query parity (count + result set) | reference | 34/34 fresh cache, 33/34 reused cache | 34/34 |
| 34-query wall time, s | 12.7 | 11.6 fresh / 27.6 reused | 4.4 (in-process), 4.95 (process per query) |
| Oracle API value checks | reference | 355,975/355,975 fresh, 353,075 reused | 355,973/355,975 |
| `cfabric-bench latency`, 100 curated queries, geomean speedup over TF | – | 1.14x | 7.14x |
| `cfabric-bench memory`, spawn ×4 workers, total RSS MB (TF 25,120) | – | 1,558 | 702 |

## Parity

### CF Rust 0.6.0rc1

| Check | Result |
|---|---|
| Golden probes (TF truth), materialized / mapped / python | bhsa 13/13, n1904 10/10, banks 11/11 in each mode (102/102) |
| 34-query suite (`query_suite.py`) | 34/34 counts, golden tuples present, result sets identical to TF |
| `cf_rust_parity` binary (raw Rust) | 34/34, 4.4 s |
| `tests/parity` pytest (34 queries, 2 order cases, TF oracle on 3 corpora, curated BHSA queries) | 43/43 passed |
| Oracle value checks (`oracle_parity.py`) | bhsa 355,973/355,975; n1904 147,306/147,308; banks 2,368/2,370 |
| `cargo test --features python` | 131 passed, 0 failed, 1 ignored (`bhsa_compile_size_and_timing_probe`, opt-in) |
| `cargo fmt --check` | **fails**: 104 hunks in 15 files (CI's first Rust step would fail) |
| pytest `libs/core/tests/python` | 97 passed, **1 failed**, 2 skipped (quran corpus not present) |
| pytest `libs/mcp/tests` / `libs/benchmarks/tests` | 166/166 / 45/45 |
| pytest repo `tests/` | **3 failed** of 3 |

The oracle counts are not comparable with the June figure (407,719/409,288): that
harness is not in the repository, so `oracle_parity.py` re-implements the check with
its own sampling (2,451 BHSA sample nodes × every node feature, edge, locality, text
format and section API).

Every failing CF Rust check, by API area:

* **Metadata (`C.levels`)**: bhsa, n1904. The slot type's average size is the float
  `1.0` where TF has the int `1` (equal under `==`; differs in type only).
* **Features (`F.otype.freqList`)**: all three corpora. CF Rust returns a frequency
  list; TF's `otype` has no `freqList` (raises `AttributeError`). An addition, not a
  wrong value.
* **Text (`T.text(56, fmt="line-term")`)**: banks. CF Rust returns `", "`, TF returns `""`.
* **pytest `test_text_formats.py::test_languages_match_tf`**: the test's hard-coded
  expected `TF_LANGUAGES` lacks `ua` (Ukrainian), but Text-Fabric 13.0.19 itself
  reports `ua` for BHSA, so CF Rust matches TF and the test's constant is stale.
* **pytest repo `tests/test_windows_paths.py`** (3 tests): imports
  `cfabric.utils.files`, a module of the pure-Python core that the Rust core no
  longer has. Orphaned; not part of CI.
* **`cargo fmt --check`**: formatting drift in `src/bin/cf_rust_parity.rs`,
  `src/bindings/{accessors,fabric,features}.rs`, `src/{compiled,io,mapped_search,mapped_sections,mapped_text,precompute,search}.rs`,
  `tests/{core_port,mapped_locality,mapped_perf,xcorpus_defects}.rs` (rustfmt 1.96).

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
has no attribute 'tolist'` (bhsa `mother` 467, `functional_parent` 1,407,
`distributional_parent` 804; n1904 `parent` 1,277, `subjref` 45); **F** bhsa `dist`
returns `None` for `-1` values (221 nodes) and its `freqList` differs; **T** banks
`T.headingFromNode` (18), `T.top`, `T.structure` return `None` (structure lost). The
golden comparison for CF Python (run on the reused cache) fails the same six banks
structure probes (`banks_top`, `banks_structure_forest`, `banks_sentence_heading`,
`banks_sentence_from_heading`, `banks_sentence_structure_pretty[_full]`).

## Speed and memory

BHSA tables below; N1904 tables are in `rc1_2026-10-14/tables.md`. "Cold" deletes the
engine's cache so the load rebuilds it (`.tf/`, `.cfm/`, `.cfr/`); the operating
system file cache stays warm (flushing it needs `sudo purge`, not available
unattended). USS is `psutil`'s unique set size; on macOS it counts compressed
private memory, which is why CF Python's USS exceeds its RSS.

### BHSA: load and memory

| Metric | TF 13.0.19 | CF Python 0.5.8 | CF Rust (loadAll) | CF Rust (openMapped) |
|---|---|---|---|---|
| Cold load, s (cache rebuilt) | 38.20 | 72.35 | 34.46 | – |
| Warm load, s (median of 3; openMapped: single run) | 4.675 | 0.759 | 0.011 | 0.005 |
| Peak RSS during cold load, MB | 6,159 | 8,652 | 3,014 | – |
| RSS after warm load, MB | 6,169 | 290 | 38 | 38 |
| USS after warm load, MB | 6,173 | 961 | 21 | 21 |
| RSS after workload, MB | 6,153 | 408 | 261 | 260 |
| USS after workload, MB | 6,130 | 990 | 124 | 123 |
| Peak RSS after workload, MB | 6,309 | 559 | 289 | 287 |

### BHSA: steady-state per-call latency (µs, median of 5 passes)

| API | TF 13.0.19 | CF Python 0.5.8 | CF Rust (loadAll) | CF Rust (openMapped) |
|---|---|---|---|---|
| F.sp.v | 0.127 | 0.189 | 0.179 | 0.175 |
| F.g_word.v | 0.134 | 0.222 | 0.206 | 0.194 |
| L.u | 0.170 | 0.879 | 0.252 | 0.237 |
| L.d | 0.725 | 3.455 | 0.346 | 0.318 |
| **L.n** | 0.336 | 1.704 | **21,648** | **20,492** |
| **L.p** | 0.313 | 1.689 | **10,670** | **10,272** |
| **L.i** | 0.037 | 0.045 | **10,538** | **10,382** |
| E.mother.f | 0.037 | 0.382 | 0.081 | 0.082 |
| **E.mother.t** | 0.045 | 0.413 | **519** | **518** |
| **E.mother.b** | 0.069 | error | **523** | **518** |
| T.sectionFromNode | 5.892 | 14.324 | 2.388 | 2.413 |
| T.text (verse) | 9.820 | 32.511 | 5.301 | 4.789 |
| T.nodeFromSection | 0.227 | 0.272 | 0.349 | 0.334 |
| N.walk full pass, s | 0.0201 | 0.1587 | 0.0469 | 0.0444 |

### BHSA: `perf_gate`-style first-call probes and workload queries

| Metric | TF 13.0.19 | CF Python 0.5.8 | CF Rust (loadAll) | CF Rust (openMapped) |
|---|---|---|---|---|
| F.sp.v (n=10000), µs | 0.44 | 0.27 | 0.26 | 0.22 |
| F.g_word.v (n=10000), µs | 0.36 | 0.35 | 0.41 | 0.36 |
| L.u (n=30), µs | 0.77 | 1.53 | 3.27 | 2.56 |
| L.d (n=30), µs | 3.27 | 8.43 | 3.80 | 3.57 |
| E.mother.f (n=500), µs | 0.18 | 0.59 | 0.98 | 0.89 |
| T.sectionFromNode (n=200), µs | 7.21 | 14.84 | 5.23 | 4.77 |
| T.text verse (n=100), µs | 12.68 | 34.91 | 14.34 | 12.63 |
| T.nodeFromSection (n=100), µs | 0.85 | 0.87 | 130.37 | 126.56 |
| F.sp.freqList, s | 0.0323 | 0.0023 | 0.0005 | 0.0006 |
| F.sp.v full sweep, s | 0.0180 | 0.0820 | 0.0500 | 0.0490 |
| q_phrase_without_verb, s | 0.3082 | 0.4642 | 0.0869 | 0.0849 |
| q_regex_gcons, s | 0.1211 | 0.2062 | 0.0333 | 0.0319 |
| q_regex_typ, s | 0.1487 | 0.1498 | 0.0285 | 0.0283 |
| q_regex_gloss_space, s | 0.0041 | 0.0053 | 0.0038 | 0.0037 |
| q_feat_exists_qere, s | 0.0678 | 0.0701 | 0.0003 | 0.0003 |
| q_feat_empty_qere, s | 0.0925 | 0.1103 | 0.0252 | 0.0241 |
| q_multivalue_neg, s | 0.2361 | 0.1169 | 0.0213 | 0.0208 |
| q_slot_equal, s | 0.8255 | 0.3322 | 0.0284 | 0.0279 |

### 34-query suite (BHSA, seconds)

CF Rust is faster than TF on 29 of 34 queries. It is slower on `q_mother_basic`
(0.066 vs 0.040), `q_mother_reverse` (0.065 vs 0.041), `q_mother_named_distinct_verse`
(0.088 vs 0.077), `q_slot_overlap_parent` (0.705 vs 0.451) and ties
`q_regex_gloss_space` (0.004). Per-query times for every engine are in
`rc1_2026-10-14/tables.md`.

### Other existing benchmarks (BHSA)

| Tool | Result |
|---|---|
| `cfabric-bench latency` (100 curated queries, 3 iterations) | CF Rust: 3.0 s vs TF 16.2 s total, geomean 7.14x, slower on 0 queries. CF Python 0.5.8: 14.7 s vs 15.5 s, geomean 1.14x, slower on 28 queries |
| `cfabric-bench memory` single / spawn ×4 / fork ×4, RSS MB | TF 6,275 / 25,120 / 6,423; CF Python 388 / 1,558 / 524; CF Rust 148 / 702 / 310 |
| `compare_bhsa_tf_mapped_subset.py` (104 queries, limit 5) | 9.95x geomean over TF, 104/104 wins |
| `compare_bhsa_mapped_wheel_overhead.py` | wheel vs raw Rust query geomean 1.125x (gate 1.3x); load 0.16 ms vs 0.07 ms |
| 104 mapped-subset queries, total ms | CF Python 0.5.8 12,226; CF Rust raw binary 1,826 |
| `bench_scalar_latency.py` | F.v 0.17-0.21 µs (pyo3 call floor 0.023 µs); its "FASTER than TF" verdict compares against the June TF gate numbers, not today's steady-state TF (0.13 µs) |
| `bench_batch_apis.py` (100k nodes) | `F.sp.vs` 10.1x over the per-call loop; `L.u_many` 1.3x, `L.d_many` 1.2x, `F.number.vs` 2.7x; `T.text_many` is **slower** than the per-call loop (0.4x words, 0.5x verses). The CHANGELOG's "~10x faster than per-node loops" holds only for `F.<feat>.vs` |
| `perf_gate --assert` | 2 target violations (non-strict): `L.u` 144 µs/call first-call (target 10), `rss_final_mb` 230 (target 80) |

## Findings for the release

1. **`L.n`, `L.p`, `L.i` take 10-22 ms per call** on BHSA (≈60,000x TF); 3-7 ms on
   N1904. Cost scales with the slot count (N1904/BHSA time ratio 0.30 ≈ slot ratio
   0.32), which points at `MappedSections::max_slot()` in `src/mapped_sections.rs`:
   it calls `self.otype.s(slot_type)` and so builds the full list of slot nodes
   (426,590 on BHSA) on every call; `L.n` calls it twice, `L.p`/`L.i` once. Not
   fixed (library source is out of scope here).
2. **`E.<edge>.t` and `E.<edge>.b` take 0.5 ms (mother) to ~5 ms (`omap@*`) per call**:
   `t()` in `src/compiled.rs` scans every forward edge row for each lookup (no
   reverse index); `b()` builds on it.
3. Steady-state `F.v`, `L.u`, `E.f`, `T.nodeFromSection` and `N.walk` are 1.3-2.5x
   slower than TF on this machine (all sub-microsecond except `N.walk`). The June
   claim "beats TF on every probe" compared CF steady-state numbers with TF's
   first-call numbers.
4. Four of the 34 parity queries run slower than TF (listed above), contrary to
   the June "all 34 ≤ TF wall time".
5. `cargo fmt --check` fails, so the CI Rust job fails before tests.
6. Parity is otherwise intact: 34/34 queries with identical result sets, 102/102
   golden probes, 2 oracle mismatches per corpus (type/addition-only, plus one
   banks `line-term` text value).

## Regressions against the June record

Compared with `cf_0.6.0_record.json` (threshold: 1.25x slower and above a noise
floor), the only flag is `perf_gate`'s **peak RSS after load: 47.3 MB vs 30.3 MB**
(1.56x; `perf_probe` measures 37.8 MB for the same load). Final RSS (230-287 MB vs
236 MB), load time, every per-call gate probe, every steady-state probe in the
record and all eight gate queries are at or below their June values.

Caveat: TF itself is 1.3-2x faster on this machine than in the June TF baseline
(`bhsa_tf.json`: load 8.58 s → 4.28 s, `T.text` 21.0 → 12.0 µs), except
`q_slot_equal` (0.137 → 0.727 s). CF Rust's ratio to TF improved on every gate
probe except `T.text` (verse): 0.89x TF in June, 0.96x TF now.

## Tooling fixes made for this run

* `libs/benchmarks/cfabric_benchmarks/perf_gate.py`: read the query templates from
  `libs/core/tests/parity/parity_queries.json`. The file moved there from
  `.claude/parity_queries.json` (commit `d8847e9`), and `perf_gate` had silently
  fallen back to different inline templates for three queries
  (`q_phrase_without_verb`, `q_regex_typ`, `q_slot_equal`), making its query times
  incomparable with the June record.
* `libs/core/tests/python/test_text_surface.py`: the BHSA `tf` path was hard-coded
  to the pre-reorganisation checkout (`/Users/cody/github/context-fabric/libs/...`),
  so three BHSA tests were always skipped; it now derives the path from the
  repository root (all three pass).
* The core test suites resolve corpora from `CFABRIC_CORPORA` and an optional
  explicit BHSA cache from `CFABRIC_BHSA_CFR` (`libs/core/tests/conftest.py`);
  `bench_scalar_latency.py` and `bench_batch_apis.py` take `--tf` / `--cache`. The
  runner points all of them at its scratch corpora and compiled cache.
* Not changed, worked around in the runner: `compare_bhsa_mapped_subset.py` and `compare_bhsa_mapped_memory.py`
  put `libs/core` on `PYTHONPATH`, which now resolves to the Rust overlay instead of
  the pure-Python core, so their "Python" side is run directly on CF Python 0.5.8.

## Notes

* Text-Fabric 13.0.19 is installed from `git+https://github.com/annotation/text-fabric@v13.0.19`
  (PyPI only offers 13.1.x now). The existing `libs/core/.venv` of the main
  checkout has 13.1.0 and was not used.
* The golden comparison and `tests/parity` read the corpora from
  `libs/benchmarks/.corpora` (a symlink into the main checkout) and write engine
  caches there (`.cfr/3`, `.cfm`); everything else uses the scratch copy in
  `/tmp/cf_rc1/corpora`.
* The final run was interrupted once during the oracle step and resumed from that
  step into the same results directory (`steps.jsonl` keeps the full history);
  the `queries` and `oracle` steps were then re-run to add the fresh-cache CF
  Python measurements.
