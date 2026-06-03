# cf-rust Notes

## 2026-06-03

- Rust was not installed on the machine. Installed Homebrew `rust` package, which provides `rustc` and `cargo`.
- Created `libs/cf-rust` with Cargo package name `cf_rust`.
- First implementation slice targets direct `.tf` loading and a basic search subset before porting the full Python API surface.
- BHSA corpus is available at `libs/benchmarks/.corpora/bhsa/tf`.
- Python parser behavior to preserve:
  - Metadata begins with `@node`, `@edge`, or `@config`, followed by `@key=value` lines, then a blank line.
  - Node feature lines can be implicit values or explicit `node-spec<TAB>value`.
  - Node specs include ranges such as `1-426590`.
  - Edge feature lines map `node-spec<TAB>node-spec`, with optional edge values when `@edgeValues` is present.
  - `otype` defines slot type from node 1 and max slot as the last contiguous node with that slot type.
  - `oslots` maps non-slot nodes to contained slot ranges.

## After mapped search candidate-filter integration

Mapped no-seed atom planning now uses the mapped node feature candidate-filter
surface for full candidate-list construction. Limited direct searches keep the
streaming `otype` scan path so generic/type-only queries can stop as soon as
the requested result count is available.

Validation:

```sh
cargo fmt
cargo test --quiet mapped_search_runs_simple_string_pool_queries
cargo test --quiet
cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
python scripts/compare_bhsa_mapped_subset.py --limit 5
```

Results:

```text
cargo test --quiet: 69 passed
loaded mapped_features=13 compile_ms=cached load_ms=10.915 nodes=1446831
all 104 mapped curated queries passed
generic_001 elapsed_ms=6.975
struct_030 elapsed_ms=135.010
quant_001 elapsed_ms=27.276
complex_001 elapsed_ms=104.963
load_speedup=24.95x python_ms=398.717 rust_ms=15.983
query_geomean_speedup=10.20x queries=104
slowest relative Rust query: complex_017 speedup=1.17x python_ms=84.445 rust_ms=72.360
```

## After Python utility helper parity

Converted another deterministic Python unit-test slice from
`libs/core/tests/unit/utils/test_helpers.py`. Added `src/utils.rs` with typed,
deterministic Rust equivalents for:

- `setFromSpec`
- `rangesFromSet`
- `rangesFromList`
- `specFromRanges`
- `specFromRangesLogical`
- `makeIndex`
- `makeInverse`
- `makeInverseVal`

The Rust API uses `BTreeSet` and `BTreeMap` for stable ordering and introduces
`CfError::InvalidSpec` for malformed range specifications.

Validation:

```sh
cargo fmt
cargo test --quiet range_spec_helpers_match_python_utility_behaviors
cargo test --quiet inverse_mapping_helpers_match_python_utility_behaviors
cargo test --quiet
cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
```

Results:

```text
cargo test --quiet: 71 passed
benchmark corpora validator: sp, tischendorf, bhsa, cuc, n1904, dss, quran, peshitta, syrnt, and lxx all loaded and returned 5 query results
BHSA mapped curated validator: loaded mapped_features=13 compile_ms=cached load_ms=10.327 nodes=1446831
all 104 mapped curated queries passed
generic_001 elapsed_ms=6.889
struct_030 elapsed_ms=139.803
quant_001 elapsed_ms=27.447
complex_001 elapsed_ms=103.341
```

## After Python-style search feature continuation lines

Ported a public Python search template behavior where explicit feature
constraint lines following an atom extend the previous atom. The Rust
materialized and mapped parsers now treat lines such as:

```text
word
word=hello
pos=interjection
```

as equivalent to:

```text
word word=hello pos=interjection
```

The continuation rule is intentionally limited to explicit feature constraint
syntax (`=`, `#`, `~`, `<`, `>`, trailing `*`, trailing `#`) so normal atom
lines and named-atom constraint lines remain unambiguous.

Validation:

```sh
cargo fmt
cargo test --quiet supports_feature_constraints_regex_limits_and_containment
cargo test --quiet mapped_search_runs_simple_string_pool_queries
cargo test --quiet
cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
```

Results:

```text
cargo test --quiet: 73 passed
benchmark corpora validator: sp, tischendorf, bhsa, cuc, n1904, dss, quran, peshitta, syrnt, and lxx all loaded and returned 5 query results
BHSA mapped curated validator: loaded mapped_features=13 compile_ms=cached load_ms=10.400 nodes=1446831
all 104 mapped curated queries passed
generic_001 elapsed_ms=7.017
struct_030 elapsed_ms=142.593
quant_001 elapsed_ms=26.930
complex_001 elapsed_ms=104.179
```

## After Python-style named atom reference parity

Verified Python's public search behavior for a later atom whose type token
matches an existing query name:

```python
list(S.search("w:word word=hello\nw"))
# [(1, 1)]
```

Python treats the second `w` as a new result atom equality-bound to the named
`w`, not as a mutation of the original atom's constraints. Rust previously used
named lines as constraint extensions and returned shorter result tuples in some
cases. Materialized and mapped search now create an equality-bound reference atom
with its own result column, including when the reference line has constraints:

```text
w1:word
w2:word
w1 < w2
w1 word=hello
w2 word=world
```

now yields `[[1, 3, 1, 3]]`, matching Python's result shape.

Validation:

```sh
cargo fmt
cargo test --quiet
cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
```

Results:

```text
cargo test --quiet: 73 passed
benchmark corpora validator: sp, tischendorf, bhsa, cuc, n1904, dss, quran, peshitta, syrnt, and lxx all loaded and returned 5 query results
BHSA mapped curated validator: loaded mapped_features=13 compile_ms=cached load_ms=10.405 nodes=1446831
all 104 mapped curated queries passed
generic_001 elapsed_ms=7.131
struct_030 elapsed_ms=136.256
quant_001 elapsed_ms=27.180
complex_001 elapsed_ms=105.848
```

## After quantified parent containment reference parity

Verified Python's search behavior for quantified parent references against the
mini corpus:

```python
list(S.search("p:phrase\n/with/\n  .. [[ w\n  w:word word=hello\n/-/"))
# [(6,)]

list(S.search("p:phrase\n/with/\n  w:word word=hello\n  w ]] ..\n/-/"))
# [(6,)]
```

The Rust quantified engines already check that top-level alternative matches
are contained in the quantified root. For these two Python-supported parent
containment relation forms, `.. [[ child` and `child ]] ..`, the relation is
therefore redundant and can be normalized out of the alternative template.
Materialized and mapped search now perform that normalization before parsing
quantifier alternatives. Bare `..` as an atom remains unsupported, matching the
checked Python behavior where that form reports a lonely-relation error and
returns no results.

Validation:

```sh
cargo fmt
cargo test --quiet supports_with_and_without_quantified_blocks
cargo test --quiet mapped_search_runs_simple_string_pool_queries
cargo test --quiet
cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
```

Results:

```text
cargo test --quiet: 73 passed
benchmark corpora validator: sp, tischendorf, bhsa, cuc, n1904, dss, quran, peshitta, syrnt, and lxx all loaded and returned 5 query results
BHSA mapped curated validator: loaded mapped_features=13 compile_ms=cached load_ms=10.472 nodes=1446831
all 104 mapped curated queries passed
generic_001 elapsed_ms=6.805
struct_030 elapsed_ms=136.616
quant_001 elapsed_ms=27.564
complex_001 elapsed_ms=104.565
```

### Implementation Decisions

- Use direct Text-Fabric parsing first instead of a compiled cache format so tests can prove corpus compatibility.
- Store node values in hash maps and maintain feature-value indexes for fast equality queries.
- Use shared string values (`Arc<str>`) to reduce duplication when expanding Text-Fabric ranges.
- Treat `oslots` as a required core feature. Even lexical benchmark paths must include it because it is critical to the corpus model.
- Preserve the long-term Context-Fabric architecture goal of compiled memory-mapped files and memory-efficient loading. The current in-memory `.tf` loader is a compatibility and test foundation, not the final storage design.
- Added a compiled cache format with per-feature string pools. This is a storage stepping stone: compiled cache bytes are now read through a memory map, but the loader still materializes `Corpus` maps after parsing. The next storage step is mmap-backed feature views that avoid rebuilding node and edge maps.

### Verification

- `cargo test` passes.
- Current Rust test count: 12 tests.
- Newly converted behavior covers node feature lookup/selection/frequency, edge forward/backward/bidirectional traversal, node ordering, and locality navigation.
- Search now supports the first quantified block subset: `/with/` and `/without/` blocks that filter a root result by existence or absence of contained subpatterns.
- Quantified block containment was corrected for blocks with multiple top-level atoms: each top-level quantified atom must be contained in the quantified root.
- Equality constraints now match integer-valued features from textual query literals, e.g. `phrase_id=1`.
- Representative BHSA curated query shapes now covered in tests:
  - lexical: `word sp=verb vt=perf`
  - structural indentation: `phrase typ=VP` containing `word sp=verb`
  - quantified: `phrase typ=NP /with/ word sp=art /-/`
- `cf_rust_validate_bhsa_curated` validates 16 representative curated BHSA templates across lexical, structural, quantified, and complex categories.
- Added per-search candidate caching and single-parse quantified blocks. This reduces repeated feature scans during nested and quantified search.
- Ported another Python integration-search slice: feature value alternatives (`pos=noun|adjective`), feature inequality (`pos#noun`), missing and present feature checks (`pos#`, `pos*`), numeric comparisons (`number<3`, `number>2`), named atom constraint lines (`w1 word=hello`), and basic named-node relations (`w1 < w2`, `w1 > w2`, `w1 # w2`).
- Ported the next Python integration-search relation slice: embedding relations (`p [[ w`, `w ]] p`), immediate adjacency (`w1 <: w2`), and edge traversal relations (`w -parent> p`, `p <parent- w`) against the mini corpus `parent` edge feature.
- Expanded quantified search support beyond `/with/` and `/without/` to include the Python integration forms `/where/`, `/have/`, and `/or/`. `/where/` behaves as a positive containment block, `/have/` starts another required positive block, and `/or/` adds alternatives within the current block.
- Relation execution now prunes partial rows as soon as both named nodes in a relation are bound. This keeps the same result semantics while avoiding some full-row expansion work for relation-heavy and quantified-contained queries.
- Relation execution still needs a fuller planner for larger multi-candidate workloads. Current pruning is local and incremental, not a complete join-order optimizer.
- Canonical ordering now ports Python `precompute.prepare.order` comparator semantics:
  - slot sets are compared first,
  - embedders sort before embedded nodes when one slot set contains the other,
  - equal slot sets use node-type rank, then node id,
  - overlapping non-containing slot sets use the smallest differing slot.
  This replaces the earlier first-slot approximation. Compiled caches now persist rank/order arrays for efficient repeated `sort_key()` and sorted subset operations; direct `.tf` loads still compute ranks lazily.
- Compiled cache rank/order behavior is covered by the mini-corpus compiled parity test.
- Compiled cache parity test passes for mini corpus equality and containment queries.
- Compiled cache loading now uses `memmap2` for the source bytes instead of `std::fs::read`, eliminating the extra full-cache heap copy before parsing. This preserves the current API while moving toward the memory-mapped storage goal.
- Added `inspect_compiled()` and `cf_rust_inspect_compiled`, a mmap-backed metadata inspection path that scans the compiled cache layout without materializing a `Corpus`. It reports feature names, encodings, row counts, string pool counts, payload byte ranges, and order/rank lengths. This creates a concrete stepping stone toward mmap-backed feature views.
- Added `MappedCompiledCorpus` and `StringPoolNodeFeatureView` for zero-copy reads of string-pool node features directly from mapped cache bytes. The view binary-searches sorted `(node, pool_id)` rows and borrows string values from the mmap. This currently covers string-pool node features such as BHSA `otype` and `sp`; mixed node features and edge features still need mapped views.
- Added `EdgeFeatureView` for mmap-backed edge target lookup, including critical `oslots` reads. The view iterates target values directly from mapped cache bytes without materializing an `EdgeFeature`.
- Improved `EdgeFeatureView` with a compact in-memory row-offset index. This avoids repeated scans over variable-length edge rows while still keeping target vectors zero-copy over mmap bytes. For BHSA `oslots`, this is about one offset per non-slot node rather than materializing every slot target. A future compiled-format improvement can persist this row-offset table in the cache itself.
- Added `cf_rust_bhsa_mapped`, the first mapped-view query path. It opens mapped `otype`, `sp`, and `oslots`, scans string-pool rows directly from mmap bytes, and answers the BHSA `word sp=verb` benchmark without materializing `Corpus`, `NodeFeature`, or `EdgeFeature` maps.
- Added `MappedSearch`, a reusable mapped-search API for one-line type queries and string-pool equality constraints. It always opens mapped `oslots`, supports named atom prefixes, returns result rows, and currently covers queries such as `word` and `word sp=verb` without materializing the full corpus.
- Expanded `MappedSearch` to support one indented containment child, e.g. `phrase\n  word pos=noun`, using mapped `oslots` targets. This starts bridging mapped search from lexical-only queries toward structural Text-Fabric search.
- BHSA selective integration test loads `otype`, `oslots`, and `sp` and confirms:
  - `word` node count: 426,590
  - `book` node count: 39
  - `word sp=verb` can be queried without failure
- Benchmark corpora validation command passes:

```sh
cd libs/cf-rust
cargo run --release --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
```

Result:

```text
ok corpus=sp load_ms=128.917 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=25.557 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=533.377 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=3.740 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=157.385 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=554.630 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=46.016 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.966 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.250 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.711 nodes=685732 slots=623693 query=word results=5
```

### Performance Measurements

Measured on 2026-06-03.

Valid Rust BHSA path with mandatory `oslots`:

```sh
cd libs/cf-rust
cargo run --release --bin cf_rust_bhsa -- ../benchmarks/.corpora/bhsa/tf
```

Result:

```text
load_ms=666.298 query_ms=6.850 nodes=1446831 words=426590 verbs=73710
```

Current direct `.tf` measurement:

```text
load_ms=690.004 query_ms=6.608 nodes=1446831 words=426590 verbs=73710
```

Rust compiled-cache BHSA path with mandatory `oslots`:

```sh
cd libs/cf-rust
cargo run --release --bin cf_rust_bhsa_compiled -- ../benchmarks/.corpora/bhsa/tf target/bhsa-core.cfr
```

Result after cache exists:

```text
compile_ms=cached load_ms=378.724 query_ms=5.901 nodes=1446831 words=426590 verbs=73710
```

Latest repeated result after feature/navigation additions:

```text
compile_ms=cached load_ms=435.801 query_ms=6.059 nodes=1446831 words=426590 verbs=73710
```

Latest repeated result after canonical comparator port:

```text
compile_ms=cached load_ms=416.763 query_ms=6.908 nodes=1446831 words=426590 verbs=73710
```

Latest result after adding compiled rank/order arrays:

```text
compile_ms=10989.434 load_ms=420.026 query_ms=5.829 nodes=1446831 words=426590 verbs=73710
compile_ms=cached load_ms=447.265 query_ms=6.809 nodes=1446831 words=426590 verbs=73710
```

Latest result after quantified-query support:

```text
compile_ms=cached load_ms=483.168 query_ms=8.517 nodes=1446831 words=426590 verbs=73710
```

Latest result after numeric equality and multi-root quantifier fix:

```text
compile_ms=cached load_ms=454.341 query_ms=7.055 nodes=1446831 words=426590 verbs=73710
```

Curated BHSA validator:

```sh
cargo run --release --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
```

Result summary:

```text
loaded features=13 load_ms=1953.052
16 representative curated queries passed
slowest observed query: complex_001 elapsed_ms=2679.217
```

After per-search candidate caching:

```text
loaded features=13 load_ms=1945.123
16 representative curated queries passed
slowest observed query: complex_001 elapsed_ms=1036.850
```

After porting additional feature constraint and basic relation syntax:

```text
cargo test --quiet
12 tests passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=154.761 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.772 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=627.468 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=3.706 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=200.726 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=647.473 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=54.268 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=74.368 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=20.722 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=115.611 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2239.183
16 representative curated queries passed
slowest observed query: complex_001 elapsed_ms=1194.769
```

After porting embedding, adjacency, and edge traversal relations:

```text
cargo test --quiet
12 tests passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=169.534 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.148 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=576.605 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.691 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=161.425 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=577.220 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.450 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.974 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.703 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.637 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2026.325
16 representative curated queries passed
slowest observed query: complex_001 elapsed_ms=955.624
```

After porting `/where/`, `/have/`, and `/or/` quantified forms:

```text
cargo test --quiet
12 tests passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=127.149 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=25.736 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=580.849 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.360 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=173.795 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=571.943 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=48.057 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.562 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.896 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.041 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2018.802
16 representative curated queries passed
slowest observed query: complex_001 elapsed_ms=983.344
```

After incremental relation pruning:

```text
cargo test --quiet
12 tests passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=123.200 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.731 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=561.611 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=3.181 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=161.478 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=568.786 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=48.658 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.515 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.209 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=103.880 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1931.261
16 representative curated queries passed
slowest observed query: complex_001 elapsed_ms=840.865
```

After switching compiled-cache byte access to `memmap2`:

```text
cargo test --quiet
12 tests passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=130.323 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=24.868 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=532.025 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=3.291 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=161.021 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=547.684 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=47.334 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.830 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.308 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.432 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1873.982
16 representative curated queries passed
slowest observed query: complex_001 elapsed_ms=874.489
```

After adding compiled metadata inspection:

```text
cargo test --quiet
13 tests passed

cargo run --release --quiet --bin cf_rust_inspect_compiled -- target/bhsa-core.cfr
bytes=54215363 node_features=2 edge_features=1 order_len=1446831 rank_len=1446831
node name=otype encoding=StringPool rows=1446831 pool=Some(13) payload=22..11574827
node name=sp encoding=StringPool rows=435820 pool=Some(14) payload=11574834..15061513
edge name=oslots rows=1020241 payload=15061527..42640707

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=125.483 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=25.254 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=615.821 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=3.092 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=202.384 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=589.619 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=48.378 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.622 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.504 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.919 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2017.223
16 representative curated queries passed
slowest observed query: complex_001 elapsed_ms=1663.267
```

After adding mmap-backed string-pool node feature views:

```text
cargo test --quiet
14 tests passed

cargo run --release --quiet --bin cf_rust_inspect_compiled -- target/bhsa-core.cfr otype 1 426590 426591
value feature=otype node=1 value=Some("word")
value feature=otype node=426590 value=Some("word")
value feature=otype node=426591 value=Some("book")

cargo run --release --quiet --bin cf_rust_inspect_compiled -- target/bhsa-core.cfr sp 1 3 143756
value feature=sp node=1 value=Some("prep")
value feature=sp node=3 value=Some("verb")
value feature=sp node=143756 value=Some("subs")

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=130.007 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.008 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=581.822 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.421 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=166.937 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=552.111 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=50.211 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=75.050 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.771 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=115.563 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2003.698
16 representative curated queries passed
slowest observed query: complex_001 elapsed_ms=1396.238
```

After adding mmap-backed edge feature target views:

```text
cargo test --quiet
15 tests passed

cargo run --release --quiet --bin cf_rust_inspect_compiled -- target/bhsa-core.cfr oslots 426591 426592 1446831
targets feature=oslots node=426591 count=28764 first=Some(1) last=Some(28764)
targets feature=oslots node=426592 count=23748 first=Some(28765) last=Some(52512)
targets feature=oslots node=1446831 count=1 first=Some(426421) last=Some(426421)

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=143.586 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=33.585 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=575.127 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.130 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=178.291 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=564.274 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.637 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.418 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.477 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.789 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2043.907
16 representative curated queries passed
slowest observed query: complex_001 elapsed_ms=948.388
```

After adding an in-memory row-offset index to mmap-backed edge views:

```text
cargo test --quiet
15 tests passed

cargo run --release --quiet --bin cf_rust_inspect_compiled -- target/bhsa-core.cfr oslots 426591 426592 1446831
targets feature=oslots node=426591 count=28764 first=Some(1) last=Some(28764)
targets feature=oslots node=426592 count=23748 first=Some(28765) last=Some(52512)
targets feature=oslots node=1446831 count=1 first=Some(426421) last=Some(426421)

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=130.981 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.775 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=562.234 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.929 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=168.756 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=571.416 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.708 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.559 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.079 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=103.587 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1992.623
16 representative curated queries passed
slowest observed query: complex_001 elapsed_ms=1429.317
```

After adding the mapped-view BHSA lexical query path:

```text
cargo test --quiet
15 tests passed

cargo run --release --quiet --bin cf_rust_bhsa_mapped -- ../benchmarks/.corpora/bhsa/tf target/bhsa-core.cfr
compile_ms=cached load_ms=19.356 query_ms=24.343 nodes=1446831 words=426590 verbs=73710 oslots_rows=1020241

cargo run --release --quiet --bin cf_rust_bhsa_compiled -- ../benchmarks/.corpora/bhsa/tf target/bhsa-core.cfr
compile_ms=cached load_ms=402.953 query_ms=6.485 nodes=1446831 words=426590 verbs=73710

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=128.490 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.659 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=676.380 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=3.032 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=181.190 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=592.433 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=50.437 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.664 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=21.311 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.030 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2076.035
16 representative curated queries passed
slowest observed query: complex_001 elapsed_ms=1051.742
```

After adding reusable `MappedSearch`:

```text
cargo test --quiet
16 tests passed

cargo run --release --quiet --bin cf_rust_bhsa_mapped -- ../benchmarks/.corpora/bhsa/tf target/bhsa-core.cfr
compile_ms=cached load_ms=17.796 query_ms=55.232 nodes=1446831 words=426590 verbs=73710 oslots_rows=1020241

cargo run --release --quiet --bin cf_rust_bhsa_compiled -- ../benchmarks/.corpora/bhsa/tf target/bhsa-core.cfr
compile_ms=cached load_ms=455.814 query_ms=6.573 nodes=1446831 words=426590 verbs=73710

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=131.429 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.430 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=563.836 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=3.746 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=169.697 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=567.249 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=50.097 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.094 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.869 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=105.476 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2008.844
16 representative curated queries passed
slowest observed query: complex_001 elapsed_ms=959.225
```

After adding mapped containment support:

```text
cargo test --quiet
16 tests passed

cargo run --release --quiet --bin cf_rust_bhsa_mapped -- ../benchmarks/.corpora/bhsa/tf target/bhsa-core.cfr
compile_ms=cached load_ms=17.147 query_ms=48.616 nodes=1446831 words=426590 verbs=73710 oslots_rows=1020241

cargo run --release --quiet --bin cf_rust_bhsa_compiled -- ../benchmarks/.corpora/bhsa/tf target/bhsa-core.cfr
compile_ms=cached load_ms=411.206 query_ms=6.837 nodes=1446831 words=426590 verbs=73710

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=136.666 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=25.977 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=559.021 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=3.349 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=165.260 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=586.207 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.290 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.150 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.595 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.979 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2031.708
16 representative curated queries passed
slowest observed query: complex_001 elapsed_ms=960.325
```

Latest standard BHSA compiled-cache benchmark:

```text
compile_ms=cached load_ms=466.496 query_ms=7.441 nodes=1446831 words=426590 verbs=73710
```

Current standard BHSA compiled-cache benchmark after additional search syntax:

```text
compile_ms=cached load_ms=466.977 query_ms=7.328 nodes=1446831 words=426590 verbs=73710
```

Current standard BHSA compiled-cache benchmark after relation syntax expansion:

```text
compile_ms=cached load_ms=467.130 query_ms=6.688 nodes=1446831 words=426590 verbs=73710
```

Current standard BHSA compiled-cache benchmark after quantifier expansion:

```text
compile_ms=cached load_ms=440.245 query_ms=6.675 nodes=1446831 words=426590 verbs=73710
```

Current standard BHSA compiled-cache benchmark after incremental relation pruning:

```text
compile_ms=cached load_ms=418.645 query_ms=6.235 nodes=1446831 words=426590 verbs=73710
```

Current standard BHSA compiled-cache benchmark after mmap-backed byte access:

```text
compile_ms=cached load_ms=469.443 query_ms=6.565 nodes=1446831 words=426590 verbs=73710
```

Current standard BHSA compiled-cache benchmark after compiled metadata inspection:

```text
compile_ms=cached load_ms=529.232 query_ms=6.269 nodes=1446831 words=426590 verbs=73710
```

Current standard BHSA compiled-cache benchmark after mmap-backed string-pool views:

```text
compile_ms=cached load_ms=420.109 query_ms=6.889 nodes=1446831 words=426590 verbs=73710
```

Current standard BHSA compiled-cache benchmark after mmap-backed edge views:

```text
compile_ms=cached load_ms=484.405 query_ms=7.167 nodes=1446831 words=426590 verbs=73710
```

Current standard BHSA compiled-cache benchmark after edge row-offset index:

```text
compile_ms=cached load_ms=455.566 query_ms=7.098 nodes=1446831 words=426590 verbs=73710
```

Current mapped-view BHSA lexical benchmark:

```text
compile_ms=cached load_ms=19.356 query_ms=24.343 nodes=1446831 words=426590 verbs=73710 oslots_rows=1020241
```

Current mapped-search BHSA lexical benchmark:

```text
compile_ms=cached load_ms=17.796 query_ms=55.232 nodes=1446831 words=426590 verbs=73710 oslots_rows=1020241
```

Current mapped-search BHSA lexical benchmark after containment support:

```text
compile_ms=cached load_ms=17.147 query_ms=48.616 nodes=1446831 words=426590 verbs=73710 oslots_rows=1020241
```

Latest Python same-feature measurement:

```text
load_ms=835.860 query_ms=168.642 verbs=73710
```

Current Python same-feature measurement after additional search syntax:

```text
load_ms=812.558 query_ms=179.156 verbs=73710
```

Current Python same-feature measurement after relation syntax expansion:

```text
load_ms=850.341 query_ms=173.323 verbs=73710
```

Current Python same-feature measurement after quantifier expansion:

```text
load_ms=571.490 query_ms=159.135 verbs=73710
```

Current Python same-feature measurement after incremental relation pruning:

```text
load_ms=738.800 query_ms=159.599 verbs=73710
```

Current Python same-feature measurement after mmap-backed byte access:

```text
load_ms=461.654 query_ms=153.712 verbs=73710
```

Current Python same-feature measurement after compiled metadata inspection:

```text
load_ms=764.782 query_ms=172.894 verbs=73710
```

Current Python same-feature measurement after mmap-backed string-pool views:

```text
load_ms=591.051 query_ms=152.622 verbs=73710
```

Current Python same-feature measurement after mmap-backed edge views:

```text
load_ms=733.149 query_ms=164.156 verbs=73710
```

Current Python same-feature measurement after edge row-offset index:

```text
load_ms=714.997 query_ms=161.928 verbs=73710
```

Current Python same-feature measurement after mapped-view query path:

```text
load_ms=605.645 query_ms=179.454 verbs=73710
```

Current Python same-feature measurement after reusable mapped search:

```text
load_ms=798.326 query_ms=166.448 verbs=73710
```

Current Python same-feature measurement after mapped containment support:

```text
load_ms=442.971 query_ms=166.902 verbs=73710
```

Direct `.tf` Rust measurement from the same run:

```text
load_ms=796.181 query_ms=7.452 nodes=1446831 words=426590 verbs=73710
```

Python same-feature measurement from the same run:

```text
load_ms=999.149 query_ms=264.829 verbs=73710
```

Latest Python same-feature measurement:

```text
load_ms=426.522 query_ms=156.052 verbs=73710
```

Compiled BHSA cache size after adding rank/order:

```text
54,215,363 bytes
```

Cache size:

```text
42,640,707 bytes
```

Python same-feature path with mandatory `oslots`:

```sh
PYTHONPATH=/Users/cody/github/context-fabric/libs/core python - <<'PY'
from pathlib import Path
from time import perf_counter
from cfabric.core.fabric import Fabric
path = Path('/Users/cody/github/context-fabric/libs/benchmarks/.corpora/bhsa/tf')
t0=perf_counter()
api=Fabric(locations=str(path), silent='deep').load('otype oslots sp', silent='deep')
t1=perf_counter()
res=list(api.S.search('word sp=verb', silent='deep'))
t2=perf_counter()
print(f'load_ms={(t1-t0)*1000:.3f} query_ms={(t2-t1)*1000:.3f} verbs={len(res)}')
PY
```

Result:

```text
load_ms=598.357 query_ms=255.308 verbs=73710
```

Current Python same-feature measurement:

```text
load_ms=417.641 query_ms=147.840 verbs=73710
```

Current valid comparison:

- Rust compiled-cache total: 378.724 + 5.901 = 384.625 ms
- Python total: 417.641 + 147.840 = 565.481 ms
- Rust uses about 32% less end-to-end time for this mandatory-`oslots` BHSA lexical query path.

Correction: the earlier lexical-only measurement without `oslots` is not an acceptable success criterion because `oslots` is critical and must always be loaded.

## After mapped equality alternatives and multiple constraints

Extended the memory-mapped search path to parse equality alternatives such as
`pos=noun|adjective` and to apply all equality constraints on a mapped atom.
This keeps the current mapped path narrow, but aligns its supported equality
semantics with the materialized search path for simple atom and one-level
containment queries.

Added mapped tests for:

- `word pos=noun|adjective`
- `word word=hello pos=interjection`
- `phrase` containing `word pos=noun|adjective`

Validation:

```text
cargo test --quiet
16 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=139.842 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.819 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=580.348 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.779 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=185.198 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=585.729 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=48.187 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.785 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.376 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=107.691 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2084.654
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=1037.541
```

Current mapped BHSA lexical benchmark:

```text
compile_ms=cached load_ms=17.187 query_ms=49.295 nodes=1446831 words=426590 verbs=73710 oslots_rows=1020241
```

Current Python same-feature measurement with mandatory `otype oslots sp`:

```text
load_ms=383.120 query_ms=147.120 verbs=73710
```

Current valid comparison:

- Rust mapped total: 17.187 + 49.295 = 66.482 ms
- Python total: 383.120 + 147.120 = 530.240 ms
- Rust mapped uses about 87% less end-to-end time for this mandatory-`oslots`
  BHSA lexical query path.

## After mapped rank access and basic relation support

Extended compiled metadata inspection to retain memory-map byte offsets for the
serialized canonical `order` and `rank` vectors. `MappedCompiledCorpus` can now
read a node sort key directly from the mapped cache without materializing the
rank array.

Extended `MappedSearch` to support exactly two named atoms with simple relation
lines:

- `w1 < w2`
- `w1 > w2`
- `w1 # w2`
- `w1 <: w2`

The `<`, `>`, and `#` relation predicates are rank/node based. The `<:`
predicate currently compares adjacent slot node numbers, which matches the
current word-slot use cases and avoids incorrectly treating structural rank
boundaries as slot gaps.

Added mapped mini-corpus tests for before, after, not-equal, adjacency, limits,
and constrained relation atoms.

Validation:

```text
cargo test --quiet
16 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=134.147 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=25.124 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=581.944 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.713 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=170.523 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=572.924 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=47.586 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.448 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.016 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=106.688 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2021.419
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=1034.254
```

Current mapped BHSA lexical benchmark:

```text
compile_ms=cached load_ms=17.128 query_ms=48.997 nodes=1446831 words=426590 verbs=73710 oslots_rows=1020241
```

Current Python same-feature measurement with mandatory `otype oslots sp`:

```text
load_ms=384.098 query_ms=146.923 verbs=73710
```

Current valid comparison:

- Rust mapped total: 17.128 + 48.997 = 66.125 ms
- Python total: 384.098 + 146.923 = 531.021 ms
- Rust mapped uses about 88% less end-to-end time for this mandatory-`oslots`
  BHSA lexical query path.

## After mapped edge relation support

Extended mapped named relation search to support edge traversal relation lines:

- `w -parent> p`
- `p <parent- w`

The relation operator stores the edge feature name and resolves the mapped edge
view from `MappedCompiledCorpus`. Forward and backward checks both scan the
mapped source row for the relevant source node; no full edge map is
materialized. This mirrors the existing materialized mini-corpus relation test
slice while preserving the mapped cache path.

Updated the mapped mini-corpus test cache to include the `parent` edge feature
and added assertions for forward and backward parent traversal.

Validation:

```text
cargo test --quiet
16 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.496 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=24.932 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=557.431 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.740 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=170.731 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=568.739 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=46.501 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.494 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.397 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.542 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1960.197
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=1057.081
```

Current mapped BHSA lexical benchmark:

```text
compile_ms=cached load_ms=17.084 query_ms=49.645 nodes=1446831 words=426590 verbs=73710 oslots_rows=1020241
```

Current Python same-feature measurement with mandatory `otype oslots sp`:

```text
load_ms=379.617 query_ms=148.172 verbs=73710
```

Current valid comparison:

- Rust mapped total: 17.084 + 49.645 = 66.729 ms
- Python total: 379.617 + 148.172 = 527.789 ms
- Rust mapped uses about 87% less end-to-end time for this mandatory-`oslots`
  BHSA lexical query path.

## After mapped structural relation support

Extended mapped named relation search to support structural containment relation
lines:

- `p [[ w`
- `w ]] p`

These predicates use the mandatory mapped `oslots` edge feature directly and do
not materialize a full `Corpus`. The implementation currently scans the mapped
target row for each candidate relation pair, which is acceptable for this
feature slice but should be revisited if mapped structural relation queries
become a primary performance target.

Added mapped mini-corpus tests for:

- phrase contains word via `p [[ w`
- word embedded in phrase via `w ]] p`
- constrained structural relation search with alternatives

Validation:

```text
cargo test --quiet
16 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=127.605 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=24.782 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=589.022 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.970 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=179.798 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=587.997 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.585 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.079 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.084 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.419 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2035.535
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=865.124
```

Current mapped BHSA lexical benchmark:

```text
compile_ms=cached load_ms=16.707 query_ms=48.782 nodes=1446831 words=426590 verbs=73710 oslots_rows=1020241
```

Current Python same-feature measurement with mandatory `otype oslots sp`:

```text
load_ms=387.690 query_ms=145.020 verbs=73710
```

Current valid comparison:

- Rust mapped total: 16.707 + 48.782 = 65.489 ms
- Python total: 387.690 + 145.020 = 532.710 ms
- Rust mapped uses about 88% less end-to-end time for this mandatory-`oslots`
  BHSA lexical query path.

## After mapped non-equality constraint support

Added mapped node-value support for mixed compiled node features. The compiled
cache now exposes `MixedNodeFeatureView`, which binary-searches sorted rows and
returns borrowed string values or integer values directly from the memory map.
Mapped search uses this for integer features such as the mini-corpus `number`
feature without materializing a full `Corpus`.

Extended mapped atom constraints to support:

- equality alternatives: `pos=noun|adjective`
- inequality: `pos#noun`
- missing: `pos#`
- present: `pos*`
- regex: `word~^h`
- numeric comparison: `number<3`, `number>2`

Candidate scans now stream mapped feature rows through a callback instead of
building a temporary row vector, preserving the memory-efficiency requirement.
The generalized matcher is slower than the earlier equality-only mapped path,
but still materially faster end-to-end than the Python same-feature BHSA path.

Added mapped mini-corpus tests mirroring the materialized constraint test slice.

Validation:

```text
cargo test --quiet
16 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=127.230 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=23.219 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=610.175 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.708 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=196.134 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=660.351 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=55.226 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=75.029 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.092 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=115.124 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2214.994
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=1148.698
```

Current mapped BHSA lexical benchmark:

```text
compile_ms=cached load_ms=17.094 query_ms=81.925 nodes=1446831 words=426590 verbs=73710 oslots_rows=1020241
```

Current Python same-feature measurement with mandatory `otype oslots sp`:

```text
load_ms=387.272 query_ms=146.509 verbs=73710
```

Current valid comparison:

- Rust mapped total: 17.094 + 81.925 = 99.019 ms
- Python total: 387.272 + 146.509 = 533.781 ms
- Rust mapped uses about 81% less end-to-end time for this mandatory-`oslots`
  BHSA lexical query path.

## After mapped quantified block support

Extended `MappedSearch` to parse and execute mapped quantified blocks:

- `/with/`
- `/without/`
- `/where/`
- `/have/`
- `/or/`
- `/-/`

The mapped implementation currently supports a single base atom and contained
block alternatives made of simple atoms. Containment uses mapped `oslots` slot
sets, so it works for both phrase-to-word and sentence-to-phrase cases without
materializing a full `Corpus`.

Added mapped mini-corpus tests matching the materialized quantified slice:

- phrase with an interjection word
- phrase without an interjection word
- sentence with two phrase children
- `/where/` plus `/have/`
- `/or/` alternatives

Validation:

