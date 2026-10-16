"""Corpus management for MCP server.

Handles loading and caching of corpora for the MCP server.
Supports multiple simultaneous corpora.
"""

from __future__ import annotations

import logging
from pathlib import Path
from typing import TYPE_CHECKING

import cfabric
from cfabric.results import CorpusInfo

if TYPE_CHECKING:
    from cfabric.core.api import Api

logger = logging.getLogger("cfabric_mcp.corpus_manager")


class CorpusManager:
    """Manages loaded corpora for the MCP server.

    Supports loading multiple corpora and switching between them.
    """

    def __init__(self) -> None:
        self._corpora: dict[str, tuple[cfabric.Fabric, Api]] = {}
        self._current: str | None = None

    def load(
        self,
        path: str,
        name: str | None = None,
        features: str | list[str] | None = None,
    ) -> CorpusInfo:
        """Load a corpus.

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
        path_obj = Path(path).expanduser().resolve()
        if not path_obj.exists():
            logger.error("Corpus path not found: %s", path)
            raise FileNotFoundError(f"Corpus path not found: {path}")

        name = name or path_obj.name
        logger.info("Loading corpus '%s' from %s", name, path_obj)

        # Load corpus
        CF = cfabric.Fabric(locations=str(path_obj), silent="deep")

        if features:
            if isinstance(features, list):
                features = " ".join(features)
            logger.debug("Loading features: %s", features)
            api = CF.load(features, silent="deep")
        else:
            logger.debug("Loading all features")
            api = CF.loadAll(silent="deep")

        if not api:
            logger.error("Failed to load corpus from %s", path)
            raise RuntimeError(f"Failed to load corpus from {path}")

        self._corpora[name] = (CF, api)
        self._current = name

        info = CorpusInfo.from_api(api, name, str(path_obj))
        logger.info(
            "Corpus '%s' loaded successfully: %d node types, %d node features, %d edge features",
            name,
            len(info.node_types),
            len(info.node_features),
            len(info.edge_features),
        )
        return info

    def get(self, name: str | None = None) -> tuple[cfabric.Fabric, Api]:
        """Get a loaded corpus.

        Parameters
        ----------
        name: str | None
            Corpus name (defaults to current)

        Returns
        -------
        tuple[Fabric, Api]
            The Fabric instance and API
        """
        name = name or self._current
        if not name:
            logger.error("Attempted to access corpus but none loaded")
            raise RuntimeError("No corpus loaded")
        if name not in self._corpora:
            logger.error("Corpus not found: %s", name)
            raise KeyError(f"Corpus not found: {name}")
        logger.debug("Accessing corpus '%s'", name)
        return self._corpora[name]

    def get_api(self, name: str | None = None) -> Api:
        """Get API for a corpus."""
        return self.get(name)[1]

    def list_corpora(self) -> list[str]:
        """List loaded corpora."""
        return list(self._corpora.keys())

    def set_current(self, name: str) -> None:
        """Set the current corpus."""
        if name not in self._corpora:
            logger.error("Cannot set current corpus - not found: %s", name)
            raise KeyError(f"Corpus not found: {name}")
        logger.info("Switched current corpus to '%s'", name)
        self._current = name

    @property
    def current(self) -> str | None:
        """Get the current corpus name."""
        return self._current

    def unload(self, name: str) -> None:
        """Unload a corpus."""
        if name in self._corpora:
            del self._corpora[name]
            logger.info("Unloaded corpus '%s'", name)
            if self._current == name:
                self._current = next(iter(self._corpora), None)
                if self._current:
                    logger.info("Current corpus switched to '%s'", self._current)
                else:
                    logger.info("No corpora remaining")
        else:
            logger.warning("Attempted to unload non-existent corpus: %s", name)

    def is_loaded(self, name: str) -> bool:
        """Check if a corpus is loaded."""
        return name in self._corpora


# Global corpus manager instance
corpus_manager = CorpusManager()
