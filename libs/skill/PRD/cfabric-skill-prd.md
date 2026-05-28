# Context-Fabric Skill Package PRD

**Version:** 0.1.0-draft
**Status:** Draft
**Author:** Auto-generated from design session
**Date:** 2026-01-27

---

## 1. Executive Summary

This PRD specifies `cfabric-skill`, a Python package that provides Claude Code skills for Context-Fabric corpus analysis. Unlike the MCP server (`cfabric-mcp`) which exposes a curated tool interface, the skill package enables direct Python API access, unlocking more powerful and flexible corpus interactions.

### Key Differentiators from MCP

| Aspect | MCP Server | Skill Package |
|--------|------------|---------------|
| Interface | 11 curated tools | Full Python API (F, E, L, T, S, N) |
| Execution | Tool calls via protocol | Direct Python execution |
| Flexibility | Structured queries only | Arbitrary code + queries |
| Scripting | Not supported | First-class support |
| Iteration | Paginated results | Native Python iteration |
| Batch ops | Limited | Full `.items()`, `.freqList()` |

---

## 2. Goals and Non-Goals

### Goals

1. **Provide a more powerful alternative to MCP** for users who need direct Python API access
2. **Enable scripting workflows** with reusable helper scripts for common patterns
3. **Support corpus exploration** with dynamic reference loading
4. **Maintain corpus-agnostic design** that works with any Context-Fabric corpus
5. **Distribute via PyPI** following Python packaging best practices
6. **Follow Agent Skills standard** for maximum portability across AI tools

### Non-Goals

- Replace `cfabric-mcp` (they serve different use cases)
- Provide corpus-specific optimizations (BHSA, Peshitta, etc.) in v1
- Support npm distribution (future roadmap)
- Bundle the MCP server (separate package dependency)

---

## 3. User Stories

### 3.1 Researcher: Batch Analysis

> As a biblical studies researcher, I want to run batch analyses across an entire corpus (e.g., frequency distributions, pattern counts) without hitting MCP pagination limits.

**Enabled by:** Direct access to `F.feature.items()`, `F.feature.freqList()`, and native Python iteration. Helper scripts: `frequency_analysis.py`, `pandas_export.py`.

### 3.2 Developer: Custom Scripts

> As a developer building a corpus tool, I want to write reusable Python scripts that Claude can execute for common operations like feature extraction, search, and export.

**Enabled by:** Bundled helper scripts in `scripts/` directory, invocable via Bash tool.

### 3.3 Linguist: Complex Navigation

> As a linguist, I want to navigate hierarchical relationships (clause → phrase → word) and access multiple features in a single operation.

**Enabled by:** Direct `L.d()`, `L.u()`, `L.n()`, `L.p()` API access. Helper script: `navigation.py`.

### 3.4 New User: Learning the API

> As a new user, I want Claude to help me learn the Context-Fabric API with examples and explanations, starting from corpus discovery and building up to complex queries.

**Enabled by:** Dynamic reference loading with syntax guides and examples in `references/`.

### 3.5 ML Researcher: Feature Matrix Extraction

> As a computational linguist, I want to extract structured feature matrices from corpus data — using F for node features, L for hierarchical context, and E for syntactic edges — so I can build classification, clustering, or neural network models with pandas, sklearn, or TensorFlow.

**Enabled by:** Pipeline pattern: corpus → feature extraction via F/L/E/S → pandas DataFrame → ML model → export. Helper scripts: `pandas_export.py`, `frequency_analysis.py`.

### 3.6 Textual Scholar: Parallel Passage Comparison

> As a textual scholar, I want to compare parallel passages within a corpus — such as synoptic Psalms, Kings/Chronicles parallels, or formulaic language — using L.d() to decompose passages into comparable units, F for lexeme/morphological features, and T.text() with different format strings for rendering.

**Enabled by:** Direct iteration over verse/clause nodes with `L.d()` and `F.lex.v()` for lexeme-set comparison. Helper scripts: `pandas_export.py`, `section_filter.py`.

### 3.7 Teacher: Study Material Generation

> As a language teacher or student, I want to generate frequency-graded vocabulary lists, reading texts, and study materials — using F.freq_lex and F.freqList() for frequency analysis, T.text() with format options for readable output, and CSV export for flashcard apps.

**Enabled by:** `frequency_analysis.py` for hapax and frequency filtering, `pandas_export.py` for structured export.

### 3.8 Non-Hebrew Corpus User: Corpus-Agnostic Analysis

> As a researcher working with a non-Hebrew corpus (Greek NT, cuneiform tablets, Quran, Syriac, or any other CF-formatted dataset), I want the skill to guide me using the same core API without Hebrew-specific assumptions — recognizing that node type hierarchies, feature names, and text formats differ by corpus.