```text
cargo test --quiet
16 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=144.347 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.063 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=588.139 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.862 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=171.867 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=570.537 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=47.454 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.492 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.898 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=113.308 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2057.249
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=1094.752
```

Current mapped BHSA lexical benchmark:

```text
compile_ms=cached load_ms=16.546 query_ms=79.895 nodes=1446831 words=426590 verbs=73710 oslots_rows=1020241
```

Current Python same-feature measurement with mandatory `otype oslots sp`:

```text
load_ms=718.795 query_ms=158.527 verbs=73710
```

Current valid comparison:

- Rust mapped total: 16.546 + 79.895 = 96.441 ms
- Python total: 718.795 + 158.527 = 877.322 ms
- Rust mapped uses about 89% less end-to-end time for this mandatory-`oslots`
  BHSA lexical query path.

## After generalized mapped relation planning

Generalized mapped named-relation search from exactly two atoms to any number of
named atoms. The mapped relation parser now stores left and right atom indexes
for every relation line, and the executor uses recursive binding with
bound-relation pruning, matching the shape of the materialized search engine.

Added mapped tests for:

- a three-word ordered relation chain
- three atoms with non-sequential structural relation endpoints

Validation:

```text
cargo test --quiet
16 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.346 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.200 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=572.594 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.879 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=170.323 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=565.869 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=47.207 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.026 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.820 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=105.671 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1988.261
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=1011.890
```

Current mapped BHSA lexical benchmark:

```text
compile_ms=cached load_ms=16.871 query_ms=81.128 nodes=1446831 words=426590 verbs=73710 oslots_rows=1020241
```

Current Python same-feature measurement with mandatory `otype oslots sp`:

```text
load_ms=390.501 query_ms=144.372 verbs=73710
```

Current valid comparison:

- Rust mapped total: 16.871 + 81.128 = 97.999 ms
- Python total: 390.501 + 144.372 = 534.873 ms
- Rust mapped uses about 82% less end-to-end time for this mandatory-`oslots`
  BHSA lexical query path.

Mapped-search feature gaps from the planned slice are now closed. Broader port
work remains before calling the full objective complete: more of the Python test
suite should be audited/ported, public API compatibility should be reviewed
against `libs/core`, and the benchmark suite should include broader workload
mixes beyond the current curated and lexical BHSA checks.

## After mapped BHSA curated-subset validator

Added `cf_rust_validate_bhsa_mapped_curated`, a mapped compiled-cache validator
for the BHSA query shapes currently supported by `MappedSearch`. It compiles or
opens a cache with mandatory `otype`, `oslots`, and the required query features,
then runs lexical, structural, and quantified templates without materializing a
full `Corpus`.

The first run exposed a real mapped containment bug: indented containment was
iterating direct `oslots` targets, so `clause` containing `phrase` returned zero
because `oslots` stores slots. Fixed mapped indented containment to use slot-set
containment, matching `Corpus::contains`, and added a mini-corpus regression for
`sentence\n  phrase`.

Mapped BHSA curated-subset validation:

```text
cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=18.313 nodes=1446831
ok query=lex_001 results=5 elapsed_ms=12.382
ok query=lex_024 results=5 elapsed_ms=8.498
ok query=lex_026 results=5 elapsed_ms=8.027
ok query=struct_001 results=5 elapsed_ms=7.074
ok query=struct_008 results=5 elapsed_ms=9.560
ok query=struct_013 results=5 elapsed_ms=6.003
ok query=struct_020 results=5 elapsed_ms=79.914
ok query=struct_024 results=5 elapsed_ms=114.259
ok query=quant_001 results=5 elapsed_ms=76.418
ok query=quant_003 results=5 elapsed_ms=273.476
ok query=quant_004 results=5 elapsed_ms=700.663
```

Python same-query subset with mandatory `otype oslots` and the same feature set:

```text
loaded features=10 load_ms=719.565
ok query=lex_001 results=5 elapsed_ms=150.608
ok query=lex_024 results=5 elapsed_ms=146.847
ok query=lex_026 results=5 elapsed_ms=170.585
ok query=struct_001 results=5 elapsed_ms=86.007
ok query=struct_008 results=5 elapsed_ms=84.454
ok query=struct_013 results=5 elapsed_ms=32.779
ok query=struct_020 results=5 elapsed_ms=154.192
ok query=struct_024 results=5 elapsed_ms=221.824
ok query=quant_001 results=5 elapsed_ms=242.075
ok query=quant_003 results=5 elapsed_ms=262.507
ok query=quant_004 results=5 elapsed_ms=363.934
```

Follow-up validation after the containment fix:

```text
cargo test --quiet
16 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=147.171 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.368 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=560.735 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.940 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=183.349 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=584.023 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.915 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.653 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.559 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=113.624 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2018.591
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=935.152
```

This mapped subset was a stronger performance proof than the single lexical
query path, but it also exposed quantified planner work: at this point
`quant_003` and `quant_004` were slower on query-only time. The next entry
records the candidate-precomputation fix for that issue.

## After mapped quantified candidate precomputation

Optimized mapped quantified block execution by precomputing candidate node lists
for each block alternative once per query. Previously, each quantified root
re-ran every child atom search, which made `/without/` and article-contained
phrase queries slower than Python on query-only time. The mapped quantifier path
now reuses candidate lists and only evaluates slot-set containment per root.

Mapped BHSA curated-subset validation after optimization:

```text
cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=45.295 nodes=1446831
ok query=lex_001 results=5 elapsed_ms=9.007
ok query=lex_024 results=5 elapsed_ms=6.357
ok query=lex_026 results=5 elapsed_ms=7.056
ok query=struct_001 results=5 elapsed_ms=6.494
ok query=struct_008 results=5 elapsed_ms=10.774
ok query=struct_013 results=5 elapsed_ms=6.326
ok query=struct_020 results=5 elapsed_ms=97.296
ok query=struct_024 results=5 elapsed_ms=127.250
ok query=quant_001 results=5 elapsed_ms=37.062
ok query=quant_003 results=5 elapsed_ms=86.673
ok query=quant_004 results=5 elapsed_ms=239.663
```

Python same-query subset:

```text
loaded features=10 load_ms=901.546
ok query=lex_001 results=5 elapsed_ms=161.684
ok query=lex_024 results=5 elapsed_ms=154.911
ok query=lex_026 results=5 elapsed_ms=178.279
ok query=struct_001 results=5 elapsed_ms=88.133
ok query=struct_008 results=5 elapsed_ms=88.217
ok query=struct_013 results=5 elapsed_ms=36.716
ok query=struct_020 results=5 elapsed_ms=159.055
ok query=struct_024 results=5 elapsed_ms=237.002
ok query=quant_001 results=5 elapsed_ms=255.470
ok query=quant_003 results=5 elapsed_ms=268.961
ok query=quant_004 results=5 elapsed_ms=385.316
```

The supported mapped subset now beats Python on load time and every measured
query-only time in this run.

Regression validation:

```text
cargo test --quiet
16 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=225.094 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=25.057 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=560.423 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.203 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=165.108 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=584.340 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=48.675 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.544 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=22.946 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=107.718 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1857.457
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=932.589
```

## After valued edge and parser edge-case parity slice

Audited the Python test fixtures for unported loader behavior and ported the
next cohesive parser slice:

- empty node feature
- single-node implicit node feature
- Unicode node feature values
- invalid files with missing header / malformed metadata section
- `@edgeValues` string-valued edge feature (`relation.tf`)
- `@edgeValues` integer-valued edge feature (`distance.tf`), including blank
  edge values where the edge exists but no value is stored

Implemented direct-parser valued edge storage on `EdgeFeature`:

- `edge_values: HashMap<(u32, u32), FeatureValue>`
- `edge_value(source, target)`
- `has_edge_values()`

Existing edge traversal APIs (`forward`, `backward`, `both`, `edge_count`) are
unchanged.

Validation:

```text
cargo test --quiet
18 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=130.653 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.523 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=569.113 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.755 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=168.918 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=564.014 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=50.598 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=74.006 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.320 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=107.038 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1972.665
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=888.901

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=45.474 nodes=1446831
11 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=248.259
```

## After config feature retention

Ported the next loader/API parity slice for `@config` features. The parser
already recognized config files, but `Corpus::load` discarded them. `Corpus`
now stores:

```text
config_features: BTreeMap<String, BTreeMap<String, Option<String>>>
```

and exposes `config_feature(name)` for metadata lookup. Fresh `.tf` loads now
retain `otext.tf` metadata such as `sectionTypes`, `sectionFeatures`, and
`fmt:text-orig-full`.

Validation:

```text
cargo test --quiet
19 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=120.369 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=24.493 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=538.056 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.890 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=162.019 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=548.724 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=45.866 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.652 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.497 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=100.062 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1832.512
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=830.715

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=11.562 nodes=1446831
11 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=235.950
```

## After compiled-cache config metadata serialization

Compiled caches now append config feature metadata after the canonical
`order`/`rank` arrays. The reader treats the config section as optional, so
older compiled caches that end after rank data remain readable and simply
materialize an empty config map. Newly written caches preserve `@config`
features such as `otext`, and metadata inspection reports config feature names
and metadata entry counts without materializing a full corpus.

The cache compiler now treats an empty feature list as "compile all features".
This lets parity tests round-trip mini corpora through the compiled cache while
preserving both node/edge features and config metadata. Mapped validators still
use mandatory `otype` and `oslots` and continue to exercise the mmap-backed
path. Existing mapped caches created before this change may not contain config
metadata until recompiled, but they remain load-compatible.

Validation:

```text
cargo test --quiet
19 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=134.214 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=24.018 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=515.924 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.836 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=145.439 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=497.569 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=43.375 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=65.683 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=16.929 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=96.090 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1662.916
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=866.378

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=41.923 nodes=1446831
11 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=231.291
```

## After computed/listing API parity slice

Ported another small Python API parity slice from feature and navigation tests.
`Corpus` now exposes:

```text
node_feature_names()
edge_feature_names()
computed_names()
levels()
order()
rank()
sort_key_tuple(nodes)
```

These methods make the Rust surface closer to Python's `Fall()`, `Eall()`,
`Call()`, `C.levels`, `C.order`, `C.rank`, and `N.sortKeyTuple` behavior
without changing the core data layout. `order()` and `rank()` reuse compiled
arrays when present and compute on demand for direct `.tf` loads. The existing
mandatory `oslots` representation remains the source of containment and level
slot counts.

Validation:

```text
cargo test --quiet
20 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=125.316 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=25.096 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=541.390 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.653 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=163.026 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=534.972 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=43.375 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.325 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=16.203 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=100.131 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1801.821
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=814.468

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=10.238 nodes=1446831
11 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=238.887
```

## After computed boundary data

Added a real `Corpus::boundary()` implementation rather than only listing the
computed name. The Rust shape is:

```text
Boundary {
  first_slots: Vec<Vec<u32>>,
  last_slots: Vec<Vec<u32>>,
}
```

It follows Python's `C.boundary.data` semantics: for each slot, collect
non-slot nodes that start at that slot and nodes that end at that slot. The
`first_slots` entries are ordered in reverse canonical order, and `last_slots`
entries are ordered in canonical order. The computation is derived from the
mandatory `oslots` feature and reuses compiled rank arrays when available.

Validation:

```text
cargo test --quiet
20 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=128.637 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=24.565 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=549.190 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.860 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=165.208 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=537.996 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=46.204 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.078 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.673 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.115 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1848.153
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=806.812

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=10.330 nodes=1446831
11 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=238.911
```

## After boundary-aware locality next/previous

Aligned `Corpus::next()` and `Corpus::previous()` more closely with Python's
`L.n()` and `L.p()` semantics. Unfiltered adjacency now uses boundary data:

- `next(n, None)` returns the next slot plus structural nodes that start at
  that slot.
- `previous(n, None)` returns structural nodes that end at the previous slot,
  followed by the previous slot.
- Typed calls still filter the boundary-aware result by `otype`, so existing
  same-type behavior such as next phrase and previous phrase remains intact.

This closes a gap introduced by the earlier simplified same-type neighbor
implementation. Search validation is unchanged because the query engine does
not call these locality methods directly.

Validation:

```text
cargo test --quiet
20 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=148.363 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=25.734 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=598.873 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.844 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=178.124 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=583.020 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=48.386 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.835 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.133 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=106.552 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2039.591
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=988.348

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=10.601 nodes=1446831
11 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=241.608
```

## After canonical walk event parity

Ported another `N.walk()` behavior slice. `Corpus` now exposes:

```text
walk(nodes: Option<&[u32]>) -> Vec<u32>
walk_events(nodes: Option<&[u32]>) -> Vec<WalkEvent>

WalkEvent::Start(node)
WalkEvent::End(node)
WalkEvent::Slot(node)
```

`walk(None)` returns the canonical order, and `walk(Some(nodes))` returns the
provided subset sorted canonically. `walk_events()` mirrors Python's
`N.walk(events=True)` behavior: non-slot nodes emit start events, slot nodes
emit a slot event, and structural nodes ending at that slot emit end events
after the slot in the same reversed end-boundary order Python uses.

The implementation derives event data from `order()` and `boundary()` rather
than introducing another persisted structure. That keeps `oslots` as the single
containment source and preserves the current memory-mapped strategy: compiled
loads can reuse cached rank/order arrays, and direct loads compute the same
views on demand.

Validation:

```text
cargo test --quiet
21 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=132.219 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=24.425 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=544.768 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.968 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=162.951 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=554.620 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=48.359 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=66.756 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.031 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.329 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1854.670
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=870.218

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=10.991 nodes=1446831
11 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=237.663
```

## After chunk sort-key parity

Ported the `N.sortKeyChunk` and `N.sortKeyChunkLength` navigation behavior as
explicit Rust key methods:

```text
Chunk { node, start, end }
sort_key_chunk(chunk) -> ChunkPositionKey
sort_key_chunk_length(chunk) -> ChunkLengthKey
```

The position key follows Python's comparator: chunk start ascending, node type
rank descending, chunk end descending, then node id ascending. The length key
follows Python's alternate comparator: span length descending, start ascending,
node type rank ascending, then node id ascending.

These keys are computed on demand from the existing type ranks and do not add
stored corpus state. This keeps the Rust port aligned with the memory-efficiency
goal while adding another navigation API parity slice.

Validation:

```text
cargo test --quiet
22 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=129.108 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.084 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=556.230 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.732 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=166.338 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=549.168 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=45.446 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.125 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.013 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=99.512 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1850.396
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=823.816

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=10.282 nodes=1446831
11 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=238.130
```

## After filtered feature frequency-list parity

Ported another feature API parity slice from Python's `freqList()` behavior.
Rust now exposes corpus-level helpers:

```text
node_frequency_list(feature_name, node_types)
edge_frequency_list(feature_name, node_types_from, node_types_to)
```

Node frequency lists can now be filtered by `otype`, matching Python's
`F.feature.freqList(nodeTypes=...)` behavior. Edge frequency lists can be
filtered by source and target `otype`, matching Python's
`E.feature.freqList(nodeTypesFrom=..., nodeTypesTo=...)` behavior.

The Rust API keeps this at the `Corpus` layer because filtering needs `otype`.
That avoids giving feature objects back-references to the corpus while still
matching the behavior. Valued edge frequency lists use:

```text
EdgeFrequency::Values(Vec<(Option<FeatureValue>, usize)>)
```

so blank values in an `@edgeValues` feature are counted as `None`. Unvalued
edge features return `EdgeFrequency::Count(count)`.

Validation:

```text
cargo test --quiet
22 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=129.523 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.697 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=543.792 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.847 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=166.342 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=549.304 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=47.070 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=64.765 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.365 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.803 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1886.550
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=801.251

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=10.466 nodes=1446831
11 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=237.585
```

## After valued edge traversal parity

Ported another edge feature parity slice from Python's valued edge traversal
API. `EdgeFeature` now exposes:

```text
forward_with_values(node) -> Vec<(target, Option<FeatureValue>)>
backward_with_values(node) -> Vec<(source, Option<FeatureValue>)>
both_with_values(node) -> Vec<(neighbor, Option<FeatureValue>)>
```

This keeps the existing node-only traversal methods intact while adding the
Python-style `(node, value)` result shape for `@edgeValues` features. Blank
valued edges are represented as `None`, which distinguishes them from explicit
integer values such as `0`. For symmetric traversal, incoming edges are loaded
first and outgoing edges take precedence for the same neighbor, matching the
Python behavior.

Validation:

```text
cargo test --quiet
22 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=128.180 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=25.456 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=537.708 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.704 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=159.034 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=534.344 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=45.869 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.184 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.406 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=101.107 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1828.638
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=814.077

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=10.189 nodes=1446831
11 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=237.253
```

## After selective-load WARP retention

Aligned `Corpus::load_features()` with the Python loader expectation that
requesting a specific feature still yields a usable corpus. Selective loads now
always retain:

```text
otype
oslots
@config metadata
```

This matters because `otype` and `oslots` are required for node typing,
canonical order, containment, locality, and querying. It also satisfies the
standing project note that `oslots` is critical and should always be loaded.

The implementation avoids parsing every unrequested large feature body: for
unrequested files it peeks at the first line to keep lightweight `@config`
features and skips unrequested node/edge features unless they are `otype` or
`oslots`.

Validation:

```text
cargo test --quiet
23 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=135.354 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.912 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=553.399 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.916 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=173.456 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=561.473 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=54.520 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.090 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.720 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=105.196 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1870.202
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=830.668

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=10.383 nodes=1446831
11 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=236.644
```

## After lightweight feature inventory

Ported the loader-facing `explore()` parity slice as:

```text
explore_features(path) -> FeatureInventory

FeatureInventory {
  nodes: Vec<String>,
  edges: Vec<String>,
  configs: Vec<String>,
}
```

This scans a `.tf` directory and categorizes feature names by their first
header line (`@node`, `@edge`, or `@config`). It does not parse or materialize
feature bodies, so it stays cheap on large corpora and fits the memory-efficiency
constraint.

Validation:

```text
cargo test --quiet
24 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=133.338 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=25.428 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=556.903 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.925 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=161.598 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=543.635 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=46.524 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.589 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.753 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.485 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1904.923
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=822.310

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=10.278 nodes=1446831
11 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=237.632
```

## After corpus inventory validator

Added a repeatable validation binary for the lightweight inventory path:

```text
cf_rust_explore_corpora <corpora-root>
```

It scans each local benchmark corpus with a `tf/` directory using
`explore_features()` and reports node feature count, edge feature count, config
feature count, and whether mandatory `otype` and `oslots` are discoverable. It
does not load feature bodies, so it is fast enough to run as a cheap preflight
before heavier corpus loading/query validation.

Inventory validation:

```text
cargo run --release --quiet --bin cf_rust_explore_corpora -- ../benchmarks/.corpora
ok corpus=sp explore_ms=3.470 nodes=37 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=tischendorf explore_ms=0.982 nodes=15 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=bhsa explore_ms=12.390 nodes=109 edges=6 configs=1 has_otype=true has_oslots=true
ok corpus=cuc explore_ms=1.231 nodes=12 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=n1904 explore_ms=3.826 nodes=56 edges=4 configs=1 has_otype=true has_oslots=true
ok corpus=dss explore_ms=3.112 nodes=68 edges=3 configs=1 has_otype=true has_oslots=true
ok corpus=quran explore_ms=1.711 nodes=38 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=peshitta explore_ms=0.555 nodes=10 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=syrnt explore_ms=1.116 nodes=41 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=lxx explore_ms=0.515 nodes=27 edges=1 configs=1 has_otype=true has_oslots=true
```

Full validation:

```text
cargo test --quiet
24 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=142.036 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=25.939 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=561.847 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.977 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=178.696 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=558.731 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=46.864 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.722 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.809 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.980 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1912.369
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=864.197

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=9.950 nodes=1446831
11 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=245.343
```

## After escaped query literal parsing

Ported another search syntax parity slice from Python's value escape handling.
Rust equality and non-equality constraints now split alternatives on unescaped
`|` only and unescape literal values before matching. Supported escapes include:

```text
\|
\=
\\
\t
\n
\#
\~
\<
\>
```

This fixes cases where a literal feature value contains a pipe, equals sign, or
backslash. Regex constraints still pass patterns through to the regex engine and
are not reinterpreted by the literal parser.

Validation:

```text
cargo test --quiet
25 passed

cargo run --release --quiet --bin cf_rust_explore_corpora -- ../benchmarks/.corpora
all 10 local benchmark corpora found otype and oslots

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=125.830 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.319 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=571.769 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.873 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=158.257 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=545.590 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=46.168 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.851 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.061 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=100.264 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1795.813
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=807.021

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=10.462 nodes=1446831
11 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=236.422
```

## After slot-set relation operators

Ported another search relation parity slice. Rust now supports slot-set
relations:

```text
a == b   same slot set
a ## b   different slot set
a && b   overlapping slot sets
a || b   disjoint slot sets
```

These relations are evaluated through the existing `oslots`-derived slot
representation. Slot nodes are treated as one-slot sets; non-slot nodes use
their `oslots` rows. This keeps `oslots` as the single source for containment
and slot-set semantics.

Validation:

```text
cargo test --quiet
25 passed

cargo run --release --quiet --bin cf_rust_explore_corpora -- ../benchmarks/.corpora
all 10 local benchmark corpora found otype and oslots

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=129.646 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.055 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=549.425 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.031 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=163.776 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=531.892 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=48.314 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.073 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.537 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=103.634 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1876.189
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=789.419

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=10.240 nodes=1446831
11 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=235.910
```

## After slot order and alignment relation operators

Ported another compact relation parity slice from Python's search syntax.
Rust now supports:

```text
a << b   left completely before right
a >> b   left completely after right
a :> b   left immediately after right
a =: b   left and right start at the same slot
a := b   left and right end at the same slot
```

These operators use `first_slot()` and `last_slot()` derived from `oslots`.
Together with the previous `==`, `##`, `&&`, `||`, `[[`, `]]`, and `<:`
support, the Rust materialized search path covers a larger portion of the
Python slot relation surface without changing the planner.

Validation:

```text
cargo test --quiet
25 passed

cargo run --release --quiet --bin cf_rust_explore_corpora -- ../benchmarks/.corpora
all 10 local benchmark corpora found otype and oslots

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=122.172 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=23.942 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=525.777 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.991 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=156.702 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=549.206 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=47.035 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=65.081 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.532 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=101.841 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1843.791
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=781.109

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=11.344 nodes=1446831
11 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=239.823
```

## After Python-vs-Rust mapped subset comparison

Added repeatable comparison scripts for the currently supported mapped BHSA
subset:

```text
scripts/python_bhsa_mapped_subset.py
scripts/compare_bhsa_mapped_subset.py
```

The Python runner uses the local `libs/core` package via `PYTHONPATH` and runs
the same 10-feature, 11-query BHSA subset as
`cf_rust_validate_bhsa_mapped_curated`. The comparison script invokes both the
Python runner and the Rust mapped validator, parses their line-oriented timing
output, and prints load/query speedups.

Current-machine comparison:

```text
python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5

load_speedup=36.58x python_ms=367.014 rust_ms=10.032
query=lex_001 speedup=13.84x python_ms=132.432 rust_ms=9.572
query=lex_024 speedup=14.50x python_ms=135.719 rust_ms=9.359
query=lex_026 speedup=18.43x python_ms=151.282 rust_ms=8.208
query=quant_001 speedup=6.40x python_ms=231.534 rust_ms=36.172
query=quant_003 speedup=2.79x python_ms=232.693 rust_ms=83.309
query=quant_004 speedup=1.53x python_ms=349.570 rust_ms=228.826
query=struct_001 speedup=14.01x python_ms=78.810 rust_ms=5.624
query=struct_008 speedup=10.43x python_ms=77.249 rust_ms=7.408
query=struct_013 speedup=5.92x python_ms=29.862 rust_ms=5.042
query=struct_020 speedup=2.00x python_ms=155.203 rust_ms=77.706
query=struct_024 speedup=1.87x python_ms=207.724 rust_ms=111.260
query_geomean_speedup=5.94x queries=11
```

Validation after adding the scripts:

```text
cargo test --quiet
24 passed

cargo run --release --quiet --bin cf_rust_explore_corpora -- ../benchmarks/.corpora
all 10 local benchmark corpora found otype and oslots

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all 10 local benchmark corpora loaded and answered a slot-type query

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1862.514
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=890.460

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=21.130 nodes=1446831
11 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=229.913
```

## After mapped slot relation parity

Ported the current materialized slot relation surface into `MappedSearch`:

```text
a == b   same slot set
a ## b   different slot set
a && b   overlapping slot sets
a || b   disjoint slot sets
a << b   left completely before right
a >> b   left completely after right
a :> b   left immediately after right
a =: b   left and right start at the same slot
a := b   left and right end at the same slot
```

The mapped path keeps these checks backed by the memory-mapped compiled cache.
It derives slot vectors from `oslots` only for the candidate nodes being
compared, with slot nodes represented as single-slot vectors. That keeps the
relation semantics aligned with materialized search without introducing a
corpus-wide slot materialization step.

Validation:

```text
cargo test --quiet
25 passed

cargo run --release --quiet --bin cf_rust_explore_corpora -- ../benchmarks/.corpora
ok corpus=sp explore_ms=0.648 nodes=37 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=tischendorf explore_ms=0.302 nodes=15 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=bhsa explore_ms=1.836 nodes=109 edges=6 configs=1 has_otype=true has_oslots=true
ok corpus=cuc explore_ms=0.233 nodes=12 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=n1904 explore_ms=1.007 nodes=56 edges=4 configs=1 has_otype=true has_oslots=true
ok corpus=dss explore_ms=1.085 nodes=68 edges=3 configs=1 has_otype=true has_oslots=true
ok corpus=quran explore_ms=0.631 nodes=38 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=peshitta explore_ms=0.203 nodes=10 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=syrnt explore_ms=0.713 nodes=41 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=lxx explore_ms=0.455 nodes=27 edges=1 configs=1 has_otype=true has_oslots=true

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=124.000 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=24.939 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=536.985 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.141 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=162.447 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=538.570 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=47.349 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.082 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.451 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.829 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1811.930
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=828.010

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=10 compile_ms=cached load_ms=10.270 nodes=1446831
11 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=236.098
```

## After mapped nested containment support

Expanded non-quantified `MappedSearch` from one atom or one indented child to a
general atom plan. The mapped extender now enforces each atom's nearest
less-indented ancestor, matching the materialized search shape for sibling and
nested containment. This brings the mapped BHSA curated subset from 11 to 15
queries by adding `struct_026`, `struct_030`, `complex_001`, and `complex_019`.

Added a query-local slot cache for mapped containment checks. It decodes `oslots`
rows from the memory-mapped cache only once per touched node within a single
search call. This preserves the mapped storage model while reducing repeated
slot-vector decoding during backtracking.

The mapped validator now inspects an existing cache before reuse and recompiles
when required node or edge features are missing. This avoids stale-cache
failures when the curated feature set changes.

Current expanded Python-vs-Rust comparison:

```text
python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5

load_speedup=28.54x python_ms=387.573 rust_ms=13.581
query=complex_001 speedup=0.40x python_ms=723.865 rust_ms=1787.741
query=complex_019 speedup=2.54x python_ms=347.802 rust_ms=137.007
query=lex_001 speedup=4.20x python_ms=134.576 rust_ms=32.013
query=lex_024 speedup=4.36x python_ms=139.154 rust_ms=31.950
query=lex_026 speedup=3.16x python_ms=156.258 rust_ms=49.374
query=quant_001 speedup=6.54x python_ms=246.428 rust_ms=37.662
query=quant_003 speedup=2.79x python_ms=241.439 rust_ms=86.624
query=quant_004 speedup=1.53x python_ms=359.263 rust_ms=234.063
query=struct_001 speedup=3.94x python_ms=79.460 rust_ms=20.167
query=struct_008 speedup=2.15x python_ms=80.171 rust_ms=37.371
query=struct_013 speedup=1.73x python_ms=30.595 rust_ms=17.664
query=struct_020 speedup=2.71x python_ms=159.375 rust_ms=58.826
query=struct_024 speedup=2.19x python_ms=212.337 rust_ms=97.141
query=struct_026 speedup=2.95x python_ms=356.226 rust_ms=120.706
query=struct_030 speedup=0.47x python_ms=283.732 rust_ms=605.985
query_geomean_speedup=2.26x queries=15
```

The query-local slot cache materially improved nested mapped queries
(`complex_001` dropped from roughly 4.8s to roughly 1.8s in the comparison
run), but `complex_001` and `struct_030` remain slower than Python. The next
planner slice should avoid scanning global child candidate lists when a parent
is already bound, using slot ranges or containment-aware candidate indexes.

Validation:

```text
cargo test --quiet
25 passed

cargo run --release --quiet --bin cf_rust_explore_corpora -- ../benchmarks/.corpora
ok corpus=sp explore_ms=0.686 nodes=37 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=tischendorf explore_ms=0.389 nodes=15 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=bhsa explore_ms=1.901 nodes=109 edges=6 configs=1 has_otype=true has_oslots=true
ok corpus=cuc explore_ms=0.261 nodes=12 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=n1904 explore_ms=1.103 nodes=56 edges=4 configs=1 has_otype=true has_oslots=true
ok corpus=dss explore_ms=1.115 nodes=68 edges=3 configs=1 has_otype=true has_oslots=true
ok corpus=quran explore_ms=0.660 nodes=38 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=peshitta explore_ms=0.214 nodes=10 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=syrnt explore_ms=0.720 nodes=41 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=lxx explore_ms=0.509 nodes=27 edges=1 configs=1 has_otype=true has_oslots=true

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=131.606 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=25.093 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=558.059 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.066 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=171.374 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=564.987 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=47.388 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.777 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.710 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.453 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1939.450
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=947.300

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=11 compile_ms=cached load_ms=10.146 nodes=1446831
15 mapped curated-subset queries passed
slowest query=complex_001 elapsed_ms=2163.001
```

## After mapped containment-aware candidate filtering

Added a query-local interval index for mapped plan candidates. For indented
atoms, mapped search now stores each candidate's first and last slot once. When
an ancestor is bound, the extender narrows child candidates to nodes whose slot
interval can fit inside the ancestor before running exact `oslots` subset
containment. Root and flat atoms skip interval metadata, so simple lexical and
type queries avoid the extra slot work.

This is still memory-conscious: the index is scoped to one search call and
stores candidate node ids plus two slot integers, while exact containment still
comes from mapped `oslots`.

Current expanded Python-vs-Rust comparison:

```text
python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5

load_speedup=38.65x python_ms=380.665 rust_ms=9.849
query=complex_001 speedup=7.50x python_ms=728.091 rust_ms=97.066
query=complex_019 speedup=3.74x python_ms=348.043 rust_ms=93.043
query=lex_001 speedup=4.69x python_ms=134.607 rust_ms=28.711
query=lex_024 speedup=4.39x python_ms=139.780 rust_ms=31.814
query=lex_026 speedup=3.17x python_ms=158.248 rust_ms=49.956
query=quant_001 speedup=6.49x python_ms=242.108 rust_ms=37.313
query=quant_003 speedup=2.81x python_ms=241.964 rust_ms=86.241
query=quant_004 speedup=1.55x python_ms=364.104 rust_ms=234.531
query=struct_001 speedup=3.83x python_ms=79.239 rust_ms=20.675
query=struct_008 speedup=2.15x python_ms=80.866 rust_ms=37.566
query=struct_013 speedup=1.68x python_ms=30.619 rust_ms=18.210
query=struct_020 speedup=3.55x python_ms=164.385 rust_ms=46.290
query=struct_024 speedup=3.06x python_ms=213.034 rust_ms=69.659
query=struct_026 speedup=4.86x python_ms=357.599 rust_ms=73.512
query=struct_030 speedup=1.93x python_ms=285.039 rust_ms=147.848
query_geomean_speedup=3.34x queries=15
```

Validation:

```text
cargo test --quiet
25 passed

cargo run --release --quiet --bin cf_rust_explore_corpora -- ../benchmarks/.corpora
ok corpus=sp explore_ms=0.757 nodes=37 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=tischendorf explore_ms=0.304 nodes=15 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=bhsa explore_ms=1.545 nodes=109 edges=6 configs=1 has_otype=true has_oslots=true
ok corpus=cuc explore_ms=0.267 nodes=12 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=n1904 explore_ms=0.949 nodes=56 edges=4 configs=1 has_otype=true has_oslots=true
ok corpus=dss explore_ms=1.124 nodes=68 edges=3 configs=1 has_otype=true has_oslots=true
ok corpus=quran explore_ms=0.622 nodes=38 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=peshitta explore_ms=0.199 nodes=10 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=syrnt explore_ms=0.651 nodes=41 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=lxx explore_ms=0.446 nodes=27 edges=1 configs=1 has_otype=true has_oslots=true

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=133.419 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=25.467 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=573.135 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.730 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=171.030 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=558.188 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=46.726 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.227 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.798 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.174 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1942.370
16 representative curated queries passed
slowest query=complex_001 elapsed_ms=931.063

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=11 compile_ms=cached load_ms=10.254 nodes=1446831
15 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=238.047
```

## After generic node type search

Ported Python-style generic node type queries using `.` in both materialized and
mapped search:

```text
.                 any loaded node
. sp=verb         any loaded node with a feature constraint
p:. ... p [[ w    generic named atom participating in relations
```

Materialized search uses a numeric `otype` feature-node order for generic
candidates, matching Python mini-corpus behavior, while preserving the existing
canonical `all_nodes()` ordering for navigation and walking. Mapped search
treats `.` as an `otype` wildcard and keeps constraints/relations unchanged.

The first BHSA validator attempt used a broad `p:.` embedding query. That was
technically valid but not suitable as a standard gate because it made every node
a possible embedder of every verb. The retained real-corpus relation query uses
`p:. function=Pred`, which still proves generic relation participation while
keeping the candidate set in the curated-query range.

Added a mapped fast path for single root atoms without relations. This avoids
building general plan candidate metadata for queries such as `.` or
`. sp=verb`, so limited generic scans stop early.

Current expanded Python-vs-Rust comparison:

```text
python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5

load_speedup=89.77x python_ms=889.515 rust_ms=9.909
query=complex_001 speedup=7.43x python_ms=719.011 rust_ms=96.772
query=complex_019 speedup=3.75x python_ms=353.053 rust_ms=94.023
query=generic_001 speedup=7.38x python_ms=49.261 rust_ms=6.675
query=generic_002 speedup=32.36x python_ms=172.791 rust_ms=5.339
query=generic_003 speedup=4.29x python_ms=410.793 rust_ms=95.858
query=lex_001 speedup=25.61x python_ms=137.136 rust_ms=5.355
query=lex_024 speedup=26.99x python_ms=143.093 rust_ms=5.302
query=lex_026 speedup=30.81x python_ms=163.117 rust_ms=5.294
query=quant_001 speedup=6.49x python_ms=246.511 rust_ms=37.969
query=quant_003 speedup=3.08x python_ms=267.471 rust_ms=86.966
query=quant_004 speedup=1.58x python_ms=374.456 rust_ms=236.684
query=struct_001 speedup=21.22x python_ms=114.223 rust_ms=5.382
query=struct_008 speedup=10.13x python_ms=81.924 rust_ms=8.085
query=struct_013 speedup=6.05x python_ms=31.457 rust_ms=5.200
query=struct_020 speedup=3.34x python_ms=157.877 rust_ms=47.303
query=struct_024 speedup=3.19x python_ms=221.932 rust_ms=69.623
query=struct_026 speedup=4.88x python_ms=362.451 rust_ms=74.217
query=struct_030 speedup=1.96x python_ms=295.187 rust_ms=150.488
query_geomean_speedup=7.18x queries=18
```

Validation:

```text
cargo test --quiet
25 passed

cargo run --release --quiet --bin cf_rust_explore_corpora -- ../benchmarks/.corpora
ok corpus=sp explore_ms=0.668 nodes=37 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=tischendorf explore_ms=0.666 nodes=15 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=bhsa explore_ms=1.891 nodes=109 edges=6 configs=1 has_otype=true has_oslots=true
ok corpus=cuc explore_ms=2.013 nodes=12 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=n1904 explore_ms=7.192 nodes=56 edges=4 configs=1 has_otype=true has_oslots=true
ok corpus=dss explore_ms=7.890 nodes=68 edges=3 configs=1 has_otype=true has_oslots=true
ok corpus=quran explore_ms=4.430 nodes=38 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=peshitta explore_ms=1.407 nodes=10 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=syrnt explore_ms=5.043 nodes=41 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=lxx explore_ms=4.003 nodes=27 edges=1 configs=1 has_otype=true has_oslots=true

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.163 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.011 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=577.888 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=3.149 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=176.916 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=576.875 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=50.281 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=75.670 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.907 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=111.907 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1931.977
19 representative curated queries passed
slowest query=complex_001 elapsed_ms=925.448

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=11 compile_ms=cached load_ms=10.202 nodes=1446831
18 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=237.108
```

