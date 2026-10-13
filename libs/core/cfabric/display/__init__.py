"""
# Display

Structured, HTML-free displays of corpus material.

*   `cfabric.display.config`: how a corpus wants to be displayed, read from
    Text-Fabric app configurations (`config.yaml`) plus computed defaults.
*   `cfabric.display.sections`: section references such as `Genesis 1:1`.
*   `cfabric.display.tree`: render trees, the nested fragments of a node that
    clients draw as plain or pretty displays.

Example
-------

>>> from cfabric.display import DisplayConfig, RenderOptions, render_node
>>> config = DisplayConfig.from_api(api, corpus_path=path)
>>> tree = render_node(api, config, verse, mode="pretty")
>>> tree.to_dict()  # JSON-ready
"""

from cfabric.display.config import DisplayConfig, TypeDisplay, find_app_config
from cfabric.display.sections import (
    node_from_section_ref,
    section_ref,
    section_ref_for,
    structure_ref,
)
from cfabric.display.tree import (
    PLAIN,
    PRETTY,
    Fragment,
    RenderOptions,
    RenderTree,
    render_node,
    render_tuple,
    tuple_containers,
    unravel,
)

__all__ = [
    "PLAIN",
    "PRETTY",
    "DisplayConfig",
    "Fragment",
    "RenderOptions",
    "RenderTree",
    "TypeDisplay",
    "find_app_config",
    "node_from_section_ref",
    "render_node",
    "render_tuple",
    "section_ref",
    "section_ref_for",
    "structure_ref",
    "tuple_containers",
    "unravel",
]
