"""Text-Fabric compatible import path: ``from cfabric.fabric import Fabric``.

Mirrors ``tf.fabric`` so code written for Text-Fabric switches to Context-Fabric
by replacing the package name ``tf`` with ``cfabric``.
"""

from cfabric._core import Fabric

__all__ = ["Fabric"]
