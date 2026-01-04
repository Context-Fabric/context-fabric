# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-01-04

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
