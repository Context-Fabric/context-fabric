# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
