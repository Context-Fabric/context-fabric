# cf-rust Port Plan

## Goal

Build a Rust port of `libs/core` that can load and query Text-Fabric corpora in `libs/benchmarks/.corpora`, verified against the BHSA Hebrew Bible corpus, and demonstrate materially better performance than the current Python implementation.

## Phases

1. Convert the core tests into Rust integration tests.
   - Start with Text-Fabric parsing, feature lookup, corpus loading, `otype`, `oslots`, and basic search.
   - Preserve Python fixture semantics from `libs/core/tests/fixtures`.
2. Implement the Rust data model.
   - Parse `.tf` node, edge, and config features.
   - Build typed feature wrappers and indexes for fast value lookup.
   - Implement corpus navigation using `otype` and `oslots`.
3. Implement query support.
   - Support type-only searches, equality constraints, regex constraints, limits, and simple indented containment.
   - Expand toward the curated BHSA query set.
4. Verify real corpora.
   - Load corpora from `libs/benchmarks/.corpora`.
   - Confirm BHSA loads without failure and supports representative queries.
5. Prove performance.
   - Add repeatable Rust and Python benchmark commands.
   - Compare BHSA load and query timings.
   - Optimize until Rust is significantly faster than Python.

## Current Status

- Package scaffold exists at `libs/cf-rust`.
- Converted first test slice in `tests/core_port.rs`:
  - implicit node feature parsing,
  - explicit node feature rows and implicit numbering after explicit rows,
  - explicit node and edge feature row expansion for comma-separated node specs and ascending/descending node ranges, including valued edge target expansion,
  - explicit range parsing from BHSA `otype.tf`,
  - mini-corpus loading,
  - selective loading that always retains mandatory `otype`, `oslots`, and config metadata,
  - lightweight feature inventory/explore support for `.tf` directories,
  - materialized and mapped node feature value lookup, materialized feature data accessors, value selection aliases, ordered items, and frequency list,
  - feature metadata helpers for `meta`, `valueType`, `description`, materialized and mapped node/edge/config feature-view metadata aliases, Python-style loaded feature lists, and typed materialized/mapped `is_loaded()` / `isLoaded()` introspection for node, edge, config, computed, and missing features,
  - describe-facing corpus overview, full corpus descriptions, materialized and mapped feature catalogs, materialized and mapped single/batch feature descriptions, text representation summaries, JSON-serializable Python-shaped `to_dict()` helpers, and materialized/mapped node/edge feature-to-node-type discovery,
  - Python `cfabric.describe` free-function API parity for corpus overviews/descriptions, feature listing, single/batch feature descriptions, text-format descriptions, and feature-to-node-type discovery, delegated to Rust-native corpus methods,
  - results-facing node, node-list, search-result, feature-metadata, corpus-summary, full corpus-description, and text-representation wrappers, including JSON-serializable `to_dict()` helpers, mapped result wrappers backed by mapped text and section lookup plus mapped feature metadata, corpus summaries, full corpus descriptions, and text representation summaries,
  - Python-style large-node result text omission for result wrappers, with an opt-out for callers that explicitly want full text,
  - basic single-node, multi-node, and explicit-format text rendering from `otext` format metadata and slot feature placeholders,
  - Python-style materialized and mapped text format splitting helpers for typed and default formats, plus mapped multi-node text aliases,
  - core section and structure metadata, section reference, section-to-node, node-to-section, Python-style section metadata aliases such as `sectionTypes()` and `structureFeatures()`, and result `section_ref` support,
  - Python-style feature frequency aliases such as `freqList`, including materialized and mapped corpus-level filtered node/edge frequency aliases,
  - Python-style materialized and mapped feature access aliases such as `Fs`, `Es`, `v`, `s`, `f`, and `t`,
  - a Rust `Fabric` facade that owns a corpus path and exposes Python-like explore/load/load-all/compile/load-compiled/open-mapped workflow entry points and aliases, including typed explore category helpers,
  - Python-style additive loading workflow through `Corpus::add_features_from()` and `Fabric::load_add()` / `loadAdd()`, preserving already-loaded features while adding new ones for immediate feature access and search,
  - materialized and mapped edge feature forward, backward, bidirectional traversal, value-aware traversal, compact value-aware aliases, edge count, edge value count, and valued-edge introspection aliases such as `hasEdgeValues()`,
  - direct edge feature frequency lists,
  - materialized and mapped edge feature ordered items,
  - edge feature edge cases for empty edge rows and self-referential edges,
  - Python CSR-style batch edge traversal helpers for materialized and mapped edges, including multi-source target union and source/target membership filtering,
  - valued edge forward, backward, and bidirectional traversal,
  - Python-compatible valued edge bidirectional precedence where outgoing values win,
  - node and edge frequency lists with `otype` filters,
  - corpus node ordering and materialized/mapped locality navigation up/down/next/previous, including mapped containment support,
  - locality navigation filters for one or more node types,
  - Python-style `u`/`d` aliases for upward/downward locality navigation, including explicit multi-type `u_types` / `d_types` aliases,
  - materialized and mapped intersecting locality navigation with Python-style `i`,
  - WARP type interval lookup equivalent to Python `otype.sInterval()`,
  - WARP-style corpus helpers for `otype.v`, `otype.sInterval`, and `oslots.s` semantics,
  - WARP-style `otype.items()` semantics for materialized and mapped node-type iteration,
  - WARP-style `oslots.items()` semantics for non-slot containment rows,
  - vector-free mapped node value interval lookup for string-pool and mixed encoded features, including Python-style `sInterval`,
  - Python storage-style materialized node feature candidate filters, including equality, multiple values, present/missing, and search-compatible typed comparisons,
  - Python storage-style mapped node feature candidate filters for string-pool and mixed encoded features, including equality, multiple values, present/missing, and typed comparisons,
  - Python standalone storage helper parity for `StringPool`, `IntFeatureArray`, `CSRArray`, `CSRArrayWithValues`, and `MmapManager` construction, missing/out-of-bounds access, row access, empty rows, tuple/dict compatibility views, value filtering, value-index lookup, JSON save/load round-trips, integer comparison/presence filters, 1-indexed batch target operations, preload/release cache state, memory usage reporting, int/string valued CSR round-trips, `.cfm` metadata loading, JSON sidecar loading, lazy `.npy` array loading, existence checks, and cache release,
  - memory-light edge backward traversal that avoids full inverse-map allocation per lookup, with mapped edge value-count introspection,
  - compiled and mapped inverse edge-value lookup parity that preserves explicit zero values distinctly from missing edge values,
  - Python-style boundary-aware unfiltered next/previous locality navigation,
  - Python-style `n`/`p` aliases for next/previous locality navigation,
  - canonical materialized/mapped node walking and Python-style walk event generation,
  - Python-style materialized and mapped chunk sort keys for position and length ordering,
  - basic type search,
  - studied search workflow for materialized and mapped search with `study()`, `fetch()`, `count()`, result-tuple `glean()` rendering, supported-relation legends, custom-set variants, lightweight plan summaries, deterministic `show_plan()` output, and order-preserving shallow first-node/prefix projections available through study objects, direct search wrappers, and Python-style aliases such as `relationsLegend()`, `showPlanWithSets()`, `searchFirstNodes()`, and `searchPrefixes()`,
  - Python-style stateful search wrapper parity through `SearchSession`, including stored execution context, `here=false` non-storage, post-study `fetch()`/`count()`/`showPlan()`, `glean()`, relation legends, performance defaults, reset-to-default behavior, invalid parameter reporting, and integer-only validation for non-`yarnRatio` parameters,
  - direct materialized and mapped search `fetch()` and `count()` convenience workflows, including custom-set variants,
  - Python-style materialized and mapped search plan display aliases such as `showPlan()`,
  - Python public search syntax constants and helpers for quantifier keywords, parent references, escape tables, quantifier token classification, quantifier-line detection, and whitespace/comment-line detection,
  - Python-style mapped search named-atom constraint lines and relation lines before atom definitions,
  - Python-style named atom references used as later atom types, including equality-bound duplicate result columns such as `w:word word=hello` followed by `w`,
  - equality and regex constraints,
  - Python-style feature continuation lines after an atom for explicit constraints such as `feature=value`, `feature<value`, `feature*`, and `feature#`,
  - Python-style bare feature constraints such as `word pos` as shorthand for feature existence,
  - escaped query literal values such as `\|`, `\=`, `\#`, `\<`, `\>`, `\\`, `\t`, and `\n` in node constraints and valued edge relations,
  - Python-style escaped spaces in query values, including node equality constraints, feature continuation lines, regex constraints, and valued edge relation operators in materialized and mapped search,
  - negative integer query values in materialized and mapped node constraints, numeric comparisons, and valued edge relations,
  - Python-style `%` query comments in materialized and mapped search, including comments inside quantified blocks,
  - `/with/` and `/without/` quantified block filtering for contained subpatterns, including nested quantified alternatives in materialized and mapped search,
  - Python-style quantified parent containment references for `.. [[ child` and `child ]] ..` inside materialized and mapped quantified alternatives,
  - Python-style quantified parent atom references such as `.. phrase_id=1`, applying constraints to the quantified parent in materialized and mapped search,
  - Python-style quantified parent relation references such as `.. < child`, `.. # child`, and `.. .feature=feature. child` in materialized and mapped search,
  - mapped quantified search now preserves full relation plans in base templates and relation-bearing alternatives, while keeping the indexed atom-containment path for relation-free alternatives,
  - search limits,
  - generic node type search with `.`,
  - materialized custom search sets through `Search::search_with_sets`,
  - simple indented containment, including inline and standalone-line Python-style atom operators backed by the existing relation operator semantics such as `[[`, `]]`, `<:`, `:>`, and `-edge>`,
  - BHSA lexical, structural, and quantified query shapes from the curated benchmark patterns.
  - node identity relation operators `=` and `#`, plus slot-set relation operators `==`, `##`, `&&`, and `||`,
  - slot order/alignment relation operators `<<`, `>>`, `:>`, `=:`, `:=`, `::`, and Python-style parameterized near operators such as `=2:`, `:2=`, `:2:`, `<0:`, and `:0>`,
  - feature-to-feature search relation operators such as `.pos.`, `.pos#pos.`, `.number=phrase_id.`, `.number#phrase_id.`, `.number<number.`, `.number>number.`, and regex-normalized `.word~.+~word.` in both materialized and mapped search,
  - materialized and mapped edge relation operators with Python-style value constraints such as `-relation=subject>`, `<relation=predicate-`, bidirectional `<relation=subject>`, value alternatives, inequality, and regex value matching,
  - edge-case feature parsing for empty, single-node, and Unicode node features,
  - direct Python-style node feature access edge cases for empty features, node `0`, large node ids, integer values, sorted selection, sorted items, and frequency counts,
  - direct Python-style edge feature set semantics for sorted/deduplicated targets, empty traversal, missing traversal, items, and edge counts,
  - Python utility helper parity for Text-Fabric range specifications, normalized range formatting, logical range formatting, value-to-node indexes, edge inverses, valued-edge inverses, version sorting, byte-size formatting, console message formatting/output, string itemization, flexible itemization, tuple projection, set conversion, feature-list flattening, dictionary-of-sets merging, recursive dictionary merging, metadata description formatting, example list formatting, and JSON deep-size estimation,
  - Python helper public-name parity for exported helper aliases such as `setFromSpec`, `rangesFromSet`, `rangesFromList`, `specFromRanges`, `specFromRangesLogical`, `makeIndex`, `makeInverse`, `makeInverseVal`, `deepSize`, `valueFromTf`, and `tfFromValue`,
  - Python attrs utility parity for Rust-shaped `AttrDict` map access, missing-key `None` behavior, recursive JSON deep dictionary conversion, and non-string iterable detection,
  - Python small helper parity for UTC timestamp creation, environment variable lookup, and architecture warning/message tuple behavior,
  - Python helper public-name parity for documented camelCase aliases such as `isInt`, `mathEsc`, `mdEsc`, `htmlEsc`, `xmlEsc`, `mdhtmlEsc`, `tsvEsc`, `pandasEsc`, `cleanName`, and `isClean`,
  - Python logging utility parity for silent-level constants, boolean/string/None silent conversion, Python logging-level numeric mapping, and public `configure_logging` / `set_logging_level` names,
  - Python utility package export parity for non-downloader helpers such as `LOCATIONS`, `collectFormats`, `setDir`, `expandDir`, `splitExt`, and `scanDir`, using typed Rust return values where Python mutates module state or logger objects,
  - Python storage API parity for `StringPool`, `IntFeatureArray`, `CSRArray`, and `CSRArrayWithValues`, including ergonomic int/string valued-edge map constructors, save/load round-trips, vectorized filtering, lazy mmap manager metadata/array caching, and cache preload/release observables,
  - Python CLI argument reader parity for help/no-arg behavior, task parsing, `all` expansion with exclusions, parameter defaults, binary/ternary flags, order independence, and illegal argument reporting,
  - Python core API alias parity for `Api`, shared `CF`/`TF` Fabric references, sorted ignored-feature reporting, and `API_REFS` documentation entries,
  - Python top-level/config constant parity for version/banner, `__version__`, warp feature names, repository/backend defaults, URL defaults, search performance defaults, and CFM dtype/sentinel constants,
  - Python `cfabric.types` public alias parity for node identifiers, feature data maps, metadata maps, array aliases, slot ranges, search-result node lists, section specs, and node-type maps,
  - Python Fabric constructor public-state parity for `banner`, `version`, `good`, normalized single-location/module state, `locationRep`, and empty requested/ignored feature tracking, while keeping Rust loading path resolution direct and Rust-native,
  - Python result serialization parity for `NodeInfo`, `NodeList`, `SearchResult`, `FeatureInfo`, and `CorpusInfo` public dictionary shapes, including Python-compatible keys and omitted optional fields,
  - Python result wrapper JSON API parity for `to_json()` / `toJson()` on `NodeInfo`, `NodeList`, `SearchResult`, `FeatureInfo`, and `CorpusInfo`, serializing the same Python-compatible public dictionary shapes,
  - Python navigation package public export parity for `Nodes`, `Locality`, and `Text` as Rust-native wrappers over existing materialized corpus ordering, locality, walking, and text-rendering behavior,
  - Python downloader helper parity for empty corpus registry listing, full Hugging Face repo ID resolution, unknown short-name errors, `CFABRIC_CACHE` cache-directory override, compiled-only allow-pattern construction, and explicit unsupported errors for cache clearing / Rust network transport,
  - Python file/path helper parity for pure path normalization, home expansion/unexpansion, slash prefixing, directory/file/extension extraction, extension replacement, path splitting, and backend representation values,
  - Python filesystem helper parity for file/directory existence checks, file creation/removal/copy/move, directory creation/removal/copy/move, directory contents, recursive file listing, ignored directories, and empty-directory detection,
  - Python structured file helper parity for current-directory helpers plus JSON/YAML string/file read/write behavior and missing-file empty-map behavior,
  - Python public loader/compiler helper parity through `Data` / `TfData`, `Compiler`, and `compile_corpus` for path component parsing, missing-file failure state, metadata-only loads, node/edge/config load state, unload, `DATA_TYPES`, data-type defaulting, simple `.tf` save round-trips, explicit compiled-cache output, and default local compiled-cache output,
  - Python `Data` loader public-surface parity for loader constants and camelCase observable state/accessors such as `loadMetaOnly`, `setDataType`, `dataLoaded`, `dataError`, `dirName`, `fileName`, `edgeValues`, `isEdge`, `isConfig`, `metaData`, and `dataType`,
  - Python feature package public export parity for `NodeFeatures`, `EdgeFeatures`, `Computed`, `Computeds`, `RankComputed`, `OrderComputed`, `LevUpComputed`, `LevDownComputed`, `OtypeFeature`, and `OslotsFeature`, implemented as Rust-native empty containers, materialized-data wrappers, and aliases over the existing feature views,
  - Python search syntax public recognizer parity for atom lines, operator-prefixed atoms, feature identity/comparison/regex/presence constraints, search names/numbers, relation lines, quantifier lines, named atoms, standalone operator lines, indentation capture, k-nearness operators, and public regex exports such as `atomRe`, `identRe`, `relRe`, `quLineRe`, `kRe`, and `whiteRe`,
  - valued edge feature parsing for `@edgeValues` string and integer values,
  - Python-compatible Text-Fabric string value escaping for `\t`, `\n`, and escaped backslashes in node and valued edge features,
  - Python-style Text-Fabric value escape helpers for reading and writing string/int feature values,
  - Python-compatible integer node feature handling that preserves explicit zero values distinctly from missing values across materialized, compiled, and mapped loads,
  - invalid `.tf` file error handling for missing headers and malformed metadata sections,
  - clean corpus loading errors for missing corpus paths and empty corpus directories,
  - Python-style metadata-only `.tf` header parsing that reads feature kind, name, and metadata without parsing or retaining data rows,
  - Python-style `desc`/`eg` metadata normalization into `description` for materialized, compiled, and mapped feature descriptions,
  - Python-compatible fallback that preserves unknown `@valueType` metadata while parsing values as strings.
  - Python-compatible `@value_type` metadata alias handling for typed feature parsing and materialized/compiled/mapped feature metadata reporting, including loaded-feature introspection.
  - config feature parsing and corpus-level retention for `@config` metadata such as `otext`.
  - feature listing and computed Python-shaped `levels` tuples, mapped selected-cache feature listing aliases such as `Fall`/`Eall`/`Call`, dynamic materialized/mapped computed access with `Cs`, materialized/mapped `order`, `rank`, tuple sort-key access, Python-style `sortNodes`, `sortKey`, `sortKeyTuple`, `sortKeyChunk`, `sortKeyChunkLength`, `allNodes`, `walkEvents`, `otypeRank`, `maxSlot`, `maxNode`, `slotType`, and `boundary`.
  - Python text-format rendering parity for materialized and mapped text fallback chains with fixed defaults such as `{special/normal:.}` and layout escapes `\t` / `\n` in format literals.
  - Python precompute helper parity for standalone `levels`, `order`, `rank`, `levUp`, `levDown`, `sections`, `sectionsFromApi`, `structure`, `boundary`, and `characters` computations, including configured level ordering, exact canonical order/rank/boundary agreement with loaded corpus state, raw embedding precompute ordering, section lookup map output, structure heading/up/down maps, and public text-format character frequency output.
  - compiled-cache reopen parity for mapped `otype`, string node features, string edge values, locality descendants, filtered descendants, section metadata and lookup, and search results.
