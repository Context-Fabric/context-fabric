# Context-Fabric MCP Server

Query Context-Fabric corpora via MCP (Model Context Protocol).

## Installation

```bash
pip install cfabric-mcp
```

## Transports

The server supports three transport modes:

| Transport | Flag | Use Case |
|-----------|------|----------|
| stdio | (default) | Claude Desktop, local MCP clients |
| SSE | `--sse PORT` | Cursor, remote MCP clients |
| Streamable HTTP | `--http PORT` | Production deployments |

## Usage

```bash
# stdio transport (Claude Desktop)
cfabric-mcp --corpus /path/to/bhsa

# SSE transport (Cursor, remote clients)
cfabric-mcp --corpus /path/to/bhsa --sse 8000

# Streamable HTTP transport (production)
cfabric-mcp --corpus /path/to/bhsa --http 8000
```

### Options

```bash
--corpus PATH              # Corpus to load (required, repeatable)
--corpus name=PATH         # Named corpus
--sse PORT                 # SSE transport on port (endpoint: /sse)
--http PORT                # Streamable HTTP transport on port (endpoint: /mcp)
--host HOST                # Host to bind (default: 0.0.0.0)
--features "sp lex"        # Load only specific features
--verbose                  # Debug logging
```

## Tools

The server exposes 11 MCP tools organized in layers for efficient discovery.

> **Note:** A usage guide is automatically provided to clients via the MCP `instructions` field at connection time.

### Discovery Tools

| Tool | Description |
|------|-------------|
| `list_corpora` | List loaded corpora |
| `describe_corpus` | Get corpus structure (node types, sections) |
| `list_features` | Browse features with optional node_type filter |
| `describe_feature` | Get feature details with sample values |
| `get_text_formats` | Get text encoding samples (cached) |

### Search Tools

| Tool | Description |
|------|-------------|
| `search` | Pattern search (results/count/statistics/passages) |
| `search_continue` | Paginate search results |
| `search_csv` | Export results to CSV file (stdio only) |
| `search_syntax_guide` | Search syntax docs (section-based) |

Search totals count all matches, including matches beyond the cursor cache's
10,000-result retention limit. Counts remain exact on cache hits. Statistics
use all matches; when only a prefix is cached, statistics may rerun the query.
Result and passage responses include `cached_count` and `results_truncated`
to distinguish retained results from `total_count`. Pagination's `has_more`
describes remaining cached results, so it becomes false at the cache boundary
even when `results_truncated` is true.

### Data Access Tools

| Tool | Description |
|------|-------------|
| `get_passages` | Get text by section references |
| `get_node_features` | Get feature values for nodes |

## Discovery Flow

The tools are designed for hierarchical, iterative discovery:

```
describe_corpus()
│
├─► list_features()                  → Browse all features
│       └─► describe_feature("sp")   → Full details + samples
│
├─► list_features(node_types=["word"])  → Filter by node type
│       └─► describe_feature(...)
│
└─► get_text_formats()               → When encoding matters

        ↓
    search()  → Execute queries
```

**Typical workflow:**
1. `describe_corpus()` - Get structure overview
2. `list_features()` or `list_features(node_types=["word"])` - Browse available features
3. `describe_feature("sp")` - Deep dive into specific feature with sample values
4. `search_syntax_guide()` - Get search syntax help (section-based)
5. `search()` - Execute search queries

### Search Return Types

| Type | Description |
|------|-------------|
| `results` | Paginated node info with cursor (default) |
| `count` | Total count only |
| `statistics` | Feature value distributions |
| `passages` | Formatted text passages |

## Client Configuration

### Claude Desktop (stdio)

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "context-fabric": {
      "command": "/path/to/venv/bin/cfabric-mcp",
      "args": ["--corpus", "/path/to/bhsa"]
    }
  }
}
```

### Cursor (SSE)

Start server:
```bash
cfabric-mcp --corpus /path/to/bhsa --sse 8000
```

Configure Cursor to connect to: `http://localhost:8000/sse`

### Custom MCP Clients

For SSE transport, connect to `http://host:port/sse`
For Streamable HTTP, connect to `http://host:port/mcp`

## License

MIT