**Enabled by:** Corpus discovery pattern: `api.C.levels.data` for node types, `api.Fall()`/`api.Eall()` for available features, `T.formats` for text formats.

### 3.9 Historical Linguist: Diachronic Analysis

> As a historical linguist, I want to compare linguistic feature distributions across text categories (early vs. late, prose vs. poetry, narrative vs. speech) — extracting features with F and S.search(), segmenting by section with T.sectionFromNode(), and building cross-tabulations for statistical comparison.

**Enabled by:** `frequency_analysis.py` with `--cross-tab` and `--book-filter`, `section_filter.py` with `--book-group` presets, `pandas_export.py` for DataFrame construction.

---

## 4. Architecture

### 4.1 Package Structure

```
libs/skill/
├── cfabric_skill/
│   ├── __init__.py              # Package entry + exports
│   ├── cli.py                   # CLI entry point (install/uninstall)
│   ├── installer.py             # Skill installation logic
│   ├── scripts/                 # Helper scripts for Claude
│   │   ├── __init__.py
│   │   ├── corpus_loader.py     # Load corpus, return api object
│   │   ├── feature_explorer.py  # Explore features with freqList
│   │   ├── batch_search.py      # Search with full iteration
│   │   ├── navigation.py        # Hierarchical navigation helpers
│   │   ├── export.py            # Export results to CSV/JSON
│   │   ├── text_extract.py      # Extract text with format options
│   │   ├── pandas_export.py     # Node features → DataFrame → CSV/parquet
│   │   ├── frequency_analysis.py # Frequency distributions, cross-tabs, hapax
│   │   ├── collocation.py       # Co-occurrence analysis for lexemes
│   │   └── section_filter.py    # Filter nodes by book/chapter/section
│   └── skills/                  # Skill definitions (copied on install)
│       └── context-fabric/
│           ├── SKILL.md         # Main skill file
│           └── references/      # On-demand reference docs
│               ├── api-quick-ref.md
│               ├── search-syntax.md
│               ├── feature-patterns.md
│               ├── navigation-patterns.md
│               ├── edge-patterns.md
│               ├── text-formats.md
│               └── usage-examples.md
├── tests/
│   ├── test_installer.py
│   ├── test_cli.py
│   └── integration/
│       └── test_skill_install.py
├── pyproject.toml
├── CHANGELOG.md
└── README.md
```

### 4.2 Skill Directory Structure (Installed)

After running `cfabric-skill install`, the following is created:

```
~/.claude/skills/context-fabric/     # or .claude/skills/ with --project
├── SKILL.md                         # Main instructions
└── references/
    ├── api-quick-ref.md
    ├── search-syntax.md
    ├── feature-patterns.md
    ├── navigation-patterns.md
    ├── edge-patterns.md
    ├── text-formats.md
    └── usage-examples.md
```

### 4.3 Component Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        Claude Code                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────┐    ┌──────────────────┐                  │
│  │  /context-fabric │    │   Bash Tool      │                  │
│  │     Skill        │───▶│   (Python exec)  │                  │
│  └──────────────────┘    └────────┬─────────┘                  │
│                                   │                             │
│  ┌──────────────────┐             │                             │
│  │   references/    │◀────────────┤  Dynamic loading            │
│  │   (on-demand)    │             │                             │
│  └──────────────────┘             ▼                             │
│                          ┌─────────────────┐                    │
│                          │  Helper Scripts │                    │
│                          │  (scripts/)     │                    │
│                          └────────┬────────┘                    │
│                                   │                             │
└───────────────────────────────────┼─────────────────────────────┘
                                    │
                                    ▼
                    ┌───────────────────────────────┐
                    │      context-fabric           │
                    │      (Core Library)           │
                    ├───────────────────────────────┤
                    │  F (Features)  │ L (Locality) │
                    │  E (Edges)     │ T (Text)     │
                    │  S (Search)    │ N (Nodes)    │
                    └───────────────────────────────┘
                                    │
                                    ▼
                    ┌───────────────────────────────┐
                    │         Corpus Data           │
                    │         (.cfm files)          │
                    └───────────────────────────────┘
```

---

## 5. Detailed Specifications

### 5.1 SKILL.md Content

The main skill file follows the Agent Skills standard:

```yaml
---
name: context-fabric
description: >
  Analyze linguistic corpora using Context-Fabric Python API. Use this skill
  when the user wants to explore corpus structure, search for patterns, access
  features, navigate hierarchies, or perform batch analysis. More powerful
  than MCP tools - enables direct Python execution with full API access.
