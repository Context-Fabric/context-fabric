"""Context-Fabric: a graph-based corpus engine for annotated text."""

from cfabric import _core
from cfabric._core import BANNER, NAME, VERSION, Fabric
from cfabric.downloader import download, get_cache_dir, list_corpora
from cfabric.results import CorpusInfo, FeatureInfo, NodeInfo, NodeList, SearchResult

__version__ = VERSION
__all__ = [
    "VERSION",
    "NAME",
    "BANNER",
    "Fabric",
    "download",
    "get_cache_dir",
    "list_corpora",
    "NodeInfo",
    "NodeList",
    "SearchResult",
    "FeatureInfo",
    "CorpusInfo",
    "__version__",
    "_core",
]
