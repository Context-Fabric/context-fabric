"""Finding corpora on disk and holding them while the browser runs.

A corpus can be given as:

*   a data directory with `.tf` files and/or a `.cfm` cache;
*   a repository directory in Text-Fabric layout (`<repo>/tf/<version>/`),
    in which case the version named in the app config, or else the newest,
    is used;
*   a short name (`bhsa`) or `org/repo` (`ETCBC/bhsa`), looked up under
    `~/text-fabric-data/github` and `~/github`, where Text-Fabric keeps
    downloaded and cloned corpora;

optionally prefixed with `name=` to choose the name shown in the browser.
"""

from __future__ import annotations

import logging
import threading
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from cfabric.display import DisplayConfig, find_app_config
from cfabric.display.config import read_app_config
from cfabric.registry import CorpusRegistry
from cfabric.utils.helpers import versionSort

logger = logging.getLogger(__name__)

SEARCH_ROOTS = ("~/text-fabric-data/github", "~/github")


@dataclass(frozen=True)
class CorpusSpec:
    """A corpus to serve: its name in the browser and its data directory."""

    name: str
    path: Path

    def to_dict(self) -> dict[str, str]:
        return {"name": self.name, "path": str(self.path)}


def _is_data_dir(path: Path) -> bool:
    return (path / ".cfm").is_dir() or any(path.glob("otype.tf"))


def _pick_version(repo: Path) -> Path | None:
    """The data directory of a repository in TF layout (`<repo>/tf/<version>`)."""
    tfDir = repo / "tf"
    if not tfDir.is_dir():
        return None
    versions = [d for d in tfDir.iterdir() if d.is_dir() and _is_data_dir(d)]
    if not versions:
        return None

    appConfig = repo / "app" / "config.yaml"
    if appConfig.is_file():
        try:
            wanted = (read_app_config(appConfig).get("provenanceSpec") or {}).get(
                "version"
            )
        except Exception:  # noqa: BLE001 - a broken config should not block startup
            wanted = None
        if wanted is not None:
            for d in versions:
                if d.name == str(wanted):
                    return d

    return max(versions, key=lambda d: versionSort(d.name))


def _find_repo(name: str, roots: tuple[str, ...]) -> Path | None:
    parts = name.strip("/").split("/")
    for root in roots:
        base = Path(root).expanduser()
        if not base.is_dir():
            continue
        if len(parts) == 2:
            candidate = base / parts[0] / parts[1]
            if candidate.is_dir():
                return candidate
            # organisation names differ in case between sources (ETCBC / etcbc)
            for org in base.iterdir():
                if org.name.lower() == parts[0].lower():
                    for repo in org.iterdir():
                        if repo.name.lower() == parts[1].lower():
                            return repo
        elif len(parts) == 1:
            for org in sorted(base.iterdir()):
                if not org.is_dir():
                    continue
                for repo in org.iterdir():
                    if repo.name.lower() == parts[0].lower() and (repo / "tf").is_dir():
                        return repo
    return None


def resolve_corpus(spec: str, roots: tuple[str, ...] = SEARCH_ROOTS) -> CorpusSpec:
    """Turn a command-line corpus specification into a name and a data directory.

    Raises
    ------
    FileNotFoundError
        If no corpus data can be found for the specification.
    """
    name: str | None = None
    target = spec
    if "=" in spec:
        (name, target) = spec.split("=", 1)
        name = name.strip() or None

    path = Path(target).expanduser()
    dataDir: Path | None = None
    repo: Path | None = None

    if path.is_dir():
        if _is_data_dir(path):
            dataDir = path
        else:
            dataDir = _pick_version(path)
            repo = path
    else:
        repo = _find_repo(target, roots)
        if repo is not None:
            dataDir = _pick_version(repo)

    if dataDir is None:
        raise FileNotFoundError(f"No corpus data found for '{spec}'")

    if name is None:
        if repo is not None:
            name = repo.name
        elif dataDir.parent.name == "tf":
            name = dataDir.parent.parent.name
        else:
            name = dataDir.name
    return CorpusSpec(name=name.lower(), path=dataDir.resolve())


class CorpusService:
    """The corpora served by the browser, with their display configurations."""

    def __init__(self, specs: list[CorpusSpec] | None = None) -> None:
        self.registry = CorpusRegistry()
        self._specs: dict[str, CorpusSpec] = {}
        self._configs: dict[str, DisplayConfig] = {}
        self._lock = threading.Lock()
        for spec in specs or ():
            self.add(spec)

    def add(self, spec: CorpusSpec) -> None:
        self.registry.register(spec.path, spec.name)
        self._specs[spec.name] = spec

    @property
    def specs(self) -> list[CorpusSpec]:
        return list(self._specs.values())

    def names(self) -> list[str]:
        return list(self._specs)

    def has(self, name: str) -> bool:
        return name in self._specs

    def load_all(self) -> None:
        for name in self._specs:
            self.config(name)

    def api(self, name: str):
        return self.registry.get_api(name)

    def config(self, name: str) -> DisplayConfig:
        with self._lock:
            config = self._configs.get(name)
        if config is not None:
            return config
        api = self.api(name)
        config = DisplayConfig.from_api(api, corpus_path=self._specs[name].path)
        with self._lock:
            self._configs[name] = config
        return config

    def card(self, name: str) -> dict[str, Any]:
        """Everything a client needs to know about a corpus to work with it."""
        api = self.api(name)
        config = self.config(name)
        F = api.F
        T = api.T
        types = []
        for nType in F.otype.all:
            (first, last) = F.otype.sInterval(nType)
            types.append(
                {
                    "type": nType,
                    "count": int(last) - int(first) + 1,
                    "isSlot": nType == F.otype.slotType,
                }
            )
        provenance = config.provenance
        return {
            "name": name,
            "title": provenance.get("corpus") or name,
            "version": str(provenance.get("version") or self._specs[name].path.name),
            "doi": provenance.get("doi"),
            "hasAppConfig": find_app_config(self._specs[name].path) is not None,
            "slotType": F.otype.slotType,
            "maxSlot": int(F.otype.maxSlot),
            "maxNode": int(F.otype.maxNode),
            "nodeTypes": types,
            "sectionTypes": list(T.sectionTypes or ()),
            "display": config.to_dict(),
        }