allowed-tools: Bash, Read
---
```

**Body content includes:**

1. **Quick start** — Loading a corpus with `Fabric()`, `loadAll()`, `makeAvailableIn(globals())`
2. **API overview** — F, E, L, T, S, N with brief descriptions
3. **Corpus discovery** — How to inspect node types, features, text formats for any corpus
4. **Common workflows** — Reordered: frequency analysis (first), search, navigation, pandas pipeline
5. **Key API idioms** — Pain point shortcuts: `L.u()[0]` pattern, safe edge access, section formatting
6. **Helper scripts** — Table of all 10 scripts with one-line descriptions
7. **Reference loading** — Instructions for loading 7 detailed reference docs on-demand

**Target length:** ~350 lines (under 500 line limit)

### 5.2 Helper Scripts

Scripts are installed to the Python environment (via pip) and invoked by Claude:

#### `corpus_loader.py`

```python
#!/usr/bin/env python3
"""Load a Context-Fabric corpus and return API object."""

import argparse
import cfabric

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('corpus_path', help='Path to corpus directory')
    parser.add_argument('--features', nargs='*', help='Specific features to load')
    args = parser.parse_args()

    CF = cfabric.Fabric(args.corpus_path)
    if args.features:
        api = CF.load(*args.features)
    else:
        api = CF.loadAll()

    # Print corpus info for Claude
    print(f"Loaded corpus: {args.corpus_path}")
    print(f"Node types: {[t[0] for t in api.C.levels.data]}")
    print(f"Features loaded: {len(api.Fall())}")

if __name__ == '__main__':
    main()
```

#### `feature_explorer.py`

```python
#!/usr/bin/env python3
"""Explore feature distributions with freqList."""

import argparse
import json
import cfabric

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('corpus_path')
    parser.add_argument('feature', help='Feature name to explore')
    parser.add_argument('--node-types', nargs='*')
    parser.add_argument('--limit', type=int, default=50)
    parser.add_argument('--format', choices=['json', 'table'], default='table')
    args = parser.parse_args()

    CF = cfabric.Fabric(args.corpus_path)
    api = CF.load(args.feature)

    freq = api.Fs(args.feature).freqList(nodeTypes=args.node_types)

    if args.format == 'json':
        print(json.dumps(freq[:args.limit]))
    else:
        for value, count in freq[:args.limit]:
            print(f"{value}\t{count}")

if __name__ == '__main__':
    main()
```

#### `pandas_export.py`

```python
#!/usr/bin/env python3
"""Export node features to pandas DataFrame / CSV / parquet."""

import argparse
import sys
from pathlib import Path
import cfabric

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('corpus_path')
    parser.add_argument('--node-type', default='word',
                        help='Node type to export, or "all"')
    parser.add_argument('--features', nargs='*', default=None,
                        help='Features to include (default: all)')
    parser.add_argument('--section', action='store_true', default=True,
                        help='Include section columns')
    parser.add_argument('--no-section', dest='section', action='store_false')
    parser.add_argument('--containment', nargs='*', default=[],
                        help='Add containment columns (e.g., clause phrase)')
    parser.add_argument('--filter', nargs=3, action='append', default=[],
                        metavar=('FEATURE', 'OP', 'VALUE'))
    parser.add_argument('--output', '-o', default=None)
    parser.add_argument('--format', choices=['csv', 'tsv', 'parquet', 'json'])
    parser.add_argument('--limit', type=int, default=None)
    args = parser.parse_args()

    import pandas as pd
    CF = cfabric.Fabric(args.corpus_path)
    api = CF.loadAll()
    F, L, T, N = api.F, api.L, api.T, api.N
    Fs = api.Fs

    features = args.features or [f for f in api.Fall() if f != 'otype']
    node_iter = N.walk() if args.node_type == 'all' else F.otype.s(args.node_type)

    rows = []
    for n in node_iter:
        row = {'node': n, 'otype': F.otype.v(n)}
        if args.section:
            section = T.sectionFromNode(n)
            for i, label in enumerate(T.sectionTypes):
                row[label] = section[i] if i < len(section) else None
        for ctype in args.containment:
            containers = L.u(n, otype=ctype)
            row[f'in_{ctype}'] = containers[0] if containers else None
        for feat in features:
            f = Fs(feat)
            row[feat] = f.v(n) if f else None
        rows.append(row)

    df = pd.DataFrame(rows)
    if args.limit:
        df = df.head(args.limit)

    # Output (infer format from extension or flag)
    # Supports csv, tsv, parquet, json
    ...
