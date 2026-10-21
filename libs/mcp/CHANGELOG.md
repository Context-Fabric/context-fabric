# Changelog

All notable changes to cfabric-mcp will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- `search_syntax_guide("quantifiers")` explained `/where/ … /have/` as "contains both": an atom under `/have/` is a new one, so its example found the clauses in which every verb has some noun beside it, including all clauses without a verb (63,877 in the BHSA, against 45,896 with both). It now names the atom in `/where/` and refers to it in `/have/`, and gives "both" as two `/with/` blocks.

### Added
- `tools.using(registry)`: run the tools on another registry of corpora (anything with `get`, `get_api`, `list_corpora` and `current`, like `CorpusManager`) for the duration of a block (a context variable, safe across threads), so that the browser can offer the same tools on its own corpora; without it they work on the server's own corpora, as before

## [0.1.8] - 2026-10-14

### Added
- `CorpusManager.load()` accepts an ordered list of corpus locations (#8)
- Search result and passage responses include `cached_count` and `results_truncated` to distinguish cached results from the full match count (#8)

### Fixed
- Search counts, statistics, and CSV exports cover all matches instead of stopping at the 10,000-result cache limit (#8)
- `get_passages` no longer fails with `AttributeError` after loading selected features from a compiled corpus (#9)
- `cfabric_mcp.__version__` now reports the installed package version

## [0.1.7] - 2026-01-15

### Fixed
- SSE and HTTP transports now start correctly (`mcp.run()` was receiving unsupported `host`/`port` arguments)
- HTTP transport now uses correct FastMCP transport name (`streamable-http` instead of `http`)
- Logger propagation disabled to prevent duplicate log messages (FastMCP configures root logging)

## [0.1.6] - 2026-01-14

### Added
- New `search_csv` tool for exporting search results to CSV files
  - Writes directly to a file path for large result sets
  - Flattens multi-node results with positional prefixes (`node0_*`, `node1_*`, etc.)
  - Configurable delimiter (comma by default, use `\t` for TSV)
  - Only available on stdio transport (returns helpful error on HTTP/SSE)

## [0.1.5] - 2026-01-12

### Fixed
- Corrected quantifier documentation in search_syntax_guide: replaced non-existent regex-style syntax (`*`, `+`, `?`) with actual `/where/`, `/have/`, `/without/`, `/with/`, `/or/`, `/-/` syntax
- Added quantifier example to examples section

## [0.1.4] - 2026-01-10

### Added
- PyPI project URLs (homepage, repository, issues)

## [0.1.3] - 2026-01-10 [YANKED]

### Note
- Broken release due to malformed pyproject.toml

## [0.1.2] - 2026-01-10

### Fixed
- Corrected search syntax documentation: `>:` changed to `:>` for "immediately after" relation
- Fixed incorrect relation operator descriptions in search_syntax_guide:
  - `=:` now correctly described as "start at same slot" (was "same slots (co-extensive)")
  - `[[` now correctly described as "left embeds right" (was "starts at same position")
  - `]]` now correctly described as "left embedded in right" (was "ends at same position")
  - `::` now correctly described as "start and end at same slot (co-extensive)" (was "directly contained in")
  - Added missing `:=` operator for "end at same slot"

## [0.1.1] - 2026-01-10

### Added
- `docs` optional dependency group for API documentation generation

## [0.1.0] - 2026-01-06

### Added

- Initial release of the Context Fabric MCP server
- CLI-based corpus loading at startup via `--corpus` argument
- Support for multiple corpora with `--corpus name=/path` syntax
- Optional feature filtering with `--features` argument
- Query result caching with cursor-based pagination (5 minute TTL)

#### Transports

- `stdio` (default) - For Claude Desktop and local MCP clients
- `--sse PORT` - SSE transport for Cursor and remote MCP clients (endpoint: `/sse`)
- `--http PORT` - Streamable HTTP transport for production (endpoint: `/mcp`)

#### Tools (10 total)

> Usage guide is provided via MCP `instructions` field at connection time.

**Discovery (5):**
- `list_corpora()` - List all loaded corpora
- `describe_corpus(corpus?)` - Corpus structure overview (node types, sections)
- `list_features(kind?, node_types?, corpus?)` - Browse features with optional filtering
- `describe_feature(feature, sample_limit?, corpus?)` - Get detailed feature info with sample values
- `get_text_formats(corpus?)` - Text encoding samples (cached per corpus)

**Search (3):**
- `search(template, return_type?, ...)` - Pattern search with flexible output:
  - `return_type="results"` - Paginated node info with cursor
  - `return_type="count"` - Just total count
  - `return_type="statistics"` - Feature distributions per matched node
  - `return_type="passages"` - Formatted text with references
  - Invalid templates return detailed error messages automatically
- `search_continue(cursor_id, offset?, limit?)` - Paginate cached results
- `search_syntax_guide(section?)` - Section-based syntax documentation

**Data Access (2):**
- `get_passages(sections, lang?, corpus?)` - Batch section lookup with optional language code
- `get_node_features(nodes, features, corpus?)` - Batch feature lookup

#### Resources

- `corpus://{name}` - Corpus information
- `corpus://{name}/node/{id}` - Node information
- `corpus://{name}/features` - Feature list
- `corpus://{name}/types` - Node type information

### Fixed

- Large node textification no longer bloats MCP responses

### Design Decisions

- Pure MCP architecture - all platforms use the same MCP interface via different transports
- Corpora are pre-loaded at server startup rather than dynamically loaded via tools
- This provides a simpler, more secure model for production deployments
- Multiple corpora can be loaded and queried by name
- Hierarchical tool design optimized for iterative discovery with minimal token usage
- Cache by (corpus, template) allows different return_types to reuse results
