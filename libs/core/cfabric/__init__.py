"""Context-Fabric: A graph-based corpus engine for annotated text.

This package provides tools for loading, navigating, and querying
annotated text corpora using a graph-based data model.

Basic usage:
    >>> import cfabric
    >>> CF = cfabric.Fabric(locations='path/to/corpus')
    >>> api = CF.load('feature1', 'feature2')
    >>> for node in api.F.feature1.s('value'):
    ...     print(api.T.text(node))
"""

from cfabric.core.fabric import Fabric
from cfabric.core.config import VERSION, NAME, BANNER
from cfabric.downloader import download, list_corpora, get_cache_dir
from cfabric.results import NodeInfo, NodeList, SearchResult, FeatureInfo, CorpusInfo

__version__ = VERSION
__all__ = [
    "Fabric",
    "VERSION",
    "NAME",
    "BANNER",
    "__version__",
    "download",
    "list_corpora",
    "get_cache_dir",
    # Result types for rich API responses
    "NodeInfo",
    "NodeList",
    "SearchResult",
    "FeatureInfo",
    "CorpusInfo",
]
