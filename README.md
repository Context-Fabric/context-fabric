<p align="center">
  <img src="assets/fabric_tan_mark_light.svg" alt="Context Fabric" width="120">
</p>

<h1 align="center">Context-Fabric</h1>

<p align="center">A graph-based corpus engine for annotated text with efficient traversal and search.</p>

## Overview

Context-Fabric provides a powerful data model for working with annotated text corpora as graphs. It enables efficient navigation, feature lookup, and pattern-based search across large textual datasets.

Forked from Dirk Roorda's [Text-Fabric](https://github.com/annotation/text-fabric).

## Installation

```bash
pip install context-fabric-core
```

## Quick Start

```python
from cfabric.core import Fabric

# Load a dataset
TF = Fabric(locations='path/to/data')
api = TF.load('feature1 feature2')

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

### Benchmarks (BHSA Hebrew Bible corpus)

| Metric | Text-Fabric | Context-Fabric | Improvement |
|--------|-------------|----------------|-------------|
| **Load Time** | 7.21s | 2.54s | **2.8x faster** |
| **Memory Usage** | 4.9 GB | 473 MB | **90% reduction** |
| Compile Time | 62s | 93s | 1.5x slower |
| Cache Size | 138 MB | 859 MB | 6.2x larger |

<p align="center">
  <img src="benchmarks/results/performance_comparison.png" alt="Performance Comparison" width="700">
</p>

The key insight: **compilation happens once, loading happens every session**. Context-Fabric optimizes for the common case with:

- **Memory-mapped arrays**: Data stays on disk, accessed on-demand
- **Efficient sparse iteration**: Uses numpy vectorized operations instead of Python loops
- **Cached materialization**: Dictionary views computed once per session

Run the benchmark yourself:

```bash
python benchmarks/compare_performance.py --source path/to/tf/data
```

## Testing

See [TESTING.md](TESTING.md) for how to run tests.

## Authors

- Cody Kingham
- Dirk Roorda

## License

MIT License - see [LICENSE](LICENSE) for details.