```

#### `frequency_analysis.py`

```python
#!/usr/bin/env python3
"""Frequency analysis: freqList, cross-tabulation, hapax."""

import argparse
import collections
import cfabric

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('corpus_path')
    parser.add_argument('feature', help='Feature to analyze')
    parser.add_argument('--cross-tab', metavar='FEATURE2',
                        help='Cross-tabulate with second feature')
    parser.add_argument('--hapax', action='store_true',
                        help='Find hapax legomena')
    parser.add_argument('--hapax-threshold', type=int, default=1)
    parser.add_argument('--node-type', default=None)
    parser.add_argument('--book-filter', nargs='*', default=None)
    parser.add_argument('--limit', type=int, default=50)
    parser.add_argument('--format', choices=['table', 'csv', 'json'],
                        default='table')
    args = parser.parse_args()

    CF = cfabric.Fabric(args.corpus_path)
    api = CF.loadAll()
    Fs = api.Fs

    if args.hapax:
        # Find items with freq_lex <= threshold
        ...
    elif args.cross_tab:
        # Cross-tabulate feature x feature2, optionally grouped by book
        import pandas as pd
        ...
    else:
        # Basic freqList
        node_types = {args.node_type} if args.node_type else None
        for val, count in Fs(args.feature).freqList(nodeTypes=node_types)[:args.limit]:
            print(f"{val}\t{count}")
```

#### `collocation.py`

```python
#!/usr/bin/env python3
"""Co-occurrence / collocation analysis for lexemes and features."""

import argparse
import collections
import cfabric

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('corpus_path')
    parser.add_argument('target', help='Target lexeme ("*" for all)')
    parser.add_argument('--window', type=int, default=3)
    parser.add_argument('--context', default='sentence',
                        help='Context boundary type')
    parser.add_argument('--feature', default='lex')
    parser.add_argument('--min-freq', type=int, default=5)
    parser.add_argument('--top-n', type=int, default=20)
    parser.add_argument('--metric', choices=['raw', 'pmi'], default='raw')
    parser.add_argument('--format', choices=['table', 'csv', 'json'],
                        default='table')
    args = parser.parse_args()

    CF = cfabric.Fabric(args.corpus_path)
    api = CF.loadAll()
    F, L, T = api.F, api.L, api.T
    Fs = api.Fs

    cooccur = collections.Counter()
    for word in F.otype.s('word'):
        if args.target != '*' and Fs(args.feature).v(word) != args.target:
            continue
        contexts = L.u(word, otype=args.context)
        if not contexts:
            continue
        context_words = L.d(contexts[0], otype='word')
        idx = list(context_words).index(word)
        for offset in range(-args.window, args.window + 1):
            if offset == 0:
                continue
            pos = idx + offset
            if 0 <= pos < len(context_words):
                collocate = Fs(args.feature).v(context_words[pos])
                if collocate:
                    cooccur[collocate] += 1

    for collocate, count in cooccur.most_common(args.top_n):
        print(f"{collocate}\t{count}")
```

#### `section_filter.py`

```python
#!/usr/bin/env python3
"""Filter nodes by section (book, chapter, verse)."""

import argparse
import cfabric

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('corpus_path')
    parser.add_argument('--book', nargs='*', default=None)
    parser.add_argument('--book-group', default=None,
                        choices=['torah', 'prophets', 'writings',
                                 'early_bh', 'late_bh', 'poetry'])
    parser.add_argument('--node-type', default='word')
    parser.add_argument('--output-mode',
                        choices=['nodes', 'text', 'features', 'count'],
                        default='nodes')
    parser.add_argument('--features', nargs='*', default=[])
    parser.add_argument('--text-format', default=None)
    args = parser.parse_args()

    CF = cfabric.Fabric(args.corpus_path)
    api = CF.loadAll()
    F, L, T = api.F, api.L, api.T

    # Resolve books from --book and --book-group
    books = set(args.book or [])
    if args.book_group:
        books |= BOOK_GROUPS[args.book_group]

    # Get slots in target books, then filter by node type
    ...
```

#### Additional Scripts

These scripts follow the same CLI pattern but are not shown in full:

| Script | Purpose |
|--------|---------|
| `batch_search.py` | Execute search with full result iteration |
| `navigation.py` | Hierarchical navigation helpers (tree traversal, safe edge access) |
| `export.py` | Export node data to CSV/JSON |
| `text_extract.py` | Extract text with format options |

### 5.3 Reference Documents

Loaded on-demand when Claude needs detailed information:

#### `api-quick-ref.md`

Concise reference for all API methods:

```markdown
# Context-Fabric API Quick Reference

