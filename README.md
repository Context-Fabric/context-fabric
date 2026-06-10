<p align="center">
  <img src="assets/fabric_tan_mark.svg" alt="Context-Fabric" width="120">
</p>

<h1 align="center">Context-Fabric</h1>

<p align="center">
  <strong>Production-ready corpus analysis for the age of AI</strong>
</p>

<p align="center">
  <a href="https://pypi.org/project/context-fabric/"><img src="https://img.shields.io/pypi/v/context-fabric?color=blue" alt="PyPI"></a>
  <a href="https://pypi.org/project/context-fabric/"><img src="https://img.shields.io/pypi/pyversions/context-fabric" alt="Python"></a>
  <a href="https://github.com/Context-Fabric/context-fabric/actions"><img src="https://img.shields.io/github/actions/workflow/status/Context-Fabric/context-fabric/ci.yml?branch=master" alt="CI"></a>
  <a href="https://github.com/Context-Fabric/context-fabric/blob/master/LICENSE"><img src="https://img.shields.io/github/license/Context-Fabric/context-fabric" alt="License"></a>
</p>

<p align="center">
  <img src="assets/demo-terminal.gif" alt="Context-Fabric MCP Server Demo" width="700">
</p>

<p align="center">
  <em>AI agents running advanced grammatical queries via the Model Context Protocol</em>
</p>

---

## Overview

Context-Fabric brings corpus analysis into the AI era. Built on the proven [Text-Fabric](https://github.com/annotation/text-fabric) data model, it introduces a memory-mapped architecture enabling parallel processing for production deployments—REST APIs, multi-worker services, and AI agent tools via MCP.

- **Built for Production** — Memory-mapped arrays enable true parallelization. Multiple workers share data instead of duplicating it.
- **AI-Native** — MCP server exposes corpus operations to Claude, GPT, and other LLM-powered tools.
- **Powerful Data Model** — Standoff annotation, graph traversal, pattern search, and arbitrary feature annotations.
- **Dramatic Efficiency** — on BHSA, ~477x faster loads and ~27x less resident memory than Text-Fabric, with per-call latency that beats it on every probe.

→ [Read the Technical Paper](docs/intro-to-cf/intro-to-cf.pdf)

---

## MCP Server for AI Agents

Context-Fabric includes **cfabric-mcp**, a Model Context Protocol server that exposes corpus operations to AI agents:

```bash
# Start the MCP server
cfabric-mcp --corpus /path/to/bhsa

# Or with SSE transport for remote clients
cfabric-mcp --corpus /path/to/bhsa --sse 8000
```

The server provides 10 tools for discovery, search, and data access—designed for iterative, token-efficient agent workflows.

→ [MCP Server Documentation](libs/mcp/README.md)

---

## Memory Efficiency

Text-Fabric loads entire corpora into memory—effective for single-user research, but each parallel worker duplicates that memory footprint. Context-Fabric keeps the corpus memory-mapped, so resident memory stays low and multiple workers share the same on-disk pages instead of duplicating the corpus in RAM.

On BHSA (1.4M nodes, 109 features), resident memory after load is **236 MB** versus Text-Fabric's **6.4 GB** — about **27x less** in a single process, with the gap widening across parallel workers because the mmap'd pages are shared.

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
from cfabric import Fabric

# Load a corpus
CF = Fabric(locations='path/to/corpus')
api = CF.load('feature1 feature2')

# Navigate nodes
for node in api.N.walk():
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

Context-Fabric trades **one-time compilation cost** for **dramatic runtime efficiency**. Compile once, benefit forever.

Measured on BHSA against Text-Fabric 13.0.19 (idle machine, 2026-06-10):

| Metric | Text-Fabric | Context-Fabric | Improvement |
|--------|-------------|----------------|-------------|
| Load time | 8.6 s | 0.018 s | ~477x faster |
| Resident memory | 6.4 GB | 236 MB | ~27x less |
| Compile time | ~8 s | ~57 s | one-time cost |

Per-call latency beats Text-Fabric on every probe (e.g. `L.d` 0.75 µs vs 4.36 µs, `T.text` 7.1 µs vs 21.0 µs), and all 34 ETCBC reference queries return Text-Fabric counts in <= Text-Fabric wall time. Full numbers and the per-call table are in the [core README](libs/core/README.md#performance) and `libs/benchmarks/baselines/cf_0.6.0_record.json`.

Run benchmarks yourself:

```bash
pip install context-fabric[benchmarks]
python -m cfabric_benchmarks.perf_gate --engine cf --assert
```

---

## Packages

| Package | Description |
|---------|-------------|
| [context-fabric](libs/core/) | Core graph engine |
| [cfabric-mcp](libs/mcp/) | MCP server for AI agents |
| [cfabric-benchmarks](libs/benchmarks/) | Performance benchmarking suite |

## Links

- [Core Changelog](libs/core/CHANGELOG.md)
- [MCP Changelog](libs/mcp/CHANGELOG.md)
- [Benchmarks Changelog](libs/benchmarks/CHANGELOG.md)
- [Testing Guide](TESTING.md)

## Citation

If you use Context-Fabric in your research, please cite:

> Kingham, Cody. ["Carrying Text-Fabric Forward: Context-Fabric and the Scalable Corpus Ecosystem."](https://github.com/Context-Fabric/context-fabric/blob/master/docs/intro-to-cf/intro-to-cf.pdf) January 2026.

## Authors

Context-Fabric by [Cody Kingham](https://github.com/codykingham), built on [Text-Fabric](https://github.com/annotation/text-fabric) by [Dirk Roorda](https://github.com/dirkroorda).

## License

MIT