## After materialized custom search sets

Ported Python-style custom search sets for the materialized search API with:

```rust
search.search_with_sets(template, &sets, limit)
```

Custom set names act as candidate sources wherever a node type token is used,
including named atoms such as `w:mywords`. The implementation sorts and dedups
set nodes per query, then applies normal feature constraints, containment, and
relations. Ordinary `search()` behavior is unchanged.

Converted the Python integration custom-set cases against the mini corpus:

```text
mywords                    custom set as a type
w pos=noun                 custom set with feature constraints
empty                      empty custom set
single                     singleton custom set
w:mywords ... w ]] p       custom sets in a relation
```

Validation:

```text
cargo test --quiet
26 passed

cargo run --release --quiet --bin cf_rust_explore_corpora -- ../benchmarks/.corpora
ok corpus=sp explore_ms=0.731 nodes=37 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=tischendorf explore_ms=0.343 nodes=15 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=bhsa explore_ms=1.877 nodes=109 edges=6 configs=1 has_otype=true has_oslots=true
ok corpus=cuc explore_ms=0.243 nodes=12 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=n1904 explore_ms=0.978 nodes=56 edges=4 configs=1 has_otype=true has_oslots=true
ok corpus=dss explore_ms=1.295 nodes=68 edges=3 configs=1 has_otype=true has_oslots=true
ok corpus=quran explore_ms=0.747 nodes=38 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=peshitta explore_ms=0.241 nodes=10 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=syrnt explore_ms=0.658 nodes=41 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=lxx explore_ms=0.441 nodes=27 edges=1 configs=1 has_otype=true has_oslots=true

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=131.872 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.553 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=590.786 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.038 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=171.713 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=549.587 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=47.433 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.628 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.762 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=101.338 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1957.985
19 representative curated queries passed
slowest query=complex_001 elapsed_ms=920.936

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=11 compile_ms=cached load_ms=10.292 nodes=1446831
18 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=236.522

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=28.61x python_ms=367.942 rust_ms=12.859
query_geomean_speedup=6.54x queries=18
```

## After mapped quantifier interval prefiltering

Added `quant_015` to the mapped BHSA curated validator and Python comparison
script, bringing the mapped curated set to the same 19 query IDs as the
materialized curated validator.

The first direct run showed `quant_015` was correct but slower than Python
because mapped quantified containment repeatedly decoded parent and child
`oslots` rows. Optimized mapped quantifier evaluation by:

- precomputing each candidate node's first and last slot,
- caching decoded slot vectors within one query,
- rejecting candidates whose slot interval cannot fit in the current base node
  before running exact containment.

This keeps the exact containment check while avoiding most repeated mmap row
decoding for multi-atom `/with/` blocks such as:

```text
sentence
/with/
  clause kind=VC
  clause kind=NC
/-/
```

Validation:

```text
cargo test --quiet
26 passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated queries passed
quant_015 elapsed_ms=61.239

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=28.70x python_ms=389.354 rust_ms=13.564
query=quant_015 speedup=3.83x python_ms=233.466 rust_ms=60.976
query_geomean_speedup=7.08x queries=19

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=125.689 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=25.270 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=534.089 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.040 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=172.072 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=554.820 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=45.974 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.315 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.651 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=103.534 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed
```

## After direct-load lazy rank caching

Added private lazy-once `order` and `rank` caches to direct `.tf` corpus loads.
Compiled-cache loads still store explicit public `order` and `rank` arrays from
the cache file, while plain `Corpus::load()` and selective
`Corpus::load_features()` now keep loading cheap and compute canonical order
only once if a later navigation, sorting, boundary, or relation path asks for
it.

An attempted eager direct-load rank build made the BHSA direct-load tests take
more than 60 seconds, so the final implementation keeps direct load time
preserved and removes repeated whole-corpus ordering work only after the first
rank-dependent call. Tests now assert mini-corpus rank/order behavior without
forcing BHSA to build ranks during load.

Validation:

```text
cargo test --quiet
26 passed, finished in 13.52s

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=129.502 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=25.531 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=555.585 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.970 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=167.725 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=553.625 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=47.799 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.865 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.835 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.993 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=25.96x python_ms=376.168 rust_ms=14.493
query_geomean_speedup=7.06x queries=19
```

## After lazy boundary caching

Added a private `OnceLock<Boundary>` cache to `Corpus`. Public
`Corpus::boundary()` still returns an owned `Boundary` for API compatibility,
but repeated internal consumers such as `walk_events()`, `next_types()`, and
`previous_types()` now borrow the cached boundary instead of rebuilding
`first_slots` and `last_slots` every time.

This complements the lazy rank cache: direct `.tf` loads stay cheap, while the
first rank/boundary-dependent call pays the derivation cost once and later
navigation calls reuse the derived data. A focused test now checks repeated
`boundary()` calls remain stable.

Validation:

```text
cargo test --quiet
26 passed, finished in 13.46s

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=154.651 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.478 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=683.335 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.821 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=205.063 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=647.761 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=62.920 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=78.620 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=26.326 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=120.873 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=63.66x python_ms=718.689 rust_ms=11.289
query_geomean_speedup=7.17x queries=19
```

## After edge feature items parity

Ported another Python edge-feature API surface by adding
`EdgeFeature::items()`. The method returns source rows sorted by source node,
with each target vector sorted and deduplicated. It works for both plain edge
features and valued edge features; edge values remain available through
`edge_value()` and the existing `*_with_values()` traversal methods.

Added tests against the mini-corpus `parent` edge and the valued `relation`
edge.

Validation:

```text
cargo test --quiet
26 passed, finished in 12.88s

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=147.782 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.743 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=623.376 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.505 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=194.811 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=605.926 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=60.067 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.487 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.874 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=108.697 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=54.35x python_ms=595.419 rust_ms=10.956
query_geomean_speedup=6.53x queries=19
```

## After feature frequency alias parity

Added Python-style feature frequency API aliases:

- `NodeFeature::freq_list()` and `NodeFeature::freqList()`
- `EdgeFeature::freq_list()` and `EdgeFeature::freqList()`

Also added direct `EdgeFeature::frequency_list()`, returning the same
`EdgeFrequency` shape used by corpus-level edge frequency queries. Plain edge
features return `EdgeFrequency::Count(total_edges)`, while valued edge features
return sorted value-frequency rows including `None` for unvalued edges.

Added `EdgeFeature::b(node)` as a short alias for bidirectional traversal,
matching the Python edge feature method name.

Validation:

```text
cargo test --quiet
26 passed, finished in 13.02s

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=146.838 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=32.380 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=660.791 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.712 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=196.571 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=621.372 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=61.865 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.718 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.415 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=113.035 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=76.23x python_ms=746.024 rust_ms=9.786
query_geomean_speedup=7.34x queries=19
```

## After WARP corpus helper parity

Added small public `Corpus` helpers for central WARP feature behavior:

- `node_type(node)` mirrors `F.otype.v(node)` for loaded corpora.
- `s_interval(node_type)` and `sInterval(node_type)` alias
  `node_type_interval(node_type)`.
- `slots(node)` mirrors `E.oslots.s(node)` semantics by returning `[node]` for
  slot nodes and the stored `oslots` targets for non-slot nodes.

The existing borrowed `slots_of(node)` method remains available for internal
and memory-conscious non-slot access. The new owned `slots(node)` helper is for
Python-style API parity and handles slot nodes, out-of-range nodes, and
non-slot containment uniformly.

Validation:

```text
cargo test --quiet
26 passed, finished in 12.82s

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.733 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.230 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=623.425 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.173 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=192.184 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=610.748 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=58.281 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.552 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.750 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.480 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=39.86x python_ms=430.091 rust_ms=10.789
query_geomean_speedup=7.20x queries=19
```

## After feature metadata helper parity

Added metadata helper methods to both `NodeFeature` and `EdgeFeature`:

- `meta()` returns the raw Text-Fabric metadata map.
- `metadata_value(key)` returns a string value for keyed metadata.
- `value_type()` reads the `valueType` metadata key.
- `description()` reads the `description` metadata key.

The raw public `metadata` field remains available. Flag-style metadata such as
`@edgeValues` is still represented as a present key with no string value, so
`metadata_value("edgeValues")` returns `None` while `meta().contains_key(...)`
can still detect the flag.

Validation:

```text
cargo test --quiet
26 passed, finished in 12.71s

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=145.952 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=33.886 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=633.007 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.227 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=193.600 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=604.421 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=60.446 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=74.197 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.383 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=112.698 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=76.59x python_ms=728.640 rust_ms=9.513
query_geomean_speedup=7.07x queries=19
```

## After EdgeFrequency ownership cleanup

Moved `EdgeFrequency` from `corpus.rs` to `feature.rs` because it is an edge
feature result shape shared by direct `EdgeFeature::frequency_list()` and
corpus-level `Corpus::edge_frequency_list(...)`. The crate-level public export
remains `cf_rust::EdgeFrequency`, so existing callers do not need to change
imports.

This removes the feature module's dependency on the corpus module and keeps
feature-layer types colocated with feature-layer behavior.

Validation:

```text
cargo test --quiet
26 passed, finished in 12.75s

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=141.739 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=33.826 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=630.029 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.053 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=196.190 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=600.225 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=60.025 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.164 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.028 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.541 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=73.34x python_ms=758.480 rust_ms=10.342
query_geomean_speedup=7.34x queries=19
```

## After mapped custom search sets

Extended custom search sets to the mapped search API:

```rust
mapped_search.search_with_sets(template, &sets, limit)
```

When an atom type token matches a custom set name, mapped search now uses the
set nodes as the candidate source and still applies mapped feature constraints,
relations, and containment. The custom set vector is query-scoped, sorted, and
deduplicated. This keeps the compiled corpus memory-mapped; only caller-provided
set nodes are held outside the mapped cache.

Mapped fixture coverage now mirrors the materialized custom-set slice:

```text
mywords                    custom set as a type
w pos=noun                 custom set with feature constraints
empty                      empty custom set
single                     singleton custom set
w:mywords ... w ]] p       custom sets in a relation
```

Validation:

```text
cargo test --quiet
26 passed

cargo run --release --quiet --bin cf_rust_explore_corpora -- ../benchmarks/.corpora
ok corpus=sp explore_ms=0.677 nodes=37 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=tischendorf explore_ms=0.294 nodes=15 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=bhsa explore_ms=1.891 nodes=109 edges=6 configs=1 has_otype=true has_oslots=true
ok corpus=cuc explore_ms=0.238 nodes=12 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=n1904 explore_ms=0.984 nodes=56 edges=4 configs=1 has_otype=true has_oslots=true
ok corpus=dss explore_ms=1.095 nodes=68 edges=3 configs=1 has_otype=true has_oslots=true
ok corpus=quran explore_ms=0.630 nodes=38 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=peshitta explore_ms=0.212 nodes=10 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=syrnt explore_ms=0.667 nodes=41 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=lxx explore_ms=0.519 nodes=27 edges=1 configs=1 has_otype=true has_oslots=true

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=134.148 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=25.070 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=567.643 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.146 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=169.643 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=557.962 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.455 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.898 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=16.880 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=105.115 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1950.480
19 representative curated queries passed
slowest query=complex_001 elapsed_ms=896.215

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=11 compile_ms=cached load_ms=10.084 nodes=1446831
18 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=237.832

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=25.15x python_ms=379.177 rust_ms=15.074
query_geomean_speedup=6.63x queries=18
```

## After Python-style feature accessor aliases

Added thin feature API aliases that mirror the compact Python names while
preserving the clearer Rust methods:

```text
NodeFeature::v(node)       alias for value(node)
NodeFeature::s(value)      alias for nodes_with_value(value)
EdgeFeature::s(node)       alias for forward(node)
EdgeFeature::f(node)       alias for forward(node)
EdgeFeature::t(node)       alias for backward(node)
```

These are pure delegation methods; they do not add new indexes or change
storage. Tests now cover `F.word.v`, `F.word.s`, `E.oslots.s`, `E.parent.f`,
and `E.parent.t` style behavior against the mini corpus.

Validation:

```text
cargo test --quiet
26 passed

cargo run --release --quiet --bin cf_rust_explore_corpora -- ../benchmarks/.corpora
ok corpus=sp explore_ms=0.655 nodes=37 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=tischendorf explore_ms=0.344 nodes=15 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=bhsa explore_ms=1.628 nodes=109 edges=6 configs=1 has_otype=true has_oslots=true
ok corpus=cuc explore_ms=0.277 nodes=12 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=n1904 explore_ms=0.993 nodes=56 edges=4 configs=1 has_otype=true has_oslots=true
ok corpus=dss explore_ms=1.126 nodes=68 edges=3 configs=1 has_otype=true has_oslots=true
ok corpus=quran explore_ms=0.657 nodes=38 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=peshitta explore_ms=0.219 nodes=10 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=syrnt explore_ms=0.734 nodes=41 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=lxx explore_ms=0.470 nodes=27 edges=1 configs=1 has_otype=true has_oslots=true

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=133.374 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=25.445 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=563.967 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.881 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=167.865 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=560.386 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=47.316 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.612 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.809 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=106.840 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1969.810
19 representative curated queries passed
slowest query=complex_001 elapsed_ms=942.167

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=11 compile_ms=cached load_ms=10.453 nodes=1446831
18 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=240.604

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=37.71x python_ms=373.623 rust_ms=9.909
query_geomean_speedup=6.65x queries=18
```

## After multi-type locality filters

Ported Python `L.u()` / `L.d()` set-filter behavior into explicit Rust
multi-type locality methods:

```rust
corpus.up_types(node, Some(&["phrase", "sentence"]))
corpus.down_types(node, Some(&["word", "phrase"]))
corpus.next_types(node, Some(&["word", "phrase"]))
corpus.previous_types(node, Some(&["word", "phrase"]))
```

The existing single-type methods (`up`, `down`, `next`, `previous`) now delegate
to these methods, preserving existing callers while sharing filter semantics.
This covers the Python integration cases where `otype` can be a set of node
types.

Validation:

```text
cargo test --quiet
26 passed

cargo run --release --quiet --bin cf_rust_explore_corpora -- ../benchmarks/.corpora
ok corpus=sp explore_ms=0.766 nodes=37 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=tischendorf explore_ms=0.316 nodes=15 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=bhsa explore_ms=1.893 nodes=109 edges=6 configs=1 has_otype=true has_oslots=true
ok corpus=cuc explore_ms=0.248 nodes=12 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=n1904 explore_ms=0.916 nodes=56 edges=4 configs=1 has_otype=true has_oslots=true
ok corpus=dss explore_ms=1.101 nodes=68 edges=3 configs=1 has_otype=true has_oslots=true
ok corpus=quran explore_ms=0.615 nodes=38 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=peshitta explore_ms=0.275 nodes=10 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=syrnt explore_ms=0.721 nodes=41 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=lxx explore_ms=0.486 nodes=27 edges=1 configs=1 has_otype=true has_oslots=true

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=130.835 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.637 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=567.131 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.170 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=174.294 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=569.641 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.008 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.292 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=20.138 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.845 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1950.717
19 representative curated queries passed
slowest query=complex_001 elapsed_ms=938.738

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=11 compile_ms=cached load_ms=10.141 nodes=1446831
18 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=238.068

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=25.59x python_ms=398.102 rust_ms=15.557
query_geomean_speedup=6.51x queries=18
```

## After mapped feature accessor aliases

Added Python-style aliases directly to mmap-backed compiled feature views:

- `StringPoolNodeFeatureView::v(node)` and `s(value)`
- `MixedNodeFeatureView::v(node)` and `s(value)`
- `EdgeFeatureView::s(source)`, `f(source)`, and `t(target)`

The node aliases borrow values from mapped cache bytes and allocate only result
vectors for selection. Edge forward aliases collect from the mapped target
iterator; backward `t(target)` scans edge rows on demand and does not materialize
a persistent inverse index. This keeps `oslots` available through the mapped
path while preserving the memory-efficiency goal.

Validation:

```text
cargo test --quiet
26 passed

cargo run --release --quiet --bin cf_rust_explore_corpora -- ../benchmarks/.corpora
ok corpus=sp explore_ms=0.802 nodes=37 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=tischendorf explore_ms=0.465 nodes=15 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=bhsa explore_ms=2.466 nodes=109 edges=6 configs=1 has_otype=true has_oslots=true
ok corpus=cuc explore_ms=0.274 nodes=12 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=n1904 explore_ms=0.996 nodes=56 edges=4 configs=1 has_otype=true has_oslots=true
ok corpus=dss explore_ms=1.115 nodes=68 edges=3 configs=1 has_otype=true has_oslots=true
ok corpus=quran explore_ms=0.604 nodes=38 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=peshitta explore_ms=0.276 nodes=10 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=syrnt explore_ms=0.739 nodes=41 edges=1 configs=1 has_otype=true has_oslots=true
ok corpus=lxx explore_ms=0.485 nodes=27 edges=1 configs=1 has_otype=true has_oslots=true

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=123.427 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=24.260 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=558.975 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.967 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=163.034 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=550.370 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=47.997 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.122 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.202 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=100.769 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1877.152
19 representative curated queries passed
slowest query=complex_001 elapsed_ms=895.237

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=11 compile_ms=cached load_ms=10.493 nodes=1446831
18 mapped curated-subset queries passed
slowest query=quant_004 elapsed_ms=236.086

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=25.91x python_ms=368.708 rust_ms=14.232
query_geomean_speedup=6.53x queries=18
```

## After WARP type interval lookup

Added `Corpus::node_type_interval(node_type)` as a Rust equivalent of Python
`otype.sInterval()`. It reuses the existing sorted `nodes_by_type` vectors and
returns the first and last node for a known type, or `None` for an unknown type.
No additional index or materialized feature copy is introduced.

Added `StringPoolNodeFeatureView::value_interval(value)` for the mapped compiled
path. This is useful for mapped `otype` interval checks because it returns only
the first and last matching node instead of allocating the full vector returned
by `s(value)`.

Validation:

```text
cargo test --quiet
26 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all 10 corpora loaded and queried
```

## After memory-light edge backward lookup

Changed materialized `EdgeFeature::backward(node)` and alias `t(node)` to scan
only for the requested target node instead of building a full inverse edge map
for every lookup. The explicit `inverse_map()` API is retained for callers that
need the whole source-by-target index, but normal reverse traversal now avoids a
large temporary allocation on corpora such as BHSA.

Added a test assertion for reverse `oslots` lookup so the required slot edge
feature exercises the memory-light path.

Validation:

```text
cargo test --quiet
26 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=127.270 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=23.802 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=550.853 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.983 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=158.629 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=542.771 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=46.804 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.207 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.548 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.761 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed
```

## After mixed mapped value interval lookup

Added `MixedNodeFeatureView::value_interval(value)` so mmap-backed mixed node
features can return the first and last matching node without allocating the full
selection vector from `s(value)`. This mirrors the string-pool interval helper
and covers integer-valued features such as the mini-corpus `number` fixture.

This keeps the compiled cache API moving toward Python WARP-style interval
primitives while preserving memory-mapped reads.

Validation:

```text
cargo test --quiet
26 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=137.857 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=25.719 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=552.751 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.911 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=159.981 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=547.298 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=47.503 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=66.354 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.961 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.873 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
18 mapped curated-subset queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=26.45x python_ms=369.017 rust_ms=13.952
query_geomean_speedup=6.54x queries=18
```

## After unknown valueType fixture parity

Audited the Python loader test fixtures for the next parser parity gap. The
Python loader preserves unknown `@valueType` metadata but falls back to string
parsing unless the type is the supported integer path. The Rust parser already
had this behavior, so this slice adds an explicit regression test using
`libs/core/tests/fixtures/invalid/bad_valueType.tf`.

The fixture remains named `invalid` in the Python tests, but its expected loader
behavior is not an error: `@valueType=float` is retained as metadata, and the
values `1.5` and `2.5` are exposed as strings.

Validation:

```text
cargo test --quiet
27 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=134.794 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.846 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=640.007 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.229 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=188.277 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=599.173 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=58.479 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.133 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.605 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=113.152 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=39.84x python_ms=380.476 rust_ms=9.550
query_geomean_speedup=7.18x queries=19
```

## After explicit node row fixture parity

Ported another Python loader behavior into the Rust regression suite: node
feature data rows can specify explicit node identifiers, and later implicit
rows resume after the highest explicit node seen so far. This is covered with a
temporary `.tf` fixture asserting nodes `5`, `10`, and then implicit node `11`.

No parser code change was required; the existing parser already handled this
shape. The added test makes that compatibility visible in the Rust suite.

Validation:

```text
cargo test --quiet
28 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=140.137 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.589 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=630.106 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.078 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=185.664 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=603.473 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.418 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.180 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.924 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.536 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed
```

## After studied search shallow projection parity

Added order-preserving shallow projection helpers to `SearchStudy`, shared by
materialized and mapped search:

```text
SearchStudy::fetch_first_nodes(limit)
SearchStudy::fetch_prefixes(width, limit)
SearchStudy::count_first_nodes(limit)
SearchStudy::count_prefixes(width, limit)
```

Also added direct materialized and mapped search wrappers:

```text
Search::search_first_nodes(template, limit)
Search::search_first_nodes_with_sets(template, sets, limit)
Search::search_prefixes(template, width, limit)
Search::search_prefixes_with_sets(template, sets, width, limit)
MappedSearch::search_first_nodes(template, limit)
MappedSearch::search_first_nodes_with_sets(template, sets, limit)
MappedSearch::search_prefixes(template, width, limit)
MappedSearch::search_prefixes_with_sets(template, sets, width, limit)
```

These cover the common Python `shallow=True` and `shallow=N` search workflow
without exposing a dynamically typed return shape in Rust. Projection
de-duplicates after truncating to the requested prefix width while preserving
first-seen result order; limits apply to projected unique values.

Validation:

```text
cargo test --quiet
45 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=148.026 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.506 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=640.247 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.534 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=191.643 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=605.787 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=59.390 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.765 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.748 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.695 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After explicit text format parity

Pinned explicit text formats for both fixture and BHSA text rendering:

```text
Corpus::text(node, Some("text-orig-full"))
Corpus::text(node, Some("fmt:text-orig-full"))
Corpus::text(node, Some("text-trans-full"))
```

The renderer now accepts either Python-style format names or the underlying
`otext` metadata key form with the `fmt:` prefix. BHSA coverage loads the
original UTF-8 text features and transliteration features, then verifies that
`text-orig-full` and `text-trans-full` both render and produce distinct output.

Validation:

```text
cargo test --quiet
32 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.598 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.426 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=580.010 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.325 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=175.614 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=575.378 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=56.540 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.066 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.604 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=108.689 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=75.62x python_ms=748.743 rust_ms=9.902
query_geomean_speedup=7.14x queries=19
```

## After core section reference parity

Added the materialized section/navigation surface needed by result wrappers and
basic Text API parity:

```text
Corpus::section_features()
Corpus::section_tuple(node, options)
Corpus::section_from_node(node, options)
Corpus::section_ref(node)
Corpus::node_from_section(section)
NodeInfo::from_corpus(...).section_ref
```

Section resolution uses `otext` `sectionTypes` and `sectionFeatures`, resolves
the node's first or last slot, walks upward to the configured section node types,
and reads the typed section feature values. The fixture now verifies Python-style
lookups such as `("S1") -> sentence node 8` and `("S1", 1) -> phrase node 6`;
`NodeInfo.section_ref` now renders `S1 1` instead of an empty placeholder.

Validation:

```text
cargo test --quiet
32 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=132.935 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.683 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=587.056 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.385 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=179.465 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=561.578 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=58.074 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.371 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.313 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=101.212 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=34.30x python_ms=496.889 rust_ms=14.487
query_geomean_speedup=7.21x queries=19
```

## After mapped config metadata access

Extended the compiled-cache inspection and mapped view layer for config
features:

```text
CompiledConfigFeature { payload_start, payload_end }
MappedCompiledCorpus::config_feature(name)
ConfigFeatureView::get(key)
ConfigFeatureView::metadata()
ConfigFeatureView::items()
```

This does not change the compiled file format. The inspector now records the
byte range of each config feature payload, and the mapped view scans that range
directly from the mmap to expose metadata such as `otext.sectionTypes`,
`otext.sectionFeatures`, and `fmt:text-orig-full` without materializing a
`Corpus`.

Validation:

```text
cargo test --quiet
33 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=140.825 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.879 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=580.201 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.038 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=181.617 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=566.709 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=58.780 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.170 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.270 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=107.231 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=30.24x python_ms=403.468 rust_ms=13.341
query_geomean_speedup=7.02x queries=19
```

## After mapped text rendering parity

Added a memory-mapped text renderer:

```text
MappedText::new(mapped_corpus)
MappedText::text(node, format)
MappedText::text_nodes(nodes, format)
```

The mapped renderer reads `otext` format metadata through
`ConfigFeatureView`, resolves slot feature placeholders through mapped
string-pool or mixed node feature views, and uses mapped `oslots` for non-slot
nodes. It accepts both `text-orig-full` and `fmt:text-orig-full` format names,
matching the materialized text API behavior from the previous slices.

The fixture regression compares mapped text output against materialized
`Corpus::text()` and `Corpus::text_nodes()` for slot nodes, phrase/sentence
nodes, explicit format names, missing formats, and caller-ordered multi-node
rendering without materializing the compiled cache into a `Corpus`.

Validation:

```text
cargo test --quiet
34 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=136.534 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.331 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=605.663 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.032 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=174.742 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=557.660 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=57.065 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.894 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.204 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=103.872 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=55.75x python_ms=715.911 rust_ms=12.842
query_geomean_speedup=7.15x queries=19
```

## After mapped section reference parity

Added a memory-mapped section lookup layer:

```text
MappedSections::new(mapped_corpus)
MappedSections::section_types()
MappedSections::section_features()
MappedSections::section_tuple(node, options)
MappedSections::section_from_node(node, options)
MappedSections::section_ref(node)
MappedSections::node_from_section(section)
```

The mapped section layer reads `otext.sectionTypes` and
`otext.sectionFeatures` through `ConfigFeatureView`, resolves node types through
mapped `otype`, uses mapped `oslots` to find section containers for the
reference slot, and reads section labels through mapped string-pool or mixed node
features. It mirrors the materialized `Corpus` section slice for the core
fixture behavior without loading the compiled cache into a `Corpus`.

Validation:

```text
cargo test --quiet
35 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=136.756 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.239 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=507.498 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=3.989 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=171.666 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=517.906 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.984 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.519 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.227 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=106.275 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=49.87x python_ms=721.018 rust_ms=14.459
query_geomean_speedup=7.36x queries=19
```

## After mapped result wrapper parity

Extended the result wrapper layer so mapped compiled-cache search results can be
shaped without materializing a `Corpus`:

```text
NodeInfo::from_mapped(text, sections, node, options)
NodeList::from_mapped_nodes(text, sections, nodes, limit, query, options)
SearchResult::from_mapped_search(text, sections, rows, template, limit, options)
```

The mapped wrappers combine `MappedText` and `MappedSections` to fill node type,
text, section reference, optional slots, and optional node feature values. The
fixture regression compares mapped `NodeInfo`, `NodeList`, and `SearchResult`
outputs against the existing materialized wrappers for the same mini-corpus
nodes and query result.

Validation:

```text
cargo test --quiet
36 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=142.434 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.768 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=552.417 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.615 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=170.245 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=557.081 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.544 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.319 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.479 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=107.167 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=66.46x python_ms=729.633 rust_ms=10.979
query_geomean_speedup=6.66x queries=19
```

## After mapped corpus summary parity

Added a mapped corpus summary constructor:

```text
CorpusInfo::from_mapped(mapped_corpus, name, path)
```

The mapped summary uses compiled metadata plus mapped `otype`, `oslots`, and
`otext` views to report node types, feature lists, slot type, max slot, max
node, and section types without materializing a `Corpus`. The fixture regression
asserts exact parity with `CorpusInfo::from_corpus()` for the mini corpus.

I deliberately did not extend mapped `FeatureInfo` in this slice because the
current compiled format does not preserve node/edge feature metadata such as
`valueType` and `description`. Adding that will require a compatible cache
format extension or version bump instead of silently changing the existing file
layout.

Validation:

```text
cargo test --quiet
37 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=127.307 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.307 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=496.418 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.884 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=153.610 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=539.911 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=48.769 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.410 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=22.848 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=98.926 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=70.99x python_ms=731.152 rust_ms=10.300
query_geomean_speedup=7.31x queries=19
```

## After mapped feature metadata parity

Added a compatible compiled-cache feature metadata extension:

```text
CFRMETA1 trailing section
CompiledNodeFeature::metadata
CompiledEdgeFeature::metadata
FeatureInfo::from_mapped(mapped_corpus, name, kind)
```

Fresh compiled caches now append node and edge feature metadata after config
metadata. This preserves `valueType`, `description`, and `edgeValues` for
mapped inspection and mapped result metadata wrappers. Existing older caches
that end after config metadata remain readable; their compiled feature metadata
maps default to empty.

The regression coverage verifies three paths:

```text
inspect_compiled(...) exposes feature metadata
load_compiled(...) restores feature metadata onto materialized features
FeatureInfo::from_mapped(...) matches FeatureInfo::from_corpus(...)
```

Validation:

```text
cargo test --quiet
38 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=133.923 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.301 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=498.381 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.864 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=152.664 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=523.875 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=50.159 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.077 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=22.271 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.796 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=40.24x python_ms=381.721 rust_ms=9.486
query_geomean_speedup=6.89x queries=19
```

## After expanded BHSA quantified curated coverage

Expanded the materialized and mapped BHSA curated validators from the previous
19-query subset to a 35-query set by adding all 20 quantified curated BHSA query
shapes:

```text
quant_001..quant_020
```

The newly included queries remain within the currently supported quantified
engine shape: one base atom plus `/with/` or `/without/` blocks containing one
or more contained atoms. The mapped validator and Python comparison now compile
and load 13 features instead of 11 by adding:

```text
vs
domain
```

This was a validator and benchmark coverage expansion rather than a search
engine rewrite. It strengthens the performance proof: the mapped Rust path is
faster than Python on every one of the 35 measured BHSA queries.

Validation:

```text
cargo test --quiet
38 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=135.935 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.310 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=481.935 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.983 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=158.065 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=517.598 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=50.542 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.262 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.244 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=100.982 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
35 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
35 mapped curated-subset queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=31.65x python_ms=375.101 rust_ms=11.851
query_geomean_speedup=6.75x queries=35
```

## After expanded BHSA complex curated coverage

Expanded the materialized and mapped BHSA curated validators from 35 to 49
queries by promoting pure nested complex BHSA query shapes:

```text
complex_002
complex_003
complex_004
complex_005
complex_006
complex_007
complex_008
complex_009
complex_010
complex_013
complex_014
complex_015
complex_017
complex_020
```

The complex query shapes left out for now are the mixed quantifier-plus-extra
base atom cases (`complex_011`, `complex_012`, `complex_016`, `complex_018`),
because mapped quantified parsing still requires exactly one base atom. The
promoted queries exercise nested containment, multiple sibling atoms, repeated
sibling atoms, and additional `domain`/`vs` constraints without changing the
validated feature set beyond the previous 13-feature expansion.

The expanded timing run exposed one narrow performance gap:

```text
complex_017 speedup=0.94x
```

That query has two identical sibling atoms:

```text
sentence
  clause kind=VC
  clause kind=VC
