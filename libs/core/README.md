<p align="center">
  <img src="../../assets/fabric_tan_mark_light.svg" alt="Context Fabric" width="120">
</p>

<h1 align="center">Context-Fabric</h1>

<p align="center">A graph-based corpus engine for annotated text with efficient traversal and search.</p>

## Overview

Context-Fabric provides a powerful data model for working with annotated text corpora as graphs. It enables efficient navigation, feature lookup, and pattern-based search across large textual datasets.

Forked from Dirk Roorda's [Text-Fabric](https://github.com/annotation/text-fabric).

## Installation

```bash
pip install context-fabric
```

## Quick Start

```python
from cfabric.core import Fabric

# Load a dataset
CF = Fabric(locations='path/to/data')
api = CF.load('feature1 feature2')

# Navigate nodes
for node in api.N():
    print(api.F.feature1.v(node))

# Use locality
embedders = api.L.u(node)
embedded = api.L.d(node)
```

## Core API

- **N** (Nodes) - Walk through nodes in canonical order
- **F** (Features) - Access node feature values
- **E** (Edges) - Access edge feature values
- **L** (Locality) - Navigate between related nodes
- **T** (Text) - Retrieve text representations
- **S** (Search) - Search using templates

## Performance

Context-Fabric uses memory-mapped numpy arrays for dramatically faster loading and reduced memory consumption compared to Text-Fabric's pickle-based caching.

### Benchmarks (BHSA Hebrew Bible corpus — 1.4M nodes, 109 features)

| Metric | Text-Fabric | Context-Fabric | Improvement |
|--------|-------------|----------------|-------------|
| **Load Time** | 7.9s | 0.7s | **11x faster** |
| **Memory Usage** | 6.3 GB | 305 MB | **95% reduction** |
| Compile Time | 8s | 91s | one-time cost |
| Cache Size | 138 MB | 859 MB | 6x larger |

<p align="center">
  <img src="../../benchmarks/results/performance_comparison.png" alt="Performance Comparison" width="700">
</p>

The key insight: **compilation happens once, loading happens every session**. Context-Fabric trades one-time compile cost for dramatic runtime efficiency:

- **Memory-mapped arrays**: Data stays on disk, accessed on-demand
- **Efficient sparse iteration**: Uses numpy vectorized operations instead of Python loops
- **Cached materialization**: Dictionary views computed once per session

### Parallel Worker Scaling

Memory-mapped arrays enable efficient parallel processing. Multiple workers share the same mmap'd data instead of each loading a full copy into RAM.

**Spawn mode** (cold start — each worker loads independently):

| Metric | Text-Fabric | Context-Fabric | Savings |
|--------|-------------|----------------|---------|
| Total (4 workers) | 7.7 GB | 1.3 GB | **84% less** |
| Per worker | 1.9 GB | 315 MB | **6x less** |

**Fork mode** (API scenario — pre-load then fork workers):

| Metric | Text-Fabric | Context-Fabric | Savings |
|--------|-------------|----------------|---------|
| Total (4 workers) | 6.3 GB | 398 MB | **94% less** |
| Per worker | 1.6 GB | 99 MB | **16x less** |

*Memory measured as total RSS after loading from cache on BHSA corpus.*

Run the benchmark yourself:

```bash
python benchmarks/compare_performance.py --source path/to/tf/data --workers 4
```

## Testing

See [TESTING.md](TESTING.md) for how to run tests.

## Authors

- Cody Kingham
- Dirk Roorda

## License

MIT License - see [LICENSE](LICENSE) for details.
