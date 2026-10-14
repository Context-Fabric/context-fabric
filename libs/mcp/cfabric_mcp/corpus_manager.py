"""Corpus management for MCP server.

Handles loading and caching of corpora for the MCP server.
Supports multiple simultaneous corpora and ordered multi-location corpora.
"""

from __future__ import annotations

import logging
from collections.abc import Iterable
from os import PathLike
from pathlib import Path
from typing import TYPE_CHECKING

import cfabric
from cfabric.results import CorpusInfo

if TYPE_CHECKING:
    from cfabric.core.api import Api

logger = logging.getLogger("cfabric_mcp.corpus_manager")

CorpusLocation = str | PathLike[str]
CorpusLocations = CorpusLocation | Iterable[CorpusLocation]


def _normalize_locations(path: CorpusLocations) -> tuple[Path, ...]:
    """Resolve and validate one or more ordered corpus locations."""
    if isinstance(path, (str, PathLike)):
        raw_locations: tuple[CorpusLocation, ...] = (path,)
    else:
        try:
            raw_locations = tuple(path)
        except TypeError as exc:
            raise TypeError(
                "Corpus path must be a path or an iterable of paths"
            ) from exc

    if not raw_locations:
        raise ValueError("Corpus locations must contain at least one path")

    locations: list[Path] = []
    for raw_location in raw_locations:
        if not isinstance(raw_location, (str, PathLike)):
            raise TypeError(
                "Corpus path members must be str or os.PathLike objects"
            )
        location = Path(raw_location).expanduser().resolve()
        if not location.exists():
            logger.error("Corpus path not found: %s", raw_location)
            raise FileNotFoundError(f"Corpus path not found: {raw_location}")
        if not location.is_dir():
            logger.error("Corpus path is not a directory: %s", raw_location)
            raise NotADirectoryError(f"Corpus path is not a directory: {raw_location}")
        locations.append(location)
    return tuple(locations)


class CorpusManager:
    """Manages loaded corpora for the MCP server.

    Supports loading multiple corpora and switching between them. One logical
    corpus may itself be composed from multiple ordered Text-Fabric locations.
    """

    def __init__(self) -> None:
        self._corpora: dict[str, tuple[cfabric.Fabric, Api]] = {}
        self._current: str | None = None

    def load(
        self,
        path: CorpusLocations,
        name: str | None = None,
        features: str | list[str] | None = None,
    ) -> CorpusInfo:
        """Load a corpus from one path or multiple ordered locations.

        Parameters
        ----------
        path: str | os.PathLike | iterable of path-like values
            Corpus location or ordered corpus locations. Later locations retain
            core Context-Fabric's normal feature-precedence semantics.
        name: str | None
            Name for the corpus (defaults to the first/base directory name)
        features: str | list[str] | None
            Features to load (defaults to all)

        Returns
        -------
        CorpusInfo
            Information about the loaded corpus
        """
        locations = _normalize_locations(path)
        location_strings = [str(location) for location in locations]
        location_display = " | ".join(location_strings)

        name = name or locations[0].name
        logger.info("Loading corpus '%s' from %s", name, location_display)

        # Preserve the order supplied by the caller. Core Fabric already knows
        # how to compose feature files from multiple locations.
        CF = cfabric.Fabric(locations=location_strings, silent="deep")

        if features:
            if isinstance(features, list):
                features = " ".join(features)
            logger.debug("Loading features: %s", features)
            api = CF.load(features, silent="deep")
        else:
            logger.debug("Loading all features")
            api = CF.loadAll(silent="deep")

        if not api:
            logger.error("Failed to load corpus from %s", location_display)
            raise RuntimeError(f"Failed to load corpus from {location_display}")

        self._corpora[name] = (CF, api)
        self._current = name

        # Keep the historical scalar path value unchanged. For composed corpora
        # provide a deterministic, human-auditable ordered representation.
        info_path = location_strings[0] if len(location_strings) == 1 else location_display
        info = CorpusInfo.from_api(api, name, info_path)
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