## F (Node Features)
- `F.{name}.v(node)` → Get value for node
- `F.{name}.s(value)` → Find nodes with value
- `F.{name}.items()` → Iterate (node, value) pairs
- `F.{name}.freqList(nodeTypes=None)` → Value frequencies
- `F.{name}.meta` → Feature metadata

## E (Edge Features)
- `E.{name}.f(node)` → Forward edges (targets)
- `E.{name}.t(node)` → Reverse edges (sources)
...
```

#### `search-syntax.md`

Complete search template syntax (ported from MCP's `search_syntax_guide`):

- Node patterns and constraints
- Structural nesting (indentation)
- Relational operators
- Quantifiers
- Examples

#### `feature-patterns.md`

Common feature access patterns:

- Finding all nodes of a type
- Filtering by multiple features
- Frequency analysis
- Cross-feature analysis

#### `navigation-patterns.md`

Hierarchical navigation patterns:

- Traversing containment
- Context extraction
- Sibling navigation
- Slot-based operations

#### `edge-patterns.md`

Edge feature patterns:

- `E.{name}.f(node)` — outgoing edges (always returns tuple, never None)
- `E.{name}.t(node)` — incoming edges (always returns tuple)
- Valueless vs. valued edges (valued return `(target, value)` tuples)
- Safe access pattern (empty-tuple guard)
- Recursive tree traversal (climb to root, collect descendants)
- `E.{name}.freqList()` quirk: returns int for valueless edges, not freq tuples
- Filtering by node type: `freqList(nodeTypesFrom=, nodeTypesTo=)`

#### `text-formats.md`

Text format reference:

- Format naming convention: `what-how-fullness[-modifier]`
- Discovering available formats: `sorted(T.formats)`
- Using `T.text(node, fmt=...)` with different formats
- `T.sectionFromNode()` / `T.nodeFromSection()` for section navigation
- Multi-format extraction (comparing representations)
- Type-default formats for non-slot nodes

#### `usage-examples.md`

Real-world usage examples from the TF/CF research community, organized by workflow pattern with links to GitHub notebooks as concrete references:

- **Frequency & distribution** — Hapax analysis, feature value exploration (`tonyjurg/Parashot`, `ETCBC/bhsa/tutorial/cookbook/featureValues`)
- **ML pipelines** — Verb semantic clustering, LSTM clause classification, LSTM POS tagging (`codykingham/verb_semantics`, `MartijnNaaijer/phdthesis`, `jarodjacobs/RNNS-LSTM_POS_Blog_Post`)
- **Parallel passage detection** — Psalms parallels, formulaic language detection, synoptic parallels (`codykingham/tfNotebooks/Psalms_parallels`, `codykingham/tfNotebooks/Pentateuch_formulaic_language`, `MartijnNaaijer/Parallels_in_the_Hebrew_Bible`)
- **Diachronic analysis** — Probabilistic language change, linguistic dating, EBH/LBH classification (`ETCBC/Probabilistic_Language_Change`, `codykingham/tfNotebooks/linguistic_dating`)
- **Cross-tabulation** — Person distribution by domain (`codykingham/tfNotebooks/HB_person_distribution`)
- **Collocation / distributional semantics** — Word vectors, verb semantic spaces (`codykingham/tfNotebooks/word_vectors`, `codykingham/verb_semantics`)
- **Non-Hebrew corpora** — Greek NT tutorials, cuneiform analysis, Quran, Dutch historical texts (`tonyjurg/Nestle1904LFT`, `CenterBLC/N1904`, `Nino-cunei/uruk`, `q-ran/quran`, `CLARIAH/wp6-missieven`)
- **Pedagogy / study materials** — Vocabulary generators, contextual review, research workshop series (`codykingham/tfNotebooks/vocabulary_gen`, `codykingham/Mahir`, `oliverglanz/Text-Fabric`)
- **Annotation round-trips** — Coreference resolution, DSS annotation (`cmerwich/participant-analysis`, `lucieperez/notebooks`)
- **Corpus creation** — Letter of Jude conversion, CATSS LXX (`MartijnNaaijer/LetterOfJude`, `codykingham/catss_lxx`)
- **Knowledge graphs / visualization** — TF knowledge graph with Cytoscape (`tonyjurg/TF_Knowledge_Graph`)
- **Syntactic tree analysis** — Song of Songs syntax, clause tree traversal (`codykingham/SongofSongs-Syntax`, `codykingham/tfNotebooks/timeSpans`)

### 5.4 CLI Interface

```bash
# Install to global scope (default)
cfabric-skill install

# Install to project scope
cfabric-skill install --project

# Uninstall
cfabric-skill uninstall
cfabric-skill uninstall --project