```

Mapped search was scanning identical atom candidate rows independently. Added a
query-local candidate-row cache keyed by atom type and constraints, so repeated
atom signatures reuse candidate rows while still building position-specific
candidate sets. After the optimization, `complex_017` is slightly faster than
Python and the 49-query geometric mean remains strongly faster.

Validation:

```text
cargo test --quiet
38 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=125.991 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.110 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=486.672 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.714 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=151.359 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=527.278 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.724 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.722 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.387 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=100.993 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
49 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
49 mapped curated-subset queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=38.71x python_ms=382.889 rust_ms=9.891
query=complex_017 speedup=1.02x python_ms=75.439 rust_ms=73.930
query_geomean_speedup=5.74x queries=49
```

## After mixed quantified BHSA base coverage

Extended materialized and mapped quantified search to accept multi-atom base
plans before applying `/with/` and `/without/` blocks to the first/root row
item. This promotes the remaining mixed quantified complex BHSA query shapes:

```text
complex_011
complex_012
complex_016
complex_018
```

The initial mapped implementation was semantically correct but slow for
`complex_011`, `complex_012`, and `complex_016` because it fully materialized
all base-plan rows before filtering, even when callers requested only the first
five results. Limited quantified searches now expand base rows adaptively:
start with a bounded base-plan search, filter those rows, and double the base
limit only if the filtered result count is still below the caller's limit. Full
unlimited searches still evaluate the whole base plan.

Mapped quantifier alternatives also sort precomputed candidates by first slot
and use binary slot-window selection before exact containment checks. The
adaptive base expansion is the material performance win for the mixed base
queries; the interval ordering remains a useful local pruning step.

Validation:

```text
cargo test --quiet
38 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=136.931 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.450 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=534.212 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=6.027 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=171.307 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=561.214 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.628 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.405 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.889 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=105.997 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
53 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated-subset queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=36.51x python_ms=403.725 rust_ms=11.057
query_geomean_speedup=7.05x queries=53
```

Every measured query in the 53-query mapped comparison was faster than Python.
The narrowest margin remains `complex_017` at 1.05x in this run; the newly
promoted mixed quantified base queries measured at 4.57x to 5.52x.

## After large-node result text omission parity

Ported the Python result-wrapper behavior that avoids expanding very large
non-slot nodes into response text. `NodeInfoOptions` now defaults to
`max_text_slots = 100`, matching Python `MAX_TEXT_SLOTS`, and
`NodeInfo::from_corpus()` / `NodeInfo::from_mapped()` return:

```text
[N slots - text omitted]
```

for non-slot nodes with more than the configured number of slots. Callers can
set `max_text_slots: None` to explicitly render full text. The new regression
test builds a temporary 101-slot phrase corpus and verifies both materialized
and mapped result wrappers, including the full-text opt-out.

Validation:

```text
cargo test --quiet
39 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=135.690 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.441 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=540.375 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=6.066 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=177.404 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=558.647 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.677 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.952 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.941 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=103.672 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated-subset queries passed
```

## After search plan summaries

Extended `SearchStudy` with a lightweight `SearchPlanSummary`, covering the
Python `showPlan()` use case at a stable Rust API level. The summary reports:

```text
template
atom_count
relation_count
result_count
has_quantifiers
```

Materialized studies build the summary directly after search execution; mapped
studies use the same shared `SearchStudy` constructor. The summary intentionally
does not expose private planner enum details, which keeps room for later join
planning changes without breaking callers.

Validation:

```text
cargo test --quiet
45 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=149.002 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.381 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=641.102 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.276 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=194.044 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=600.580 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=60.440 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.551 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.157 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.825 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated-subset queries passed
```

## After studied search workflow

Added a typed Rust equivalent for the Python `S.study()`, `S.fetch()`, and
`S.count()` workflow. Both materialized `Search` and mapped `MappedSearch`
now expose:

```rust
let study = corpus.search().study("word")?;
let rows = study.fetch(Some(10));
let count = study.count(None);
```

and custom-set variants:

```rust
let study = corpus.search().study_with_sets(template, &sets)?;
let study = MappedSearch::new(&mapped).study_with_sets(template, &sets)?;
```

The current implementation eagerly computes the full result set once and stores
it in `SearchStudy`; `fetch(limit)` and `count(limit)` operate on that cached
result set. This matches the public workflow shape while preserving the current
simple Rust executor. A later planner can make `SearchStudy` lazier without
changing callers.

Validation:

```text
cargo test --quiet
45 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=146.368 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.113 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=637.718 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.375 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=195.703 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=588.572 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=58.868 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.719 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.759 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=111.954 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated-subset queries passed
```

## After Rust Fabric facade

Added a small Rust `Fabric` facade to mirror the Python entry-point workflow
without hiding the typed Rust APIs:

```rust
let fabric = Fabric::new(path);
let inventory = fabric.explore()?;
let corpus = fabric.load_all()?;
let selective = fabric.load("word, pos")?;
fabric.compile(cache_path, Vec::<&str>::new())?;
let compiled = fabric.load_compiled(cache_path)?;
let mapped = fabric.open_mapped(cache_path)?;
```

`Fabric::load()` and `Fabric::compile()` accept a `FeatureSpec` trait so callers
can pass whitespace/comma separated strings, arrays, slices, or vectors of
feature names. This keeps the Rust API explicit while covering the Python
`Fabric(...).explore()`, `loadAll()`, `load("word pos")`, and `compile()` usage
pattern.

Validation:

```text
cargo test --quiet
44 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=143.710 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.506 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=618.977 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.409 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=188.216 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=595.858 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=58.432 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=74.214 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.769 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.926 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated-subset queries passed
```

## After mapped edge value relation operators

Extended the compiled cache format with an optional trailing edge-value section:

```text
CFREDGE1
feature count
feature name
edge value row count
source target value
```

The core edge target payload remains unchanged, so old caches remain readable.
New caches append explicit edge values after the existing feature metadata
section. `inspect_compiled()` now records per-edge-feature value row offsets,
`load_compiled()` restores `EdgeFeature.edge_values`, and `EdgeFeatureView`
can read explicit edge values directly from the mmap without materializing the
cache.

Mapped search now supports the same value-constrained edge relation operators
as materialized search:

```text
w -relation=subject> p
p <relation=predicate- w
w1 -distance=0> w2
```

Added tests for compiled materialized reload, direct mmap edge-value lookup,
and mapped search value-constrained edge relations over both string and integer
edge values.

Validation:

```text
cargo test --quiet
43 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.372 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.252 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=528.197 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.682 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=171.851 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=557.847 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=55.285 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.689 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.999 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.341 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated-subset queries passed
```

## After materialized edge value relation operators

Ported another Python search relation shape for direct `.tf` loads:
value-constrained edge relations.

```text
w -relation=subject> p
p <relation=predicate- w
w1 -distance=0> w2
```

The materialized search parser now splits edge relation operators with an
embedded `=` into edge name plus expected value. Evaluation requires both the
edge target relation and the stored edge value to match. Matching reuses the
same string/int compatibility helper as node feature constraints, so string
edge values such as `subject` and integer edge values such as `0` are both
covered.

At this stage the compiled mmap edge payload still stored target rows only, so
mapped value-constrained edge relations were intentionally left out rather than
implemented with partial value semantics. The later `CFREDGE1` edge-value
section addressed this limitation while retaining old-cache compatibility.

Validation:

```text
cargo test --quiet
41 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=139.207 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.207 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=545.071 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.152 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=165.879 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=562.475 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=51.424 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.804 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.471 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=106.947 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated-subset queries passed
```

## After feature relation search operators

Ported another Python search-syntax slice: named-node feature relation
operators. Materialized and mapped search now parse and evaluate:

```text
a .feature. b
a .left=right. b
a .left#right. b
a .left<right. b
a .left>right. b
```

The evaluator compares node feature values on the two bound nodes and treats
missing feature values as a non-match. Equality handles string/int compatible
values where possible; ordering uses typed comparison for matching numeric
values and falls back to string ordering for string values. The mapped executor
uses the existing string-pool/mixed node feature views so this works without
materializing compiled caches.

Added regression coverage for materialized equality, inequality, numeric
less-than, numeric greater-than, and a cross-feature no-match case on the mini
corpus. Added mapped coverage for equality, inequality, and numeric less-than
using the compiled mini corpus.

Validation:

```text
cargo test --quiet
40 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=127.736 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.792 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=535.434 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.080 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=167.631 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=553.721 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.726 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=65.396 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.056 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.820 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated-subset queries passed
```

## After WARP oslots items parity

Added `Corpus::oslots_items()` as a Python-style helper for
`E.oslots.items()` semantics. It returns only non-slot containment rows,
starting at `max_slot + 1`, while preserving generic `EdgeFeature::items()`
behavior for ordinary edge features.

This keeps the memory-conscious internal API intact: search and navigation still
use borrowed `slots_of(node)` slices, while the public parity helper returns an
owned vector like the other current Rust-facing convenience APIs.

Validation:

```text
cargo test --quiet
30 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=137.089 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.324 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=630.253 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.173 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=184.202 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=601.075 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.922 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.208 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.925 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.890 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed
```

## After computed levels tuple parity

Aligned `Corpus::levels()` with Python `C.levels.data` tuple semantics. The
Rust API now returns:

```text
(node_type, average_slots, min_node, max_node)
```

instead of the earlier temporary `(node_type, node_count, slot_count)` shape.
The ordering still follows the existing type-rank ordering from most
encompassing type down to the slot type. The mini-corpus regression now asserts:

```text
("sentence", 5.0, 8, 8)
("phrase", 2.5, 6, 7)
("word", 1.0, 1, 5)
```

Validation:

```text
cargo test --quiet
30 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=134.889 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.326 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=657.625 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.153 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=188.288 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=604.450 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=54.321 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.545 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=26.218 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=113.232 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed
```

## After describe-facing feature catalog parity

Audited `cfabric.describe` for portable API behavior and added the core Rust
data surfaces it depends on:

```text
node_feature_types(feature_name)
edge_feature_source_types(feature_name)
feature_catalog(kind, node_types)
```

The Python describe implementation samples level ranges to infer which node
types have a feature. The Rust implementation uses the loaded feature maps
directly, so it is exact for materialized corpora. Edge catalog filtering follows
Python `list_features(..., node_types=...)` behavior by checking source node
types with outgoing edges.

Validation:

```text
cargo test --quiet
30 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=139.593 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.919 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=628.192 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.075 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=186.923 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=601.441 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=55.565 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.888 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.755 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=114.671 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed
```

## After describe-facing corpus overview parity

Added the lightweight Rust equivalent of Python `describe_corpus_overview()`:

```text
Corpus::overview(name)
Corpus::section_types()
CorpusOverview
NodeTypeOverview
```

The overview reports node types from Python-shaped `levels()` data, including
node counts and slot-type marking, and reads section levels from
`otext.sectionTypes` when present. The mini-corpus regression asserts the
`sentence`, `phrase`, and `word` node type rows plus section levels
`sentence,phrase`.

Validation:

```text
cargo test --quiet
30 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.075 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.504 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=631.877 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.139 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=186.299 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=597.504 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.392 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.796 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.970 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.152 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed
```

## After describe-facing feature description parity

Added the lightweight Rust equivalent of Python `describe_feature()`:

```text
Corpus::describe_feature(feature_name, sample_limit)
FeatureDescription
FeatureValueSample
```

The Rust description includes feature kind, `valueType`, description metadata,
node types, unique value count, frequency samples, edge `has_values`, and a
missing-feature error string. Node feature samples are built from
`NodeFeature::frequency_list()`. Valued edge samples are built from
`EdgeFeature::frequency_list()`. Plain edge features report `has_values=false`
with no value samples, matching Python's describe behavior.

Validation:

```text
cargo test --quiet
30 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=143.463 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.105 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=629.529 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.186 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=179.332 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=603.869 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=51.319 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.230 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.362 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=111.056 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed
```

## After results-facing wrapper parity

Added `src/results.rs` with the Rust subset of Python `cfabric.results` that
can be supported by the current corpus API:

```text
NodeInfo
NodeInfoOptions
NodeList
SearchResult
```

`NodeInfo::from_corpus()` records node id, node type, optional non-slot slots,
and optional node feature values. Text and section references are intentionally
empty for now because the Rust port does not yet have the text rendering and
section lookup layer. `NodeList::from_nodes()` and `SearchResult::from_search()`
preserve Python's total-count behavior when a display limit is applied.

Validation:

```text
cargo test --quiet
31 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=142.550 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.679 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=629.185 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.185 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=184.934 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=594.757 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.935 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.774 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.324 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=114.674 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed
```

## After results-facing corpus info parity

Added the Rust equivalent of Python `CorpusInfo.from_api()`:

```text
CorpusInfo
CorpusNodeTypeInfo
CorpusInfo::from_corpus(corpus, name, path)
```

The summary records corpus name/path, Python-shaped node type rows with counts,
average slots, min/max node ids, node and edge feature names, slot type,
`max_slot`, `max_node`, and section types. This reuses `Corpus::levels()` and
`Corpus::section_types()` so it stays aligned with the describe-facing APIs.

Validation:

```text
cargo test --quiet
31 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=149.496 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.577 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=629.445 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.099 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=187.884 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=588.023 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.038 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.929 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.869 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.098 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed
```

## After results-facing feature info parity

Added the Rust equivalent of Python `FeatureInfo.from_api()`:

```text
FeatureInfo
FeatureInfo::from_corpus(corpus, name, kind)
```

The wrapper reports feature name, kind, `valueType`, description metadata, and
edge `has_values` when the requested feature exists for the requested kind. It
returns `None` for missing features or for a kind mismatch, matching the
lightweight optional behavior of Python `FeatureInfo.from_api()`.

Validation:

```text
cargo test --quiet
31 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.305 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.264 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=629.105 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.159 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=187.322 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=600.218 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.807 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.272 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.576 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.225 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed
```

## After basic text rendering parity

Added a first Rust text rendering layer:

```text
Corpus::text(node, format)
NodeInfo::from_corpus(...).text
```

The renderer reads `otext` metadata keys such as `fmt:text-orig-full`, replaces
slot feature placeholders like `{word}`, and supports fallback placeholders such
as BHSA's `{qere_utf8/g_word_utf8}`. Non-slot node text is rendered by walking
the node's slots and concatenating the rendered slot text. This is not yet a
full port of Python `T.text()`, but it covers the core format-substitution path
and removes the empty text field for result wrappers when the required text
features are loaded.

Validation:

```text
cargo test --quiet
32 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=142.257 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.801 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=631.910 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.092 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=181.265 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=603.928 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=54.527 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.033 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.885 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=111.598 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed
```

## After multi-node text rendering parity

Extended the text layer with Python `T.text(iterable)` style behavior:

```text
Corpus::text_nodes(nodes, format)
NodeList::from_nodes(...).text
```

The multi-node renderer preserves caller order and concatenates each node's
rendered text; it does not sort the node list. `NodeList` now carries the text
for the displayed, limit-applied nodes, matching Python result-wrapper total
count versus displayed-node behavior.

Validation:

```text
cargo test --quiet
32 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=139.322 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.357 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=626.647 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.333 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=182.257 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=596.968 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=55.079 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.831 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=26.320 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.909 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed
```

## After valued edge bidirectional precedence parity

Ported another Python `EdgeFeature.b()` edge case into the Rust regression
suite. For valued bidirectional edges, when `1 -> 2` and `2 -> 1` both exist
with different values, the bidirectional lookup for node `1` returns node `2`
with the outgoing edge value. The existing Rust `both_with_values()` behavior
already matched this rule by merging incoming values before outgoing values; the
new test pins that compatibility.

Validation:

```text
cargo test --quiet
29 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=136.968 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.374 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=629.991 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.196 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=184.465 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=595.613 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=51.982 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.856 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=26.099 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.875 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
19 representative curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
19 mapped curated-subset queries passed
```

## After direct search plan display parity

Added a deterministic Rust equivalent for the Python `S.showPlan()` workflow.
`SearchStudy::show_plan(details)` returns a plain-text summary, and materialized
plus mapped search expose direct wrappers:

```text
Search::show_plan(template, details)
Search::show_plan_with_sets(template, sets, details)
MappedSearch::show_plan(template, details)
MappedSearch::show_plan_with_sets(template, sets, details)
```

The summary reports the normalized plan fields already exposed through
`SearchPlanSummary`: template, atom count, relation count, result count, and
whether quantifiers are present. With `details=true`, it also includes the
cached result rows from the studied execution.

Validation:

```text
cargo test --quiet
45 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=148.361 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.760 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=632.751 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=6.385 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=199.104 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=605.100 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=59.788 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.146 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.580 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.390 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After describe text representation parity

Added materialized corpus text representation summaries matching the Python
`describe_text_formats()` data shape:

```text
TextFormatSample
TextFormatInfo
TextRepresentationInfo
Corpus::text_representations()
```

The helper parses `otext` metadata for paired `fmt:*orig*` and `fmt:*trans*`
formats, renders slot samples, de-duplicates repeated originals, and greedily
keeps samples that add new original-script characters. Mini-corpus behavior
matches Python's no-pair message, while BHSA now has explicit test coverage
that the Hebrew `text-orig-full` / transliterated `text-trans-full` pair is
detected and sampled.

Validation:

```text
cargo test --quiet
46 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=142.993 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.765 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=626.076 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.035 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=189.287 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=601.904 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=60.325 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.014 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.016 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=111.982 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After mapped describe text representation parity

Extended the same text-format description surface to compiled mmap-backed
corpora:

```text
MappedText::text_representations()
```

The mapped implementation reads `otext` metadata through the compiled config
feature view and renders samples through `MappedText`, without materializing a
`Corpus`. Fixture coverage now compares mapped and materialized outputs for
both the mini-corpus no-pair case and a synthetic paired original/transliterated
format corpus.

Validation:

```text
cargo test --quiet
47 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=142.755 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.089 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=628.410 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.152 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=192.811 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=601.960 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=58.578 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.070 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.493 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=113.153 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After full corpus description parity

Added a typed materialized equivalent for Python `describe_corpus()`:

```text
CorpusDescription
Corpus::describe_corpus(name)
```

The wrapper combines the already-ported describe surfaces into one discovery
object: node type overview, section types, text representation summary, node
feature catalog, and edge feature catalog. The mini-corpus regression checks
that the full description agrees with `overview()`, carries the no-pair text
format summary, and keeps node and edge feature catalogs separated.

Validation:

```text
cargo test --quiet
47 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=143.080 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.478 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=632.773 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.182 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=193.991 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=591.078 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=58.834 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=74.202 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.456 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=113.502 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After batch describe helper parity

Added typed materialized equivalents for Python's batch describe helper and
node-feature-to-object-type map:

```text
Corpus::describe_features(feature_names, sample_limit)
Corpus::all_node_feature_types()
```

`describe_features()` preserves missing-feature error entries from
`describe_feature()`, and `all_node_feature_types()` returns every loaded node
feature mapped to the node types where it has values. Mini-corpus regression
coverage checks node-type mappings for `word` and `phrase_id`, absence of
missing features, and batch descriptions for node, valued-edge, and missing
features.

Validation:

```text
cargo test --quiet
47 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=140.163 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.411 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=617.565 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.217 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=183.900 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=590.605 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=59.082 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.711 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.203 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=107.863 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After edge feature type discovery parity

Extended describe-facing feature type discovery to cover both ends of edge
features:

```text
Corpus::edge_feature_target_types(feature_name)
Corpus::all_edge_feature_source_types()
Corpus::all_edge_feature_target_types()
```

This complements the existing source-type discovery and aligns with Python edge
helpers that distinguish `nodeTypesFrom` and `nodeTypesTo`. Mini-corpus
regression coverage now checks source and target type discovery for `parent`
and `oslots`, plus missing edge behavior and all-edge source/target maps.

Validation:

```text
cargo test --quiet
47 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=145.378 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.152 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=630.013 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.062 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=193.591 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=608.654 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=59.664 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.062 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.196 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.836 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After corpus frequency alias parity

Added Python-style corpus-level frequency aliases for filtered frequency
queries:

```text
Corpus::node_freq_list(feature_name, node_types)
Corpus::node_freqList(feature_name, node_types)
Corpus::edge_freq_list(feature_name, node_types_from, node_types_to)
Corpus::edge_freqList(feature_name, node_types_from, node_types_to)
```

These delegate to the existing `node_frequency_list()` and
`edge_frequency_list()` implementations, preserving the already-ported
`nodeTypes`, `nodeTypesFrom`, and `nodeTypesTo` filtering behavior while giving
callers ergonomic aliases that mirror the Python API. Fixture coverage now pins
both snake-case and Python-style alias spellings.

Validation:

```text
cargo test --quiet
47 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=142.672 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.668 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=635.045 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.182 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=189.133 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=604.903 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=61.496 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.257 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.599 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=108.957 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After mapped full corpus description parity

Added mmap-backed full corpus description assembly:

```text
CorpusDescription::from_mapped(corpus, text, name)
```

This combines mapped corpus summary metadata, mapped text representation
metadata, and mapped node/edge feature catalog entries into the same
`CorpusDescription` shape returned by `Corpus::describe_corpus()`. The new
fixture regression test compares the mapped description exactly against the
materialized mini-corpus description, without materializing the compiled cache.

Validation:

```text
cargo fmt && cargo test --quiet
48 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=142.976 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.212 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=573.019 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.109 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=176.853 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=575.927 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=56.368 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.928 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.940 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.112 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After loaded feature introspection parity

Added typed equivalents for Python's loaded-feature discovery helpers:

```text
Corpus::all_node_features(warp)
Corpus::Fall(warp)
Corpus::all_edge_features(warp)
Corpus::Eall(warp)
Corpus::all_computed_features()
Corpus::Call()
Corpus::is_loaded(features)
```

`is_loaded()` returns a sorted map of requested feature names to
`LoadedFeatureInfo`, with explicit `LoadedFeatureKind` values for node, edge,
config, and computed data, and `None` for missing or not-loaded names. The
metadata shape mirrors Python's `isLoaded(pretty=False)` behavior by separating
`valueType` into `value_type` and leaving the remaining metadata in the
metadata map. Fixture coverage now checks node, edge, config, computed, WARP
filtering, and missing-feature cases.

Validation:

```text
cargo fmt && cargo test --quiet
48 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=139.395 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.281 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=571.970 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.935 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=173.015 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=568.848 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=56.719 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.997 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=22.988 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=103.028 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After mapped loaded feature introspection parity

Extended the loaded-feature discovery helpers to mmap-backed compiled corpora:

```text
MappedCompiledCorpus::all_node_features(warp)
MappedCompiledCorpus::Fall(warp)
MappedCompiledCorpus::all_edge_features(warp)
MappedCompiledCorpus::Eall(warp)
MappedCompiledCorpus::all_computed_features()
MappedCompiledCorpus::Call()
MappedCompiledCorpus::is_loaded(features)
```

The mapped implementation reads node and edge metadata from the compiled cache
manifest and decodes config metadata through `ConfigFeatureView`, so callers can
inspect loaded feature status without materializing the corpus. Fixture coverage
now compares mapped feature lists and `is_loaded()` maps exactly against the
materialized `Corpus` output, including edge-valued features, config metadata,
computed data, and missing-feature entries.

Validation:

```text
cargo fmt && cargo test --quiet
49 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.149 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.325 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=589.565 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.100 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=175.760 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=561.796 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=56.213 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.048 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.824 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=106.090 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After mapped feature type discovery and catalog parity

Extended describe-facing metadata helpers to mmap-backed compiled corpora:

```text
MappedCompiledCorpus::node_feature_types(feature_name)
MappedCompiledCorpus::edge_feature_source_types(feature_name)
MappedCompiledCorpus::edge_feature_target_types(feature_name)
MappedCompiledCorpus::all_node_feature_types()
MappedCompiledCorpus::all_edge_feature_source_types()
MappedCompiledCorpus::all_edge_feature_target_types()
MappedCompiledCorpus::feature_catalog(kind, node_types)
```

The mapped implementation derives Python-style node-type level ordering from
`otype` and `oslots`, scans node/edge rows through mmap views, and applies the
same source-type filtering semantics as `Corpus::feature_catalog()`. Fixture
coverage now compares mapped type discovery maps and filtered catalogs exactly
against the materialized mini-corpus output without materializing the compiled
cache.

Validation:

```text
cargo fmt && cargo test --quiet
50 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=141.868 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.220 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=565.615 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.167 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=185.836 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=568.909 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=56.239 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.649 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=22.763 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=105.675 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After mapped feature description parity

Extended feature descriptions to mmap-backed compiled corpora:

```text
MappedCompiledCorpus::describe_feature(feature_name, sample_limit)
MappedCompiledCorpus::describe_features(feature_names, sample_limit)
```

The mapped implementation aggregates node feature values through string-pool
and mixed node-feature views, mirrors materialized frequency sorting, and
computes valued edge samples from the compiled edge-value section. Fixture
coverage now compares mapped and materialized descriptions exactly for node
features, unvalued edge features, valued edge features, missing features, and
batch description maps without materializing the compiled cache.

Validation:

```text
cargo fmt && cargo test --quiet
51 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=143.416 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.639 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=575.907 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.418 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=174.542 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=558.999 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=57.899 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.494 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.057 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=106.435 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After mapped frequency list parity

Added mapped corpus-level filtered frequency helpers:

```text
MappedCompiledCorpus::node_frequency_list(feature_name, node_types)
MappedCompiledCorpus::node_freq_list(feature_name, node_types)
MappedCompiledCorpus::node_freqList(feature_name, node_types)
MappedCompiledCorpus::edge_frequency_list(feature_name, node_types_from, node_types_to)
MappedCompiledCorpus::edge_freq_list(feature_name, node_types_from, node_types_to)
MappedCompiledCorpus::edge_freqList(feature_name, node_types_from, node_types_to)
```

The mapped implementation applies the same node-type source/target filters as
the materialized `Corpus` helpers, aggregates node values and valued edge
values through mmap views, and returns `EdgeFrequency::Count` for unvalued edge
features. Feature descriptions now reuse these public mapped frequency helpers
instead of carrying separate private counting logic. Fixture coverage compares
mapped and materialized filtered node and edge frequencies exactly, including
alias methods and missing-feature errors.

Validation:

```text
cargo fmt && cargo test --quiet
52 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=142.296 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.301 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=568.038 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.108 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=181.954 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=552.250 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=56.918 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.419 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=22.399 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.916 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After mapped containment and locality parity

Extended `MappedSections` with mmap-backed containment and basic locality
helpers:

```text
MappedSections::contains(parent, child)
MappedSections::up(node, node_type)
MappedSections::up_types(node, node_types)
MappedSections::down(node, node_type)
MappedSections::down_types(node, node_types)
```

The mapped implementation uses `otype`, `oslots`, and compiled rank arrays to
match materialized containment and canonical ordering without materializing the
compiled cache. A regression caught and fixed the slot-node `down()` case:
`slots()` remains slot-aware for text/section rendering, but `down()` now reads
`oslots` directly so slots have no children, matching `Corpus::down()`. Fixture
coverage compares mapped and materialized containment, up/down locality, and
type-filtered variants exactly.

Validation:

```text
cargo fmt && cargo test --quiet
53 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.766 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.251 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=584.251 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.388 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=177.138 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=560.852 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=57.727 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.250 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.367 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=106.090 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After mapped next/previous locality parity

Completed mapped locality navigation by adding boundary-adjacent helpers:

```text
MappedSections::next(node, node_type)
MappedSections::next_types(node, node_types)
MappedSections::previous(node, node_type)
MappedSections::previous_types(node, node_types)
```

The mapped implementation derives adjacent slots from slot nodes or `oslots`
intervals, finds nodes whose first or last slot touches the adjacent slot, and
uses compiled rank arrays to preserve materialized boundary ordering. Fixture
coverage now compares mapped and materialized `next`/`previous` behavior
exactly for unfiltered, single-type, and multi-type locality calls.

Validation:

```text
cargo fmt && cargo test --quiet
53 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=143.320 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.454 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=574.691 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.272 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=180.263 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=567.986 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=57.900 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.974 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.990 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=105.781 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After mapped walk event parity

Extended `MappedSections` with canonical walk helpers:

```text
MappedSections::all_nodes()
MappedSections::walk(nodes)
MappedSections::walk_events(nodes)
```

The mapped implementation sorts nodes by compiled rank arrays and derives
ending boundary events from `oslots` so `WalkEvent::Start`, `WalkEvent::Slot`,
and `WalkEvent::End` ordering matches `Corpus::walk_events()` without
materializing the compiled cache. Fixture coverage now compares mapped and
materialized all-node order, subset walking, full walk events, and filtered
walk events exactly.

Validation:

```text
cargo fmt && cargo test --quiet
53 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.559 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.567 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=568.299 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.472 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=172.406 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=566.964 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=56.232 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.833 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.186 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=107.666 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After mapped chunk sort-key parity

Extended compiled/mapped ordering helpers with Python-style chunk sort keys:

```text
MappedCompiledCorpus::sort_key_chunk(chunk)
MappedCompiledCorpus::sort_key_chunk_length(chunk)
```

The mapped implementation reuses the persisted rank arrays and derives type
rank from mapped `otype` level metadata so chunk position and length ordering
match materialized `Corpus` ordering. `ChunkPositionKey` and `ChunkLengthKey`
now use internal constructors shared by the materialized and mapped paths,
keeping their comparison semantics centralized. Fixture coverage compares
mapped and materialized sorted chunk sequences exactly.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=131.344 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.821 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=573.987 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.531 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=178.291 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=556.049 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=55.980 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.634 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.325 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.788 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After mapped order/rank parity

Added direct mapped access to persisted ordering arrays:

```text
MappedCompiledCorpus::order()
MappedCompiledCorpus::rank()
MappedCompiledCorpus::sort_key_tuple(nodes)
```

These read the compiled cache's canonical `order` and `rank` arrays directly
from mmap storage and expose tuple sort keys for valid nodes without
materializing the corpus. Fixture coverage compares mapped `order`, `rank`, and
tuple sort keys exactly against the materialized corpus and pins out-of-range
mapped sort keys as `None`.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=141.252 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.539 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=568.973 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.562 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=173.195 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=565.540 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=55.161 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.686 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.853 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=107.471 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After mapped boundary parity

Added direct mapped boundary access:

```text
MappedSections::boundary()
```

The mapped implementation assembles `Boundary { first_slots, last_slots }`
from mmap-backed `otype` and `oslots` views and reuses the same slot-start and
slot-end ordering as mapped locality and walk-event helpers. Fixture coverage
compares the full mapped boundary exactly against `Corpus::boundary()` and pins
representative first/last slot vectors. The parity test caught incorrect
explicit expectations in the new spot checks; the full equality assertion
confirmed the implementation already matched materialized ordering.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=145.109 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.919 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=574.273 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.072 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=172.083 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=564.735 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=55.714 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.459 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.314 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.465 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After mapped edge traversal parity

Extended `EdgeFeatureView` with Python-style traversal helpers:

```text
EdgeFeatureView::forward(node)
EdgeFeatureView::forward_with_values(node)
EdgeFeatureView::backward(node)
EdgeFeatureView::backward_with_values(node)
EdgeFeatureView::both(node)
EdgeFeatureView::b(node)
EdgeFeatureView::both_with_values(node)
EdgeFeatureView::edge_count()
```

The mapped implementation builds on existing mmap-backed `s`, `f`, `t`, and
`edge_value` primitives, preserving materialized target ordering and outgoing
value precedence for bidirectional value rows. Fixture coverage now checks
forward/backward/both traversal, aliases, value-aware traversal, and edge
counts for both unvalued `oslots` and valued `relation`/`distance` edge
features. While writing the tests, fixture reads corrected two expected values:
`relation` has three incoming valued edges for node `6`, and `distance` has a
value on edge `1 -> 3`.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=139.014 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.351 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=568.951 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.884 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=171.369 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=559.255 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=57.401 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.956 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=22.894 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.564 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After mapped feature items parity

Added ordered item collection helpers to mmap-backed feature views:

```text
StringPoolNodeFeatureView::items()
MixedNodeFeatureView::items()
EdgeFeatureView::items()
```

The node feature views now expose node-ordered `(node, value)` rows as mapped
values, while edge feature views expose source-ordered `(source, targets)` rows
with sorted, deduplicated target vectors matching materialized
`EdgeFeature::items()`. Fixture coverage checks string-pool node items, mixed
node items, and exact `oslots` edge items parity against the materialized
corpus.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=143.571 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.543 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=560.978 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.255 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=172.470 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=565.668 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=56.268 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.062 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.443 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=101.555 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
53 mapped curated queries passed
```

## After mapped node selection alias parity

Added Python-style value selection aliases to mmap-backed node feature views:

```text
StringPoolNodeFeatureView::nodes_with_value(value)
StringPoolNodeFeatureView::select(value)
MixedNodeFeatureView::nodes_with_value(value)
MixedNodeFeatureView::select(value)
```

String-pool mapped features now expose direct value-to-node selection from the
encoded value interval index, and mixed mapped features expose the same behavior
for typed mapped values. Fixture coverage checks both canonical and alias calls
for string and integer mapped node values.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=135.187 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.470 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=589.806 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.890 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=184.502 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=561.197 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=56.415 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.456 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.636 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=107.689 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=12.280 nodes=1446831
53 mapped curated queries passed
```

## After dynamic feature accessor parity

Added Python-style dynamic feature accessors:

```text
Corpus::Fs(name)
Corpus::Es(name)
MappedCompiledCorpus::node_feature(name)
MappedCompiledCorpus::Fs(name)
MappedCompiledCorpus::Es(name)
```

Materialized `Fs` and `Es` are thin aliases for existing typed feature lookups.
Mapped node lookup now returns `MappedNodeFeatureView`, an enum over string-pool
and mixed node feature views, with delegated `row_count`, `v`, `s`,
`nodes_with_value`, `select`, `value_interval`, and `items` methods. This gives
callers a single dynamic mapped node-feature surface while preserving the
encoding-specific views for optimized code paths. Fixture coverage checks
present and missing materialized feature aliases, dynamic mapped string and
integer node features, and mapped edge `Es` lookup.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=135.907 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.705 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=571.209 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.379 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=182.722 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=573.424 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=55.892 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.177 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.837 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=103.860 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.902 nodes=1446831
53 mapped curated queries passed
```

## After canonical sortNodes parity

Added Python-style canonical node sorting helpers:

```text
Corpus::sorted_nodes(nodes)
Corpus::sortNodes(nodes)
MappedCompiledCorpus::sort_nodes(nodes)
MappedCompiledCorpus::sorted_nodes(nodes)
MappedCompiledCorpus::sortNodes(nodes)
```

The materialized API keeps the existing in-place `sort_nodes(&mut [u32])` and
adds returning helpers that accept any `IntoIterator<Item = u32>`, matching the
shape of Python `N.sortNodes()`. The mapped implementation sorts through the
persisted rank array in the compiled cache and keeps unknown nodes after known
nodes. Fixture coverage now checks list-like input, map-key set-like input,
empty inputs, single nodes, and mixed node types for both materialized and
mapped corpora.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=143.042 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.572 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=566.649 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.186 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=175.956 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=556.137 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=59.771 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.697 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.000 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=106.720 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=14.336 nodes=1446831
53 mapped curated queries passed
```

## After dynamic computed accessor parity

Added Python-style dynamic computed accessors:

```text
ComputedFeatureData
Corpus::computed_feature(name)
Corpus::Cs(name)
MappedCompiledCorpus::computed_feature(name)
MappedCompiledCorpus::Cs(name)
```

`ComputedFeatureData` currently covers the computed surfaces already ported:
`levels`, `order`, `rank`, and `boundary`. Materialized accessors reuse the
existing in-memory computations. Mapped accessors read persisted order/rank
arrays from the compiled cache, derive levels from mmap-backed `otype`/`oslots`,
and reuse mapped section boundary logic. Fixture coverage checks all four
computed names plus missing-feature behavior, and verifies mapped results match
materialized results exactly.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=148.878 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.592 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=576.087 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.000 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=174.220 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=566.491 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=55.821 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.163 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.649 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.642 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=11.033 nodes=1446831
53 mapped curated queries passed
```

## After text format splitting helper parity

Added Python-style text format helper methods:

```text
Corpus::split_format(template)
Corpus::splitFormat(template)
Corpus::split_default_format(template)
Corpus::splitDefaultFormat(template)
MappedText::split_format(template)
MappedText::splitFormat(template)
MappedText::split_default_format(template)
MappedText::splitDefaultFormat(template)
```

`split_format` recognizes a leading `nodeType#` prefix only when the node type
exists in loaded `otype` data, otherwise it falls back to the slot type and
leaves the template unchanged. `split_default_format` recognizes
`nodeType-default` only for known node types. The mapped implementation derives
the type set from mmap-backed `otype` rows. Fixture coverage checks typed
formats, unknown type prefixes, no-prefix templates, valid default formats, and
invalid default names for both materialized and mapped text APIs.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=139.615 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.719 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=565.986 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.181 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=170.481 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=562.344 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=56.675 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.061 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.481 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.344 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.699 nodes=1446831
53 mapped curated queries passed
```

## After section alias parity

Added Python-style section navigation aliases:

```text
Corpus::sectionTuple(node, options)
Corpus::sectionFromNode(node, options)
Corpus::nodeFromSection(section)
MappedSections::sectionTuple(node, options)
MappedSections::sectionFromNode(node, options)
MappedSections::nodeFromSection(section)
```

These are thin aliases over the existing snake-case section helpers and mirror
the names exposed by Python's `T.sectionTuple`, `T.sectionFromNode`, and
`T.nodeFromSection`. Fixture coverage checks materialized aliases against their
snake-case methods and checks mapped aliases against materialized behavior for
section node tuples, section headings, and section-to-node lookup.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=144.442 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.933 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=624.986 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.820 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=192.467 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=595.214 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=62.165 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.738 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.266 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=112.688 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=11.977 nodes=1446831
53 mapped curated queries passed
```

## After locality next/previous alias parity

Added Python-style short locality aliases:

```text
Corpus::n(node, node_type)
Corpus::p(node, node_type)
MappedSections::n(node, node_type)
MappedSections::p(node, node_type)
```

These mirror Python `L.n()` and `L.p()` for next and previous locality
navigation and delegate to the existing materialized and mapped `next` and
`previous` implementations. Fixture coverage checks unfiltered and filtered
aliases against the canonical methods for both materialized and mmap-backed
section/locality APIs.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=142.750 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.346 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=594.211 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.226 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=171.310 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=568.011 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=51.383 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=66.618 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=20.794 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.161 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.456 nodes=1446831
53 mapped curated queries passed
```

## After intersecting locality parity

Added Python-style intersecting locality navigation:

```text
Corpus::intersecting(node, node_type)
Corpus::intersecting_types(node, node_types)
Corpus::i(node, node_type)
MappedSections::intersecting(node, node_type)
MappedSections::intersecting_types(node, node_types)
MappedSections::i(node, node_type)
```

This ports the core behavior of Python `L.i()`: slot nodes return no
intersectors, non-slot nodes return all other nodes that share at least one
slot, optional type filters include slot nodes only when the slot type is
requested, and results are canonical sorted. The mapped implementation computes
the same result from mmap-backed `otype` and `oslots` data. Fixture coverage
checks unfiltered, single-type, multi-type, and slot-node cases, plus mapped
parity against materialized results.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=147.876 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.282 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=600.726 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.128 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=178.700 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=574.214 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=50.806 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.216 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.032 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=106.200 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=13.552 nodes=1446831
53 mapped curated queries passed
```

## After locality up/down alias parity

Added Python-style short locality aliases:

```text
Corpus::u(node, node_type)
Corpus::d(node, node_type)
MappedSections::u(node, node_type)
MappedSections::d(node, node_type)
```

These mirror Python `L.u()` and `L.d()` for upward and downward locality
navigation and delegate to the existing materialized and mapped `up` and `down`
implementations. Fixture coverage checks filtered and unfiltered aliases
against the canonical methods for both materialized and mmap-backed
section/locality APIs.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=143.598 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.826 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=616.553 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.050 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=178.573 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=574.165 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=51.893 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.780 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.051 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=107.837 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=15.935 nodes=1446831
53 mapped curated queries passed
```

## After Fabric workflow alias parity

Added Python-style workflow aliases to the Rust `Fabric` facade:

```text
Fabric::loadAll()
Fabric::loadCompiled(cache_path)
Fabric::openMapped(cache_path)
```

These delegate to the existing snake-case `load_all`, `load_compiled`, and
`open_mapped` workflow methods. Fixture coverage now exercises the aliases in
the same facade test that explores, loads, compiles, materializes a compiled
cache, opens a mapped cache, and queries through mapped search.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=136.203 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.102 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=613.280 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.082 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=178.935 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=583.756 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.062 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.414 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.117 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.002 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=14.353 nodes=1446831
53 mapped curated queries passed
```

## After search showPlan alias parity

Added Python-style search plan display aliases:

```text
Search::showPlan(template, details)
MappedSearch::showPlan(template, details)
SearchStudy::showPlan(details)
```

These delegate to the existing deterministic `show_plan` implementations for
materialized search, mapped search, and studied search results. Fixture coverage
checks both direct search aliases and study aliases in the materialized and
mapped studied-search workflows.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=139.641 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.277 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=614.677 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.268 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=172.535 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=572.580 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.416 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.715 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.195 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.180 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=11.218 nodes=1446831
53 mapped curated queries passed
```

## After direct search fetch/count workflow parity

Added direct search workflow helpers:

```text
Search::fetch(template, limit)
Search::count(template, limit)
MappedSearch::fetch(template, limit)
MappedSearch::count(template, limit)
```

Python `S.fetch()` and `S.count()` operate after a prior `study()` call. The
Rust search wrappers are stateless, so these helpers perform the equivalent
observable workflow by studying the provided template and immediately fetching
or counting from the resulting `SearchStudy`. Fixture coverage checks direct
materialized and mapped fetch/count behavior against the existing studied
workflow.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=144.518 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.716 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=620.290 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.157 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=175.877 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=568.044 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.657 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.227 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.028 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=106.899 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.200 nodes=1446831
53 mapped curated queries passed
```

