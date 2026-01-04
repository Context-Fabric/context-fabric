"""
Core API of Context-Fabric.

This API deals with the basic CF data model: a graph of nodes and edges,
annotated by features.

The core API consists of:

*   `N`: see `nodes.Nodes` (walk through nodes)
*   `F`: see `nodefeature.NodeFeature` (retrieve feature values for nodes)
*   `E`: see `edgefeature.EdgeFeature` (retrieve feature values for edges)
*   `L`: see `locality.Locality` (move between levels)
*   `T`: see `text.Text` (get the text)
*   `S`: see `search.search` (search by templates)

plus some additional methods.

Based on Text-Fabric by Dirk Roorda.
"""

from cfabric.core.fabric import FabricCore
from cfabric.core.parameters import VERSION, NAME, BANNER

__version__ = VERSION
__all__ = ["FabricCore", "VERSION", "NAME", "BANNER"]
