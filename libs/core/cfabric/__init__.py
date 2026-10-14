"""Context-Fabric: a graph-based corpus engine for annotated text.

The core API mirrors Text-Fabric's (`tf.fabric.Fabric`)::

    from cfabric import Fabric

    TF = Fabric(locations="path/to/tf")
    api = TF.load("sp lex")
    for node in api.F.sp.s("verb"):
        print(api.T.text(node))
"""

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
]
