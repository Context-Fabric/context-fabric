# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.3] - 2026-01-10 ([ck])

### Added
- PyPI project URLs (homepage, repository, issues)

## [0.1.2] - 2026-01-10 ([ck]) [YANKED]

### Note
- Broken release due to malformed pyproject.toml

## [0.1.1] - 2026-01-10 ([ck])

### Fixed
- Text-Fabric version detection now correctly reads from `tf.parameters.VERSION` instead of attempting to access a non-existent `__version__` attribute

## [0.1.0] - 2026-01-09 ([ck])

Initial release of the Context-Fabric benchmarking suite.

### Added
- **Memory benchmark**: Single-process, spawn, and fork mode memory measurement
- **Latency benchmark**: Search query execution time comparison with curated BHSA patterns
- **Progressive loading benchmark**: Multi-corpus scaling analysis with linear regression
- **CLI tool** (`cfabric-bench`): Commands for memory, latency, progressive, and full suite
- **Visualization**: PDF/PNG charts for all benchmark types
- **Corpora management**: Download and validation scripts for Text-Fabric corpora
- **Result loaders**: Reconstruct benchmark results from saved CSV/JSON for chart regeneration

---

[ck]: https://github.com/codykingham
