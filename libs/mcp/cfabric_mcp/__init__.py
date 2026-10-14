"""Context-Fabric MCP Server.

This package provides an MCP (Model Context Protocol) server that exposes
Context-Fabric corpus operations to AI agents.

Usage:
    # Run as CLI
    cfabric-mcp

    # Or import and run programmatically
    from cfabric_mcp import mcp
    mcp.run(transport="stdio")
"""

import logging
import sys
from importlib.metadata import PackageNotFoundError, version

# Configure logging for MCP server
# Use stderr since stdout is reserved for MCP protocol messages
_handler = logging.StreamHandler(sys.stderr)
_handler.setFormatter(
    logging.Formatter(
        fmt="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S",
    )
)

# Create package logger
logger = logging.getLogger("cfabric_mcp")
logger.addHandler(_handler)
logger.setLevel(logging.INFO)
# Prevent propagation to root logger (avoids duplicate messages
# when FastMCP configures root logging via logging.basicConfig())
logger.propagate = False

from cfabric_mcp.server import mcp, main
from cfabric_mcp.corpus_manager import corpus_manager, CorpusManager

try:
    __version__ = version("cfabric-mcp")
except PackageNotFoundError:
    __version__ = "0+unknown"

__all__ = ["mcp", "main", "corpus_manager", "CorpusManager", "logger", "__version__"]
