"""
# Display

Structured, HTML-free displays of corpus material: a port of Text-Fabric's
`tf.advanced` without HTML. Moved here from the core (it was only on the
browser's branches), so that the browser works with any core; it uses only
the core's public API (`F`, `L`, `T`, `E`).

*   `display.config`: how a corpus wants to be displayed, read from
    Text-Fabric app configurations (`config.yaml`) plus computed defaults.
*   `display.sections`: section references such as `Genesis 1:1`.
*   `display.tree`: render trees, the nested fragments of a node that
    clients draw as plain or pretty displays. `unravel` must stay identical
    to Text-Fabric's output.

Example
-------

>>> from cfabric_browser.display import DisplayConfig, RenderOptions, render_node
>>> config = DisplayConfig.from_api(api, corpus_path=path)
>>> tree = render_node(api, config, verse, mode="pretty")
>>> tree.to_dict()  # JSON-ready
"""

from cfabric_browser.display.config import DisplayConfig, TypeDisplay, find_app_config
from cfabric_browser.display.sections import (
    node_from_section_ref,
    section_ref,
    section_ref_for,
    structure_ref,
)
from cfabric_browser.display.tree import (
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
