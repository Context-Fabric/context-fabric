<p align="center">
  <img src="https://raw.githubusercontent.com/Context-Fabric/context-fabric/master/assets/fabric_tan_mark.svg" alt="Context Fabric" width="120">
</p>

<h1 align="center">Context-Fabric</h1>

<p align="center">A graph-based corpus engine for annotated text with efficient traversal and search.</p>

## Overview

Context-Fabric works with annotated text corpora as graphs: nodes (words,
phrases, verses, books, ...) carry feature values, edges relate nodes, and
templates search for structural patterns. It reads Text-Fabric datasets and
keeps the Text-Fabric API, on a Rust core that serves every query from a
memory-mapped compiled cache.

Forked from Dirk Roorda's [Text-Fabric](https://github.com/annotation/text-fabric).

## Installation

```bash
pip install context-fabric
```

Requires Python 3.13 or later. Wheels are built for Linux (x86_64, aarch64),
macOS (arm64, x86_64) and Windows (x86_64); other platforms build from the
source distribution, which needs a Rust toolchain.

Optional extras:

| Extra | Adds |
|---|---|
| `context-fabric[numpy]` | NumPy, for `F.<feature>.vs_array` |
| `context-fabric[download]` | `huggingface_hub`, for `cfabric.download` |
| `context-fabric[mcp]` | the `cfabric-mcp` server for AI agents |
| `context-fabric[benchmarks]` | the `cfabric-benchmarks` suite |

## Quick start

```python
from cfabric import Fabric

CF = Fabric(locations="path/to/tf")
api = CF.loadAll()                     # or CF.load("sp lex") for selected features
F, L, T, S = api.F, api.L, api.T, api.S

for word in F.otype.s("word")[:5]:
    clause = L.u(word, otype="clause")[0]
    print(T.sectionFromNode(word), F.sp.v(word), T.text(clause))

for (phrase, word) in S.search("""
phrase function=Pred
  word sp=verb
""")[:5]:
    print(T.text(phrase))
```

The first load of a corpus compiles its `.tf` files into a `.cfr` cache in
`<corpus>/.cfr/4/` (or, when that directory is read-only, under
`cfabric.get_cache_dir()`: `$CFABRIC_CACHE`, else the platform cache
directory). Later loads map the cache and are near-instant; the cache is
rebuilt when a `.tf` file changes or a release changes the cache format.

## Core API

- **N** (Nodes): walk nodes in canonical order
- **F** (Features): node feature values
- **E** (Edges): edge feature values
- **L** (Locality): embedders, embeddees, neighbours of nodes
- **T** (Text): text formats, sections, structure
- **S** (Search): search templates
- **C** (Computed): precomputed data (levels, order, rank, levUp, levDown, ...)

## Text-Fabric compatibility

Context-Fabric keeps the `tf.core` API: `Fabric` (also importable as
`cfabric.fabric.Fabric`), the `api` object with its aliases (`Feature`, `Fs`,
`AllFeatures`, `TF`, ...), and the F/E/C/L/N/T/S members keep Text-Fabric's
names, signatures, and return types; `tests/python/test_tf_api_surface.py`
checks this against Text-Fabric 13. Search templates parse as in Text-Fabric,
and `.tf` files written by `TF.save` load in Text-Fabric. On BHSA, the 34
reference ETCBC queries return Text-Fabric's result sets.

### Differences from Text-Fabric

- **Result order**: search results come in a deterministic order that can
  differ from Text-Fabric's (which depends on its search strategy); results
  match as sets. Within a result tuple, nodes follow the template's atoms.
- **`S.search`** returns a tuple (a set with `shallow`) rather than a
  generator, a malformed template may raise instead of returning no results,
  and a feature repeated within one atom applies every condition (Text-Fabric
  keeps the last). `S.count()` returns the count.
- **`silent` and progress options** are accepted; only the `TF.info` /
  `warning` / `error` messages honor `silent`.
- **`C.levUp` / `C.levDown`** are lazy sequences indexed like Text-Fabric's
  tuples; `row(n)` reads one node's row.
- **Slot and level rows** (`E.oslots.s`, `E.oslots.data`, `C.levDown.data`,
  `C.rank.data`) are tuples where Text-Fabric has `array('I')`.
- **`T.nodeFromSection`** matches section values regardless of str/int type
  (Text-Fabric needs the feature's type); `T.nodeFromHeading` is type-exact.
- **`TF.features`** entries describe the feature files but carry no `data`;
  feature data is read through `F` / `E` / `C`.
- **Additions**: `F.otype.freqList`, `E.oslots.f/t/b`, `E.oslots.dataInv`, and
  the batch calls `F.<feature>.vs` / `vs_array`, `L.u_many` / `L.d_many`,
  `T.text_many`.
- **Out of scope**: volumes/works and MQL.

## Performance

Measured on BHSA (Hebrew Bible, 1.4M nodes, 109 features) against Text-Fabric
13.0.19 on the same machine with `cfabric_benchmarks.perf_gate`
(`libs/benchmarks/baselines/cf_0.6.0_record.json` and `bhsa_tf.json`):

| | Text-Fabric | Context-Fabric |
|---|---|---|
| Load (compiled) | 4.3 s | 0.003 s |
| Memory added by loading | 6.1 GB | 8 MB |
| Peak memory after the gate workload | 6.3 GB | 262 MB |
| 12 gate queries, total | 2.36 s | 0.19 s |
| First load (compiles the cache) | ~38 s | ~30 s |
| Cache on disk | 138 MB | 614 MB |

Steady-state latency per call, µs:

| Call | Text-Fabric | Context-Fabric |
|---|---|---|
| `F.sp.v` | 0.064 | 0.028 |
| `E.mother.f` | 0.033 | 0.031 |
| `E.mother.t` | 0.044 | 0.029 |
| `L.u` | 0.115 | 0.144 |
| `L.d` | 0.608 | 0.153 |
| `L.n` / `L.p` | 0.250 / 0.212 | 0.113 / 0.099 |
| `L.i` | 5.25 | 0.75 |
| `T.sectionFromNode` | 5.68 | 0.51 |
| `T.text` (verse) | 9.35 | 1.64 |
| `T.nodeFromSection` | 0.223 | 0.202 |

`L.u` and a full `N.walk()` (17 ms vs 11 ms) build fresh Python ints where
Text-Fabric returns pre-built tuples. Because the corpus stays memory-mapped,
worker processes share its pages instead of each holding a copy, which suits
multi-worker API deployments.

Run the gate (from outside the repository root):

```bash
pip install "context-fabric[benchmarks]"
python -m cfabric_benchmarks.perf_gate --engine cf --assert --strict
```

## Versioning

`Cargo.toml` is the single source of truth for the release version (SemVer, e.g.
`0.6.0-rc.1`). Maturin publishes the wheel under its PEP 440 rendering (`0.6.0rc1`;
`pyproject.toml` declares `version` as dynamic), and `build.rs` exports the same
rendering to the Rust core, so `cfabric.VERSION`, `cfabric.__version__`, and the
banner always equal `importlib.metadata.version("context-fabric")`. Bump the
version in `Cargo.toml` only.

## Development

See [TESTING.md](https://github.com/Context-Fabric/context-fabric/blob/master/TESTING.md)
for building and testing, and the [changelog](https://github.com/Context-Fabric/context-fabric/blob/master/libs/core/CHANGELOG.md).

## Authors

- Cody Kingham
- Dirk Roorda

## License

MIT License - see [LICENSE](https://github.com/Context-Fabric/context-fabric/blob/master/LICENSE) for details.
