# Context-Fabric Corpus Distribution Strategy

## Overview

This document outlines the strategy for distributing Context-Fabric corpora (.tf and .cfm files) to enable community contributions and fast, reliable downloads.

---

## Recommended Platform: Hugging Face Hub

### Why Hugging Face Hub

| Requirement | HF Hub Capability |
|-------------|-------------------|
| Community uploads | Anyone can create `username/corpus-name` datasets |
| No maintainer bottleneck | Users publish independently |
| Download optimization | Global CDN, chunked downloads, built-in caching |
| Python API | Excellent `huggingface_hub` library |
| Versioning | Full git history, tags, branches |
| Large file support | 500GB per file, TB+ per repo |
| Cost | Free for public datasets |

### Download Performance

The `huggingface_hub` library provides:
- **Local caching**: Downloaded files cached at `~/.cache/huggingface/`
- **Resumable downloads**: Interrupted downloads resume automatically
- **Streaming**: Can stream data without full download
- **Parallel downloads**: Multiple files downloaded concurrently
- **Revision pinning**: Download specific versions by tag/commit

### Alternatives Considered

| Platform | Pros | Cons |
|----------|------|------|
| **Zenodo** | Academic DOIs, CERN backing, 50+ year preservation | Weaker download API, 50GB limit |
| **Internet Archive** | Unlimited free storage, preservation mission | Less polished API, slower downloads |
| **GitHub Releases** | Familiar to developers | 2GB limit, project-owned only |

**Verdict:** Zenodo is good for archival/citation purposes but HF Hub is better for active distribution due to superior download infrastructure.

---

## Implementation Design

### Namespace Convention

```
huggingface.co/datasets/{username}/cfabric-{corpus-name}
```

Examples:
- `etcbc/cfabric-bhsa` - BHSA Hebrew Bible (official)
- `johndoe/cfabric-my-corpus` - Community contribution
- `context-fabric/cfabric-demo` - Official demo corpus

### Dataset Structure

Each corpus repository should contain:
```
cfabric-{corpus-name}/
├── README.md              # Dataset card (required)
├── .tf/                   # Text-Fabric source files
│   ├── otype.tf
│   ├── oslots.tf
│   ├── otext.tf
│   └── {features}.tf
├── .cfm/                  # Compiled memory-mapped format (optional)
│   └── 1/
│       ├── meta.json
│       ├── warp/
│       ├── features/
│       └── edges/
└── corpus_info.json       # CF metadata (version, features, stats)
```

### Dataset Card Template (README.md)

```markdown
---
license: cc-by-4.0
task_categories:
  - text-classification
language:
  - he  # ISO 639-1 code
tags:
  - context-fabric
  - corpus
  - linguistics
---

# {Corpus Name}

## Description
Brief description of the corpus.

## Features
| Feature | Type | Description |
|---------|------|-------------|
| word | str | Surface form |
| lemma | str | Lemma |
| ...

## Usage

```python
from huggingface_hub import snapshot_download
import cfabric

# Download corpus
path = snapshot_download("username/cfabric-corpus", repo_type="dataset")

# Load with Context-Fabric
TF = cfabric.Fabric(locations=path)
api = TF.loadAll()
```

## Citation
```bibtex
@dataset{...}
```

## License
CC-BY-4.0
```

---

## cfabric Integration API

### Download Function

```python
# cfabric/downloader/download.py

from huggingface_hub import snapshot_download, hf_hub_download
from pathlib import Path

def download(
    corpus_id: str,
    *,
    revision: str | None = None,
    force: bool = False,
    compiled_only: bool = False,
) -> Path:
    """Download a corpus from Hugging Face Hub.

    Args:
        corpus_id: Either a short name from the registry (e.g., 'bhsa')
                   or a full HF repo ID (e.g., 'etcbc/cfabric-bhsa').
        revision: Specific version (tag, branch, or commit hash).
        force: Re-download even if cached.
        compiled_only: Only download .cfm files (faster load).

    Returns:
        Path to the downloaded corpus directory.

    Example:
        >>> path = cfabric.download('bhsa')
        >>> TF = cfabric.Fabric(locations=path)

        >>> # Or with full repo ID
        >>> path = cfabric.download('johndoe/cfabric-custom')
    """
    # Resolve short name to full repo ID
    repo_id = _resolve_corpus_id(corpus_id)

    # Build download patterns
    allow_patterns = None
    if compiled_only:
        allow_patterns = [".cfm/**", "corpus_info.json", "README.md"]

    # Download from HF Hub
    local_path = snapshot_download(
        repo_id=repo_id,
        repo_type="dataset",
        revision=revision,
        allow_patterns=allow_patterns,
        force_download=force,
    )

    return Path(local_path)
```

### Registry (Optional Shortnames)

