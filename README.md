<p align="center">
  <img src="assets/fabric_tan_mark_light.svg" alt="Context-Fabric" width="120">
</p>

<h1 align="center">Context-Fabric</h1>

<p align="center">
  <strong>Production-ready corpus analysis for the age of AI</strong>
</p>

<p align="center">
  <a href="https://pypi.org/project/context-fabric/"><img src="https://img.shields.io/pypi/v/context-fabric?color=blue" alt="PyPI"></a>
  <a href="https://pypi.org/project/context-fabric/"><img src="https://img.shields.io/pypi/pyversions/context-fabric" alt="Python"></a>
  <a href="https://github.com/codykingham/context-fabric/actions"><img src="https://img.shields.io/github/actions/workflow/status/codykingham/context-fabric/ci.yml?branch=master" alt="CI"></a>
  <a href="https://github.com/codykingham/context-fabric/blob/master/LICENSE"><img src="https://img.shields.io/github/license/codykingham/context-fabric" alt="License"></a>
</p>

---

## The Next Chapter

In 2016, [Text-Fabric](https://github.com/annotation/text-fabric) changed how researchers work with annotated text. Its standoff data model, data science focus, and Python-native processing brought computational analysis to ancient texts, manuscripts, and linguistic corpora worldwide.

**Context-Fabric carries that legacy forward into the AI era.**

Built on the same proven graph-based data model, Context-Fabric introduces a memory-mapped architecture that enables what Text-Fabric couldn't: true parallel processing for production deployments. This means corpus analysis can now power REST APIs, multi-worker services, and—crucially—AI agent tools via the Model Context Protocol (MCP).

---

## Why Context-Fabric?

### Built for Parallelization

Text-Fabric loads entire corpora into memory—effective for single-user research, but each parallel worker duplicates that memory footprint. Context-Fabric's memory-mapped arrays change the equation:

| Scenario | Text-Fabric | Context-Fabric | Improvement |
|----------|-------------|----------------|-------------|
| Single process | 5.2 GB | 300 MB | 94% less |
| 4 workers (spawn) | 6.0 GB | 1.2 GB | 79% less |
| 4 workers (fork) | 6.3 GB | 397 MB | **94% less** |

*Benchmarks on BHSA Hebrew Bible corpus (1.4M nodes). Memory measured as total RSS after loading from cache.*

Multiple workers share the same memory-mapped data instead of each loading a copy. This architecture unlocks production use cases that were previously impractical.

### Ready for AI Agents

Context-Fabric includes **cfabric-mcp**, a Model Context Protocol server that exposes corpus operations to AI agents. Claude, GPT, and other LLM-powered tools can now search, navigate, and analyze annotated text corpora directly.

```bash
# Start the MCP server
cfabric-mcp --corpus /path/to/bhsa

# Or with SSE transport for remote clients
cfabric-mcp --corpus /path/to/bhsa --sse 8000
```

The server provides 10 tools for discovery, search, and data access—designed for iterative, token-efficient agent workflows.

→ [MCP Server Documentation](libs/mcp/README.md)

### Same Powerful Data Model

Context-Fabric preserves Text-Fabric's core strengths:

- **Standoff annotation**: Layers of analysis without modifying source text
- **Graph traversal**: Navigate hierarchical structures (words → clauses → sentences → documents)
- **Pattern search**: Find complex linguistic patterns with structural templates
- **Feature system**: Arbitrary annotations on any node or edge

---

## Installation

```bash
# Core library
pip install context-fabric

# With MCP server
pip install context-fabric[mcp]
```

## Quick Start

```python
from cfabric.core import Fabric

# Load a corpus
CF = Fabric(locations='path/to/corpus')
api = CF.load('feature1 feature2')

# Navigate nodes
for node in api.N():
    print(api.F.feature1.v(node))

# Traverse structure
embedders = api.L.u(node)  # nodes containing this node
embedded = api.L.d(node)   # nodes within this node

# Search patterns
results = api.S.search('''
clause
  phrase function=Pred
    word sp=verb
''')
```

## Core API

| API | Purpose |
|-----|---------|
| **N** | Walk nodes in canonical order |
| **F** | Access node features |
| **E** | Access edge features |
| **L** | Navigate locality (up/down the hierarchy) |
| **T** | Retrieve text representations |
| **S** | Search with structural templates |

---

## Performance

Context-Fabric optimizes for the common case: **compilation happens once, loading happens every session**.

| Metric | Text-Fabric | Context-Fabric |
|--------|-------------|----------------|
| Load time | 8.1s | 0.5s (16x faster) |
| Memory | 5.2 GB | 300 MB (94% less) |
| Compile time | 62s | 30s (2x faster) |
| Cache size | 138 MB | 387 MB |

The larger cache size pays off immediately in faster loads and dramatically in parallel deployments.

<p align="center">
  <img src="benchmarks/results/performance_comparison.png" alt="Performance Comparison" width="700">
</p>

Run benchmarks yourself:

```bash
python benchmarks/compare_performance.py --source path/to/tf/data --workers 4
```

---

## Packages

| Package | Description |
|---------|-------------|
| [context-fabric](libs/core/) | Core graph engine |
| [cfabric-mcp](libs/mcp/) | MCP server for AI agents |

## Links

- [Core Changelog](libs/core/CHANGELOG.md)
- [MCP Changelog](libs/mcp/CHANGELOG.md)
- [Testing Guide](TESTING.md)

## Authors

Context-Fabric by [Cody Kingham](https://github.com/codykingham), built on [Text-Fabric](https://github.com/annotation/text-fabric) by [Dirk Roorda](https://github.com/dirkroorda).

## License

MIT