# Show installed location
cfabric-skill status

# Show version
cfabric-skill --version
```

### 5.5 Installation Logic

```python
# installer.py

from pathlib import Path
import shutil
from importlib.resources import files

def get_skill_source() -> Path:
    """Get path to bundled skill files."""
    return files("cfabric_skill").joinpath("skills", "context-fabric")

def get_global_dest() -> Path:
    """Get global skill destination."""
    return Path.home() / ".claude" / "skills" / "context-fabric"

def get_project_dest() -> Path:
    """Get project skill destination."""
    return Path.cwd() / ".claude" / "skills" / "context-fabric"

def install(project: bool = False) -> Path:
    """Install skill files."""
    src = get_skill_source()
    dest = get_project_dest() if project else get_global_dest()

    if dest.exists():
        shutil.rmtree(dest)

    shutil.copytree(src, dest)
    return dest

def uninstall(project: bool = False) -> bool:
    """Remove installed skill files."""
    dest = get_project_dest() if project else get_global_dest()
    if dest.exists():
        shutil.rmtree(dest)
        return True
    return False
```

---

## 6. pyproject.toml Specification

```toml
[build-system]
requires = ["setuptools>=61.0", "wheel"]
build-backend = "setuptools.build_meta"

[project]
name = "cfabric-skill"
version = "0.1.0"
description = "Claude Code skills for Context-Fabric corpus analysis"
readme = "README.md"
license = {text = "MIT"}
requires-python = ">=3.10"
authors = [
    {name = "Cody Kingham", email = "cody.kingham@example.com"}
]
keywords = [
    "context-fabric",
    "text-fabric",
    "claude-code",
    "agent-skills",
    "corpus-linguistics",
    "biblical-studies"
]
classifiers = [
    "Development Status :: 3 - Alpha",
    "Intended Audience :: Science/Research",
    "License :: OSI Approved :: MIT License",
    "Programming Language :: Python :: 3",
    "Programming Language :: Python :: 3.10",
    "Programming Language :: Python :: 3.11",
    "Programming Language :: Python :: 3.12",
    "Programming Language :: Python :: 3.13",
    "Topic :: Text Processing :: Linguistic",
]

dependencies = [
    "context-fabric>=0.5.0",
]

[project.optional-dependencies]
dev = [
    "pytest>=7.0",
    "pytest-cov",
]

[project.scripts]
cfabric-skill = "cfabric_skill.cli:main"

[project.urls]
Homepage = "https://context-fabric.org"
Documentation = "https://context-fabric.org/docs"
Repository = "https://github.com/cody-kingham/context-fabric"
Issues = "https://github.com/cody-kingham/context-fabric/issues"

[tool.setuptools]
include-package-data = true

[tool.setuptools.packages.find]
where = ["."]
include = ["cfabric_skill*"]

[tool.setuptools.package-data]
cfabric_skill = [
    "skills/**/*.md",
    "skills/**/*.py",
    "scripts/*.py",
]
```

---

## 7. User Experience Flow

### 7.1 Installation

```bash
# 1. Install the package
pip install cfabric-skill

# 2. Install skills to Claude Code
cfabric-skill install

# Output:
# Installed context-fabric skill to ~/.claude/skills/context-fabric/
#
# Usage: Invoke with /context-fabric or let Claude auto-detect
# based on corpus analysis tasks.
```

### 7.2 Invocation

**Explicit:**
```
User: /context-fabric analyze the distribution of verbal stems in BHSA
```

**Auto-detection (via description matching):**
```
User: I want to explore the feature distribution in my Hebrew corpus at ~/corpora/bhsa
Claude: [Loads context-fabric skill based on description match]
```

### 7.3 Typical Interaction

```
User: Load the BHSA corpus and show me the top 20 verbal stems by frequency

Claude: I'll load the corpus and analyze verbal stem frequencies.

[Executes Python code using F.vs.freqList()]

