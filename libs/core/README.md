<p align="center">
  <img src="../../assets/fabric_tan_mark.svg" alt="Context Fabric" width="120">
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
from cfabric import Fabric

# Load a dataset
CF = Fabric(locations='path/to/data')
api = CF.load('feature1 feature2')

# Navigate nodes
for node in api.N.walk():
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

## Compatibility

Context-Fabric is `tf.core` API-compatible: the N/F/E/L/T/S surfaces keep
Text-Fabric's signatures and return types, search templates parse identically, and
`TF.save`-authored `.tf` files reload in Text-Fabric. On BHSA, 34/34 reference
ETCBC queries match Text-Fabric result counts and sets.

Known divergences:

- **Result-list ordering** is unspecified-but-deterministic. Parity is defined by
  result counts and sets, not list order (Text-Fabric's order is search-strategy
  dependent). Within-tuple column order matches template atom order.
- **`silent` / progress knobs** are accepted but are no-ops.
- **Volumes/works and MQL** are out of scope.
- **`C.levUp` / `C.levDown`** are exposed as lazy views keyed by node id.

## Performance

Context-Fabric uses a Rust core that keeps the corpus memory-mapped at all times.
Loading is near-instant, resident memory stays low, and per-call latency beats
Text-Fabric on every probe. Compilation to the `.cfr` cache happens once; loading
happens every session.

### Benchmarks (BHSA Hebrew Bible corpus — 1.4M nodes, 109 features)

Measured against Text-Fabric 13.0.19 on an idle machine, 2026-06-10
(`libs/benchmarks/baselines/cf_0.6.0_record.json`).

| Metric | Text-Fabric | Context-Fabric | Improvement |
|--------|-------------|----------------|-------------|
| **Load time** | 8.6 s | 0.018 s | **~477x faster** |
| **Resident memory** | 6.4 GB | 236 MB | **~27x less** |
| Compile time | ~8 s | ~57 s | one-time cost |
| Cache size | 138 MB | ~498 MB | larger (mmap'd, on-demand) |

### Per-call latency (steady state)

Median per-call latency after warmup. Context-Fabric reads from mmap'd CSR
indexes and cached view metadata rather than loading the corpus into RAM.

| Operation | Text-Fabric | Context-Fabric |
|-----------|-------------|----------------|
| `F.<feat>.v` (node feature) | 0.43 µs | 0.33 µs |
| `E.<feat>.f` (edge feature) | 0.23 µs | 0.14 µs |
| `L.u` (containing nodes) | 0.79 µs | 0.62 µs |
| `L.d` (contained nodes) | 4.36 µs | 0.75 µs |
| `T.sectionFromNode` | 9.55 µs | 3.2 µs |
| `T.text` (verse) | 21.0 µs | 7.1 µs |

All 34 ETCBC reference queries return Text-Fabric counts in <= Text-Fabric wall time.

Because the data stays memory-mapped, multiple workers (spawn or fork) share the
same on-disk pages instead of each duplicating the corpus in RAM, which is the
core advantage for multi-worker API deployments.

Run the benchmarks yourself:

```bash
pip install context-fabric[benchmarks]
python -m cfabric_benchmarks.perf_gate --engine cf --assert \
    --targets baselines/targets.json --baseline baselines/bhsa_tf.json
```

## Testing

See [TESTING.md](TESTING.md) for how to run tests.

## Authors

- Cody Kingham
- Dirk Roorda

## License

MIT License - see [LICENSE](LICENSE) for details.
