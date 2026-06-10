# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - 2026-06-10 ([ck])

Rust core rewrite. The pure-Python engine is replaced by a Rust/PyO3 implementation
packaged through maturin as `context-fabric`. The corpus stays memory-mapped at all
times; speed comes from cached view metadata, precomputed CSR indexes stored inside
the `.cfr`, and algorithmic fixes to the search engine. The public `tf.core` API
surface is preserved (existing signatures and return types unchanged).

### Changed
- Replaced the pure-Python core engine with the Rust/PyO3 implementation packaged through maturin.
- Core corpus loading is mmap-first; compiled cache and memory-mapped loading are public (`loadCompiled()`, `openMapped()`).
- Dropped the in-memory search executor; the mapped engine is now the only search engine (`search.rs` 2816 -> 590 lines).
- Bumped cache path versions (CFM 2, CFR 3); pre-v3 default caches are auto-invalidated and recompiled.

### Added
- **TF-core compatibility**: 34/34 real-world ETCBC query parity (counts and result sets match Text-Fabric); 407,719/409,288 public-API value checks pass against TF 13.0.19 on BHSA; valued edges; `TF.save` round-trip (TF reloads CF-authored `.tf`); `sets=`/`shallow=` search; multi-location corpus loading.
- **Performance** (BHSA, measured on an idle machine 2026-06-10; see `libs/benchmarks/baselines/cf_0.6.0_record.json`): steady-state per-call latency beats Text-Fabric on every probe — `L.u` 0.62 µs, `L.d` 0.75 µs, `E.mother.f` 0.14 µs, `T.sectionFromNode` 3.2 µs, `T.text` (verse) 7.1 µs, `F.v` 0.33 µs; load 0.018 s vs TF 8.6 s; resident memory 236 MB vs TF 6.4 GB; all 34 queries run in <= TF wall time.
- **Additive batch APIs** (opt-in; existing scalar methods unchanged): `F.<feat>.vs` / `vs_array` (node-array value lookup returning a list / numpy array), `L.u_many` / `L.d_many`, `T.text_many` — ~10x faster than per-node loops.
- **`.cfr` v3 format**: appended `levUp` / `levDown` / `boundary` / `sections` CSR sections (plain u32, mmap'd for random access, 0 added RSS); caches auto-invalidate via the bumped path version.
- Computed-feature views for `levUp` / `levDown` / `sections` / `characters` / `boundary`; `F.otype.all`, `N.sortKeyChunk`.
- Text-Fabric oracle parity gates, golden comparison tooling, a 34-query parity gate, a recorded per-call performance gate, and public API compatibility tests.

### Fixed
- Search engine: operator-prefixed atom lines (`< w`, `<: w`, `:> w`, `=: w`, `:= w`, `:k> w`) now emit TF's two-edge semantics (sibling relation added, not substituted for embedding), eliminating corpus-wide cross-products, always-empty results, and parse errors (was 8/34 queries correct, 19 timeouts, 3 crashes, 4 wrong-zeros).
- Quantifiers (`/without/`, `/where/have/`, `/with/or/`) rewritten to TF's single global sub-search + set algebra instead of one sub-search per candidate root.
- Adjacency and k-near relations use a slot index (binary search) instead of O(n^2) scans.
- Mapped-view performance: feature views no longer rebuild mmap views per call; locality, sections, and text contexts are handle-owned with lazy format compilation.
- Cross-corpus defects: multi-location compile, empty-string text fallback, `@levelConstraints`, section-0 feature selection, fast `nodeFromSection` miss.
- `L.i` ordering (removed a spurious reversal, confirmed against the TF oracle).
- Quadratic precompute paths (`levUp` via first-slot candidates, `sections` via precomputed level arrays): BHSA v3 compile dropped from 2.5h+ to ~57 s.
- Text-Fabric valued edge parsing for implicit `@edgeValues` rows and compact `target:value` syntax.
- `Fabric.load("a b c")` whitespace feature split; `freqList` node-type/edge filters; `T.text` accepts int or iterable; `sectionTuple` split from `sectionFromNode`; `lastSlot`; `S.glean`/`S.showPlan`; `walk(events=True)`.
- Downstream MCP and benchmark package imports for the new top-level `cfabric` API surface.

### Known divergences from Text-Fabric
- Result-*list* ordering is unspecified-but-deterministic; parity is defined by result counts and sets, not list order (TF's order is search-strategy dependent). Within-tuple column order matches template atom order.
- `silent` / progress-banner knobs are accepted but are no-ops.
- Volumes/works and MQL are out of scope.
- `C.levUp` / `C.levDown` are exposed as lazy views keyed by node id.

## [0.5.7] - 2026-01-15 ([ck])

### Fixed
- `.cfm` loading now auto-loads `textFeatures` before initializing Text API (mirrors `.tf` behavior)
  - Previously, `addText(api)` was called before section/format features were loaded, causing spurious "Node feature not loaded" errors
  - `load("")` now works correctly with `.cfm` format, providing full `api.T` functionality
- Logger propagation disabled to prevent duplicate log messages when used with libraries that configure root logging (e.g., FastMCP)

## [0.5.6] - 2026-01-15 ([ck])

### Fixed
- Windows path handling in `expandDir()`: absolute paths with drive letters (e.g., `D:/data/corpus`) were incorrectly treated as relative paths and prepended with current directory
  - Now uses `os.path.isabs()` instead of `startswith("/")` for cross-platform absolute path detection
  - Regression introduced in v0.5.1 when fixing hidden directory paths

## [0.5.5] - 2026-01-10 ([ck])

### Changed
- PyPI homepage now points to context-fabric.ai website
- Added documentation URL to project metadata

## [0.5.4] - 2026-01-10 ([ck])

### Added
- Comprehensive docstrings for search module (`Search` class, `syntax.py`, `semantics.py`)
- Module-level documentation for `utils/cli.py`

### Changed
- Documentation dependencies: replaced Sphinx with Griffe for API doc generation

## [0.5.3] - 2026-01-10 ([ck])

### Changed
- Renamed primary API attribute from `api.TF` to `api.CF` (Context-Fabric branding)
- `api.TF` remains as backward-compatible alias
- Updated all docstrings and comments from "TF" to "CF" and "Text-Fabric" to "Context-Fabric"
- `@writtenBy` metadata in compiled `.cfm` files now shows "Context-Fabric"

### Added
- `CF` entry in `API_REFS` documentation alongside existing `TF` entry
- Unit tests for API alias equivalence (`api.CF is api.TF`)

## [0.5.2] - 2026-01-09 ([ck])

### Added
- `benchmarks` optional dependency for installing cfabric-benchmarks suite

## [0.5.1] - 2026-01-09 ([ck])

### Fixed
- Path expansion bug in `expandDir()` that incorrectly handled hidden directory paths
  - `.corpora` was resolved to `{cwd}corpora` instead of `{cwd}/.corpora` (missing separator)
  - Caused by `str.replace(".", curDir, 1)` which didn't add a path separator
  - Now correctly handles: `.hidden` dirs, `./explicit` refs, bare `.`, and relative paths

## [0.5.0] - 2026-01-09 ([ck])

### Performance
- Vectorized feature filtering in StringPool/IntFeatureArray (~8x algorithmic speedup, ~1.3x net vs TF)
- Embedding relationship cache with RAM preloading (~1.7x speedup, offsetting mmap overhead; trades ~100MB RAM)
- SPIN search algorithm uses vectorized constraint filtering for mmap-backed features

### Changed
- Fix load("") behavior: empty string no longer loads all features
- Auto-preload embedding cache by default (controlled by CF_EMBEDDING_CACHE env var)

### Added
- Public preload API: C.levUp.preload(), C.levDown.preload(), .release(), .is_cached

## [0.4.1] - 2026-01-08 ([ck])

### Fixed
- Edge features with string values (`@edgeValues` + `@valueType=str`) now compile and load correctly from `.cfm` cache
- Compiler now preserves trailing whitespace in text format strings (e.g., `{word} ` in otext)
  - Previously, `.strip()` was used when parsing metadata which removed significant trailing spaces
  - This caused `T.text()` to produce different output between `.tf` and `.cfm` loading for corpora using format strings with literal trailing spaces (lxx, syrnt, tischendorf)
- String edge values are encoded as integer indices with JSON lookup to enable memory-mapping
- Removed silent fallback from corrupted `.cfm` to `.tf` loading - now fails loudly with actionable error message

### Changed
- Replaced misleading "legacy" terminology with "dict-based" (.tf loading) vs "mmap-based" (.cfm loading) throughout codebase
- Removed unused constants from TF fork: `EXPRESS_SYNC_LEGACY`, `APP_CONFIG_OLD`, `APP_CODE`

## [0.4.0] - 2026-01-07 ([ck])

### Added
- `Compiler.compile()` now accepts optional `precomputed` parameter for passing already-loaded data
- `Compiler._compile_from_precomputed()` for optimized compilation when all features are pre-loaded
- `Fabric._gather_precomputed_data()` to collect loaded data for compilation (only used when ALL features are loaded)

### Changed
- When ALL features are loaded via `loadAll()` before calling `compile()`, pre-computed data is passed to the Compiler to avoid redundant .tf file parsing. Falls back to disk-based compilation when only a subset of features is loaded.

## [0.3.1] - 2026-01-06 ([ck])

### Fixed
- Defensive bounds checking for out-of-bounds node IDs across the codebase:
  - `StringPool.get()` and `IntFeatureArray.get()` return `None` for invalid nodes
  - `RankComputed.__getitem__` returns fallback rank for invalid nodes
  - Added `safe_rank_key()` helper used by node/edge features, search, and navigation
  - Prevents crashes when corpus metadata references nodes beyond feature array bounds
  - Useful for corpora created as subsets of larger corpora where edge features may reference nodes outside the subset

## [0.3.0] - 2026-01-05 ([ck])

### Added
- `cfabric/results.py` - Rich result types for MCP server integration:
  - `NodeInfo` - Node representation with type, text, section, and features
  - `NodeList` - Paginated list of nodes with metadata
  - `SearchResult` - Search results with full node context
  - `FeatureInfo` - Feature metadata (name, kind, value type, description)
  - `CorpusInfo` - Corpus metadata (node types, features, section structure)
- `NodeInfo.section_ref` - Human-readable section reference (e.g., "Genesis 1:1")
- `FeatureInfo.sample_values` - Top N values by frequency for feature discovery
- `FeatureInfo.total_unique_values` - Count of unique feature values

### Changed
- Renamed `TF` variable to `CF` in documentation examples (README.md)

### Fixed
- `.cfm` loading now populates `TF.features` metadata for API compatibility with `.tf` loading
- `NodeInfo` converts numpy types to Python int for JSON serialization
- `FeatureInfo` handles both `valueType` (.tf) and `value_type` (.cfm) metadata keys
- `T.text()` now handles numpy integer node IDs from search results

## [0.2.1] - 2026-01-05 ([ck])

### Added
- Type annotations on function signatures across all public APIs
- GitHub Actions CI workflow for automated testing

### Changed
- Test directory internal structure now mirrors package layout (`tests/unit/features/`, `tests/unit/storage/`, etc.)
- `silentConvert()` now always returns a string (was `str | bool`)

### Fixed
- `versionSort()` handles non-matching regex patterns
- `rangesFromSet()` / `rangesFromList()` type safety improvements
- `console()` accepts optional `file` parameter
- `tfFromValue()` uses isinstance for proper type narrowing
- `explore()` explicitly returns `None` when `show=False`

## [0.2.0] - 2026-01-05 ([ck])

### Changed
- **Monorepo structure**: Reorganized to `libs/core/` layout for future multi-package support
- **Module reorganization**: Flat `core/` directory split into logical subdirectories:
  - `core/` - Main entry points (Fabric, Api, config)
  - `features/` - Feature classes (NodeFeature, EdgeFeature, warp features)
  - `io/` - Data loading and compilation
  - `storage/` - Memory-mapped storage backends (CSR, StringPool)
  - `navigation/` - Corpus navigation (Nodes, Locality, Text)
  - `precompute/` - Pre-computation logic
  - `search/` - Search engine
  - `utils/` - Utilities (helpers, files, timestamp)
- **Renamed modules** for clarity:
  - `parameters.py` → `config.py`
  - `nodefeature.py` → `features/node.py`
  - `edgefeature.py` → `features/edge.py`
  - `data.py` → `io/loader.py`
  - `compile.py` → `io/compiler.py`
  - `strings.py` → `storage/string_pool.py`
  - `generic.py` → `utils/attrs.py`
  - `command.py` → `utils/cli.py`
- Tests moved to `libs/core/tests/` alongside package

### Added
- `cfabric/types.py` - Type aliases for improved type safety
- `cfabric/py.typed` - PEP 561 marker for type checker support
- `cfabric/downloader/` - Stub for Hugging Face Hub corpus downloads
- `corpus-distribution-plan.md` - Strategy for community corpus distribution
- Workspace-level `pyproject.toml` with mypy configuration
- Package-level `pyproject.toml` in `libs/core/`

### Fixed
- Updated 47 legacy `tf.core` references to `cfabric`

### Testing
- All 700 tests passing
- Test fixtures path resolution fixed for new structure

## [0.1.0] - 2026-01-04 ([ck])

Initial release. Forked from Dirk Roorda's [Text-Fabric](https://github.com/annotation/text-fabric)
with a new memory-mapped storage format.

### Added
- Graph-based corpus engine for annotated text with efficient traversal and search
- Core APIs: N (Nodes), F (Features), E (Edges), L (Locality), T (Text), S (Search)
- Memory-mapped `.cfm` format using numpy arrays for on-demand data access
- Binary caching with gzip compression

### Performance (vs Text-Fabric on BHSA corpus — 1.4M nodes, 109 features)
- **2.9x faster** load time (2.4s vs 7.0s)
- **74% less memory** (1.6 GB vs 6.1 GB)
- **92% memory reduction** in fork mode with 4 parallel workers (440 MB vs 5.8 GB)
- **66% memory reduction** in spawn mode with 4 parallel workers (3.3 GB vs 9.8 GB)
- At cost of increased compile time and disk storage (good tradeoff)

Memory-mapped architecture makes Context-Fabric well-suited for API deployment scenarios
(e.g., MCP servers, FastAPI) where multiple workers share corpus data without duplicating memory.

### Testing
- 478 unit tests covering core functionality
- 87 integration tests for end-to-end workflows
- Requires Python 3.13+

---

[ck]: https://github.com/codykingham
