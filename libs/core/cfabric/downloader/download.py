"""Download functionality for Context-Fabric corpora.

This module provides the main download function for fetching corpora
from Hugging Face Hub.
"""

from __future__ import annotations

from pathlib import Path

from cfabric.downloader.registry import _resolve_corpus_id


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
            If None, downloads the latest version.
        force: Re-download even if cached locally.
        compiled_only: Only download .cfm files (faster load, skip .tf source).

    Returns:
        Path to the downloaded corpus directory.

    Raises:
        ValueError: If corpus_id is not found and doesn't look like a repo ID.
        ImportError: If huggingface_hub is not installed.

    Example:
        >>> import cfabric
        >>> path = cfabric.download('bhsa')
        >>> CF = cfabric.Fabric(locations=path)

        >>> # Or with full repo ID for community corpora
        >>> path = cfabric.download('researcher/cfabric-my-corpus')

        >>> # Pin to specific version
        >>> path = cfabric.download('bhsa', revision='v2023.1')
    """
    try:
        from huggingface_hub import snapshot_download
    except ImportError as e:
        raise ImportError(
            "huggingface_hub is required for corpus downloads. "
            "Install it with: pip install huggingface-hub"
        ) from e

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