Here are the top 20 verbal stems in BHSA:
| Stem | Count |
|------|-------|
| qal  | 50847 |
| piel | 6800  |
| hiphil | 9496 |
...
```

---

## 8. Testing Strategy

### 8.1 Unit Tests

- `test_installer.py`: Test install/uninstall logic
- `test_cli.py`: Test CLI argument parsing

### 8.2 Integration Tests

- `test_skill_install.py`: End-to-end installation verification
- Verify skill files are correctly placed
- Verify SKILL.md validates against Agent Skills spec

### 8.3 Manual Testing

- Load skill in Claude Code
- Execute common workflows
- Verify reference loading works

---

## 9. Distribution and Versioning

### 9.1 PyPI Publishing

- Package name: `cfabric-skill`
- Import name: `cfabric_skill`
- Tag pattern: `skill-v*` (e.g., `skill-v0.1.0`)

### 9.2 Version Synchronization

The skill package version is independent of `context-fabric` core but declares a minimum dependency:

```toml
dependencies = [
    "context-fabric>=0.5.0",
]
```

### 9.3 CHANGELOG

Maintain `libs/skill/CHANGELOG.md` following Keep a Changelog format.

---

## 10. Future Roadmap

### Phase 2: Corpus Profiles (v0.2.0)

Corpus-specific extensions for richer out-of-the-box experience:

- Corpus-specific feature catalogs and search cookbooks
- Predefined book groups and section presets
- Domain-specific pain point helpers (e.g., BHSA weqetal detection)

### Phase 3: Annotation Round-Trip Support (v0.3.0)

Integration with annotation workflows:

- Integration with the Recorder module for external annotation tools
- Custom feature creation and `TF.save()` patterns
- Cross-corpus alignment helpers

### Phase 4: Session & Caching (v0.4.0)

Persistence and performance across interactions:

- Session state persistence across Claude interactions
- Result caching for expensive computations (pickle/parquet)
- Multi-corpus loading patterns

---

## 11. Open Questions

### Resolved

1. **Distribution channel?** → PyPI only (v1)
2. **Skill purpose?** → Powerful alternative to MCP with direct Python API
3. **Installation method?** → Explicit CLI command
4. **Installation scope?** → Both global and project with CLI flag
5. **Package scope?** → Skills only (depends on context-fabric)
6. **Execution model?** → Both inline code and helper scripts
7. **Documentation?** → Dynamic loading
8. **Corpus support?** → Generic only (v1)

### Open

1. **Helper script discovery**: Should Claude auto-detect available scripts, or should they be documented in SKILL.md?
   - Recommendation: Document in SKILL.md with brief descriptions

2. **Reference loading trigger**: What signals should trigger reference loading?
   - Recommendation: Explicit instruction in SKILL.md ("If you need detailed API reference, read references/api-quick-ref.md")

3. **Error handling**: How should scripts report errors to Claude?
   - Recommendation: Exit codes + stderr for errors, stdout for results

---

## 12. Success Metrics

1. **Adoption**: Downloads from PyPI, GitHub stars
2. **Usage**: Skill invocation frequency (if telemetry available)
3. **Feedback**: GitHub issues, user reports
4. **Completeness**: % of MCP functionality also available via skill

---

## 13. Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| context-fabric | >=0.5.0 | Core corpus API |
| setuptools | >=61.0 | Build system |

---

## 14. Appendix A: Agent Skills Standard Compliance

The skill follows the [Agent Skills specification](https://agentskills.io/specification):

| Requirement | Status |
|-------------|--------|
| SKILL.md required | ✅ |
| Name in frontmatter | ✅ `context-fabric` |
| Description in frontmatter | ✅ |
| Name matches directory | ✅ |
| Name format (lowercase, hyphens) | ✅ |
| SKILL.md < 500 lines | ✅ Target ~350 |
| References in references/ | ✅ |

---

## 15. Appendix B: Comparison with MCP Tools

| MCP Tool | Skill Equivalent | Enhanced Capability |
|----------|------------------|---------------------|
| `describe_corpus` | `api.C.levels.data` | Direct access to computed data |
| `list_features` | `api.Fall()`, `api.Eall()` | No filtering overhead |
| `describe_feature` | `F.{name}.freqList()` | Full frequency data, no sampling |
| `search` | `S.search()` | Native iteration, no pagination |
| `get_passages` | `T.text()`, `L.d()` | Arbitrary navigation |
| `get_node_features` | `F.{name}.v()` | Batch via `.items()` |
| — | `F.{name}.s(value)` | Find nodes by value (not in MCP) |
| — | `L.n()`, `L.p()` | Sibling navigation (not in MCP) |
| — | `E.{name}.f/t()` | Edge traversal (limited in MCP) |

---

## 16. Appendix C: Example SKILL.md

```markdown
---
name: context-fabric
description: >
  Analyze linguistic corpora using Context-Fabric Python API. Use when users
  want to explore corpus structure, search patterns, access features, navigate
  hierarchies, or perform batch analysis. Enables direct Python execution
  with full API access including F, E, L, T, S, N.
allowed-tools: Bash, Read
---

# Context-Fabric Corpus Analysis

This skill provides direct Python API access to Context-Fabric corpora.

## Quick Start

```python
import cfabric

CF = cfabric.Fabric('/path/to/corpus')
api = CF.loadAll()
api.makeAvailableIn(globals())
# Now F, E, L, T, S, N are available directly
```

