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

**Enabled by:** Direct access to `F.feature.items()`, `F.feature.freqList()`, and native Python iteration.

### 3.2 Developer: Custom Scripts

> As a developer building a corpus tool, I want to write reusable Python scripts that Claude can execute for common operations.

**Enabled by:** Bundled helper scripts in `scripts/` directory.

### 3.3 Linguist: Complex Navigation

> As a linguist, I want to navigate hierarchical relationships (clause → phrase → word) and access multiple features in a single operation.

**Enabled by:** Direct `L.d()`, `L.u()`, `L.n()`, `L.p()` API access.

### 3.4 New User: Learning the API

> As a new user, I want Claude to help me learn the Context-Fabric API with examples and explanations.

**Enabled by:** Dynamic reference loading with syntax guides and examples.

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
│   │   └── export.py            # Export results to CSV/JSON
│   └── skills/                  # Skill definitions (copied on install)
│       └── context-fabric/
│           ├── SKILL.md         # Main skill file
│           └── references/      # On-demand reference docs
│               ├── api-quick-ref.md
│               ├── search-syntax.md
│               ├── feature-patterns.md
│               └── navigation-patterns.md
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
    └── navigation-patterns.md
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

1. **Quick start** - Loading a corpus, basic API pattern
2. **API overview** - F, E, L, T, S, N with brief descriptions
3. **Execution patterns** - When to use inline code vs helper scripts
4. **Reference loading** - Instructions for loading detailed docs on-demand
5. **Common workflows** - Corpus exploration, search, batch analysis

**Target length:** ~300 lines (under 500 line limit)

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

#### Additional Scripts

| Script | Purpose |
|--------|---------|
| `batch_search.py` | Execute search with full result iteration |
| `navigation.py` | Navigate L.d/L.u hierarchies with output |
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

Optional corpus-specific extensions:

```bash
# Install BHSA-specific references and examples
cfabric-skill install --profile bhsa
```

Profiles would include:
- Corpus-specific feature references
- Common search patterns for that corpus
- Domain-specific helper scripts

### Phase 3: npm Distribution (v0.3.0)

Thin npm wrapper for broader Claude Code ecosystem:

```bash
npm install -g @cfabric/skill
```

The npm package would:
- Contain only SKILL.md and references/
- No Python code (users still need pip for helper scripts)
- Auto-install to `~/.claude/skills/` via postinstall

### Phase 4: Interactive Mode (v0.4.0)

Enhanced skill with:
- Interactive corpus selection
- Session state persistence
- Result caching

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
| SKILL.md < 500 lines | ✅ Target ~300 |
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
  hierarchies, or perform batch analysis. More powerful than MCP - enables
  direct Python execution with full API access including F, E, L, T, S, N.
allowed-tools: Bash, Read
---

# Context-Fabric Corpus Analysis

This skill provides direct Python API access to Context-Fabric corpora,
enabling more powerful analysis than the MCP tool interface.

## Quick Start

```python
import cfabric

# Load corpus
CF = cfabric.Fabric('/path/to/corpus')
api = CF.loadAll()
api.makeAvailableIn(globals())

# Now use F, E, L, T, S, N directly
```

## Core APIs

| API | Purpose | Example |
|-----|---------|---------|
| `F` | Node features | `F.sp.v(node)` returns part of speech |
| `E` | Edge features | `E.mother.f(node)` returns related nodes |
| `L` | Locality | `L.d(node, 'word')` returns contained words |
| `T` | Text | `T.text(node)` returns text content |
| `S` | Search | `S.search(template)` finds patterns |
| `N` | Nodes | `N.sortNodes(nodes)` orders nodes |

## When to Use Inline Code vs Helper Scripts

**Use inline code for:**
- One-off queries
- Custom analysis logic
- Exploratory work

**Use helper scripts for:**
- Batch operations (scripts/batch_search.py)
- Feature exploration (scripts/feature_explorer.py)
- Structured exports (scripts/export.py)

## Loading References

For detailed syntax and patterns, read the reference files:
- `references/api-quick-ref.md` - Complete API reference
- `references/search-syntax.md` - Search template syntax
- `references/feature-patterns.md` - Common feature patterns
- `references/navigation-patterns.md` - Hierarchy navigation

## Common Workflows

### 1. Explore Corpus Structure

```python
# See all node types and counts
for ntype, avg, minN, maxN in api.C.levels.data:
    print(f"{ntype}: {maxN - minN + 1} nodes")
```

### 2. Feature Distribution

```python
# Top 20 parts of speech by frequency
for pos, count in F.sp.freqList()[:20]:
    print(f"{pos}: {count}")
```

### 3. Search with Full Results

```python
results = S.search('''
clause
  phrase function=Pred
    word sp=verb
''')

for clause, phrase, word in results:
    print(T.sectionFromNode(clause), T.text(clause))
```

### 4. Navigate Hierarchy

```python
# Get verse containing a word, then all words in that verse
verse = L.u(word_node, otype='verse')[0]
all_words = L.d(verse, otype='word')
```
```

---

*End of PRD*