- Implemented direct `.tf` parsing, corpus feature loading, selective non-WARP feature loading, node feature indexes, mandatory edge containment through `oslots`, and a basic query engine.
- Added a compiled binary cache format in `src/compiled.rs` with string-pool encoding for string node features.
- Compiled cache now stores canonical `order` and `rank` arrays so compiled loads can sort by rank instead of recomputing comparator ranks.
- Compiled cache now appends config feature metadata after rank arrays while retaining compatibility with older caches that end after rank data.
- Compiled cache now appends a tagged node/edge feature metadata section for `valueType`, `description`, and `edgeValues` while retaining compatibility with older caches that end after config metadata.
- Added mmap-backed compiled-cache views for string-pool node features, mixed node features, edge features, config metadata, order/rank access, Python-style mapped feature aliases, vector-free mapped string-pool value interval lookup, mapped text rendering, mapped section references, and a reusable `MappedSearch` path that keeps mandatory `oslots` loaded.
- Mapped search now supports the same current slot-set and slot order/alignment relation operators as materialized search: `==`, `##`, `&&`, `||`, `<<`, `>>`, `:>`, `=:`, and `:=`.
- Mapped search now supports multi-child and nested indented containment for non-quantified templates, with query-local `oslots` slot caching and containment-aware candidate interval filtering to reduce repeated mmap row decoding and global child-candidate scans.
- Mapped search now reuses candidate rows for identical atom signatures within a plan, avoiding duplicate mmap feature scans in repeated-sibling query shapes.
- Materialized and mapped quantified search now support multi-atom base plans before applying `/with/` and `/without/` filters to the root row item; limited searches expand base rows adaptively so result-limited validation does not materialize full expensive base plans.
- Mapped quantified search now preserves base-plan relations and relation-bearing alternative plans instead of reducing them to atom-only checks.
- Materialized limited quantified search now streams and filters base rows as soon as they are complete, avoiding broad base-plan materialization for result-limited mixed base-plus-quantifier queries.
- Materialized quantified search now uses slot-interval indexed candidate pruning for root-contained alternatives, reducing broad candidate scans while retaining exact containment checks.
- Materialized indented containment search now uses the same slot-interval candidate pruning against already-bound parent nodes for ordinary nested structural queries, quantified base plans, and nested quantifier alternatives.
- Materialized and mapped search now support Python-style generic node type queries with `.`, including feature constraints and relation participation.
- Materialized and mapped search now support custom set candidate sources, including constraints and named relations, through `search_with_sets`.
- Materialized search now routes atom constraint filtering through the shared `NodeFeature` candidate-filter API, keeping planner filtering behavior aligned with direct feature filtering.
- Mapped search now centralizes no-seed full-candidate constraint filtering through the mapped feature candidate-filter API while preserving streaming early-exit behavior for limited direct searches.
- Added `src/bin/cf_rust_bhsa.rs`, `src/bin/cf_rust_bhsa_compiled.rs`, `src/bin/cf_rust_bhsa_mapped.rs`, `src/bin/cf_rust_validate_corpora.rs`, `src/bin/cf_rust_explore_corpora.rs`, `src/bin/cf_rust_validate_bhsa_curated.rs`, and `src/bin/cf_rust_validate_bhsa_mapped_curated.rs` as repeatable verification commands.
- Added `scripts/python_bhsa_mapped_subset.py` and `scripts/compare_bhsa_mapped_subset.py` as repeatable current-machine Python-vs-Rust timing comparisons for the expanded 104-query mapped-supported BHSA curated set, with enforced minimum load/query speedups, shared-query coverage, and default failure on individually slower Rust queries.
- Current Python-vs-Rust mapped BHSA timing proof passes with `load_speedup=37.50x` and `query_geomean_speedup=10.09x` across 104 shared queries, with no individual slower Rust query under the default gate.
- Added `scripts/compare_bhsa_mapped_memory.py` as a repeatable Python-vs-Rust peak RSS comparison for the same expanded 104-query mapped-supported BHSA curated set. Current proof passes with `python_peak_rss_mb=936.80`, `rust_peak_rss_mb=178.50`, and `rust_vs_python_rss_ratio=0.191`.
- Validated every local benchmark corpus with a `tf/` directory using mandatory `otype` and `oslots`, plus a basic slot-type query.
- Scope guard: remaining parity work is limited to documented or tested Python
  public API input/output behavior. Rust internals may stay Rust-native, and
  untested/undocumented edge behavior is not a port target unless it is required
  to match documented or tested public behavior.

