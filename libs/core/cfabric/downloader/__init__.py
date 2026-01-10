"""Corpus downloader for Context-Fabric.

This module provides functionality to download corpora from Hugging Face Hub.

Usage:
    >>> import cfabric
    >>> path = cfabric.download('bhsa')
    >>> CF = cfabric.Fabric(locations=path)

See corpus-distribution-plan.md for full documentation.
"""

from cfabric.downloader.registry import list_corpora, CORPUS_REGISTRY
from cfabric.downloader.download import download
from cfabric.downloader.paths import get_cache_dir

__all__ = ["download", "list_corpora", "get_cache_dir", "CORPUS_REGISTRY"]
