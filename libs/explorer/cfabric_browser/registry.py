"""Thread-safe registry of named corpora.

A `CorpusRegistry` holds several corpora at once, each under a short name
(e.g. `bhsa`, `dss`), for the browser's server and its search workers. Moved
here from the core (it was only on the browser's branches), so that the
browser works with any core: the Python one and the Rust one (0.6).

Corpora are registered with `register()` and loaded on first use, at most
once per corpus, also when several threads ask for the same corpus at the same
time. `get`, `get_api`, `list_corpora` and `current` are also what the tools of
`cfabric-mcp` ask of a registry (`cfabric_mcp.tools.using`).
"""

from __future__ import annotations

import logging
import threading
from dataclasses import dataclass, field
from pathlib import Path
from typing import TYPE_CHECKING

from cfabric import Fabric

if TYPE_CHECKING:
    from cfabric.core.api import Api

logger = logging.getLogger(__name__)


@dataclass
class CorpusEntry:
    """A registered corpus and, once loaded, its Fabric and Api."""

    name: str
    path: Path
    features: str | None = None
    fabric: Fabric | None = None
    api: Api | None = None
    load_lock: threading.Lock = field(default_factory=threading.Lock, repr=False)

    @property
    def loaded(self) -> bool:
        return self.api is not None


class CorpusRegistry:
    """Holds named corpora and loads each at most once.

    Examples
    --------
    >>> registry = CorpusRegistry()
    >>> registry.register("~/text-fabric-data/github/ETCBC/bhsa/tf/2021", "bhsa")
    >>> api = registry.get_api("bhsa")  # loads on first use
    """

    def __init__(self) -> None:
        self._lock = threading.RLock()
        self._entries: dict[str, CorpusEntry] = {}
        self._current: str | None = None

    # registration and loading

    def register(
        self,
        path: str | Path,
        name: str | None = None,
        features: str | list[str] | None = None,
    ) -> str:
        """Register a corpus without loading it.

        Parameters
        ----------
        path: str | Path
            Directory with the corpus data (`.tf` files and/or a `.cfm` cache).
        name: str | None
            Name for the corpus. Defaults to the directory name.
        features: str | list[str] | None
            Features to load. Defaults to all features.

        Returns
        -------
        str
            The name under which the corpus is registered.

        Raises
        ------
        FileNotFoundError
            If the path does not exist.
        ValueError
            If the name is already registered for a different path.
        """
        pathObj = Path(path).expanduser().resolve()
        if not pathObj.exists():
            logger.error("Corpus path not found: %s", path)
            raise FileNotFoundError(f"Corpus path not found: {path}")

        name = name or pathObj.name
        if isinstance(features, list):
            features = " ".join(features)

        with self._lock:
            existing = self._entries.get(name)
            if existing is not None:
                if existing.path != pathObj:
                    raise ValueError(
                        f"Corpus name '{name}' is already used for {existing.path}"
                    )
                return name
            self._entries[name] = CorpusEntry(name, pathObj, features or None)
            if self._current is None:
                self._current = name
        logger.info("Registered corpus '%s' at %s", name, pathObj)
        return name

    def _entry(self, name: str | None) -> CorpusEntry:
        with self._lock:
            name = name or self._current
            if not name:
                logger.error("Attempted to access corpus but none loaded")
                raise RuntimeError("No corpus loaded")
            entry = self._entries.get(name)
        if entry is None:
            logger.error("Corpus not found: %s", name)
            raise KeyError(f"Corpus not found: {name}")
        return entry

    def _ensure_loaded(self, name: str) -> CorpusEntry:
        entry = self._entry(name)
        if entry.loaded:
            return entry
        with entry.load_lock:
            if entry.loaded:
                return entry
            logger.info("Loading corpus '%s' from %s", name, entry.path)
            fabric = Fabric(locations=str(entry.path), silent="deep")
            if entry.features:
                api = fabric.load(entry.features, silent="deep")
            else:
                api = fabric.loadAll(silent="deep")
            # the Python core gives False for a corpus it could not load
            if api is not None and api is not False:
                entry.fabric = fabric
                entry.api = api
                return entry
        logger.error("Failed to load corpus from %s", entry.path)
        raise RuntimeError(f"Failed to load corpus from {entry.path}")

    # access

    def get(self, name: str | None = None) -> tuple[Fabric, Api]:
        """The Fabric and Api of a corpus, loading it if needed.

        Parameters
        ----------
        name: str | None
            Corpus name. Defaults to the current corpus.

        Raises
        ------
        RuntimeError
            If no name is given and there is no current corpus.
        KeyError
            If the corpus is not registered.
        """
        entry = self._ensure_loaded(self._entry(name).name)
        assert entry.fabric is not None and entry.api is not None
        return (entry.fabric, entry.api)

    def get_api(self, name: str | None = None) -> Api:
        """The Api of a corpus, loading it if needed."""
        return self.get(name)[1]

    # bookkeeping

    def list_corpora(self) -> list[str]:
        """Names of the loaded corpora."""
        with self._lock:
            return [n for (n, e) in self._entries.items() if e.loaded]

    @property
    def current(self) -> str | None:
        """The name of the current (default) corpus."""
        return self._current