## Verification Commands

```sh
cd libs/cf-rust
cargo test
cargo run --release --bin cf_rust_bhsa -- ../benchmarks/.corpora/bhsa/tf
cargo run --release --bin cf_rust_bhsa_compiled -- ../benchmarks/.corpora/bhsa/tf target/bhsa-core.cfr
cargo run --release --bin cf_rust_bhsa_mapped -- ../benchmarks/.corpora/bhsa/tf target/bhsa-core.cfr
cargo run --release --bin cf_rust_explore_corpora -- ../benchmarks/.corpora
cargo run --release --bin cf_rust_validate_corpora -- ../benchmarks/.corpora
cargo run --release --bin cf_rust_validate_bhsa_curated -- ../benchmarks/.corpora/bhsa/tf 5
cargo run --release --bin cf_rust_validate_bhsa_mapped_curated -- ../benchmarks/.corpora/bhsa/tf target/bhsa-mapped-curated.cfr 5
python scripts/compare_bhsa_mapped_subset.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
python scripts/compare_bhsa_mapped_memory.py --tf-path ../benchmarks/.corpora/bhsa/tf --cache-path target/bhsa-mapped-curated.cfr --limit 5
```

## Completion Status (0.6.0)

The June 2026 evaluation of 0.6.0rc1 against text-fabric 13.0.19 on BHSA found the
data model faithful (407,719/409,288 API checks) but surfaced three defect classes:
search-engine operator/quantifier semantics (only 8/34 ETCBC queries correct),
mapped-view performance (per-call view rebuilds), and binding-surface gaps
(`save`/`sets`/`shallow`/`lastSlot` ignored). These were remediated in the
`concurrent-rainbow` work (see `.claude/plans/`), and the release is now gated:

- **Search**: 34/34 ETCBC queries match TF counts and result sets, 0 crashes, all <= TF wall time (gate: `tests/parity` 34-query suite).
- **Performance**: steady-state per-call latency beats TF on every probe; load 0.018 s, RSS 236 MB (gate: recorded `libs/benchmarks/baselines/cf_0.6.0_record.json`).
- **Parity**: TF-oracle pytest (`CONTEXT_FABRIC_RUN_PARITY=1`) and golden 3-mode comparison (`tests/golden`) regenerated against TF truth.
- **API surface**: `sets`/`shallow`/`lastSlot`/`freqList` filters wired; `TF.save` round-trips back into Text-Fabric.

Documented divergences (intentional, see core CHANGELOG/README): result-*list*
order is unspecified-but-deterministic (parity is by count + set); `silent`/progress
knobs are no-ops; volumes/works and MQL are out of scope; `C.levUp`/`C.levDown` are
lazy views keyed by node id.

- Explicitly out of scope by user decision: Python downloader/network transport and exact Python `.cfm` internals unless a public local-cache behavior depends on them.