## After custom-set fetch/count workflow parity

Added direct custom-set search workflow helpers:

```text
Search::fetch_with_sets(template, sets, limit)
Search::count_with_sets(template, sets, limit)
MappedSearch::fetch_with_sets(template, sets, limit)
MappedSearch::count_with_sets(template, sets, limit)
```

These complement the existing `search_with_sets`, `study_with_sets`,
`search_first_nodes_with_sets`, and prefix projection helpers. They reuse the
same stateless Rust workflow as direct `fetch`/`count`: study the provided
template with the provided custom sets, then fetch or count from the resulting
`SearchStudy`. Fixture coverage checks materialized and mapped behavior against
existing custom-set study results, including limited fetch/count behavior.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=146.178 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.629 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=607.386 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.152 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=174.717 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=572.641 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.952 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.096 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.780 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.925 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.511 nodes=1446831
53 mapped curated queries passed
```

## After search projection alias parity

Added Python-style aliases for materialized and mapped direct search projection
and custom-set plan display helpers:

```text
Search::searchFirstNodes(template, limit)
Search::searchFirstNodesWithSets(template, sets, limit)
Search::searchPrefixes(template, width, limit)
Search::searchPrefixesWithSets(template, sets, width, limit)
Search::showPlanWithSets(template, sets, details)
MappedSearch::searchFirstNodes(template, limit)
MappedSearch::searchFirstNodesWithSets(template, sets, limit)
MappedSearch::searchPrefixes(template, width, limit)
MappedSearch::searchPrefixesWithSets(template, sets, width, limit)
MappedSearch::showPlanWithSets(template, sets, details)
```

These aliases forward to the canonical snake-case helpers and keep the existing
stateless Rust search semantics. Fixture coverage compares alias output against
canonical helper output for materialized and mapped search, including custom-set
first-node and prefix projections.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=143.018 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.808 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=616.687 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=3.493 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=174.789 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=567.059 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.525 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.699 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=20.466 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=103.934 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=15.031 nodes=1446831
53 mapped curated queries passed
```

## After Text section and structure metadata alias parity

Added parsed Text API metadata helpers and Python-style aliases on materialized
and mapped section surfaces:

```text
Corpus::sectionTypes()
Corpus::sectionFeatures()
Corpus::structure_types()
Corpus::structureTypes()
Corpus::structure_features()
Corpus::structureFeatures()
MappedSections::sectionTypes()
MappedSections::sectionFeatures()
MappedSections::structure_types()
MappedSections::structureTypes()
MappedSections::structure_features()
MappedSections::structureFeatures()
```

The materialized `section_types()` helper now uses the same CSV parser as the
other `otext` metadata helpers. Fixture coverage checks canonical and
camel-case aliases for section metadata and empty structure metadata on both
materialized and mapped corpus paths.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.293 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.125 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=598.379 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.046 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=174.746 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=578.740 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.357 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.243 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.017 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=107.560 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=11.229 nodes=1446831
53 mapped curated queries passed
```

## After parameterized near relation parity

Added Python-style parameterized near relation operators to materialized and
mapped search:

```text
=k:  near first slot
:k=  near last slot
:k:  near first and last boundary
<k:  near before
:k>  near after
```

The Rust query parser accepts concrete numeric operator tokens such as `=2:`,
`:2=`, `:2:`, `<0:`, and `:0>`. Fixture coverage exercises every operator on
both materialized and mapped mini-corpus searches, including a negative case for
too-small first-slot distance.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=135.986 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.387 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=619.128 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.166 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=173.139 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=583.088 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.623 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.260 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.789 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.017 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=13.983 nodes=1446831
53 mapped curated queries passed
```

## After exact boundary relation parity

Added Python's exact same-boundary relation operator to materialized and mapped
search:

```text
::  left and right start and end at the same slot
```

This complements the parameterized `:k:` near-boundary operator added in the
previous slice. Fixture coverage checks `p1 :: p2` on the mini corpus in both
materialized and mapped search paths, where only identical phrases share both
boundaries.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=149.919 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.182 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=612.216 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.151 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=175.687 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=577.397 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=54.076 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.059 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.191 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=101.897 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=9.676 nodes=1446831
53 mapped curated queries passed
```

## After regex-normalized feature relation parity

Added Python's `.f~r~g.` feature relation form to materialized and mapped
search. The relation removes the regex pattern from both string feature values
and compares the normalized strings:

```text
w1 .word~.+~word. w2
```

Fixture coverage checks a positive case where stripping `.+` from both words
leaves equal empty strings, and a negative case where stripping only `^h` from
`hello` does not normalize it to `world`. The implementation returns no match
for non-string feature values, matching Python's string-feature validation
intent for this relation family.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=142.629 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.153 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=608.881 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=3.481 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=172.364 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=578.208 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=50.844 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.648 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.624 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=106.367 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=14.358 nodes=1446831
53 mapped curated queries passed
```

## After node equality relation parity

Added Python's node identity equality relation to materialized and mapped search:

```text
=  left equal to right as a node
```

Rust already supported `#` for node inequality. Fixture coverage now checks
`p1 = p2` over the mini corpus in both search paths, yielding only identical
phrase node pairs `6,6` and `7,7`.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=146.879 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.730 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=604.198 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=3.429 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=176.766 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=579.226 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.842 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.012 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.441 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.109 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=14.128 nodes=1446831
53 mapped curated queries passed
```

## After bidirectional edge relation parity

Added Python's bidirectional edge relation operator to materialized and mapped
search:

```text
<edge>
<edge=value>
```

The relation matches if the named edge exists in either direction between the
two named nodes. The valued form checks the edge value in whichever direction
matches. Fixture coverage checks both `<parent>` and `<relation=subject>` in
reverse phrase-to-word query order, proving the operator is not just a synonym
for forward traversal.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=139.552 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.669 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=608.418 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=3.287 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=174.223 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=584.289 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=54.367 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.537 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.135 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.845 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=14.113 nodes=1446831
53 mapped curated queries passed
```

## After explicit atom operator parity

Added Python-style operator-before-atom search syntax to materialized and
mapped search:

```text
[[ atom
]] atom
```

Default indentation and explicit `[[` preserve parent-contains-child behavior.
Explicit `]]` flips containment so the indented atom must contain the nearest
less-indented parent. Mapped search also updates candidate preselection for
`]]` by selecting nodes that contain the parent slot interval.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=140.312 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.100 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=616.433 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=3.244 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=175.796 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=580.875 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.785 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.378 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.017 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=108.015 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=15.802 nodes=1446831
53 mapped curated queries passed
```

## After escaped valued edge and mapped alternative parity

Closed another query parser gap around escaped literal values. Materialized
search already handled escaped node constraint alternatives, but valued edge
relations kept raw backslashes. Mapped search also split equality alternatives
with plain `split('|')`, so `word=a\|b` was incorrectly interpreted as two
alternatives.

The parser now:

- unescapes materialized valued edge relation values for `-edge=value>`,
  `<edge=value-`, and `<edge=value>`;
- uses escape-aware alternative splitting for mapped equality and inequality
  constraints;
- preserves the existing Python-style escapes for `\|`, `\=`, and `\\`.

Fixture coverage exercises escaped node values and escaped valued edge
relations in both materialized and mapped search.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=146.948 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.412 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=639.426 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=3.501 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=188.362 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=596.804 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.794 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.106 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.760 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=108.870 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.376 nodes=1446831
53 mapped curated queries passed
```

Current expanded Python-vs-Rust comparison:

```text
python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=74.40x python_ms=750.353 rust_ms=10.086
query_geomean_speedup=6.98x queries=53
slowest relative Rust query: complex_017 speedup=1.04x python_ms=77.073 rust_ms=74.039
all 53 measured BHSA mapped curated queries were faster than Python
```

## After mapped sInterval alias parity

Added Python-style `sInterval()` aliases for mapped node feature interval
lookups:

- `MappedNodeFeatureView::sInterval()`
- `StringPoolNodeFeatureView::sInterval()`
- `MixedNodeFeatureView::sInterval()`

These delegate to the existing vector-free `value_interval()` implementations.
Fixture coverage pins the alias on direct `otype`, dynamic string-pool, and
mixed integer feature views.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.047 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=33.461 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=636.164 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.048 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=187.016 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=601.522 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=60.386 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.329 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.970 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.193 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.202 nodes=1446831
53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=27.11x python_ms=382.564 rust_ms=14.113
query_geomean_speedup=6.93x queries=53
slowest relative Rust query: complex_017 speedup=1.04x python_ms=78.396 rust_ms=75.580
all 53 measured BHSA mapped curated queries were faster than Python
```

## After navigation alias parity

Added more Python-style navigation aliases:

- `Corpus::allNodes()`
- `Corpus::walkEvents()`
- `Corpus::sortKeyChunk()` / `Corpus::sortKeyChunkLength()`
- `MappedCompiledCorpus::sortKeyChunk()` /
  `MappedCompiledCorpus::sortKeyChunkLength()`
- `MappedSections::allNodes()`
- `MappedSections::walkEvents()`

These delegate to existing typed Rust helpers and are covered beside the
existing materialized and mapped canonical walking and chunk-sort tests.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.398 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.144 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=628.545 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.453 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=189.407 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=593.496 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=58.605 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=75.430 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.368 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=107.985 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.084 nodes=1446831
53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=40.89x python_ms=458.466 rust_ms=11.211
query_geomean_speedup=7.08x queries=53
slowest relative Rust query: complex_017 speedup=1.10x python_ms=80.456 rust_ms=73.383
all 53 measured BHSA mapped curated queries were faster than Python
```

## After sort key alias parity

Added Python-style node ordering aliases for materialized and mapped corpora:

- `Corpus::sortKey()` / `Corpus::sortKeyTuple()`
- `MappedCompiledCorpus::sortKey()` / `MappedCompiledCorpus::sortKeyTuple()`

The aliases delegate to the existing typed snake-case sort-key APIs and are
covered beside the existing canonical ordering tests. This brings the Rust
navigation surface closer to Python's `N.sortKey()` and `N.sortKeyTuple()`
without changing the underlying rank representation.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=140.662 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.335 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=613.765 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.730 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=194.947 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=592.177 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=57.903 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.274 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.871 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=111.808 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.054 nodes=1446831
53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=72.50x python_ms=785.424 rust_ms=10.834
query_geomean_speedup=6.77x queries=53
slowest relative Rust query: complex_017 speedup=1.04x python_ms=77.459 rust_ms=74.227
all 53 measured BHSA mapped curated queries were faster than Python
```

## After otype rank API parity

Added public node-type rank helpers for materialized and mapped corpora:

- `Corpus::otype_rank()` / `Corpus::otypeRank()`
- `MappedCompiledCorpus::otype_rank()` / `MappedCompiledCorpus::otypeRank()`

The rank values expose the same canonical node-type ordering already used by
`sortNodes()` and chunk sort keys. The slot type has rank `0`, with larger
container types ranked after it. Mapped ranks are derived from compiled
`levels()` and now have fixture parity against the materialized mini corpus.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=140.774 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.976 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=632.566 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.036 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=194.078 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=588.409 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=59.083 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.489 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.888 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.079 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.347 nodes=1446831
53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=49.47x python_ms=699.091 rust_ms=14.132
query_geomean_speedup=7.05x queries=53
slowest relative Rust query: complex_017 speedup=1.06x python_ms=79.164 rust_ms=74.699
all 53 measured BHSA mapped curated queries were faster than Python
```

## After mapped edge value introspection parity

Added mapped compiled edge accessors for value metadata:

- `EdgeFeatureView::edge_value_count()` reports the number of stored non-empty
  edge values in the compiled value section;
- `EdgeFeatureView::has_edge_values()` reports whether any stored edge values
  are available.

This mirrors the materialized `EdgeFeature::has_edge_values()` surface without
changing Rust's typed traversal split between target-only methods and
`*_with_values()` methods. Fixture coverage pins unvalued `oslots`, string
valued `relation`, and integer valued `distance`, including the distinction
between stored non-empty values and blank valued-edge rows that return `None`.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=145.760 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.541 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=636.461 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.375 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=198.059 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=596.609 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=58.673 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.482 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.339 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.376 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.549 nodes=1446831
53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=68.92x python_ms=759.421 rust_ms=11.019
query_geomean_speedup=6.94x queries=53
slowest relative Rust query: complex_017 speedup=1.07x python_ms=80.132 rust_ms=74.613
all 53 measured BHSA mapped curated queries were faster than Python
```

## After mapped text alias parity

Added a Python-style `MappedText::textNodes()` alias for the existing mapped
compiled multi-node text renderer. Rust cannot overload `text()` the way Python
accepts either a single node or an iterable, so this keeps the public mapped
text surface aligned with the project's established alias pattern while
retaining explicit Rust types.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=148.610 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.806 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=635.144 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.410 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=200.546 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=618.268 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=59.597 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.670 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.746 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=115.228 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.318 nodes=1446831
53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=35.29x python_ms=514.176 rust_ms=14.571
query_geomean_speedup=6.97x queries=53
slowest relative Rust query: complex_017 speedup=1.03x python_ms=78.000 rust_ms=75.540
all 53 measured BHSA mapped curated queries were faster than Python
```

## After isLoaded alias parity

Added Python-style loaded feature introspection aliases:

- `Corpus::isLoaded()`
- `MappedCompiledCorpus::isLoaded()`

These delegate to the typed `is_loaded()` APIs and preserve sorted map output,
including requested missing-feature entries. The existing materialized and
mapped loaded-feature tests now assert both spellings return identical results
for explicit feature lists and full loaded-feature introspection.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=139.603 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.856 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=628.309 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.304 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=195.381 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=602.218 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=60.229 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.094 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.370 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.493 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.062 nodes=1446831
53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=37.49x python_ms=382.834 rust_ms=10.212
query_geomean_speedup=7.01x queries=53
slowest relative Rust query: complex_017 speedup=1.04x python_ms=76.394 rust_ms=73.519
all 53 measured BHSA mapped curated queries were faster than Python
```

## After feature view valueType alias parity

Added Python-style `valueType()` aliases on materialized feature views:

- `NodeFeature::valueType()`
- `EdgeFeature::valueType()`

These are thin aliases over the existing typed `value_type()` helpers and read
the original `@valueType` metadata from the feature's `meta` map. Fixture
coverage pins both node and edge feature aliases against the mini corpus.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.820 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=34.292 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=615.308 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.429 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=181.920 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=592.832 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=58.662 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.876 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.129 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=108.015 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.705 nodes=1446831
53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=31.83x python_ms=408.399 rust_ms=12.832
query_geomean_speedup=7.00x queries=53
slowest relative Rust query: complex_017 speedup=1.01x python_ms=75.880 rust_ms=74.919
all 53 measured BHSA mapped curated queries were faster than Python
```

## After materialized feature data accessor parity

Added explicit Rust accessors for the Python feature data surface:

- `NodeFeature::data()`
- `EdgeFeature::data()`
- `EdgeFeature::data_inv()`
- `EdgeFeature::dataInv()`

`data()` returns the underlying materialized feature mapping by reference.
`data_inv()` and the Python-style `dataInv()` alias return the canonical
inverse edge map. This ports the Python unit-test expectation that feature
objects expose their raw forward data and that edge features expose inverse
data for incoming traversal. Fixture coverage now pins node data access,
edge forward data access, and both inverse spellings against the mini corpus.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=143.960 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=32.169 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=622.553 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.515 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=195.130 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=600.518 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=60.891 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.574 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.535 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=111.405 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=9.793 nodes=1446831
53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=68.64x python_ms=698.777 rust_ms=10.181
query_geomean_speedup=6.97x queries=53
slowest relative Rust query: complex_017 speedup=1.05x python_ms=76.789 rust_ms=73.098
all 53 measured BHSA mapped curated queries were faster than Python
```

## After WARP extent alias parity

Added Python-style WARP extent accessors on materialized and mapped corpus
handles:

- `Corpus::max_slot()` / `maxSlot()`
- `Corpus::max_node()` / `maxNode()`
- `Corpus::slot_type()` / `slotType()`
- `MappedCompiledCorpus::max_slot()` / `maxSlot()`
- `MappedCompiledCorpus::max_node()` / `maxNode()`
- `MappedCompiledCorpus::slot_type()` / `slotType()`

The materialized accessors expose the existing typed fields. The mapped
accessors derive `slotType` and `maxSlot` from the mmap-backed `otype` feature
and use the compiled rank length for `maxNode`, avoiding cache materialization.
Fixture coverage pins both spellings against the mini corpus and compares
mapped values to the parsed corpus.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=143.384 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.258 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=615.801 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.774 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=191.611 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=603.387 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=58.892 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=74.845 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.303 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=112.304 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.596 nodes=1446831
53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=77.06x python_ms=714.214 rust_ms=9.268
query_geomean_speedup=7.06x queries=53
slowest relative Rust query: complex_017 speedup=1.06x python_ms=77.616 rust_ms=73.233
all 53 measured BHSA mapped curated queries were faster than Python
```

## After explore category helper parity

Added typed category helpers to `FeatureInventory`:

- `nodes()`
- `edges()`
- `configs()`
- `categories()`

`categories()` returns a sorted `BTreeMap<String, Vec<String>>` keyed by the
same category names used by Python `Fabric.explore()`: `nodes`, `edges`, and
`configs`. This preserves the existing typed fields while making the Rust
explore result consumable in the Python-style category shape. Fixture coverage
now checks both slice accessors and category-map entries for the mini corpus.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=142.574 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.472 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=629.312 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.023 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=191.050 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=592.929 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=60.464 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.734 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.927 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.370 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=9.998 nodes=1446831
53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=69.10x python_ms=712.358 rust_ms=10.309
query_geomean_speedup=7.12x queries=53
slowest relative Rust query: complex_017 speedup=1.08x python_ms=80.027 rust_ms=74.086
all 53 measured BHSA mapped curated queries were faster than Python
```

## After multi-type locality alias parity

Added explicit multi-type locality aliases for the compact Python-style
navigation surface:

- `Corpus::u_types()`
- `Corpus::d_types()`
- `MappedSections::u_types()`
- `MappedSections::d_types()`

Python accepts a set of node types through `L.u()` and `L.d()`. Rust keeps the
single-type `u()` / `d()` signatures typed and explicit, so these new aliases
delegate to the existing `up_types()` and `down_types()` multi-type helpers
without changing return types. Fixture coverage now pins materialized and
mapped aliases against the canonical multi-type implementations.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=146.321 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.726 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=628.241 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.033 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=195.149 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=591.084 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=61.234 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.643 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.484 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.862 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=9.712 nodes=1446831
53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=43.22x python_ms=618.936 rust_ms=14.320
query_geomean_speedup=7.00x queries=53
slowest relative Rust query: complex_017 speedup=1.04x python_ms=77.347 rust_ms=74.731
all 53 measured BHSA mapped curated queries were faster than Python
```

## After value-aware edge alias parity

Added compact value-aware edge traversal aliases on materialized and mapped
edge feature views:

- `EdgeFeature::f_with_values()`
- `EdgeFeature::t_with_values()`
- `EdgeFeature::b_with_values()`
- `EdgeFeatureView::f_with_values()`
- `EdgeFeatureView::t_with_values()`
- `EdgeFeatureView::b_with_values()`

Python's compact `f()`, `t()`, and `b()` methods return `(node, value)` pairs
for valued edges. Rust keeps the existing `f()` / `t()` / `b()` methods typed
as node-only traversal, and exposes value-bearing traversal explicitly through
these aliases. They delegate to the existing `forward_with_values()`,
`backward_with_values()`, and `both_with_values()` helpers. Fixture coverage
pins materialized string-valued, materialized integer-valued, mapped unvalued,
and mapped valued edge cases.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=136.494 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.298 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=622.281 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.477 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=190.547 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=602.390 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=59.787 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=74.591 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.637 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.592 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=9.945 nodes=1446831
53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=66.03x python_ms=607.074 rust_ms=9.194
query_geomean_speedup=6.88x queries=53
slowest relative Rust query: complex_017 speedup=1.01x python_ms=76.623 rust_ms=75.512
all 53 measured BHSA mapped curated queries were faster than Python
```

## After search glean helper parity

Added lightweight result tuple rendering helpers:

- `Search::glean()`
- `MappedSearch::glean()`

These mirror Python `S.glean()` at a typed Rust API level. Empty result tuples
render as an empty string; non-empty tuples render through the default text
format using the existing materialized or mapped text layer. Fixture coverage
checks empty tuples, single-node tuples, and multi-node tuples for both
materialized and mapped mini-corpus search contexts.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=141.556 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=32.461 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=624.158 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.155 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=194.680 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=596.587 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=58.928 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=75.943 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.557 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.290 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.059 nodes=1446831
53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=69.80x python_ms=653.566 rust_ms=9.364
query_geomean_speedup=6.96x queries=53
slowest relative Rust query: complex_017 speedup=1.03x python_ms=76.333 rust_ms=73.933
all 53 measured BHSA mapped curated queries were faster than Python
```

## After mapped named-atom grammar parity

Ported two Python search grammar conveniences into the mapped search parser:

- relation lines are now resolved after parsing all atoms, so a relation may
  appear before the named atom definitions it references;
- later `name constraint...` lines now extend the constraints for an existing
  named atom instead of being treated as a new node type or custom set.

This aligns mapped search with the existing materialized parser behavior and
with the Python integration query shape:

```text
w1 < w2
w1:word
w2:word
w1 word=hello
w2 word=world
```

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=146.417 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=32.813 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=634.006 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.834 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=196.390 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=620.276 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=59.250 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.723 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=26.085 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=114.787 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.607 nodes=1446831
53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=56.87x python_ms=574.550 rust_ms=10.103
query_geomean_speedup=7.02x queries=53
slowest relative Rust query: complex_017 speedup=1.00x python_ms=76.230 rust_ms=76.035
all 53 measured BHSA mapped curated queries were faster than Python
```

## After result serialization helper parity

Added serde-backed serialization support for result-facing structures:

- `NodeInfo::to_dict()`
- `NodeList::to_dict()`
- `SearchResult::to_dict()`
- `FeatureInfo::to_dict()`
- `CorpusInfo::to_dict()`

This ports the Python result-wrapper expectation that `to_dict()` output is JSON
serializable. `FeatureValue` serializes as a plain string or integer, and
`FeatureKind` / `LoadedFeatureKind` serialize as lowercase strings. Fixture
coverage checks node, node-list, search-result, feature-info, and corpus-info
JSON values and runs them through `serde_json::to_string()`.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=149.172 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.484 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=619.917 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.515 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=192.644 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=590.783 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=59.882 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.391 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.000 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=113.370 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.410 nodes=1446831
53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=46.06x python_ms=660.963 rust_ms=14.349
query_geomean_speedup=6.91x queries=53
slowest relative Rust query: complex_017 speedup=1.02x python_ms=75.943 rust_ms=74.714
all 53 measured BHSA mapped curated queries were faster than Python
```

## After describe serialization helper parity

Added Python-shaped `to_dict()` helpers for describe-facing structures:

- `CorpusOverview::to_dict()`
- `CorpusDescription::to_dict()`
- `NodeTypeOverview::to_dict()`
- `FeatureCatalogEntry::to_dict()`
- `FeatureDescription::to_dict()`
- `TextFormatSample::to_dict()`
- `TextFormatInfo::to_dict()`
- `TextRepresentationInfo::to_dict()`
- `FeatureValueSample::to_dict()`

These helpers intentionally do not expose raw Rust field names where Python
uses a different dictionary shape. Node type entries use `type`, overview and
full descriptions wrap section levels under `sections.levels`, full corpus
descriptions expose node features as `features`, and text format dictionaries
use `original_script` / `transliteration`. Missing feature descriptions return
the compact Python-style error dictionary without `node_types` or
`sample_values`.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=139.365 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.718 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=585.222 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.596 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=175.535 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=544.967 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=56.454 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.576 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.658 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=105.480 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.307 nodes=1446831
all 53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=93.06x python_ms=869.416 rust_ms=9.343
query_geomean_speedup=7.18x queries=53
slowest relative Rust query: complex_017 speedup=1.03x python_ms=76.589 rust_ms=74.350
```

## After nested quantified block parity

Ported fixture-level nested quantified block support into both materialized and
mapped search.

Previously both parsers rejected a quantifier token inside an active quantified
block. The parser now tracks nested quantifier depth so an outer alternative can
contain a full nested quantified template. Materialized search compiles
quantified alternatives into either a normal `QueryPlan` or a recursive
quantified template and evaluates nested blocks against the first node of the
contained base match. Mapped search mirrors that shape with
`MappedQuantifierAlternative`, retaining the existing precomputed path for
simple alternatives and using a recursive mapped fallback for nested quantified
alternatives.

Fixture coverage now includes:

```text
sentence
/with/
  phrase
  /with/
    word word=hello
  /-/
/-/
```

The query returns sentence `8` in both materialized and mapped mini-corpus
search.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=137.989 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.514 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=571.202 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.406 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=171.843 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=563.210 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=57.938 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.665 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.550 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=106.288 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.529 nodes=1446831
all 53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=76.10x python_ms=749.492 rust_ms=9.849
query_geomean_speedup=7.04x queries=53
slowest relative Rust query: complex_017 speedup=1.05x python_ms=79.917 rust_ms=76.306
```

## After enforced performance comparison gates

Strengthened `scripts/compare_bhsa_mapped_subset.py` so the Python-vs-Rust BHSA
comparison is an actual validation gate. The script now fails when:

- Rust load speedup is below `--min-load-speedup` (default `2.0x`);
- Rust query geometric-mean speedup is below `--min-query-geomean-speedup`
  (default `2.0x`);
- any shared query is slower in Rust, unless `--allow-slower-query` is passed.

This makes the objective's performance requirement directly executable instead
of relying on a human reading the printed timing table.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=133.249 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.587 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=565.536 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.102 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=167.012 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=541.079 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.942 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.061 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.328 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=103.692 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=11.042 nodes=1446831
all 53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=41.26x python_ms=380.575 rust_ms=9.223
query_geomean_speedup=6.91x queries=53
slowest relative Rust query: complex_017 speedup=1.02x python_ms=77.311 rust_ms=75.745
```

## After search relation legend parity

Added a shared supported-relation legend for the Rust search surface:

- `search::relations_legend()`
- `Search::relations_legend()`
- `Search::relationsLegend()`
- `MappedSearch::relations_legend()`
- `MappedSearch::relationsLegend()`

This ports the Python `S.relationsLegend()` discoverability surface in a typed
Rust form. The legend documents node identity/order relations, slot-set
relations, containment operators, parameterized near operators, feature-to-
feature relations, and edge / valued-edge relations. Materialized and mapped
search use the same static text so the public relation surface stays consistent.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=134.159 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.223 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=576.312 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.100 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=160.671 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=537.772 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=48.828 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.762 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.315 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=97.089 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.308 nodes=1446831
all 53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=26.90x python_ms=384.286 rust_ms=14.288
query_geomean_speedup=6.93x queries=53
slowest relative Rust query: complex_017 speedup=1.04x python_ms=79.131 rust_ms=76.062
```

## After materialized edge value introspection parity

Aligned materialized edge feature introspection with the mapped edge view:

- `EdgeFeature::edge_value_count()`
- `EdgeFeature::hasEdgeValues()`

`edge_value_count()` returns the number of stored valued-edge entries, while
`hasEdgeValues()` is the Python-style alias for `has_edge_values()`. Fixture
coverage checks direct `.tf` parsing, loaded unvalued `parent` edges, and loaded
valued `relation` edges.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=137.560 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.341 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=588.216 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.392 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=164.671 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=538.442 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.538 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.858 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.047 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=101.883 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=9.772 nodes=1446831
all 53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=26.33x python_ms=389.859 rust_ms=14.809
query_geomean_speedup=6.88x queries=53
slowest relative Rust query: complex_017 speedup=1.02x python_ms=77.206 rust_ms=75.551
```

## After mapped edge value introspection alias parity

Added the Python-style mapped edge introspection alias:

- `EdgeFeatureView::hasEdgeValues()`

This aligns the mapped edge view with materialized `EdgeFeature::hasEdgeValues()`
and keeps valued-edge introspection consistent across storage modes. Fixture
coverage checks unvalued mapped `oslots` and valued mapped `relation` /
`distance` features.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=133.439 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.317 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=572.512 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.245 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=163.908 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=543.388 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.604 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.590 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.611 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=98.303 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=9.706 nodes=1446831
all 53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=24.39x python_ms=366.527 rust_ms=15.025
query_geomean_speedup=6.84x queries=53
slowest relative Rust query: complex_017 speedup=1.02x python_ms=76.505 rust_ms=75.284
```

## After mapped node feature metadata helper parity

Added materialized-style metadata helpers to mapped node feature views:

- `MappedNodeFeatureView::meta()`
- `MappedNodeFeatureView::metadata_value()`
- `MappedNodeFeatureView::value_type()`
- `MappedNodeFeatureView::valueType()`
- `MappedNodeFeatureView::description()`
- `StringPoolNodeFeatureView::{meta, metadata_value, value_type, valueType, description}`
- `MixedNodeFeatureView::{meta, metadata_value, value_type, valueType, description}`

The mapped views now retain cloned compiled metadata alongside their mmap-backed
payload offsets. This keeps string-pool, mixed, and dynamic mapped node feature
views aligned with materialized `NodeFeature` metadata access without changing
the compiled cache format.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=135.428 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.594 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=622.864 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.194 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=166.106 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=558.676 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.662 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.037 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.650 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=99.815 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.194 nodes=1446831
all 53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=28.60x python_ms=407.596 rust_ms=14.253
query_geomean_speedup=6.96x queries=53
slowest relative Rust query: complex_017 speedup=1.05x python_ms=78.499 rust_ms=75.041
```

## After mapped edge feature metadata helper parity

Added materialized-style metadata helpers to mapped edge feature views:

- `EdgeFeatureView::meta()`
- `EdgeFeatureView::metadata_value()`
- `EdgeFeatureView::value_type()`
- `EdgeFeatureView::valueType()`
- `EdgeFeatureView::description()`

Mapped edge views now retain cloned compiled metadata alongside their mmap row
offsets and edge-value offsets. This aligns mapped `oslots`, unvalued edges,
and valued edges with the materialized `EdgeFeature` metadata surface without
changing the compiled cache format.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=134.772 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.800 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=574.000 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.175 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=164.573 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=536.791 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.011 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.795 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.578 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.035 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=9.671 nodes=1446831
all 53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=38.70x python_ms=389.458 rust_ms=10.064
query_geomean_speedup=7.14x queries=53
slowest relative Rust query: complex_017 speedup=1.07x python_ms=82.013 rust_ms=76.483
```

## After mapped config metadata helper parity

Added config metadata helpers to mapped config feature views:

- `ConfigFeatureView::metadata_value()`
- `ConfigFeatureView::meta()`

`metadata_value()` provides direct string-value lookup for config metadata, and
`meta()` is an alias for the owned metadata map returned by `metadata()`. This
aligns mapped config features with the materialized metadata access pattern
while preserving mmap-backed iteration for the underlying rows.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=136.232 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.924 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=623.953 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.484 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=176.565 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=566.735 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.662 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.458 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.525 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.208 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.617 nodes=1446831
all 53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=29.17x python_ms=381.508 rust_ms=13.078
query_geomean_speedup=7.04x queries=53
slowest relative Rust query: complex_017 speedup=1.05x python_ms=79.903 rust_ms=75.795
```

## After enforced performance query coverage gate

Strengthened `scripts/compare_bhsa_mapped_subset.py` so the performance proof
also enforces query coverage. The comparison now fails when:

- fewer than `--min-shared-queries` shared query timings are present
  (default `53`);
- Python emits a query id that Rust does not emit;
- Rust emits a query id that Python does not emit.

This prevents the speedup check from accidentally passing on a reduced or
mismatched query subset.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=144.750 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.534 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=607.678 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=3.177 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=174.824 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=565.408 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=54.992 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.802 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=20.762 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=105.803 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.383 nodes=1446831
all 53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=26.61x python_ms=376.073 rust_ms=14.132
query_geomean_speedup=7.02x queries=53
slowest relative Rust query: complex_017 speedup=1.03x python_ms=78.659 rust_ms=76.493
```

## After mapped selected-cache feature listing alias parity

Added fixture coverage for mapped compiled feature listing helpers:

- `MappedCompiledCorpus::all_node_features()`
- `MappedCompiledCorpus::Fall()`
- `MappedCompiledCorpus::all_edge_features()`
- `MappedCompiledCorpus::Eall()`
- `MappedCompiledCorpus::all_computed_features()`
- `MappedCompiledCorpus::Call()`

The mapped fixture intentionally compiles a selected feature subset, so coverage
asserts the exact selected-cache feature lists with and without WARP features
rather than comparing against the fully materialized mini corpus. Computed
feature names still match materialized output.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=131.726 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.915 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=604.029 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.004 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=174.513 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=590.147 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.832 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.059 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.193 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=103.134 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.581 nodes=1446831
all 53 mapped curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=39.44x python_ms=381.349 rust_ms=9.670
query_geomean_speedup=7.00x queries=53
slowest relative Rust query: complex_017 speedup=1.09x python_ms=80.365 rust_ms=73.843
```

## After expanded BHSA mapped timing coverage to 54 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_027`:

```text
word sp=verb vs=qal vt=perf
```

The new query is intentionally a lexical multi-feature constraint rather than a
new grammar form. It broadens the enforced real-corpus workload while keeping
the slice focused on benchmark coverage. The strict comparison gate now
requires `54` shared query timings, preventing either side from omitting the new
query.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=139.570 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.984 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=610.548 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.097 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=178.258 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=566.788 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=51.851 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.439 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.331 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=106.298 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=14.227 nodes=1446831
ok query=lex_027 results=5 elapsed_ms=5.041
all 54 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1699.015
ok query=lex_027 results=5 elapsed_ms=5.118
all 54 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=25.46x python_ms=374.392 rust_ms=14.707
query=lex_027 speedup=27.62x python_ms=141.598 rust_ms=5.127
query_geomean_speedup=7.12x queries=54
slowest relative Rust query: complex_017 speedup=1.07x python_ms=79.192 rust_ms=74.265
```

## After expanded BHSA language-feature timing coverage to 55 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_028`:

```text
word language=Aramaic
```

This pulls the already-compiled `language` node feature into the mandatory
timing gate. The strict comparison now requires `55` shared query timings, so
the Python and Rust scripts cannot drift silently.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=146.038 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.416 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=600.730 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.110 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=175.928 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=575.207 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.917 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.378 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.857 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.442 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.121 nodes=1446831
ok query=lex_028 results=5 elapsed_ms=12.299
all 55 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1709.797
ok query=lex_028 results=5 elapsed_ms=0.631
all 55 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=34.96x python_ms=499.780 rust_ms=14.295
query=lex_028 speedup=15.95x python_ms=131.841 rust_ms=8.268
query_geomean_speedup=7.27x queries=55
slowest relative Rust query: complex_017 speedup=1.04x python_ms=79.862 rust_ms=76.463
```

## After expanded BHSA Hiphil-perfect timing coverage to 56 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_029`:

```text
word sp=verb vs=hif vt=perf
```

This adds another low-complexity lexical query from the benchmark source using
only features already present in the mapped cache. The strict comparison now
requires `56` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.768 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.097 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=587.698 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.120 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=173.265 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=576.331 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.177 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.305 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.014 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=103.119 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.086 nodes=1446831
ok query=lex_029 results=5 elapsed_ms=5.140
all 56 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1738.188
ok query=lex_029 results=5 elapsed_ms=2.920
all 56 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=29.28x python_ms=377.552 rust_ms=12.894
query=lex_029 speedup=26.34x python_ms=134.924 rust_ms=5.122
query_geomean_speedup=7.46x queries=56
slowest relative Rust query: complex_017 speedup=1.03x python_ms=78.791 rust_ms=76.348
```

## After expanded BHSA Qal-verb timing coverage to 57 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_025` from the benchmark source:

```text
word sp=verb vs=qal
```

This adds another low-complexity lexical query over already-compiled features.
The strict comparison now requires `57` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=141.668 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.405 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=603.599 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.087 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=168.394 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=572.493 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.224 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.396 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.046 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=107.705 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=14.296 nodes=1446831
ok query=lex_025 results=5 elapsed_ms=4.959
all 57 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1716.294
ok query=lex_025 results=5 elapsed_ms=6.478
all 57 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=26.05x python_ms=378.493 rust_ms=14.530
query=lex_025 speedup=31.16x python_ms=162.332 rust_ms=5.210
query_geomean_speedup=7.58x queries=57
slowest relative Rust query: complex_017 speedup=1.05x python_ms=78.115 rust_ms=74.357
```

## After expanded BHSA feminine singular noun timing coverage to 58 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_030`:

```text
word sp=subs gn=f nu=sg
```

This covers another benchmark-source low-complexity lexical query over already
compiled features. The local query id is `lex_030` because `lex_027` is already
used in the Rust timing set for an earlier validated verb query. The strict
comparison now requires `58` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=137.178 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.581 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=610.719 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.141 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=172.640 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=575.804 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.148 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.752 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.170 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=103.418 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=15.154 nodes=1446831
ok query=lex_030 results=5 elapsed_ms=5.988
all 58 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1749.279
ok query=lex_030 results=5 elapsed_ms=7.898
all 58 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=34.54x python_ms=366.363 rust_ms=10.607
query=lex_030 speedup=26.42x python_ms=139.204 rust_ms=5.268
query_geomean_speedup=7.69x queries=58
slowest relative Rust query: complex_017 speedup=1.05x python_ms=78.683 rust_ms=75.120
```

## After expanded BHSA subject-phrase timing coverage to 59 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_002` from the benchmark source:

```text
phrase function=Subj
```

This is the first additional structural single-feature query after expanding
the lexical set. It uses the already-compiled `function` feature. The strict
comparison now requires `59` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=141.703 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.976 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=605.078 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.183 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=168.578 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=581.368 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.665 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=64.056 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.661 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=102.738 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=14.613 nodes=1446831
ok query=struct_002 results=5 elapsed_ms=5.307
all 59 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1714.237
ok query=struct_002 results=5 elapsed_ms=2.162
all 59 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=25.54x python_ms=377.680 rust_ms=14.786
query=struct_002 speedup=14.33x python_ms=74.761 rust_ms=5.218
query_geomean_speedup=7.88x queries=59
slowest relative Rust query: complex_017 speedup=1.18x python_ms=88.416 rust_ms=74.958
```

## After expanded BHSA object-phrase timing coverage to 60 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_003` from the benchmark source:

```text
phrase function=Objc
```

This adds another structural single-feature query over the already-compiled
`function` feature. The strict comparison now requires `60` shared query
timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.852 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.904 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=594.086 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.099 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=177.987 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=582.161 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.351 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.624 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.947 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=103.325 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=15.521 nodes=1446831
ok query=struct_003 results=5 elapsed_ms=5.047
all 60 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1674.015
ok query=struct_003 results=5 elapsed_ms=1.601
all 60 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=38.74x python_ms=375.427 rust_ms=9.691
query=struct_003 speedup=15.32x python_ms=74.101 rust_ms=4.838
query_geomean_speedup=8.02x queries=60
slowest relative Rust query: complex_017 speedup=1.14x python_ms=87.566 rust_ms=76.821
```

## After expanded BHSA complement-phrase timing coverage to 61 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_004` from the benchmark source:

```text
phrase function=Cmpl
```

This adds another structural single-feature query over the already-compiled
`function` feature. The strict comparison now requires `61` shared query
timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=144.826 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.914 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=596.847 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.116 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=173.567 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=564.357 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.253 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.972 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.600 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.160 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.162 nodes=1446831
ok query=struct_004 results=5 elapsed_ms=5.090
all 61 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1711.076
ok query=struct_004 results=5 elapsed_ms=1.907
all 61 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=27.24x python_ms=375.040 rust_ms=13.768
query=struct_004 speedup=15.29x python_ms=75.719 rust_ms=4.951
query_geomean_speedup=8.01x queries=61
slowest relative Rust query: complex_017 speedup=1.04x python_ms=77.491 rust_ms=74.827
```

## After expanded BHSA adjunct-phrase timing coverage to 62 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_005` from the benchmark source:

```text
phrase function=Adju
```

This adds another structural single-feature query over the already-compiled
`function` feature. The strict comparison now requires `62` shared query
timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=139.080 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.011 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=614.096 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.205 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=178.903 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=571.098 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.503 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.640 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.567 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=103.611 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=14.613 nodes=1446831
ok query=struct_005 results=5 elapsed_ms=5.146
all 62 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1735.502
ok query=struct_005 results=5 elapsed_ms=0.673
all 62 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=26.25x python_ms=377.646 rust_ms=14.388
query=struct_005 speedup=12.69x python_ms=78.121 rust_ms=6.156
query_geomean_speedup=8.17x queries=62
slowest relative Rust query: complex_017 speedup=1.05x python_ms=80.106 rust_ms=76.200
```

## After expanded BHSA time-phrase timing coverage to 63 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_006` from the benchmark source:

```text
phrase function=Time
```

This adds another structural single-feature query over the already-compiled
`function` feature. The strict comparison now requires `63` shared query
timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=136.955 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.992 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=599.472 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.043 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=179.324 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=563.802 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=51.517 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.051 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.996 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.663 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=13.664 nodes=1446831
ok query=struct_006 results=5 elapsed_ms=5.197
all 63 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1704.852
ok query=struct_006 results=5 elapsed_ms=0.305
all 63 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=35.17x python_ms=387.558 rust_ms=11.021
query=struct_006 speedup=16.56x python_ms=87.689 rust_ms=5.296
query_geomean_speedup=8.43x queries=63
slowest relative Rust query: complex_017 speedup=1.09x python_ms=80.519 rust_ms=74.190
```

## After expanded BHSA location-phrase timing coverage to 64 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_007` from the benchmark source:

```text
phrase function=Loca
```

This completes the currently enforced phrase-function single-feature group
from `struct_001` through `struct_007`. The strict comparison now requires `64`
shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=136.558 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.849 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=603.641 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.247 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=177.189 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=574.833 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.398 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=66.261 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.246 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=103.829 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.108 nodes=1446831
ok query=struct_007 results=5 elapsed_ms=5.236
all 64 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1699.004
ok query=struct_007 results=5 elapsed_ms=0.215
all 64 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=24.38x python_ms=368.375 rust_ms=15.110
query=struct_007 speedup=14.76x python_ms=74.418 rust_ms=5.043
query_geomean_speedup=8.25x queries=64
slowest relative Rust query: complex_017 speedup=1.04x python_ms=78.576 rust_ms=75.452
```

## After expanded BHSA noun-phrase timing coverage to 65 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_009` from the benchmark source:

```text
phrase typ=NP
```

This moves the enforced structural set into phrase-type queries beyond the
already-covered `struct_008` verbal phrase query. The strict comparison now
requires `65` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=141.397 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.176 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=609.486 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.167 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=166.259 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=567.294 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=48.581 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.818 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.014 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=99.413 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=11.672 nodes=1446831
ok query=struct_009 results=5 elapsed_ms=7.048
all 65 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1702.679
ok query=struct_009 results=5 elapsed_ms=3.593
all 65 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=26.40x python_ms=368.720 rust_ms=13.966
query=struct_009 speedup=10.56x python_ms=76.540 rust_ms=7.247
query_geomean_speedup=8.30x queries=65
slowest relative Rust query: complex_017 speedup=1.03x python_ms=77.985 rust_ms=76.072
```

## After expanded BHSA prepositional-phrase timing coverage to 66 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_010` from the benchmark source:

```text
phrase typ=PP
```

This adds another phrase-type single-feature query over the already-compiled
`typ` feature. The strict comparison now requires `66` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=144.479 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.649 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=610.260 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.102 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=174.846 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=553.160 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.592 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.816 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.892 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=105.673 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=12.810 nodes=1446831
ok query=struct_010 results=5 elapsed_ms=7.040
all 66 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1707.964
ok query=struct_010 results=5 elapsed_ms=5.552
all 66 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=36.76x python_ms=380.262 rust_ms=10.345
query=struct_010 speedup=10.54x python_ms=76.600 rust_ms=7.266
query_geomean_speedup=8.44x queries=66
slowest relative Rust query: complex_017 speedup=1.07x python_ms=79.857 rust_ms=74.716
```

## After expanded BHSA conjunction-phrase timing coverage to 67 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_011` from the benchmark source:

```text
phrase typ=CP
```

This adds another phrase-type single-feature query over the already-compiled
`typ` feature. The strict comparison now requires `67` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=141.653 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.845 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=625.718 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.173 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=179.842 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=571.519 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.116 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.071 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.913 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=105.417 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=11.224 nodes=1446831
ok query=struct_011 results=5 elapsed_ms=7.706
all 67 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1735.566
ok query=struct_011 results=5 elapsed_ms=4.363
all 67 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=31.07x python_ms=377.760 rust_ms=12.157
query=struct_011 speedup=10.97x python_ms=80.273 rust_ms=7.317
query_geomean_speedup=8.65x queries=67
slowest relative Rust query: complex_017 speedup=1.03x python_ms=80.677 rust_ms=78.206
```

## After expanded BHSA adverb-phrase timing coverage to 68 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_012` from the benchmark source:

```text
phrase typ=AdvP
```

This adds another phrase-type single-feature query over the already-compiled
`typ` feature. The strict comparison now requires `68` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=146.488 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.957 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=617.424 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.141 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=170.446 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=574.524 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=51.406 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.306 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.651 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=105.739 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=14.657 nodes=1446831
ok query=struct_012 results=5 elapsed_ms=7.832
all 68 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1701.062
ok query=struct_012 results=5 elapsed_ms=0.584
all 68 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=24.23x python_ms=371.591 rust_ms=15.339
query=struct_012 speedup=9.56x python_ms=76.507 rust_ms=8.000
query_geomean_speedup=8.42x queries=68
slowest relative Rust query: complex_017 speedup=1.04x python_ms=79.064 rust_ms=75.836
```

## After expanded BHSA nominal-clause timing coverage to 69 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_014` from the benchmark source:

```text
clause kind=NC
```

This adds another clause-kind single-feature query over the already-compiled
`kind` feature. The strict comparison now requires `69` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=144.927 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.786 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=609.149 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.970 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=170.337 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=565.595 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=50.039 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.565 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.986 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=105.768 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=15.110 nodes=1446831
ok query=struct_014 results=5 elapsed_ms=5.032
all 69 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1706.795
ok query=struct_014 results=5 elapsed_ms=0.516
all 69 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=30.81x python_ms=374.445 rust_ms=12.155
query=struct_014 speedup=5.42x python_ms=27.673 rust_ms=5.102
query_geomean_speedup=8.35x queries=69
slowest relative Rust query: complex_017 speedup=1.09x python_ms=80.622 rust_ms=73.914
```

## After expanded BHSA quotation-clause timing coverage to 70 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_015` from the benchmark source:

```text
clause domain=Q
```

This adds another clause single-feature query over the already-compiled
`domain` feature. The strict comparison now requires `70` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=131.205 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.728 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=566.961 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.096 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=171.578 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=553.562 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=50.302 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=65.393 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.148 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=101.123 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=9.803 nodes=1446831
ok query=struct_015 results=5 elapsed_ms=5.021
all 70 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1599.221
ok query=struct_015 results=5 elapsed_ms=2.395
all 70 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=25.34x python_ms=362.627 rust_ms=14.313
query=struct_015 speedup=5.64x python_ms=28.349 rust_ms=5.024
query_geomean_speedup=8.19x queries=70
slowest relative Rust query: complex_017 speedup=1.04x python_ms=77.533 rust_ms=74.865
```

## After expanded BHSA narrative-clause timing coverage to 71 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_016` from the benchmark source:

```text
clause domain=N
```

This adds another clause single-feature query over the already-compiled
`domain` feature. The strict comparison now requires `71` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=137.913 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.611 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=569.895 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.983 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=163.110 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=550.176 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.942 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=66.658 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.201 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=100.713 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=9.674 nodes=1446831
ok query=struct_016 results=5 elapsed_ms=5.200
all 71 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1641.274
ok query=struct_016 results=5 elapsed_ms=1.055
all 71 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=38.14x python_ms=424.128 rust_ms=11.119
query=struct_016 speedup=5.58x python_ms=28.143 rust_ms=5.041
query_geomean_speedup=8.20x queries=71
slowest relative Rust query: complex_017 speedup=1.13x python_ms=83.286 rust_ms=73.957
```

## After expanded BHSA wayyiqtol-clause timing coverage to 72 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_017` from the benchmark source:

```text
clause typ=Way0
```

This adds another clause single-feature query over the already-compiled `typ`
feature. The strict comparison now requires `72` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=131.692 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.058 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=565.067 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.977 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=168.080 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=546.606 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=50.095 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.247 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.776 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=101.799 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=9.760 nodes=1446831
ok query=struct_017 results=5 elapsed_ms=5.090
all 72 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1598.290
ok query=struct_017 results=5 elapsed_ms=0.802
all 72 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=31.29x python_ms=363.777 rust_ms=11.625
query=struct_017 speedup=5.78x python_ms=28.831 rust_ms=4.986
query_geomean_speedup=8.19x queries=72
slowest relative Rust query: complex_017 speedup=1.13x python_ms=83.045 rust_ms=73.495
```

## After expanded BHSA nominal-clause-type timing coverage to 73 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_018` from the benchmark source:

```text
clause typ=NmCl
```

This adds another clause single-feature query over the already-compiled `typ`
feature. The strict comparison now requires `73` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=140.788 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.381 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=571.253 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.022 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=171.709 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=544.804 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=49.411 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=66.979 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.159 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=100.973 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=9.976 nodes=1446831
ok query=struct_018 results=5 elapsed_ms=5.213
all 73 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1601.215
ok query=struct_018 results=5 elapsed_ms=0.990
all 73 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=26.50x python_ms=374.488 rust_ms=14.129
query=struct_018 speedup=8.09x python_ms=40.439 rust_ms=4.997
query_geomean_speedup=8.18x queries=73
slowest relative Rust query: complex_017 speedup=1.02x python_ms=76.256 rust_ms=75.115
```

## After expanded BHSA infinitive-clause timing coverage to 74 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_019` from the benchmark source:

```text
clause typ=InfC
```

This adds another clause single-feature query over the already-compiled `typ`
feature. The strict comparison now requires `74` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=135.868 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.236 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=559.028 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=3.423 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=163.939 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=561.223 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=51.390 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.708 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.056 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=99.882 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=13.532 nodes=1446831
ok query=struct_019 results=5 elapsed_ms=5.042
all 74 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1616.404
ok query=struct_019 results=5 elapsed_ms=0.582
all 74 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=27.62x python_ms=367.132 rust_ms=13.294
query=struct_019 speedup=5.46x python_ms=26.943 rust_ms=4.935
query_geomean_speedup=8.23x queries=74
slowest relative Rust query: complex_017 speedup=1.07x python_ms=78.455 rust_ms=73.106
```

## After expanded BHSA predicate-phrase containment coverage to 75 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_021` from the benchmark source:

```text
clause
  phrase function=Pred
```

This adds another embedded structural query over clause-to-phrase containment.
The strict comparison now requires `75` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=139.852 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.754 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=578.890 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.564 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=182.033 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=550.635 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=55.470 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=67.509 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.882 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.966 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.680 nodes=1446831
ok query=struct_021 results=5 elapsed_ms=57.943
all 75 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1615.279
ok query=struct_021 results=5 elapsed_ms=26.942
all 75 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=58.88x python_ms=709.747 rust_ms=12.054
query=struct_021 speedup=3.11x python_ms=181.293 rust_ms=58.350
query_geomean_speedup=8.06x queries=75
slowest relative Rust query: complex_017 speedup=1.02x python_ms=76.964 rust_ms=75.463
```

## After expanded BHSA verbal-clause object containment coverage to 76 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_022` from the benchmark source:

```text
clause kind=VC
  phrase function=Objc
```

This adds a constrained embedded structural query over verbal clauses and object
phrases. The strict comparison now requires `76` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=136.261 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.153 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=554.197 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.030 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=172.351 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=548.376 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=56.220 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.345 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.959 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=103.241 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.914 nodes=1446831
ok query=struct_022 results=5 elapsed_ms=36.219
all 76 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1638.908
ok query=struct_022 results=5 elapsed_ms=22.558
all 76 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=75.06x python_ms=703.113 rust_ms=9.367
query=struct_022 speedup=3.99x python_ms=136.178 rust_ms=34.129
query_geomean_speedup=7.93x queries=76
slowest relative Rust query: complex_017 speedup=1.04x python_ms=76.636 rust_ms=73.899
```

## After expanded BHSA noun-phrase substantive containment coverage to 77 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_023` from the benchmark source:

```text
phrase typ=NP
  word sp=subs
```

This adds a phrase-to-word containment query over noun phrases and substantive
words. The strict comparison now requires `77` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=130.349 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.664 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=567.081 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.389 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=175.560 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=567.863 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=56.879 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.000 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.415 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.791 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.196 nodes=1446831
ok query=struct_023 results=5 elapsed_ms=87.495
all 77 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1615.091
ok query=struct_023 results=5 elapsed_ms=17.614
all 77 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=70.74x python_ms=693.124 rust_ms=9.798
query=struct_023 speedup=4.74x python_ms=380.371 rust_ms=80.173
query_geomean_speedup=7.96x queries=77
slowest relative Rust query: complex_017 speedup=1.05x python_ms=78.331 rust_ms=74.668
```

## After expanded BHSA prepositional-phrase containment coverage to 78 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_025` from the benchmark source:

```text
phrase typ=PP
  word sp=prep
```

This adds a phrase-to-word containment query over prepositional phrases and
preposition words. The strict comparison now requires `78` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=140.799 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.663 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=577.570 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.284 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=172.495 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=570.328 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=56.943 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.852 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=22.916 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.399 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.434 nodes=1446831
ok query=struct_025 results=5 elapsed_ms=68.820
all 78 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1594.795
ok query=struct_025 results=5 elapsed_ms=14.115
all 78 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=75.97x python_ms=706.843 rust_ms=9.304
query=struct_025 speedup=4.62x python_ms=319.266 rust_ms=69.150
query_geomean_speedup=8.07x queries=78
slowest relative Rust query: complex_017 speedup=1.06x python_ms=78.588 rust_ms=74.487
```

## After expanded BHSA predicate-object containment coverage to 79 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_027` from the benchmark source:

```text
clause
  phrase function=Pred
  phrase function=Objc
```

This adds another multi-child embedded structural query over clauses containing
predicate and object phrases. The strict comparison now requires `79` shared
query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=152.275 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=33.606 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=645.680 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.025 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=187.139 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=624.419 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=58.277 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.381 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.327 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=111.191 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=11.068 nodes=1446831
ok query=struct_027 results=5 elapsed_ms=76.235
all 79 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1944.000
ok query=struct_027 results=5 elapsed_ms=117.072
all 79 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=77.15x python_ms=811.746 rust_ms=10.521
query=struct_027 speedup=4.00x python_ms=300.134 rust_ms=75.102
query_geomean_speedup=7.69x queries=79
slowest relative Rust query: complex_017 speedup=1.02x python_ms=79.657 rust_ms=77.767
```

## After expanded BHSA sentence-clause containment coverage to 80 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_028` from the benchmark source:

```text
sentence
  clause
```

This adds sentence-to-clause containment coverage. The strict comparison now
requires `80` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=147.878 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.712 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=651.501 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.258 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=193.908 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=622.941 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=59.394 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.760 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.580 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.058 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=14.551 nodes=1446831
ok query=struct_028 results=5 elapsed_ms=72.168
all 80 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1901.556
ok query=struct_028 results=5 elapsed_ms=30.324
all 80 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=37.85x python_ms=393.023 rust_ms=10.385
query=struct_028 speedup=2.38x python_ms=172.503 rust_ms=72.539
query_geomean_speedup=7.71x queries=80
slowest relative Rust query: complex_017 speedup=1.09x python_ms=83.793 rust_ms=76.837
```

## After expanded BHSA verse verbal-clause containment coverage to 81 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `struct_029` from the benchmark source:

```text
verse
  clause kind=VC
```

This adds verse-to-verbal-clause containment coverage. The strict comparison now
requires `81` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=142.474 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.221 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=660.305 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.198 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=192.176 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=617.904 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.043 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.596 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.852 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=108.052 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.245 nodes=1446831
ok query=struct_029 results=5 elapsed_ms=52.170
all 81 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1903.804
ok query=struct_029 results=5 elapsed_ms=14.473
all 81 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=29.00x python_ms=401.969 rust_ms=13.859
query=struct_029 speedup=2.36x python_ms=124.537 rust_ms=52.685
query_geomean_speedup=7.57x queries=81
slowest relative Rust query: complex_017 speedup=1.05x python_ms=80.366 rust_ms=76.435
```

## After expanded BHSA substantive lexical timing coverage to 82 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_002` from the benchmark source:

```text
word sp=subs
```

This adds another simple lexical feature query over the already-compiled `sp`
feature. The strict comparison now requires `82` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=139.244 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.504 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=640.359 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=1.954 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=189.384 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=596.581 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.102 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.743 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.991 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=108.811 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=11.143 nodes=1446831
ok query=lex_002 results=5 elapsed_ms=5.540
all 82 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1927.322
ok query=lex_002 results=5 elapsed_ms=7.915
all 82 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=37.74x python_ms=371.011 rust_ms=9.830
query=lex_002 speedup=27.01x python_ms=135.519 rust_ms=5.018
query_geomean_speedup=7.73x queries=82
slowest relative Rust query: complex_017 speedup=1.17x python_ms=89.453 rust_ms=76.150
```

## After expanded BHSA preposition lexical timing coverage to 83 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_003` from the benchmark source:

```text
word sp=prep
```

This adds another simple lexical feature query over the already-compiled `sp`
feature. The strict comparison now requires `83` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=141.726 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.309 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=651.119 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.285 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=182.696 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=607.207 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.673 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.079 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.479 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=106.761 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.783 nodes=1446831
ok query=lex_003 results=5 elapsed_ms=5.565
all 83 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1902.519
ok query=lex_003 results=5 elapsed_ms=5.739
all 83 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=27.94x python_ms=381.879 rust_ms=13.668
query=lex_003 speedup=29.52x python_ms=165.494 rust_ms=5.606
query_geomean_speedup=7.85x queries=83
slowest relative Rust query: complex_017 speedup=1.07x python_ms=82.147 rust_ms=76.820
```

## After expanded BHSA conjunction lexical timing coverage to 84 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_004` from the benchmark source:

```text
word sp=conj
```

This adds another simple lexical feature query over the already-compiled `sp`
feature. The strict comparison now requires `84` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.449 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.145 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=648.809 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.304 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=181.461 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=619.968 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.013 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.669 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.793 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.719 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=14.641 nodes=1446831
ok query=lex_004 results=5 elapsed_ms=5.392
all 84 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1913.606
ok query=lex_004 results=5 elapsed_ms=4.778
all 84 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=35.68x python_ms=391.709 rust_ms=10.979
query=lex_004 speedup=23.60x python_ms=131.010 rust_ms=5.551
query_geomean_speedup=7.89x queries=84
slowest relative Rust query: complex_017 speedup=1.09x python_ms=83.769 rust_ms=77.075
```

## After expanded BHSA proper noun lexical timing coverage to 85 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_005` from the benchmark source:

```text
word sp=nmpr
```

This adds another simple lexical feature query over the already-compiled `sp`
feature. The strict comparison now requires `85` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=141.772 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.723 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=638.750 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.192 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=190.343 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=608.726 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.466 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.230 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.981 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=107.517 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=13.191 nodes=1446831
ok query=lex_005 results=5 elapsed_ms=5.406
all 85 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1892.030
ok query=lex_005 results=5 elapsed_ms=3.057
all 85 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=35.42x python_ms=377.594 rust_ms=10.660
query=lex_005 speedup=25.55x python_ms=130.096 rust_ms=5.092
query_geomean_speedup=7.99x queries=85
slowest relative Rust query: complex_017 speedup=1.08x python_ms=82.328 rust_ms=76.138
```

## After expanded BHSA article lexical timing coverage to 86 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_006` from the benchmark source:

```text
word sp=art
```

This adds another simple lexical feature query over the already-compiled `sp`
feature. The strict comparison now requires `86` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=140.852 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.066 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=639.758 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.473 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=181.045 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=602.664 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.150 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.772 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.271 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=106.472 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=15.318 nodes=1446831
ok query=lex_006 results=5 elapsed_ms=5.185
all 86 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1927.615
ok query=lex_006 results=5 elapsed_ms=2.444
all 86 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=36.31x python_ms=372.565 rust_ms=10.261
query=lex_006 speedup=23.84x python_ms=130.193 rust_ms=5.460
query_geomean_speedup=8.18x queries=86
slowest relative Rust query: complex_017 speedup=1.06x python_ms=81.529 rust_ms=77.102
```

## After expanded BHSA adjective lexical timing coverage to 87 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_007` from the benchmark source:

```text
word sp=adjv
```

This adds another simple lexical feature query over the already-compiled `sp`
feature. The strict comparison now requires `87` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=144.953 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.907 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=630.228 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.210 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=181.684 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=594.684 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.434 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.963 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.662 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=107.031 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.346 nodes=1446831
ok query=lex_007 results=5 elapsed_ms=5.371
all 87 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1909.003
ok query=lex_007 results=5 elapsed_ms=0.973
all 87 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=26.88x python_ms=373.791 rust_ms=13.905
query=lex_007 speedup=25.16x python_ms=129.085 rust_ms=5.130
query_geomean_speedup=8.16x queries=87
slowest relative Rust query: complex_017 speedup=1.05x python_ms=80.839 rust_ms=77.057
```

## After expanded BHSA adverb lexical timing coverage to 88 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_008` from the benchmark source:

```text
word sp=advb
```

This adds another simple lexical feature query over the already-compiled `sp`
feature. The strict comparison now requires `88` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=141.511 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.339 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=656.396 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.208 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=185.674 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=617.021 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=55.560 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=76.137 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.729 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=108.381 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.413 nodes=1446831
ok query=lex_008 results=5 elapsed_ms=5.069
all 88 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1932.834
ok query=lex_008 results=5 elapsed_ms=0.634
all 88 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=25.22x python_ms=379.481 rust_ms=15.047
query=lex_008 speedup=23.17x python_ms=127.906 rust_ms=5.520
query_geomean_speedup=8.23x queries=88
slowest relative Rust query: complex_017 speedup=1.05x python_ms=80.003 rust_ms=76.538
```

## After expanded BHSA perfect-tense lexical timing coverage to 89 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_009` from the benchmark source:

```text
word vt=perf
```

This adds a simple lexical feature query over the already-compiled `vt`
feature. The strict comparison now requires `89` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=140.205 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.943 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=640.798 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.141 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=185.966 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=600.198 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.668 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.549 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.770 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=108.854 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=9.901 nodes=1446831
ok query=lex_009 results=5 elapsed_ms=5.366
all 89 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1897.620
ok query=lex_009 results=5 elapsed_ms=2.444
all 89 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=35.09x python_ms=379.342 rust_ms=10.810
query=lex_009 speedup=26.67x python_ms=134.133 rust_ms=5.029
query_geomean_speedup=8.56x queries=89
slowest relative Rust query: complex_017 speedup=1.07x python_ms=82.989 rust_ms=77.498
```

## After expanded BHSA imperfect-tense lexical timing coverage to 90 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_010` from the benchmark source:

```text
word vt=impf
```

This adds a simple lexical feature query over the already-compiled `vt`
feature. The strict comparison now requires `90` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=132.625 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.763 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=645.666 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.202 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=186.376 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=605.168 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.542 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.517 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.670 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=108.084 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.235 nodes=1446831
ok query=lex_010 results=5 elapsed_ms=5.304
all 90 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1927.966
ok query=lex_010 results=5 elapsed_ms=1.468
all 90 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=25.20x python_ms=377.147 rust_ms=14.967
query=lex_010 speedup=24.53x python_ms=129.365 rust_ms=5.273
query_geomean_speedup=8.43x queries=90
slowest relative Rust query: complex_017 speedup=1.05x python_ms=81.065 rust_ms=77.405
```

## After expanded BHSA wayyiqtol lexical timing coverage to 91 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_011` from the benchmark source:

```text
word vt=wayq
```

This adds a simple lexical feature query over the already-compiled `vt`
feature. The strict comparison now requires `91` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=149.279 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.945 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=650.056 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.242 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=182.697 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=617.195 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.121 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.901 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.184 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.126 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.275 nodes=1446831
ok query=lex_011 results=5 elapsed_ms=5.129
all 91 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1906.537
ok query=lex_011 results=5 elapsed_ms=1.515
all 91 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=26.00x python_ms=372.119 rust_ms=14.310
query=lex_011 speedup=24.79x python_ms=128.999 rust_ms=5.204
query_geomean_speedup=8.54x queries=91
slowest relative Rust query: complex_017 speedup=1.09x python_ms=82.435 rust_ms=75.928
```

## After expanded BHSA imperative lexical timing coverage to 92 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_012` from the benchmark source:

```text
word vt=impv
```

This adds a simple lexical feature query over the already-compiled `vt`
feature. The strict comparison now requires `92` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=141.358 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.308 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=649.332 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.211 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=180.142 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=597.527 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.933 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.915 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=17.147 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.075 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=13.842 nodes=1446831
ok query=lex_012 results=5 elapsed_ms=5.308
all 92 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1894.743
ok query=lex_012 results=5 elapsed_ms=0.561
all 92 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=33.53x python_ms=374.072 rust_ms=11.158
query=lex_012 speedup=24.34x python_ms=131.959 rust_ms=5.421
query_geomean_speedup=8.89x queries=92
slowest relative Rust query: complex_017 speedup=1.09x python_ms=83.826 rust_ms=77.040
```

## After expanded BHSA active participle lexical timing coverage to 93 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_013` from the benchmark source:

```text
word vt=ptca
```

This adds a simple lexical feature query over the already-compiled `vt`
feature. The strict comparison now requires `93` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.123 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.623 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=645.172 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.059 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=191.891 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=612.323 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=51.233 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.571 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.236 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.055 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=9.807 nodes=1446831
ok query=lex_013 results=5 elapsed_ms=5.424
all 93 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1912.682
ok query=lex_013 results=5 elapsed_ms=0.967
all 93 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=35.20x python_ms=384.366 rust_ms=10.919
query=lex_013 speedup=25.72x python_ms=131.979 rust_ms=5.131
query_geomean_speedup=9.03x queries=93
slowest relative Rust query: complex_017 speedup=1.08x python_ms=83.359 rust_ms=76.990
```

## After expanded BHSA infinitive construct lexical timing coverage to 94 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_014` from the benchmark source:

```text
word vt=infc
```

This adds a simple lexical feature query over the already-compiled `vt`
feature. The strict comparison now requires `94` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=144.127 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.292 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=645.801 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.074 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=189.880 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=597.178 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.534 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.148 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.784 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=105.165 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=12.087 nodes=1446831
ok query=lex_014 results=5 elapsed_ms=5.301
all 94 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1921.660
ok query=lex_014 results=5 elapsed_ms=0.711
all 94 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=39.10x python_ms=381.084 rust_ms=9.746
query=lex_014 speedup=24.98x python_ms=140.365 rust_ms=5.618
query_geomean_speedup=8.99x queries=94
slowest relative Rust query: complex_017 speedup=1.16x python_ms=88.992 rust_ms=76.906
```

## After expanded BHSA Qal stem lexical timing coverage to 95 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_015` from the benchmark source:

```text
word vs=qal
```

This adds a simple lexical feature query over the already-compiled `vs`
feature. The strict comparison now requires `95` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=143.205 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=35.768 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=644.065 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.210 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=191.582 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=613.509 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=62.020 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=74.170 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.486 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=114.623 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=9.922 nodes=1446831
ok query=lex_015 results=5 elapsed_ms=5.323
all 95 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1884.802
ok query=lex_015 results=5 elapsed_ms=4.378
all 95 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=87.31x python_ms=956.547 rust_ms=10.956
query=lex_015 speedup=25.86x python_ms=136.419 rust_ms=5.275
query_geomean_speedup=9.13x queries=95
slowest relative Rust query: complex_017 speedup=1.05x python_ms=81.597 rust_ms=77.545
```

## After expanded BHSA Piel stem lexical timing coverage to 96 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_016` from the benchmark source:

```text
word vs=piel
```

This adds a simple lexical feature query over the already-compiled `vs`
feature. The strict comparison now requires `96` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=145.268 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.650 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=652.621 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.941 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=190.135 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=621.337 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=59.829 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.567 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.767 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.621 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.644 nodes=1446831
ok query=lex_016 results=5 elapsed_ms=5.608
all 96 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1949.305
ok query=lex_016 results=5 elapsed_ms=0.790
all 96 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=51.37x python_ms=638.578 rust_ms=12.432
query=lex_016 speedup=22.87x python_ms=128.563 rust_ms=5.622
query_geomean_speedup=9.07x queries=96
slowest relative Rust query: complex_017 speedup=1.04x python_ms=80.640 rust_ms=77.479
```

## After expanded BHSA Hiphil stem lexical timing coverage to 97 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_017` from the benchmark source:

```text
word vs=hif
```

This adds another simple lexical feature query over the already-compiled `vs`
feature. The strict comparison now requires `97` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=148.708 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.108 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=645.003 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.829 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=193.738 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=622.243 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=58.403 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.576 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.823 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.828 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.015 nodes=1446831
ok query=lex_017 results=5 elapsed_ms=5.164
all 97 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1988.410
ok query=lex_017 results=5 elapsed_ms=1.069
all 97 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=28.53x python_ms=386.451 rust_ms=13.546
query=lex_017 speedup=24.66x python_ms=128.992 rust_ms=5.230
query_geomean_speedup=9.13x queries=97
slowest relative Rust query: complex_017 speedup=1.05x python_ms=83.358 rust_ms=79.027
```

## After expanded BHSA Niphal stem lexical timing coverage to 98 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_018` from the benchmark source:

```text
word vs=nif
```

This adds another simple lexical feature query over the already-compiled `vs`
feature. The strict comparison now requires `98` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=144.126 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=33.102 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=648.312 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.100 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=193.287 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=619.120 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=58.595 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.102 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.593 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=113.199 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.291 nodes=1446831
ok query=lex_018 results=5 elapsed_ms=5.578
all 98 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1921.489
ok query=lex_018 results=5 elapsed_ms=0.497
all 98 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=24.55x python_ms=377.599 rust_ms=15.381
query=lex_018 speedup=24.83x python_ms=127.730 rust_ms=5.145
query_geomean_speedup=9.29x queries=98
slowest relative Rust query: complex_017 speedup=1.05x python_ms=80.066 rust_ms=76.209
```

## After expanded BHSA masculine lexical timing coverage to 99 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_019` from the benchmark source:

```text
word gn=m
```

This adds another simple lexical feature query over the already-compiled `gn`
feature. The strict comparison now requires `99` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=145.051 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.234 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=650.291 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.993 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=200.882 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=622.378 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=60.964 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.207 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.714 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=111.869 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=13.735 nodes=1446831
ok query=lex_019 results=5 elapsed_ms=5.281
all 99 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1881.323
ok query=lex_019 results=5 elapsed_ms=10.573
all 99 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=26.35x python_ms=377.740 rust_ms=14.334
query=lex_019 speedup=26.67x python_ms=138.377 rust_ms=5.189
query_geomean_speedup=9.41x queries=99
slowest relative Rust query: complex_017 speedup=1.07x python_ms=81.549 rust_ms=76.095
```

## After expanded BHSA feminine lexical timing coverage to 100 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_020` from the benchmark source:

```text
word gn=f
```

This adds the paired feminine gender query over the already-compiled `gn`
feature. The strict comparison now requires `100` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=150.434 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=32.648 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=645.537 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.006 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=188.377 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=607.457 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=59.885 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=75.225 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.299 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=111.631 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=12.312 nodes=1446831
ok query=lex_020 results=5 elapsed_ms=5.335
all 100 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1915.521
ok query=lex_020 results=5 elapsed_ms=2.983
all 100 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=28.80x python_ms=428.413 rust_ms=14.878
query=lex_020 speedup=25.16x python_ms=129.667 rust_ms=5.154
query_geomean_speedup=9.41x queries=100
slowest relative Rust query: complex_017 speedup=1.07x python_ms=81.683 rust_ms=76.459
```

## After expanded BHSA singular lexical timing coverage to 101 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_021` from the benchmark source:

```text
word nu=sg
```

This adds the singular number query over the already-compiled `nu` feature. The
strict comparison now requires `101` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=141.791 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=33.175 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=646.157 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.692 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=192.505 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=619.480 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=58.739 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.164 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.324 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.713 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.226 nodes=1446831
ok query=lex_021 results=5 elapsed_ms=5.179
all 101 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1913.657
ok query=lex_021 results=5 elapsed_ms=11.638
all 101 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=28.57x python_ms=374.422 rust_ms=13.105
query=lex_021 speedup=25.82x python_ms=140.027 rust_ms=5.423
query_geomean_speedup=9.60x queries=101
slowest relative Rust query: complex_017 speedup=1.06x python_ms=81.068 rust_ms=76.473
```

## After expanded BHSA plural lexical timing coverage to 102 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_022` from the benchmark source:

```text
word nu=pl
```

This adds the plural number query over the already-compiled `nu` feature. The
strict comparison now requires `102` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=143.268 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=33.395 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=659.281 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.207 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=199.853 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=623.082 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=61.165 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.347 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.779 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=112.627 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=14.244 nodes=1446831
ok query=lex_022 results=5 elapsed_ms=5.088
all 102 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1910.787
ok query=lex_022 results=5 elapsed_ms=3.880
all 102 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=28.02x python_ms=379.794 rust_ms=13.555
query=lex_022 speedup=25.41x python_ms=131.936 rust_ms=5.193
query_geomean_speedup=9.74x queries=102
slowest relative Rust query: complex_017 speedup=1.06x python_ms=80.500 rust_ms=75.929
```

## After expanded BHSA dual lexical timing coverage to 103 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding `lex_023` from the benchmark source:

```text
word nu=du
```

This adds the dual number query over the already-compiled `nu` feature. The
strict comparison now requires `103` shared query timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=137.970 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.428 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=639.368 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.111 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=182.363 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=591.178 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.016 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.784 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=19.543 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=107.833 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=14.517 nodes=1446831
ok query=lex_023 results=5 elapsed_ms=5.122
all 103 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1908.401
ok query=lex_023 results=5 elapsed_ms=0.362
all 103 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=34.49x python_ms=376.076 rust_ms=10.905
query=lex_023 speedup=24.34x python_ms=132.049 rust_ms=5.426
query_geomean_speedup=9.99x queries=103
slowest relative Rust query: complex_017 speedup=1.06x python_ms=81.929 rust_ms=77.340
```

## After covering upstream BHSA masculine imperative lexical timing to 104 queries

Expanded the curated BHSA validator and Python/Rust performance comparison by
adding a local `lex_031` entry for the missing upstream `lex_028` benchmark
source template:

```text
word sp=verb vt=impv gn=m
```

This avoids relabeling the already-validated historical `lex_028`, `lex_030`,
and related local entries while still covering the upstream masculine
imperative query. The strict comparison now requires `104` shared query
timings.

Validation:

```text
cargo fmt && cargo test --quiet
54 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=142.298 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.285 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=653.726 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=2.094 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=188.286 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=602.094 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.287 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.367 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=18.954 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.223 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.386 nodes=1446831
ok query=lex_031 results=5 elapsed_ms=5.393
all 104 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1935.833
ok query=lex_031 results=5 elapsed_ms=1.525
all 104 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=25.37x python_ms=373.972 rust_ms=14.739
query=lex_031 speedup=28.97x python_ms=157.415 rust_ms=5.433
query_geomean_speedup=9.91x queries=104
slowest relative Rust query: complex_017 speedup=1.08x python_ms=81.611 rust_ms=75.864
```

## After adding Python-style `value_type` metadata alias coverage

Ported another small upstream result/feature metadata compatibility behavior:
Python metadata helpers accept both `valueType` and `value_type` keys. Rust now
uses the same fallback while parsing node and edge feature values and while
reporting feature metadata through materialized feature accessors. The new test
builds a tiny temporary corpus with `@value_type=int`, verifies integer parsing,
and checks `FeatureInfo::from_corpus` reports `int`.

Validation:

```text
cargo fmt && cargo test --quiet
55 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=135.411 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.143 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=562.207 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.918 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=166.881 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=572.524 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=51.184 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.797 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.102 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=108.423 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=11.116 nodes=1446831
ok query=lex_031 results=5 elapsed_ms=5.601
all 104 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1898.085
ok query=lex_031 results=5 elapsed_ms=1.607
all 104 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=36.95x python_ms=380.442 rust_ms=10.297
query_geomean_speedup=9.91x queries=104
slowest relative Rust query: complex_017 speedup=1.05x python_ms=79.806 rust_ms=76.328
```

## After extending `value_type` metadata alias coverage to compiled and mapped caches

Extended the previous Python-style metadata alias compatibility through the
compiled cache metadata model and the memory-mapped feature views. The
regression coverage now verifies a temporary `@value_type=int` feature through
direct parsing, materialized corpus loading, `Corpus::is_loaded`, `load_compiled`,
`MappedCompiledCorpus`, mapped `is_loaded`, mapped feature catalog rows,
mapped feature descriptions, and `FeatureInfo::from_mapped`.

Validation:

```text
cargo fmt && cargo test --quiet
55 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=134.430 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.757 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=538.625 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.899 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=166.168 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=555.717 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=50.244 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=68.545 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=22.795 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=106.654 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=15.669 nodes=1446831
ok query=lex_031 results=5 elapsed_ms=5.439
all 104 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1897.571
ok query=lex_031 results=5 elapsed_ms=1.553
all 104 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=34.75x python_ms=590.396 rust_ms=16.989
query_geomean_speedup=9.97x queries=104
slowest relative Rust query: complex_017 speedup=1.05x python_ms=79.778 rust_ms=75.839
```

## After adding standalone atom operator-line search parsing

Ported another Python search syntax behavior from the `opLineRe` coverage:
standalone atom operator lines now apply to the following atom in both
materialized and mapped search parsing. The supported executable operators are
the containment pair already implemented by the engine, `[[` and `]]`.
Regression coverage checks that:

```text
phrase
  [[
  word
```

matches the normal indented containment query, and that:

```text
word
  ]]
  phrase
```

returns the same pairs with row order reversed. Both forms are covered for the
materialized and mapped search paths.

Validation:

```text
cargo fmt && cargo test --quiet
55 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=130.604 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=31.930 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=546.243 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.493 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=172.942 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=561.700 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=54.085 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.231 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.350 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=107.315 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=14.369 nodes=1446831
ok query=lex_031 results=5 elapsed_ms=5.221
all 104 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1925.872
ok query=lex_031 results=5 elapsed_ms=1.530
all 104 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=38.26x python_ms=411.421 rust_ms=10.754
query_geomean_speedup=9.87x queries=104
slowest relative Rust query: complex_017 speedup=1.06x python_ms=81.789 rust_ms=77.445
```

## After extending atom operator lines to existing relation operators

Broadened the standalone atom operator-line implementation so materialized and
mapped search atoms carry the existing relation-operator enum instead of a
separate containment-only enum. This lets indented atom operators reuse the
same semantics as named relation lines. The containment interval shortcut is
still preserved for mapped `[[` / `]]`; other operators fall back to the normal
candidate set and are filtered by the shared relation evaluator.

New regression coverage checks standalone operator-line forms for:

```text
w1:word
  <:
  w2:word
```

and:

```text
w:word
  -parent>
  p:phrase
```

against the equivalent named-relation queries in both materialized and mapped
search.

Validation:

```text
cargo fmt && cargo test --quiet
55 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=141.048 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.662 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=564.317 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.647 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=174.385 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=563.118 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=54.437 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=69.694 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.435 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=105.884 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.096 nodes=1446831
ok query=lex_031 results=5 elapsed_ms=5.355
all 104 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1898.777
ok query=lex_031 results=5 elapsed_ms=1.443
all 104 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=29.60x python_ms=387.487 rust_ms=13.092
query_geomean_speedup=9.82x queries=104
slowest relative Rust query: complex_017 speedup=1.09x python_ms=83.580 rust_ms=76.474
```

## After covering integer node zero versus missing fixture behavior

Ported another Python `.cfm` compile/reload regression around integer node
features. The `score.tf` mini-corpus fixture intentionally contains explicit
zero values and missing rows:

```text
2 -> 0
3 -> missing
5 -> 0
```

The Rust tests now assert that materialized, compiled, and mapped loads all
preserve that distinction:

```text
score.v(2) == 0
score.v(3) == None
score.s(0) == [2, 5]
```

The selected mapped mini-corpus cache now includes `score`, and the feature
listing expectations were updated accordingly.

Validation:

```text
cargo fmt && cargo test --quiet
55 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=133.162 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.699 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=560.634 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.330 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=177.985 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=559.482 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.816 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.711 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.729 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=107.471 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=11.790 nodes=1446831
ok query=lex_031 results=5 elapsed_ms=5.321
all 104 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1943.800
ok query=lex_031 results=5 elapsed_ms=1.559
all 104 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=27.34x python_ms=387.386 rust_ms=14.169
query_geomean_speedup=9.82x queries=104
slowest relative Rust query: complex_017 speedup=1.06x python_ms=81.555 rust_ms=76.898
```

## After covering range and comma node specs in parser tests

Ported another Text-Fabric loader compatibility behavior into the Rust
integration suite. Explicit node specs can combine ranges and comma-separated
parts, and both node features and edge features must expand every source and
target node:

```text
1-3,5    shared
1-2,4    7-8,10    linked
```

The new test asserts node value selection, implicit numbering after explicit
ranges, edge forward expansion, edge value propagation to expanded pairs, and
inverse/backward traversal from expanded target ranges.

Validation:

```text
cargo fmt && cargo test --quiet
56 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=131.078 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=26.393 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=572.310 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.898 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=178.336 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=587.558 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=54.422 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.531 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.166 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=106.179 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.222 nodes=1446831
all 104 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1907.387
all 104 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=28.22x python_ms=389.055 rust_ms=13.786
query_geomean_speedup=9.85x queries=104
slowest relative Rust query: complex_017 speedup=1.08x python_ms=83.108 rust_ms=76.653
```

## After covering empty and self-referential edge feature behavior

Ported another direct edge-feature unit behavior from the Python suite into the
Rust integration tests. The new coverage exercises:

- an explicit empty edge row, preserving `items()` while producing no forward,
  backward, or bidirectional traversal results,
- a self-referential edge, which should appear in forward, backward,
  bidirectional, inverse, item, and frequency-count paths exactly once.

Validation:

```text
cargo fmt && cargo test --quiet
57 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=144.255 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=27.431 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=563.636 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.169 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=168.489 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=568.353 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.038 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.131 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=22.577 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.691 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=12.412 nodes=1446831
all 104 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1911.481
all 104 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=28.03x python_ms=389.834 rust_ms=13.908
query_geomean_speedup=9.99x queries=104
slowest relative Rust query: complex_017 speedup=1.08x python_ms=83.302 rust_ms=77.407
```

## After pinning cross-feature equality and inequality search relations

Expanded the materialized and mapped search tests for Python-style
feature-to-feature relation operators. Earlier coverage already exercised
same-feature equality/inequality, numeric ordering, and regex-normalized string
relations. The new assertions pin explicit cross-feature equality and
inequality using mini-corpus integer features:

```text
w:word word=hello
p:phrase phrase_id=1
w .number=phrase_id. p

w:word word=hello
p:phrase phrase_id=2
w .number#phrase_id. p
```

This closes a syntax/behavior gap from the Python search parser tests for
`.left=right.` and `.left#right.` relation forms, in both materialized and
mapped execution paths.

Validation:

```text
cargo fmt && cargo test --quiet
57 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=130.217 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=28.075 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=573.102 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.830 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=176.554 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=563.617 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=53.655 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.160 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.472 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.511 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.816 nodes=1446831
all 104 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1883.882
all 104 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=28.59x python_ms=387.869 rust_ms=13.565
query_geomean_speedup=9.80x queries=104
slowest relative Rust query: complex_017 speedup=1.09x python_ms=83.025 rust_ms=76.508
```

## After pinning Python-style query comments

Ported another Python search integration behavior into the Rust test suite:
query lines whose first non-space character is `%` are comments. The new
assertions cover both materialized and mapped search, and include comments in
ordinary templates and inside quantified `/with/` blocks.

Validation:

```text
cargo fmt && cargo test --quiet
57 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=138.299 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.938 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=560.242 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.672 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=178.078 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=564.078 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=52.688 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.386 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.164 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=104.966 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=14.398 nodes=1446831
all 104 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1902.387
all 104 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=29.08x python_ms=398.231 rust_ms=13.693
query_geomean_speedup=9.84x queries=104
slowest relative Rust query: complex_017 speedup=1.07x python_ms=80.536 rust_ms=75.464
```

## After matching Python Text-Fabric string value decoding

Fixed a loader parity gap in the Rust `.tf` parser. Python's `valueFromTf()`
decodes Text-Fabric string value escapes by converting `\t` and `\n` to actual
tab/newline characters while treating `\\` as an escaped literal backslash.
The Rust parser already handled tabs and newlines, but left doubled backslashes
uncollapsed.

The Rust parser now uses the same decode rule for string node values and valued
edge string values. The new test pins all three escape forms:

```text
has\ttab       -> has<TAB>tab
has\nnewline   -> has<NEWLINE>newline
has\\backslash -> has\backslash
```

Validation:

```text
cargo fmt && cargo test --quiet
58 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=134.533 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.112 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=563.647 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.826 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=178.535 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=556.684 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=54.172 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=71.320 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=23.315 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.702 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=13.580 nodes=1446831
all 104 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2126.152
all 104 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=26.35x python_ms=405.094 rust_ms=15.371
query_geomean_speedup=9.86x queries=104
slowest relative Rust query: complex_017 speedup=1.05x python_ms=80.666 rust_ms=76.626
```

## After matching Python descending range expansion

Fixed a parser parity gap for Text-Fabric node specs. Python's `setFromSpec()`
accepts descending ranges such as `5-3` and expands them as `{3, 4, 5}`. The
Rust parser previously rejected descending ranges. `parse_node_spec()` now
normalizes range bounds before expansion, so the same behavior applies to node
feature rows, edge sources, and edge targets.

Added coverage for:

```text
5-3    reversed
4-2    8-6    reversed
```

Validation:

```text
cargo fmt && cargo test --quiet
59 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=142.842 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.215 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=579.644 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.793 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=177.907 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=608.325 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=55.943 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.658 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.583 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.255 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.116 nodes=1446831
all 104 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2216.083
all 104 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=42.33x python_ms=425.477 rust_ms=10.051
query_geomean_speedup=9.68x queries=104
slowest relative Rust query: complex_017 speedup=1.08x python_ms=82.322 rust_ms=76.404
```

## After matching Python bare feature search constraints

Found a search syntax parity gap while comparing the Python syntax tests to the
Rust query parsers. Python accepts a bare feature constraint such as `word pos`
in actual search, and it behaves like the existence constraint `word pos*`:

```text
word pos    -> words with a `pos` value
phrase pos  -> no results in the mini corpus
```

Rust previously rejected bare feature constraints as unsupported syntax. The
materialized and mapped query parsers now treat any non-empty constraint token
that does not match an explicit operator form as `Exists`.

Validation:

```text
cargo fmt && cargo test --quiet
59 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=136.064 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=33.164 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=587.727 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.552 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=179.691 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=600.469 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=54.691 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=70.703 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.451 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=109.584 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=9.676 nodes=1446831
all 104 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2234.604
all 104 materialized curated queries passed

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=48.39x python_ms=481.134 rust_ms=9.943
query_geomean_speedup=9.43x queries=104
slowest relative Rust query: complex_017 speedup=1.11x python_ms=84.739 rust_ms=76.257
```

## After materialized quantified containment interval pruning

Optimized the materialized search path for quantified alternatives. Previously,
`/with/` and `/without/` checks scanned every candidate node for each root and
then called `contains(root, candidate)`. This was correct but expensive on BHSA
for broad quantified shapes such as:

```text
sentence
/with/
  clause kind=VC
  clause kind=NC
/-/
```

The materialized `CandidateCache` now keeps two views per atom signature:

- the existing sorted candidate node vector,
- a slot-interval vector sorted by first slot.

Root-contained alternative searches use the interval vector to binary-search
the root's slot window and only run the exact `contains()` check on candidates
whose first slot falls inside that window. This preserves semantics for
non-contiguous slot sets while avoiding most broad candidate scans.

The biggest immediate materialized validator win was `quant_015`, which dropped
from roughly 525 ms in the previous run to 12.708 ms here. The mixed
base-plan-plus-quantifier complex cases (`complex_011`, `complex_012`, and
`complex_016`) remain slow because their base plan expansion still dominates;
those need a separate base-plan ordering/pruning pass.

Validation:

```text
cargo fmt && cargo test --quiet
59 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=148.229 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.955 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=588.519 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.759 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=199.485 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=593.291 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=57.539 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=73.915 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=24.981 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=114.527 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.185 nodes=1446831
all 104 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2147.747
all 104 materialized curated queries passed
quant_015 elapsed_ms=12.708
complex_011 elapsed_ms=6113.774
complex_012 elapsed_ms=22848.231
complex_016 elapsed_ms=27536.178

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=29.35x python_ms=397.859 rust_ms=13.556
query_geomean_speedup=9.92x queries=104
slowest relative Rust query: complex_017 speedup=1.06x python_ms=80.872 rust_ms=76.090
```

## After streaming limited materialized quantified base rows

The previous materialized quantifier optimization fixed broad contained
alternative scans, but mixed base-plan-plus-quantifier queries still spent most
of their time materializing many base rows before filtering:

```text
clause kind=VC
/with/
  phrase function=Time
/-/
  phrase function=Pred
    word vt=perf
```

For limited quantified searches, materialized search now streams base rows
through `extend_quantified_matches()`. Each complete base row is checked against
the quantifier blocks immediately, and recursion stops once the requested
result limit is satisfied. Unlimited quantified searches keep the full
materialized behavior, preserving `study()` and full count semantics.

This turns the previously pathological mixed materialized query tail into
bounded work:

```text
complex_011: 6113.774 ms  -> 688.223 ms
complex_012: 22848.231 ms -> 1534.345 ms
complex_016: 27536.178 ms -> 1319.712 ms
```

Validation:

```text
cargo fmt && cargo test --quiet
59 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=142.190 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=30.711 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=583.707 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=5.629 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=174.239 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=590.274 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=56.783 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=74.647 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.027 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=110.915 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=9.976 nodes=1446831
all 104 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=2171.168
all 104 materialized curated queries passed
complex_011 elapsed_ms=688.223
complex_012 elapsed_ms=1534.345
complex_016 elapsed_ms=1319.712

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=29.79x python_ms=425.050 rust_ms=14.269
query_geomean_speedup=10.11x queries=104
slowest relative Rust query: complex_017 speedup=1.05x python_ms=79.320 rust_ms=75.369
```

## After parent-scoped materialized containment candidate pruning

Extended the materialized slot-interval candidate index beyond quantified root
alternatives. Ordinary indented atom expansion now detects when the active
parent relation is normal containment (`[[` or the implicit indentation
relation) and asks the candidate cache for nodes inside the already-bound
parent's slot window before running the exact parent constraint check.

The same helper is used by:

- ordinary non-quantified search recursion,
- limited quantified base-plan streaming,
- nested quantified alternative matching.

This removes the major remaining materialized nested-query bottleneck. Before
this pass, indented structural queries still scanned the global child candidate
set for every bound parent node. After this pass, each bound parent uses the
slot interval index to only visit plausible children, while the existing
`contains()` check remains the semantic authority.

Representative materialized BHSA validator timings:

```text
struct_030: 284.323 ms -> 28.456 ms
complex_001: 1054.256 ms -> 24.121 ms
complex_003: 1586.042 ms -> 26.397 ms
complex_011: 688.223 ms -> 17.423 ms
complex_012: 1534.345 ms -> 16.210 ms
complex_016: 1319.712 ms -> 16.606 ms
```

Validation:

```text
cargo fmt && cargo test --quiet
59 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
ok corpus=sp load_ms=140.732 nodes=590290 slots=399392 query=sign results=5
ok corpus=tischendorf load_ms=29.271 nodes=152077 slots=137711 query=word results=5
ok corpus=bhsa load_ms=569.326 nodes=1446831 slots=426590 query=word results=5
ok corpus=cuc load_ms=4.275 nodes=10697 slots=8145 query=sign results=5
ok corpus=n1904 load_ms=179.816 nodes=497525 slots=137779 query=word results=5
ok corpus=dss load_ms=583.185 nodes=2108303 slots=1430241 query=sign results=5
ok corpus=quran load_ms=57.746 nodes=218282 slots=128219 query=word results=5
ok corpus=peshitta load_ms=72.061 nodes=459510 slots=426835 query=word results=5
ok corpus=syrnt load_ms=25.369 nodes=120922 slots=109640 query=word results=5
ok corpus=lxx load_ms=115.730 nodes=685732 slots=623693 query=word results=5

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
loaded mapped_features=13 compile_ms=cached load_ms=10.505 nodes=1446831
all 104 mapped curated queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
loaded features=13 load_ms=1868.577
all 104 materialized curated queries passed
struct_030 elapsed_ms=28.456
complex_001 elapsed_ms=24.121
complex_003 elapsed_ms=26.397
complex_011 elapsed_ms=17.423
complex_012 elapsed_ms=16.210
complex_016 elapsed_ms=16.606

python scripts/compare_bhsa_mapped_subset.py --limit 5
load_speedup=71.86x python_ms=725.862 rust_ms=10.101
query_geomean_speedup=9.93x queries=104
slowest relative Rust query: complex_017 speedup=1.06x python_ms=78.794 rust_ms=74.180
```

## After compiled-cache reopen parity test conversion

Converted the Python compile-reload cycle coverage into the Rust port for the
mapped `.cfr` path. The new test opens the same compiled cache twice and checks
that reopened mapped access preserves:

- `otype` values for every node,
- string feature values for every word node,
- unfiltered and word-filtered descendant locality for sentence nodes,
- simple search results.

Validation:

```text
cargo fmt && cargo test --quiet
60 passed
```

## After compiled section-cache reopen parity test conversion

Converted the Python section reload-cycle coverage into an explicit mapped Rust
test. The new test opens the same all-feature `.cfr` cache twice and checks
that section metadata and `node_from_section()` lookups agree after reopen.

Validation:

```text
cargo fmt && cargo test --quiet mapped_compile_reload_cycle_preserves_section_metadata_and_lookup
1 passed

cargo test --quiet
61 passed
```

## After compiled string-edge reopen parity test conversion

Converted the Python string-valued edge reload-cycle coverage into an explicit
mapped Rust test. The new test opens the same compiled `.cfr` cache twice,
checks the cached `relation` string edge values directly, and compares all
forward string edge/value rows for source nodes 1 through 5 after reopen.

Validation:

```text
cargo fmt && cargo test --quiet mapped_compile_reload_cycle_preserves_string_edge_values
1 passed

cargo test --quiet
62 passed
```

## After direct node-feature unit parity test conversion

Converted another Python `NodeFeature` unit-test slice into a direct Rust test
that constructs `NodeFeature` without a corpus. The test covers empty data,
node id `0`, large node ids, integer values, sorted value selection, sorted
items, and frequency counts.

Validation:

```text
cargo fmt && cargo test --quiet node_feature_direct_edge_cases_match_python_unit_behaviors
1 passed

cargo test --quiet
63 passed
```

## After direct edge-feature set semantics parity

Aligned direct `EdgeFeature` construction with Python's set-like edge data by
sorting and deduplicating target vectors in `EdgeFeature::new_with_values()`.
Added a direct unit test that checks duplicate targets do not inflate forward
traversal, backward traversal, items, or edge counts, while missing nodes return
empty traversal results.

Validation:

```text
cargo fmt && cargo test --quiet edge_feature_direct_edges_are_set_like_as_in_python_units
1 passed

cargo test --quiet
64 passed
```

## After negative integer query syntax parity

Converted the Python search syntax coverage for negative integer values into a
fixture-level Rust test. The new test covers negative integer node equality,
negative numeric comparison, and negative valued edge relation searches in both
materialized and mapped search.

Validation:

```text
cargo fmt && cargo test --quiet supports_negative_integer_query_values_like_python_syntax
1 passed

cargo test --quiet
65 passed
```

## After corpus loading error parity

Converted the Python Fabric loading error-path coverage into typed Rust loader
assertions. The new test checks that a missing corpus path returns an `Io`
error and an empty corpus directory returns `MissingFeature("otype")`.

Validation:

```text
cargo fmt && cargo test --quiet corpus_loading_reports_clean_errors_for_missing_or_empty_locations
1 passed

cargo test --quiet
66 passed
```

## After metadata-only parser parity

Ported the Python loader `metaOnly=True` behavior as
`parse_tf_file_metadata()`. The new parser entry point shares the normal
header/metadata reader, returns feature kind/name/metadata, and deliberately
does not parse data rows. The regression test proves this by reading metadata
from a valid feature and from a feature whose data row would fail full integer
parsing.

Validation:

```text
cargo fmt

cargo test --quiet parses_tf_metadata_without_loading_data_rows_like_python_meta_only
1 passed

cargo test --quiet
67 passed
```

## After Text-Fabric value write-escape helper parity

Audited Python's `valueFromTf()` / `tfFromValue()` utility tests. Rust already
used the `valueFromTf()` equivalent internally while parsing string node and
edge feature values, but did not expose the conversion or provide the inverse
writer-side escaping helper. Added public `value_from_tf()` and
`tf_from_value()` helpers next to the feature value parser so read/write
escaping rules stay paired.

Validation:

```text
cargo fmt

cargo test --quiet text_fabric_value_escape_helpers_match_python_round_trip_behaviors
1 passed

cargo test --quiet
68 passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
104 queries passed
```

## After materialized node feature candidate-filter parity

Added the materialized counterpart to the mapped candidate filters from the
Python storage-vectorization audit. `NodeFeature` now supports candidate-list
filtering by one value, multiple values, present/missing values, and
less-than/greater-than comparisons using the same compatible numeric-string
semantics as materialized search constraints. This gives planner code a common
candidate filtering surface for materialized and mapped execution paths.

Validation:

```text
cargo fmt

cargo test --quiet node_feature_direct_edge_cases_match_python_unit_behaviors
1 passed

cargo test --quiet
69 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local corpora passed, including bhsa
```

## After materialized search candidate-filter integration

Routed materialized search atom constraint filtering through the shared
`NodeFeature` candidate-filter API. Equality, inequality, present/missing, and
typed comparison constraints now use the same direct feature filtering surface
as external callers; regex constraints remain inline because they need the
compiled regex. This keeps planner behavior aligned with the newly ported
storage-style filters.

Validation:

```text
cargo fmt

cargo test --quiet supports_feature_constraints_regex_limits_and_containment
1 passed

cargo test --quiet
69 passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
104 queries passed
```

## After batch edge traversal parity

Audited Python's CSR batch operation tests. Added equivalent batch helpers to
materialized `EdgeFeature` and mapped `EdgeFeatureView`: multi-source target
union via `get_all_targets()` / `all_targets()` and
`filter_sources_with_targets_in()` for source/target membership filtering.
The APIs use `BTreeSet` to preserve Python set semantics with deterministic
ordering.

Validation:

```text
cargo fmt

cargo test --quiet edge_feature_forward_backward_both_and_count
1 passed

cargo test --quiet mapped_compiled_edge_features_read_values_without_materializing
1 passed

cargo test --quiet
68 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local corpora passed, including bhsa
```

## After otype items parity

Audited Python WARP `otype.items()` behavior. Added materialized
`node_type_items()` / `otype_items()` / `otypeItems()` and mapped
`MappedSections` equivalents so callers can iterate `(node, node_type)` rows
without reading the raw `otype` feature directly. Fixture tests cover exact
mini-corpus rows and mapped/materialized parity.

Validation:

```text
cargo fmt

cargo test --quiet loads_mini_corpus_and_queries_basic_types
1 passed

cargo test --quiet mapped_sections_match_materialized_containment_and_locality_without_materializing_cache
1 passed

cargo test --quiet
69 passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
104 queries passed
```

## After desc/eg metadata normalization parity

Audited Python's `formatMeta()` helper coverage. Rust now normalizes parsed
feature metadata with `desc` and optional `eg` into a `description` field,
removing the short keys in the same style as Python. Because normalization
happens in the parser, metadata-only reads, materialized loads, compiled
caches, and mapped views all expose the same description.

Validation:

```text
cargo fmt

cargo test --quiet normalizes_desc_and_eg_metadata_like_python_format_meta
1 passed

cargo test --quiet
69 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local corpora passed, including bhsa
```

## After mapped node feature candidate-filter parity

Audited Python's storage-level `StringPool` and `IntFeatureArray` vectorized
filter tests. Rust mapped feature views already supported whole-feature value
selection and interval lookup, but did not expose caller-candidate filtering.
Added mapped candidate filters for string-pool and mixed node features:
single-value equality, multiple-value equality, present/missing value filters,
and typed less-than/greater-than comparisons. The dynamic
`MappedNodeFeatureView` delegates to the encoding-specific views so planner code
can use one API.

Validation:

```text
cargo fmt

cargo test --quiet mapped_compiled_string_pool_features_read_values_without_materializing
1 passed

cargo test --quiet
68 passed
```

## After compiled inverse edge-value parity

Extended compiled-cache edge-value coverage to mirror Python's inverse lookup
tests for edge values with explicit zero and missing values. Materialized
compiled reload and mapped views now both assert that `distance.t` style
backward traversal preserves `0` for edge `1 -> 2` and `None`/missing for edge
`2 -> 3`.

Validation:

```text
cargo fmt && cargo test --quiet compiled_cache_preserves_edge_values_for_materialized_reload
1 passed

cargo test --quiet mapped_compiled_edge_features_read_values_without_materializing
1 passed

cargo test --quiet
66 passed
```

## After mapped quantified relation-plan parity

Audited a remaining search syntax gap against Python: relation lines inside
mapped quantified alternatives, such as `w1 <: w2` under `/with/`, were being
lost because mapped alternatives were parsed as atom lists only. The mapped
quantifier parser now preserves full `MappedRelationPlan` alternatives, so
relation-bearing alternatives behave like ordinary mapped search plans.

The first implementation precomputed every alternative by running the full
plan, which was correct for the new fixture case but too broad for BHSA; the
curated mapped validator stalled in the quantified query tail. The final
implementation keeps the previous indexed candidate path for relation-free
alternatives and only uses full relation rows when an alternative actually has
relations. That preserves Python-facing behavior without regressing the common
single-atom quantified searches.

New fixture coverage:

```text
phrase
/with/
  w1:word word=hello
  w2:word word=beautiful
  w1 <: w2
/-/
```

returns `[[6]]` in mapped search.

Validation:

```text
cargo fmt

cargo test --quiet mapped_search_runs_simple_string_pool_queries
1 passed

cargo test --quiet
73 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
104 queries passed
```

## After escaped-space query value parity

Audited Python search tokenization around escaped spaces. Python protects `\ `
before splitting feature strings, then unescapes it back to a literal space
during feature/edge operator parsing. Rust was still using plain
`split_whitespace()` in materialized and mapped search, so public search
patterns with literal spaces in values were split into invalid partial
constraints.

Added escaped-whitespace-aware query tokenization to both search backends and
extended value unescaping so `\ ` becomes a literal space. Coverage now includes
node equality constraints, feature continuation lines, regex constraints, and
valued edge relation operators in materialized and mapped search.

New fixture coverage creates a temporary corpus with `word = "good morning"`
and `relation = "main phrase"` and verifies:

```text
word word=good\ morning
word
word=good\ morning
word word~good\ morning
w:word
p:phrase
w -relation=main\ phrase> p
```

Validation:

```text
cargo fmt

cargo test --quiet search_supports_python_style_escaped_spaces_in_values
1 passed

cargo test --quiet
74 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
104 queries passed
```

## After escaped operator literal coverage

Extended the existing escaped query literal fixture to cover literal `#`, `<`,
and `>` values in addition to the earlier `|`, `=`, and backslash cases. The
new coverage verifies both materialized and mapped search for node equality,
node inequality, and valued edge relation operators. No parser code change was
needed after the escaped-whitespace tokenizer work; the broader fixture proved
the existing unescape path already handled these operator literals.

Validation:

```text
cargo fmt

cargo test --quiet parses_escaped_query_literals_for_nodes_and_edges
1 passed

cargo test --quiet
74 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
104 queries passed
```

## After quantified parent atom reference parity

Audited Python's quantified parent-context handling in `deContext()`. Python
allows `..` inside a quantified alternative as an atom standing for the
quantified parent, so constraints can be applied to the parent itself:

```text
phrase
/with/
  .. phrase_id=1
/-/
```

Rust already normalized parent containment relation shorthands such as
`.. [[ w` and `w ]] ..`, but it treated the atom form as a literal node type.
Materialized search now binds a `..` atom inside quantified alternatives to the
current quantifier root and applies the atom constraints to that root. Mapped
search keeps `..` atoms as root checks during relation-free alternative
precomputation while preserving the existing indexed child-candidate path.

Validation:

```text
cargo fmt

cargo test --quiet supports_with_and_without_quantified_blocks
1 passed

cargo test --quiet mapped_search_runs_simple_string_pool_queries
1 passed

cargo test --quiet
74 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
104 queries passed
```

## After quantified parent relation reference parity

Probed Python behavior for `..` as a relation endpoint inside quantified
alternatives. On the mini corpus, Python returns phrase `6` for these forms:

```text
p:phrase
/with/
  .. < w
  w:word word=hello
/-/

p:phrase
/with/
  w:word word=hello
  .. # w
/-/

p:phrase
/with/
  w:word word=hello
  .. .phrase_id=number. w
/-/
```

Materialized search now resolves relation endpoints named `..` to the current
quantifier root while evaluating quantified alternatives. Ordinary non-
quantified relation binding is unchanged.

Mapped search now stores relation endpoints as either atom indexes or a parent
endpoint. Parent-endpoint alternatives are evaluated per quantifier root with
root-aware relation checks, while alternatives without parent endpoints keep
the existing precomputed path.

Validation:

```text
cargo fmt

cargo test --quiet supports_with_and_without_quantified_blocks
1 passed

cargo test --quiet mapped_search_runs_simple_string_pool_queries
1 passed

cargo test --quiet
74 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
104 queries passed
```

## Current mapped BHSA performance proof

Re-ran the Python-vs-Rust mapped BHSA comparison after the recent search parser
and quantified-parent parity work. The benchmark script compares Python
`cfabric` against the Rust mapped validator on the same 104-query curated BHSA
set, requires at least 104 shared queries, requires at least 2x load and query
geomean speedup, and fails by default if any individual shared query is slower
in Rust.

Command:

```text
python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
```

Result:

```text
load_speedup=69.37x python_ms=711.361 rust_ms=10.254
query_geomean_speedup=10.25x queries=104
```

The slowest individual speedup was still above parity:

```text
query=complex_017 speedup=1.12x python_ms=82.099 rust_ms=73.239
```

## After additive feature loading parity

Ported the public outcome of Python's `Fabric.load(add=True)` workflow. Rust's
`Fabric` facade is intentionally stateless, so the behavior is exposed as
`Corpus::add_features_from()` plus `Fabric::load_add()` / `loadAdd()`: callers
can load a selective corpus, add more features into the same `Corpus`, and use
the newly loaded features immediately.

Fixture coverage starts with `fabric.load("word")`, verifies `pos` is absent,
then calls `fabric.load_add(&mut corpus, "pos")`. The same corpus retains
`word`, gains `pos`, still does not load unrelated `number`, and can immediately
run:

```text
word pos=interjection
```

The alias path then adds `number` via `loadAdd()`.

Validation:

```text
cargo fmt

cargo test --quiet fabric_facade_explores_loads_compiles_and_opens_mapped_corpora
1 passed

cargo test --quiet
74 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
104 queries passed
```

## After Python-style edge value search constraints

Closed a search syntax parity gap in valued edge relation operators. Python
parses edge operators with the same feature-condition forms as node features,
so these patterns are now covered in both materialized and mapped search:

```text
w:word
p:phrase
w -relation=missing|a\|b> p

w:word
p:phrase
w -relation#a\#b> p

w:word
p:phrase
w -relation~^a[<>]b$> p
```

The implementation now uses an edge-value matcher enum instead of treating
`-edge=value>` as a single exact-value special case. Equality splits unescaped
`|` alternatives, inequality negates all alternatives, and regex constraints
compile once during query parsing.

Validation:

```text
cargo test --quiet supports_escaped_query_value_literals
1 passed

cargo test --quiet
74 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
104 queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=35.29x python_ms=370.699 rust_ms=10.503
query_geomean_speedup=10.00x queries=104
slowest query speedup: complex_017 speedup=1.08x python_ms=76.049 rust_ms=70.569
```

## After additional utility helper parity

Ported another small public helper slice from `cfabric.utils.helpers`:

- `version_sort()` / `versionSort()` returns a sortable key compatible with
  Python's `versionSort()` for dotted numeric-plus-alpha version strings.
- `nbytes()` formats byte counts with Python-compatible right-aligned unit
  strings such as `  100B`, `  2.0KB`, `  2.0MB`, and `  2.0GB`.
- `itemize()` handles `None`, empty strings, default whitespace splitting, and
  explicit separator splitting like Python's `itemize()`.

Validation:

