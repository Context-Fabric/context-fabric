"""Corpus management for MCP server.

Handles loading and caching of corpora for the MCP server.
Supports multiple simultaneous corpora.

The bookkeeping lives in `cfabric.registry.CorpusRegistry`, which is shared
with other Context-Fabric services.
"""

from __future__ import annotations

import logging
from pathlib import Path

from cfabric.registry import CorpusRegistry
from cfabric.results import CorpusInfo

logger = logging.getLogger("cfabric_mcp.corpus_manager")


class CorpusManager(CorpusRegistry):
    """Manages loaded corpora for the MCP server.

    Supports loading multiple corpora and switching between them.
    """

    def load(
        self,
        path: str | Path,
        name: str | None = None,
        features: str | list[str] | None = None,
    ) -> CorpusInfo:
        """Load a corpus, replacing any corpus loaded earlier under the same name.

        Parameters
        ----------
        path: str
            Path to the corpus directory
        name: str | None
            Name for the corpus (defaults to directory name)
        features: str | list[str] | None
            Features to load (defaults to all)

        Returns
        -------
        CorpusInfo
            Information about the loaded corpus
        """
        pathObj = Path(path).expanduser().resolve()
        if not pathObj.exists():
            logger.error("Corpus path not found: %s", path)
            raise FileNotFoundError(f"Corpus path not found: {path}")
        resolvedName = name or pathObj.name
        if self.is_registered(resolvedName):
            self.unload(resolvedName)
        return super().load(path, name=resolvedName, features=features)


# Global corpus manager instance
corpus_manager = CorpusManager()