```python
# cfabric/downloader/registry.py

CORPUS_REGISTRY = {
    # Official corpora (maintained by Context-Fabric org)
    "bhsa": {
        "repo_id": "etcbc/cfabric-bhsa",
        "description": "BHSA Hebrew Bible (Biblia Hebraica Stuttgartensia Amstelodamensis)",
        "language": "hbo",
        "version": "2023.1",
    },
    "peshitta": {
        "repo_id": "etcbc/cfabric-peshitta",
        "description": "Syriac Peshitta",
        "language": "syc",
    },
    # Community can add their own via PR or just use full repo_id
}

def list_corpora() -> dict[str, dict]:
    """List registered corpora with metadata."""
    return CORPUS_REGISTRY.copy()

def _resolve_corpus_id(corpus_id: str) -> str:
    """Resolve short name to full HF repo ID."""
    if "/" in corpus_id:
        return corpus_id  # Already a full repo ID
    if corpus_id in CORPUS_REGISTRY:
        return CORPUS_REGISTRY[corpus_id]["repo_id"]
    raise ValueError(f"Unknown corpus: {corpus_id}. Use list_corpora() to see available corpora, or provide a full HF repo ID.")
```

### Cache Management

```python
# cfabric/downloader/paths.py

import os
from pathlib import Path
from platformdirs import user_cache_dir

def get_cache_dir() -> Path:
    """Get the cfabric cache directory.

    Resolution order:
    1. CFABRIC_CACHE environment variable
    2. Platform-specific cache directory

    Note: By default, huggingface_hub caches to ~/.cache/huggingface/
    This function returns the cfabric-specific cache for compiled corpora.
    """
    env_dir = os.environ.get("CFABRIC_CACHE")
    if env_dir:
        return Path(env_dir)
    return Path(user_cache_dir("cfabric", "cfabric"))

def clear_cache(corpus_id: str | None = None) -> None:
    """Clear downloaded corpus cache.

    Args:
        corpus_id: Specific corpus to clear, or None for all.
    """
    from huggingface_hub import scan_cache_dir, delete_cache_folder
    # Implementation details...
```

---

## Usage Patterns

### Basic Download and Load

```python
import cfabric

# Download (cached after first time)
path = cfabric.download('bhsa')

# Load corpus
TF = cfabric.Fabric(locations=path)
api = TF.loadAll()

# Use the API
for word in api.F.otype.s('word')[:10]:
    print(api.T.text(word))
```

### Pin Specific Version

```python
# Pin to a specific release
path = cfabric.download('bhsa', revision='v2023.1')

# Or a specific commit for reproducibility
path = cfabric.download('bhsa', revision='abc123def456')
```

### Download Community Corpus

```python
# Anyone can upload to HF Hub under their namespace
path = cfabric.download('researcher123/cfabric-ancient-greek')
TF = cfabric.Fabric(locations=path)
```

### Fast Load (Compiled Only)

```python
# Skip .tf files, only download pre-compiled .cfm
path = cfabric.download('bhsa', compiled_only=True)
```

---

## Community Contribution Workflow

### For Corpus Contributors

1. **Create HF account** at huggingface.co
2. **Create dataset repo** named `cfabric-{your-corpus}`
3. **Upload files** via web UI or `huggingface_hub` library:

```python
from huggingface_hub import HfApi

api = HfApi()
api.create_repo("my-corpus", repo_type="dataset")
api.upload_folder(
    folder_path="./my-corpus-data/",
    repo_id="myusername/cfabric-my-corpus",
    repo_type="dataset",
)
```

4. **Add Dataset Card** (README.md) with `context-fabric` tag
5. **Share the repo ID** - users can download with:
   ```python
   cfabric.download('myusername/cfabric-my-corpus')
   ```

### For Official Registry Inclusion

Submit a PR to add your corpus to `CORPUS_REGISTRY` in `cfabric/downloader/registry.py`.

---

## Dependencies

Add to `pyproject.toml`:

```toml
[project]
dependencies = [
    "numpy>=1.26",
    "pyyaml>=5.3",
    "huggingface-hub>=0.20",  # For corpus downloads
    "platformdirs>=3.0",       # For cache directory
]
```

---

## Future Considerations

### Zenodo Mirror for Archival

For long-term preservation and academic citation, consider:
- Mirroring official corpora to Zenodo for DOI assignment
- Zenodo provides 50+ year preservation guarantee
- Can auto-sync releases from HF Hub to Zenodo

### Corpus Validation

Future feature: Validate corpus structure before upload:
```python
cfabric.validate_corpus("./my-corpus/")  # Check .tf file integrity
```

### Corpus Search

Leverage HF Hub's search API:
```python
cfabric.search_corpora(language="hbo")  # Find Hebrew corpora
```