## Core APIs

| API | Purpose | Example |
|-----|---------|---------|
| `F` | Node features | `F.sp.v(node)` → part of speech |
| `E` | Edge features | `E.mother.f(node)` → related nodes |
| `L` | Locality | `L.d(node, 'word')` → contained words |
| `T` | Text | `T.text(node)` → text content |
| `S` | Search | `S.search(template)` → find patterns |
| `N` | Nodes | `N.sortNodes(nodes)` → order nodes |

## Corpus Discovery

Before analyzing any corpus, discover its structure:

```python
# Node type hierarchy and counts
for ntype, avg, minN, maxN in api.C.levels.data:
    print(f"{ntype}: {maxN - minN + 1} nodes")

# Available features
print("Node features:", sorted(api.Fall()))
print("Edge features:", sorted(api.Eall()))

# Available text formats
print("Text formats:", sorted(T.formats))
```

## Common Workflows

### 1. Frequency & Distribution Analysis

```python
# Top values for any feature
for val, count in F.sp.freqList()[:20]:
    print(f"{val}: {count}")

# Filter by node type
for val, count in F.vs.freqList(nodeTypes={'word'})[:10]:
    print(f"{val}: {count}")

# Hapax legomena
hapax = [w for w in F.otype.s('word') if F.freq_lex.v(w) == 1]

# Cross-tabulation by section
from collections import Counter
by_book = {}
for w in F.otype.s('word'):
    if F.sp.v(w) == 'verb':
        book = T.sectionFromNode(w)[0]
        by_book.setdefault(book, Counter())[F.vs.v(w)] += 1
```

### 2. Search with Full Results

```python
results = S.search('''
clause
  phrase function=Pred
    word sp=verb
''')

for clause, phrase, word in results:
    ref = '{} {}:{}'.format(*T.sectionFromNode(clause))
    print(ref, T.text(clause))
```

### 3. Navigate Hierarchy

```python
# Get verse containing a word, then all words in that verse
verse = L.u(word_node, otype='verse')[0]
all_words = L.d(verse, otype='word')
```

### 4. Pandas DataFrame Pipeline

```python
import pandas as pd

results = S.search('''
clause
  phrase function=Pred
    word sp=verb
''')

rows = []
for clause, phrase, word in results:
    rows.append({
        'lex': F.lex.v(word),
        'vs': F.vs.v(word),
        'vt': F.vt.v(word),
        'book': T.sectionFromNode(word)[0],
    })
df = pd.DataFrame(rows)
print(df['vs'].value_counts())
df.to_csv('results.csv', index=False)
```

## Key API Idioms

**Upward navigation** — `L.u()` returns a tuple; index `[0]` for single containers:
```python
lex_node = L.u(word, otype='lex')[0]
clause = L.u(word, otype='clause')[0]
```

**Safe edge access** — `E.{name}.f()` returns a tuple that may be empty:
```python
mothers = E.mother.f(node)
parent = mothers[0] if mothers else None
```

**Section references**:
```python
ref = '{} {}:{}'.format(*T.sectionFromNode(node))
node = T.nodeFromSection(('Genesis', 1, 1))
```

**Find nodes by feature value** (indexed):
```python
verbs = F.sp.s('verb')  # all nodes where sp == 'verb'
```

**Discover text formats**:
```python
print(sorted(T.formats))
T.text(node, fmt='text-orig-plain')
```

## Helper Scripts

| Script | Purpose |
|--------|---------|
| `corpus_loader.py` | Load corpus, return API object |
| `feature_explorer.py` | Explore features with freqList |
| `batch_search.py` | Search with full result iteration |
| `navigation.py` | Tree traversal, safe edge access helpers |
| `export.py` | Export node data to CSV/JSON |
| `text_extract.py` | Extract text with format options |
| `pandas_export.py` | Node features → DataFrame → CSV/parquet |
| `frequency_analysis.py` | Distributions, cross-tabs, hapax |
| `collocation.py` | Co-occurrence analysis for lexemes |
| `section_filter.py` | Filter nodes by book/chapter/section |

## Loading References

For detailed syntax and patterns, read the reference files:
- `references/api-quick-ref.md` — Complete API reference
- `references/search-syntax.md` — Search template syntax
- `references/feature-patterns.md` — Common feature patterns
- `references/navigation-patterns.md` — Hierarchy navigation
- `references/edge-patterns.md` — Edge features and tree traversal
- `references/text-formats.md` — Text format discovery and usage
- `references/usage-examples.md` — Real-world notebooks organized by workflow
```

---

*End of PRD*