```text
cargo test --quiet public_misc_helpers_match_python_utility_behaviors
1 passed

cargo fmt

cargo test --quiet
75 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After helper public-name closure pass

Closed concrete Python public helper-name gaps found during the conversion audit.
The underlying Rust-native functions already existed; this slice exposes and
tests the Python public names that callers would expect from the tested
`cfabric.utils.helpers` surface:

- `setFromSpec`
- `rangesFromSet`
- `rangesFromList`
- `specFromRanges`
- `specFromRangesLogical`
- `makeIndex`
- `makeInverse`
- `makeInverseVal`
- `deepSize`
- `valueFromTf`
- `tfFromValue`
- `console` plus testable `console_message` formatting

This is deliberately limited to documented/tested public helper input/output
behavior. It does not add private Python internals or downloader/network
transport.

Validation:

```text
cargo test --quiet helpers_match_python_utility_behaviors
8 passed

cargo test --quiet text_fabric_value_escape_helpers_match_python_round_trip_behaviors
1 passed

cargo test --quiet public_misc_helpers_match_python_utility_behaviors
1 passed

cargo fmt --check
passed

cargo test --quiet
96 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=49.21x
query_geomean_speedup=10.01x
queries=104
```

## After public I/O compiler entrypoint parity

Closed a concrete `cfabric.io.__all__` parity gap with Rust-native wrappers:

- `Data` is now a public alias for `TfData`.
- `Compiler` wraps a source `.tf` directory and exposes `compile(None)` with a
  deterministic default local output path.
- `compile_corpus(source_dir, output_path)` compiles to an explicit output path,
  or to the default local path when `output_path` is `None`.
- `default_compiled_output_path(source_dir)` returns
  `{source_dir}/.cfm/{CFM_VERSION}/corpus.cfr`.

This does not copy Python `.cfm` internals. The wrapper exposes the same public
local compile/load workflow while retaining the Rust compiled-cache format.

Validation:

```text
cargo test --quiet public_io_compiler_wrappers_match_python_compile_entrypoints
1 passed

cargo test --quiet public_tf_data_loader_matches_python_loader_behaviors
1 passed

cargo test --quiet
97 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=33.97x
query_geomean_speedup=10.01x
queries=104
```

## After standalone levUp/levDown precompute parity

Closed the tested/documented `cfabric.precompute.prepare` public export gap for
raw embedding precomputation:

- `lev_up` / `levUp` returns per-node embedder rows in Python's raw precompute
  order, which is reverse canonical for each row.
- `lev_down` / `levDown` returns non-slot embedded-node rows for non-slot
  parents, ordered canonically, matching Python's raw `levDown` data shape.

This is distinct from the higher-level locality API: `L.u()` and `L.d()` expose
navigation behavior, while these helpers expose raw precompute tables. The test
asserts the Python raw-data ordering on the mini corpus.

Validation:

```text
cargo test --quiet public_precompute_helpers_match_corpus_levels_order_and_rank
1 passed

cargo test --quiet
97 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After standalone sections precompute parity

Closed the tested public section-map side of `cfabric.precompute.prepare`:

- `sections(corpus)` returns `sec1`, `sec2`, `seq_from_node`, and
  `node_from_seq` maps from a loaded corpus.
- `sectionsFromApi(corpus)` is the Python public-name alias for the same
  high-level API-derived section computation.

The Rust helper uses typed, Rust-native map shapes with stringified section
headings as keys. It is intentionally scoped to the public lookup behavior
tested elsewhere by `T.nodeFromSection()` / section metadata preservation. It
does not add Python's private structure-precompute internals.

Validation:

```text
cargo test --quiet public_precompute_helpers_match_corpus_levels_order_and_rank
1 passed

cargo fmt --check
passed

cargo test --quiet
97 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After standalone structure precompute parity

Closed the remaining public `cfabric.precompute.prepare.structure` export with
a typed Rust-native helper:

- `structure(corpus)` returns `heading_from_node`, `node_from_heading`,
  `multiple`, `top`, `up`, and `down`.
- The helper returns `None` when no `structureTypes` metadata is configured,
  matching the empty-structure public behavior exposed by the mini corpus.
- Fixture coverage now copies the mini corpus, enables
  `structureTypes=sentence,phrase` and
  `structureFeatures=sentence_id,phrase_id`, then verifies top/up/down and
  heading lookup maps.

This keeps the representation typed instead of copying Python tuple/dict
internals, while preserving the documented public structure-map behavior.

Validation:

```text
cargo test --quiet public_precompute_helpers_match_corpus_levels_order_and_rank
1 passed

cargo test --quiet
97 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After MmapManager storage helper parity

Ported the public behavior covered by Python's `cfabric.storage.mmap_manager`
unit tests into a Rust-shaped `MmapManager`:

- cached metadata loading from `.cfm/{version}/meta.json`,
- `max_slot()`, `max_node()`, `slot_type()`, and `node_types()` metadata
  accessors,
- lazy numeric `.npy` array loading through `get_array()`,
- JSON sidecar loading through `get_json()`,
- `exists()` checks for `meta.json`,
- `close()` clears cached arrays and metadata.

The Rust helper includes a small NumPy `.npy` reader for the numeric dtypes
needed by CF-style array sidecars. The main Rust corpus cache still uses the
existing compiled binary format and mapped views.

Validation:

```text
cargo test --quiet public_mmap_manager_matches_python_lazy_storage_behaviors
1 passed

cargo fmt

cargo test --quiet
85 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
bhsa load_ms=583.621 nodes=1446831 slots=426590 query=word results=5
```

## After public TfData loader helper parity

Added a Rust-shaped `TfData` wrapper for the public behavior covered by
Python's `cfabric.io.loader.Data` unit tests:

- path, directory, filename, and extension parsing,
- missing-file load failure with `data_error`,
- normal and metadata-only `.tf` loads,
- node, edge, and config kind detection,
- `DATA_TYPES` plus value-type selection and unknown-type fallback to `str`,
- `unload()` state clearing,
- simple node/edge/config `.tf` save support with existing Text-Fabric value
  escaping.

The wrapper reuses the existing parser and feature types instead of duplicating
the corpus loader internals.

Validation:

```text
cargo test --quiet public_tf_data_loader_matches_python_loader_behaviors
1 passed

cargo fmt

cargo test --quiet
86 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
bhsa load_ms=582.771 nodes=1446831 slots=426590 query=word results=5
```

## After core Api alias parity

Added a Rust-shaped `Api<T>` wrapper for the Python `cfabric.core.api.Api`
alias behavior covered by unit tests:

- `CF` and `TF` are two public names for the same shared object,
- `cf()` and `tf()` return cloned shared references to that same object,
- `cf_and_tf_are_same_object()` exposes the alias invariant directly,
- ignored features are sorted and deduplicated,
- `api_refs()` includes matching `CF` and `TF` documentation entries.

Validation:

```text
cargo test --quiet public_api_aliases_match_python_cf_tf_behavior
1 passed

cargo fmt

cargo test --quiet
87 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
bhsa load_ms=581.235 nodes=1446831 slots=426590 query=word results=5
```

## After downloader helper parity

Added a Rust `downloader` module for the deterministic public behavior in
Python's `cfabric.downloader` package:

- empty corpus registry listing through `corpus_registry()` /
  `list_corpora()`,
- full Hugging Face repo IDs pass through `resolve_corpus_id()`,
- unknown short corpus IDs produce the Python-style guidance error,
- `get_cache_dir()` honors `CFABRIC_CACHE` and otherwise falls back to a
  cfabric cache path,
- `build_download_request()` preserves revision, force, and compiled-only
  allow-pattern behavior,
- `clear_cache()` and `download()` return explicit unsupported errors instead
  of silently pretending network transport exists.

Network download transport remains a known gap. Python delegates that behavior
to `huggingface_hub.snapshot_download()`; the Rust port now exposes the request
shape but does not yet call a Hugging Face client.

Validation:

```text
cargo test --quiet public_downloader_helpers_match_python_registry_and_path_behaviors
1 passed

cargo fmt

cargo test --quiet
88 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
bhsa load_ms=573.177 nodes=1446831 slots=426590 query=word results=5
```

## After public search syntax constants parity

Exposed Rust equivalents for the public constants in Python's
`cfabric.search.syntax` module:

- quantifier keywords: `QWHERE`, `QHAVE`, `QWITHOUT`, `QWITH`, `QOR`, `QEND`,
- quantifier groups: `QINIT`, `QCONT`, `QTERM`,
- `PARENT_REF`,
- escape tables: `ESCAPES`, `VAL_ESCAPES`,
- helper predicates for quantifier category checks, quantifier-line detection,
  and whitespace/comment-line detection.

The executor already used these tokens internally; this slice makes the syntax
vocabulary public for callers and tests.

Validation:

```text
cargo test --quiet public_search_syntax_constants_match_python_syntax_module
1 passed

cargo fmt

cargo test --quiet
89 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
bhsa load_ms=594.111 nodes=1446831 slots=426590 query=word results=5
```

## After stateful SearchSession wrapper parity

Added `SearchSession`, a stateful wrapper around the existing Rust `Search`
engine for Python `cfabric.search.search.Search` public behavior:

- stores the current `SearchStudy` when `search(..., here=true)` or
  `study(..., here=true)` is used,
- leaves stored state untouched for `here=false`,
- exposes post-study `fetch()`, `count()`, and `showPlan()` without requiring
  the template again,
- returns explicit errors for post-study methods before a study exists,
- exposes `glean()` and relation legend aliases,
- exposes Python performance defaults: `yarnRatio=1.25`, `tryLimitFrom=40`,
  and `tryLimitTo=40`,
- supports performance reset-to-default, invalid parameter reporting, and
  integer-only validation for non-`yarnRatio` parameters.

The existing stateless `Search` API remains unchanged; `SearchSession` is the
Python-shaped wrapper layer.

Validation:

```text
cargo test --quiet public_search_session_matches_python_stateful_search_wrapper_behaviors
1 passed

cargo fmt

cargo test --quiet
90 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
bhsa load_ms=576.276 nodes=1446831 slots=426590 query=word results=5
```

## After standalone precompute levels/order/rank parity

Added a public `precompute` module with Rust-shaped equivalents for the
deterministic Python `cfabric.precompute.prepare` helpers:

- `levels()` computes node-type average slot coverage, first/last node bounds,
  default size-based ordering, and configured level ordering.
- `order()` computes canonical node ordering from node types, `oslots`, levels,
  slot type, and corpus bounds.
- `rank()` computes node rank positions from canonical order.

The focused test builds standalone inputs from the mini corpus and checks exact
agreement with the loaded `Corpus::levels()`, `Corpus::order()`, and
`Corpus::rank()` results.

Validation:

```text
cargo test --quiet public_precompute_helpers_match_corpus_levels_order_and_rank
1 passed

cargo fmt

cargo test --quiet
91 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
bhsa load_ms=590.437 nodes=1446831 slots=426590 query=word results=5
```

## After standalone precompute boundary parity

Extended the public `precompute` module with `boundary()`, matching Python's
`cfabric.precompute.prepare.boundary` behavior:

- collects non-slot nodes by first slot and last slot,
- sorts first-slot rows in reverse canonical rank,
- sorts last-slot rows in canonical rank,
- returns the same `Boundary` shape used by loaded `Corpus` computed data.

Validation extends the existing focused precompute test to assert exact
agreement with `Corpus::boundary()` on the mini corpus.

```text
cargo test --quiet public_precompute_helpers_match_corpus_levels_order_and_rank
1 passed

cargo fmt

cargo test --quiet
91 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
bhsa load_ms=581.989 nodes=1446831 slots=426590 query=word results=5
```

## After standalone precompute characters parity

Extended the public `precompute` module with `characters()`, matching the
observable output of Python's `cfabric.precompute.prepare.characters`:

- counts characters by text feature,
- combines character counts for each configured text format,
- returns sorted `(character, count)` rows per format,
- handles Unicode characters as full scalar values.

This is public output parity only. The Rust port should not mirror deeper
Python precompute internals unless they are exposed through public API behavior.

Validation:

```text
cargo test --quiet public_precompute_helpers_match_corpus_levels_order_and_rank
1 passed

cargo fmt

cargo test --quiet
91 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
bhsa load_ms=586.320 nodes=1446831 slots=426590 query=word results=5
```

## After public config constant parity

Added a public `config` module for Python `cfabric.core.config` and top-level
constant behavior:

- version/name/banner/API version,
- warp feature names and `WARP`,
- repository/backend defaults and URL constants,
- branch/DOI/default local server constants,
- search performance defaults,
- CFM format version, dtype names, and missing-value sentinels.

The Rust crate also re-exports `CF_VERSION`, `CF_NAME`, and `CF_BANNER` aliases
for top-level-style access without conflicting with Cargo package metadata.

Validation:

```text
cargo test --quiet public_config_constants_match_python_config_api_values
1 passed

cargo fmt

cargo test --quiet
92 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
bhsa load_ms=585.550 nodes=1446831 slots=426590 query=word results=5
```

## After describe free-function API parity

Added a public `describe` module with Python `cfabric.describe`-style
free-function wrappers:

- `describe_corpus_overview()`
- `describe_corpus()`
- `list_features()`
- `describe_feature()`
- `describe_features()`
- `describe_text_formats()`
- `get_feature_otypes()`
- `get_all_feature_otypes()`

These functions delegate to the existing Rust-native `Corpus` methods, keeping
the implementation single-sourced while matching the Python module-level API
shape and output structs.

Validation:

```text
cargo test --quiet public_describe_free_functions_match_python_describe_module_behavior
1 passed

cargo fmt

cargo test --quiet
93 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
bhsa load_ms=585.749 nodes=1446831 slots=426590 query=word results=5
```

## After standalone CSR storage helper parity

Added Rust-shaped public equivalents for the Python `cfabric.storage.csr`
helpers:

- `CSRArray` with row access, `get_as_tuple()`, JSON `save()`/`load()`,
  1-indexed `get_all_targets()`, `filter_sources_with_targets_in()`,
  `preload_to_ram()`, `release_cache()`, `is_cached()`, and
  `memory_usage_bytes()`.
- `CSRArrayWithValues` with 0-indexed row access, `get_as_dict()`, int/string
  `CsrValue`, and JSON `save()`/`load()`.

Validation:

```text
cargo test --quiet public_csr_storage_helpers_match_python_storage_behaviors
1 passed

cargo fmt

cargo test --quiet
84 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
bhsa load_ms=579.632 nodes=1446831 slots=426590 query=word results=5
```

## After logging utility helper parity

Ported the public behavior covered by `cfabric.utils.logging` tests:

- silent-level constants `VERBOSE`, `AUTO`, `TERSE`, `DEEP`, and `SILENT_D`;
- `silentConvert()` handling for `None`, booleans, valid strings, and invalid
  strings;
- Python logging-level numeric mapping values through `level_map()` /
  `LEVEL_MAP()` and `logging_level()`.

The Rust API uses a small typed `SilentInput` enum for Python's dynamic input
shape, while preserving the externally visible conversion results.

Validation:

```text
cargo test --quiet public_logging_helpers_match_python_utility_behaviors
1 passed

cargo fmt

cargo test --quiet
79 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After standalone storage helper parity

Added Rust-shaped public equivalents for the Python `cfabric.storage.string_pool`
helpers:

- `StringPool` with `from_dict()`, `get()`, `get_value_index()`,
  `filter_by_value()`, `filter_by_values()`, `save()`, and `load()`.
- `IntFeatureArray` with `from_dict()`, `get()`, equality/multi-value filters,
  less-than/greater-than filters, present/missing filters, `save()`, and
  `load()`.
- `MISSING_STR_INDEX` is exposed as `usize::MAX`.

These helpers intentionally use JSON save/load for the public standalone helper
surface. The high-performance corpus cache remains the existing compiled binary
format with mmap-backed views.

Validation:

```text
cargo test --quiet public_storage_helpers_match_python_string_pool_behaviors
1 passed

cargo fmt

cargo test --quiet
83 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After flattenToSet utility helper parity

Ported Python's `flattenToSet()` behavior through a typed Rust input enum:

- `FlattenFeatureSpec::Name(name)` represents direct string feature names.
- `FlattenFeatureSpec::Values(value)` represents the Python tuple/list case
  where the second item is flattened with `setFromValue()`.
- `flatten_to_set()` / `flattenToSet()` deduplicate and sort into a
  `BTreeSet<String>`.

Validation:

```text
cargo test --quiet public_misc_helpers_match_python_utility_behaviors
1 passed

cargo fmt

cargo test --quiet
82 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After attrs utility helper parity

Ported the public behavior from `cfabric.utils.attrs` that maps cleanly to a
typed Rust API:

- `AttrDict` wraps a `BTreeMap<String, serde_json::Value>` and supports
  construction, missing-key `None` behavior, `get`, `set`, `update`, `keys`,
  `values`, `items`, `len`, and `is_empty`.
- `deepdict()` recursively clones JSON objects/arrays/scalars.
- `deepAttrDict()` / `deep_attr_dict()` provide the Rust-shaped recursive
  conversion hook over JSON values.
- `isIterable()` / `is_iterable()` treats JSON arrays and objects as iterable
  and strings/scalars/null as non-iterable, matching the Python utility's
  non-string iterable intent.

This intentionally does not emulate Python attribute syntax in Rust. The parity
surface is the external data behavior needed by the port.

Validation:

```text
cargo test --quiet public_attr_helpers_match_python_utility_behaviors
1 passed

cargo fmt

cargo test --quiet
82 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After small helper utility parity

Ported a compact public slice from `cfabric.utils.helpers`:

- `utcnow()` returns the current UTC-equivalent `SystemTime`.
- `var()` reads an environment variable and returns `None` for missing values.
- `check32()` returns `(is_32_bit, warning, message)` with the intended
  architecture-specific warning/message behavior.

The Python implementation of `check32()` appears to compare the boolean
architecture result to `2**63 - 1`, which makes the warning branch always run.
The Rust port keeps the boolean and warning/message pair aligned with the
documented intent and the current Python tests' public tuple-shape assertions.

Validation:

```text
cargo test --quiet public_small_helpers_match_python_utility_behaviors
1 passed

cargo fmt

cargo test --quiet
81 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After CLI argument reader utility parity

Ported the public behavior covered by `cfabric.utils.cli.readArgs()` into a
typed Rust API:

- `read_args()` / `readArgs()` parse explicit argument slices instead of reading
  process-global `sys.argv`.
- `CliFlagSpec`, `CliFlagValue`, and `CliReadResult` model the Python task,
  parameter, flag, and help/error result shapes.
- Help/no-argument handling returns help/error messages instead of printing
  through `console()`.
- Task parsing, `all` expansion with `not_in_all`, parameter defaults,
  `param=` default fallback, binary flags, ternary flags, order independence,
  and illegal argument reporting match the Python utility tests.

Validation:

```text
cargo test --quiet public_cli_argument_reader_matches_python_utility_behaviors
1 passed

cargo fmt

cargo test --quiet
80 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After structured JSON/YAML file helper parity

Ported a focused structured file slice from `cfabric.utils.files`:

- `getCwd()` and `chDir()` current-directory helpers.
- `readJson()` / `writeJson()` for direct strings and file-backed JSON data.
- `readYaml()` / `writeYaml()` for direct strings and file-backed YAML data.

Missing JSON/YAML files return empty maps, matching the Python helper behavior
covered by the source tests. The Rust APIs use typed `serde_json::Value` and
`serde_yaml::Value` values and return `Result<Option<String>>` for writers:
`None` when writing to a file, `Some(text)` when returning serialized text.

Validation:

```text
cargo test --quiet public_structured_file_helpers_match_python_file_utility_behaviors
1 passed

cargo fmt

cargo test --quiet
78 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After flexible itemization and projection helper parity

Ported another direct `cfabric.utils.helpers` slice:

- `fitemize()` with a typed Rust input enum for Python's dynamic inputs. It
  handles comma/whitespace-split strings, integers, booleans with Python-style
  `True`/`False` casing, explicit item vectors, empty strings, and `None`.
- `project()` with a typed projection enum. Width `1` returns first-column
  values; wider projections return prefix tuples, matching Python's
  `project(iterableOfTuples, maxDimension)` behavior.

Validation:

```text
cargo test --quiet public_misc_helpers_match_python_utility_behaviors
1 passed

cargo fmt

cargo test --quiet
75 passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
104 queries passed
```

## After set, merge, and metadata helper parity

Ported more direct `cfabric.utils.helpers` behavior:

- `set_from_value()` / `setFromValue()` for `None`, existing sets, strings,
  integer filtering, vectors, and scalar integers.
- `set_from_str()` / `setFromStr()` for comma/whitespace splitting and `None`.
- `merge_dict_of_sets()` / `mergeDictOfSets()` for in-place union semantics.
- `merge_dict()` / `mergeDict()` for recursive JSON-object map merging.
- `format_meta()` / `formatMeta()` for combining `desc` and `eg` into
  `description` while removing the original keys.

Validation:

```text
cargo test --quiet public_misc_helpers_match_python_utility_behaviors
1 passed

cargo fmt

cargo test --quiet
75 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After example formatting and JSON deep-size helper parity

Ported another small helper slice:

- `make_examples()` / `makeExamples()` matches Python's compact node-list
  formatting for short lists and head/tail formatting for lists longer than ten
  nodes.
- `deep_size_json()` / `deepSizeJson()` gives a typed Rust equivalent for the
  Python `deepSize()` behaviors covered by tests: basic values are non-zero,
  larger arrays are larger than smaller arrays, nested structures are counted,
  and empty arrays/objects remain non-zero. The Rust API is deliberately scoped
  to `serde_json::Value` instead of pretending to accept arbitrary Python
  objects.

Validation:

```text
cargo test --quiet public_misc_helpers_match_python_utility_behaviors
1 passed

cargo fmt

cargo test --quiet
75 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After pure file/path helper parity

Ported a focused non-mutating slice from `cfabric.utils.files`:

- `normpath()`, `abspath()`, `expanduser()`, and `unexpanduser()`.
- `prefixSlash()`, `dirNm()`, `fileNm()`, `extNm()`, `stripExt()`,
  `replaceExt()`, and `splitPath()`.
- `backendRep()` for the Python-tested `norm`, `tech`, `name`, `machine`, and
  `url` cases, plus nearby representation/cache/page cases for API completeness.

The Rust `dirNm()` implementation had to special-case bare filenames because
Rust's `Path::parent("file.txt")` yields `"."`, while Python's
`os.path.dirname("file.txt")` returns an empty string.

Validation:

```text
cargo test --quiet public_path_helpers_match_python_file_utility_behaviors
1 passed

cargo fmt

cargo test --quiet
76 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After filesystem helper parity

Ported a focused filesystem slice from `cfabric.utils.files`:

- `isFile()`, `isDir()`, `fileExists()`, and `dirExists()`.
- `fileMake()`, `fileRemove()`, `fileCopy()`, and `fileMove()`.
- `dirMake()`, `dirRemove()`, `dirCopy()`, and `dirMove()`.
- `dirContents()`, `dirAllFiles()`, and `dirEmpty()`.

The Rust APIs return `Result` for mutating or listing operations while matching
Python's public behavior for missing files/directories where Python suppresses
errors. Added a generic `From<std::io::Error>` for `CfError` so these utility
wrappers can use standard filesystem calls ergonomically.

Validation:

```text
cargo test --quiet public_filesystem_helpers_match_python_file_utility_behaviors
1 passed

cargo fmt

cargo test --quiet
77 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After Fabric constructor public-state parity

Added a narrow `Fabric` public-state slice for observable Python constructor
behavior:

- `Fabric::new()` now retains normalized single-location state, default module
  state, `banner`, `version`, `good`, empty requested-feature tracking, and empty
  ignored-feature tracking.
- Added `Fabric::from_locations()` and `Fabric::with_modules()` for callers that
  need Python-shaped constructor inputs without changing the Rust-native direct
  load path behavior.
- Added snake_case and Python-style camelCase accessors for `locationRep`,
  `featuresRequested`, and `featuresIgnored`.

This deliberately does not port Python's internal multi-directory search
machinery. Public loading behavior remains direct and already covered by the
existing `load`, `loadAll`, `loadAdd`, compiled-cache, mapped-cache, and
all-corpora validation tests.

Validation:

```text
cargo test --quiet public_fabric_constructor_metadata_matches_python_public_state
1 passed

cargo fmt

cargo test --quiet
94 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After result serialization public API parity

Closed a public output mismatch in `cfabric.results` compatibility. The Rust
result structs keep their existing Rust-facing fields, but their `to_dict()`
methods now emit Python-compatible dictionary shapes:

- `NodeInfo` serializes node type as `otype`, not `node_type`, and omits empty
  `section_ref`, empty/missing `slots`, and empty/missing `features`.
- `NodeList` serializes only Python's public `nodes`, `total_count`, and
  `query` keys; the Rust convenience `text` field remains available on the
  struct but is not part of the Python-shaped dictionary output.
- `SearchResult` serializes nested rows through the Python-compatible
  `NodeInfo` dictionaries.
- `FeatureInfo` omits `has_values` when it is not applicable, matching Python.
- `CorpusInfo` serializes node type rows with Python's `type` and `avg_slots`
  keys while preserving the Rust struct field names internally.

Validation:

```text
cargo test --quiet wraps_nodes_and_search_results_for_serializable_result_shapes
1 passed

cargo fmt

cargo test --quiet
94 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After search syntax recognizer public API parity

Audited `cfabric.search.syntax` and its unit tests. Python exposes many compiled
regular expressions as public module values. In Rust, exposing Python regex
objects would not be a useful API, so this slice exposes Rust-native recognizer
helpers and typed capture structs for the same public syntax shapes:

- Atom lines and operator-prefixed atom lines.
- Feature identity, comparison, regex, missing, and existence constraint forms.
- Search names, signed integer literals, named atom prefixes, and indentation.
- Relation lines, quantifier lines, standalone operator lines, operator
  stripping, and k-nearness operators.

This is public syntax-surface parity only; it does not change planner behavior.

Validation:

```text
cargo test --quiet public_search_syntax_recognizers_match_python_regex_examples
1 passed

cargo fmt

cargo test --quiet
95 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After text-format fallback/default rendering parity

Closed a Text API rendering gap in both materialized `Corpus::text()` and mapped
`MappedText::text()`.

Python text formats allow fallback chains and fixed defaults, for example
`{special/normal:.}`. The Rust renderer already supported feature fallback
chains but returned an empty string when all features were absent. It now emits
the fixed default after `:` when no feature in the chain has a non-empty value.

The renderer also now decodes `\t` and `\n` in literal parts of the format
specification, matching the public `otext` format behavior described by the
Python Text API. Unknown backslash escapes are preserved.

Validation:

```text
cargo test --quiet text_formats_support_python_fallback_defaults_and_layout_escapes
1 passed

cargo fmt

cargo test --quiet
96 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After loader public-surface alias parity

Added another small `cfabric.io.loader.Data` compatibility slice to the Rust
`TfData` wrapper. The parser behavior was already covered, so this slice exposes
the observable public names used by Python callers:

- Loader constants `ERROR_CUTOFF`, `MEM_MSG`, and `FATAL_MSG`.
- CamelCase aliases for `loadMetaOnly`, `setDataType`, `dataLoaded`,
  `dataError`, `dirName`, `fileName`, `edgeValues`, `isEdge`, `isConfig`,
  `metaData`, and `dataType`.
- Verified that invalid `valueType` metadata falls back to `"str"` through the
  public setter, matching Python's public behavior.

This does not port Python logging/timestamp internals in `Data.load()`; Rust
keeps the current direct parser behavior and exposes the same public state.

Validation:

```text
cargo test --quiet public_tf_data_loader_matches_python_loader_behaviors
1 passed

cargo fmt

cargo test --quiet
96 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After helper camelCase public-name parity

Added documented Python helper aliases over existing Rust utility behavior:

- `isInt`
- `mathEsc`, `mdEsc`, `htmlEsc`, `xmlEsc`, `mdhtmlEsc`
- `tsvEsc`, `pandasEsc`
- `cleanName`, `isClean`

The underlying behavior was already implemented and tested through snake_case
Rust functions. This slice makes the public names match Python callers and docs
without changing internals.

Validation:

```text
cargo test --quiet public_
23 passed

cargo fmt

cargo test --quiet
96 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After result-wrapper JSON API parity

Python `cfabric.results` exposes `to_json()` on the public result wrappers.
Rust already emitted Python-compatible `to_dict()` shapes, so this slice adds
`to_json()` plus `toJson()` aliases for:

- `NodeInfo`
- `NodeList`
- `SearchResult`
- `FeatureInfo`
- `CorpusInfo`

The JSON output serializes the same Python-compatible dictionary shape, including
`otype` for node type and omitted optional fields.

Validation:

```text
cargo test --quiet wraps_nodes_and_search_results_for_serializable_result_shapes
1 passed

cargo fmt

cargo test --quiet
96 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After utility package export parity

Closed another non-downloader utility API slice from Python's public
`cfabric.utils` package exports:

- `LOCATIONS`
- `collectFormats` / `collect_formats`
- `setDir` and `expandDir`
- `splitExt`
- `scanDir`
- `configure_logging` and `set_logging_level`

The Rust versions expose typed return values where Python mutates module-level
state or logger objects, but the public observable behavior covered here is the
same: format metadata collection, basic path expansion, extension splitting,
sorted directory scanning, and silent-level-to-logging-level mapping.

Scope guard: parity means documented or tested Python public API input/output
behavior only. Downloader/network transport remains out of scope, and
undocumented internals are not a Rust port target.

Validation:

```text
cargo test --quiet public_
23 passed

cargo fmt

cargo test --quiet
96 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa
```

## After feature package public export parity

Closed the documented/tested `cfabric.features` public export names without
copying Python internals:

- `NodeFeatures` and `EdgeFeatures` are constructable empty containers.
- `OtypeFeature` and `OslotsFeature` are public aliases over the existing typed
  node and edge feature views.
- `Computed`, `Computeds`, `RankComputed`, `OrderComputed`, `LevUpComputed`,
  and `LevDownComputed` are Rust-native wrappers over already-materialized
  computed data.

Validation:

```text
cargo test --quiet public_precompute_helpers_match_corpus_levels_order_and_rank
1 passed

cargo fmt

cargo test --quiet
97 passed
```

## After navigation package public export parity

Closed the documented/tested `cfabric.navigation` public export names:

- `Nodes` delegates to materialized corpus canonical ordering, type ranking,
  tuple sort keys, walks, and walk events.
- `Locality` delegates to materialized corpus intersecting, up, down, next, and
  previous navigation methods.
- `Text` delegates to materialized corpus node and node-list text rendering.

These are Rust-native wrappers over existing public behavior, not a Python API
object graph port.

Validation:

```text
cargo test --quiet navigation
3 passed

cargo test --quiet mapped_text_renders_otext_formats_without_materializing_corpus
1 passed
```

## After package version and types public alias parity

Closed two public-surface gaps:

- Added `__version__` as a package-level version alias over `VERSION`.
- Added `cf_rust::types` aliases corresponding to Python `cfabric.types`:
  `Node`, `FeatureValue` through the existing enum, `NodeFeatureData`,
  `EdgeFeatureData`, `EdgeFeatureValueData`, `MetaData`, `FeatureMetaData`,
  `NodeArray`, `IndexArray`, `OffsetArray`, `SlotRange`, `SearchResult`,
  `SectionSpec`, and `NodesByType`.

These are type-level compatibility exports only; they do not add runtime
behavior or new storage internals.

Validation:

```text
cargo test --quiet public_
25 passed
```

## After search syntax regex export parity

Closed Python `cfabric.search.syntax` public regex export names by adding
Rust `LazyLock<Regex>` statics for:

- `atomRe`, `atomOpRe`
- `compRe`, `identRe`, `indentLineRe`
- `kRe`, `nameRe`, `namesRe`, `numRe`
- `noneRe`, `trueRe`
- `opLineRe`, `opStripRe`
- `quLineRe`, `relRe`, `reRe`, `whiteRe`

The existing Rust parser functions remain the execution path; these exports are
for public syntax-surface parity with the documented/tested Python module.

Validation:

```text
cargo test --quiet public_search_syntax_recognizers_match_python_regex_examples
1 passed
```

## After CSR valued-edge constructor parity

Closed the remaining ergonomic public constructor gap from Python
`CSRArrayWithValues.from_dict_of_dicts()` tests:

- Added `from_int_dict_of_dicts()` for plain integer valued-edge maps.
- Added `from_string_dict_of_dicts()` for plain string valued-edge maps.

The canonical Rust storage still uses `CsrValue`; these helpers only avoid
requiring callers to pre-wrap every edge value when constructing public storage
objects.

Validation:

```text
cargo test --quiet public_csr_storage_helpers_match_python_storage_behaviors
1 passed
```

## Completion audit pass: objective gates and types export fix

Re-audited the original objective against current files and commands:

- `libs/cf-rust` exists with `Cargo.toml`, `PLAN.md`, `NOTES.md`, converted
  integration tests, Rust source modules, validation binaries, and comparison
  scripts.
- Full Rust test suite passes.
- Every local benchmark corpus with a `tf/` directory loads and answers a basic
  slot-type query, including BHSA.
- BHSA materialized curated query set passes.
- BHSA mapped curated query set passes.
- Python-vs-Rust mapped BHSA timing comparison passes with Rust significantly
  faster.

The audit found one concrete public export gap: Python `cfabric.types` exposes
`FeatureValue`, while Rust only exposed it at the crate root. Fixed by publicly
re-exporting `FeatureValue` from `cf_rust::types` and extending the public types
test.

Validation:

```text
cargo fmt --check
passed

cargo test --quiet
98 passed

cargo run --release --quiet --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
all local benchmark corpora passed, including bhsa

cargo run --release --quiet --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
104 curated BHSA queries passed

cargo run --release --quiet --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
104 mapped curated BHSA queries passed

python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
load_speedup=37.50x
query_geomean_speedup=10.09x
queries=104
```

## Final public-surface audit

Compared Python public exports from `cfabric` modules against Rust exports and
coverage. Python `__all__` surfaces audited:

- top-level package and `core`
- `describe`
- `downloader` helper surface, excluding network transport by explicit user
  decision
- `features` and `features.warp`
- `io`
- `navigation`
- `precompute`
- `results`
- `storage`
- `types`
- `utils`

The remaining non-`__all__` Python modules under `search` such as graph,
semantics, spin, stitch, and execution internals are implementation machinery,
not additional required public API I/O for this Rust port. Rust implements the
public search entrypoints, syntax recognizers, stateful session behavior,
relations, quantified behavior, custom sets, result fetching/counting, and
curated BHSA query behavior through Rust-native internals.

No known documented/tested public API input/output parity gaps remain.

## After Python-vs-Rust memory comparison

Added `scripts/compare_bhsa_mapped_memory.py` to measure peak resident set size
for the same mapped BHSA 104-query workload used by the timing comparison. The
script builds the Rust release binary first, then measures the release binary
directly so Cargo's memory use is not counted against Rust.

Validation:

```text
python scripts/compare_bhsa_mapped_memory.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
python_peak_rss_mb=936.80
rust_peak_rss_mb=178.50
rust_vs_python_rss_ratio=0.191
```

Rust mapped BHSA memory consumption is not degraded versus Python for this
validated workload; it uses about 19.1% of Python's peak RSS.

## After BHSA load-all comparison

Added `scripts/compare_bhsa_load_all.py` and
`src/bin/cf_rust_bhsa_load_all.rs` for materialized BHSA load-all measurement.
Also added `src/bin/cf_rust_bhsa_mapped_load_all.rs` for cached all-feature
compiled/mapped measurement.

The mapped benchmark subset loads these 13 features:

- `otype`, `oslots`, `sp`, `vt`, `vs`, `gn`, `nu`, `ps`, `language`,
  `function`, `typ`, `kind`, `domain`

BHSA has 116 top-level `.tf` feature files in the current local corpus. Python
reports 125 loaded feature objects in the load-all script because its API also
includes computed/derived feature objects.

Materialized load-all comparison:

```text
python_features=125
rust_features=116
python_load_ms=394.241
rust_load_ms=11134.804
python_query_ms=22349.607
rust_query_ms=5.894
python_peak_rss_mb=1970.16
rust_peak_rss_mb=2128.23
rust_vs_python_rss_ratio=1.080
```

The materialized Rust `.tf` load-all path is not the memory-optimized path; it
uses about 8.0% more peak RSS than Python in this workload and has much slower
all-feature raw `.tf` load time, though query time is much faster after load.

Cached compiled/mapped all-feature Rust measurement:

```text
rust_mapped_features=116
compile_ms=cached
rust_mapped_load_ms=91.110
rust_mapped_query_ms=6.676
rust_mapped_peak_rss_mb=157.11
```

The all-feature mapped path is the intended memory-efficient Rust path once the
`.cfr` cache exists.
