# Context Fabric

A graph-based corpus engine for annotated text with efficient traversal and search.

## Overview

Context Fabric provides a powerful data model for working with annotated text corpora as graphs. It enables efficient navigation, feature lookup, and pattern-based search across large textual datasets.

Based on [Text-Fabric](https://github.com/annotation/text-fabric) by Dirk Roorda.

## Installation

```bash
pip install context-fabric-core
```

## Quick Start

```python
from core import FabricCore

# Load a dataset
TF = FabricCore(locations='path/to/data')
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

## Testing

See [TESTING.md](TESTING.md) for how to run tests.

## Authors

- Cody Kingham
- Dirk Roorda

## License

MIT License - see [LICENSE](LICENSE) for details.
