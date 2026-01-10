# Changelog

All notable changes to cfabric-mcp will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
