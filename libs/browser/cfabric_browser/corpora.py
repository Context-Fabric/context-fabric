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

from cfabric.display import (
    DisplayConfig,
    find_app_config,
    node_from_section_ref,
    section_ref_for,
)
from cfabric.display.config import read_app_config
from cfabric.registry import CorpusRegistry
from cfabric.utils.helpers import versionSort

from cfabric_browser.books import book_index, canonical_book

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


LATEST = "latest"


def _pick_version(repo: Path, version: str | None = None) -> Path | None:
    """The data directory of a repository in TF layout (`<repo>/tf/<version>`).

    Without `version`: the version named in the app config, or else the newest.
    With `version`: that version, or the newest for `latest`.

    Raises
    ------
    FileNotFoundError
        If the requested version is not on disk; the message lists those that are.
    """
    tfDir = repo / "tf"
    if not tfDir.is_dir():
        return None
    versions = [d for d in tfDir.iterdir() if d.is_dir() and _is_data_dir(d)]
    if not versions:
        return None

    if version == LATEST:
        return max(versions, key=lambda d: versionSort(d.name))
    if version is not None:
        for d in versions:
            if d.name == version:
                return d
        available = ", ".join(sorted((d.name for d in versions), key=versionSort))
        raise FileNotFoundError(
            f"{repo.parent.name}/{repo.name} has no version {version}; "
            f"on this computer: {available}"
        )

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

    A version can be chosen with `@`: `DT-UCPH/sp@7.2.4`, or `sp@latest` for
    the newest version on this computer.

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

    version: str | None = None
    path = Path(target).expanduser()
    if not path.is_dir() and "@" in target:
        (target, version) = target.rsplit("@", 1)
        version = version.strip() or None
        path = Path(target).expanduser()

    dataDir: Path | None = None
    repo: Path | None = None

    if path.is_dir():
        if _is_data_dir(path):
            dataDir = path
        else:
            dataDir = _pick_version(path, version)
            repo = path
    else:
        repo = _find_repo(target, roots)
        if repo is not None:
            dataDir = _pick_version(repo, version)

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


def _app_title(repo: Path) -> str | None:
    config = repo / "app" / "config.yaml"
    if not config.is_file():
        return None
    try:
        provenance = read_app_config(config).get("provenanceSpec") or {}
    except Exception:  # noqa: BLE001 - a broken config only costs the title
        return None
    title = provenance.get("corpus")
    return str(title) if title else None


def discover(roots: tuple[str, ...] = SEARCH_ROOTS) -> list[dict[str, Any]]:
    """Corpora on this computer, in Text-Fabric layout (`<org>/<repo>/tf/<version>`).

    A corpus found under several roots is listed once, from the first root.
    """
    found: dict[str, dict[str, Any]] = {}
    for root in roots:
        base = Path(root).expanduser()
        if not base.is_dir():
            continue
        for org in sorted(base.iterdir()):
            if not org.is_dir():
                continue
            for repo in sorted(org.iterdir()):
                tfDir = repo / "tf"
                if not tfDir.is_dir():
                    continue
                versions = sorted(
                    (d.name for d in tfDir.iterdir() if d.is_dir() and _is_data_dir(d)),
                    key=versionSort,
                )
                if not versions:
                    continue
                key = f"{org.name}/{repo.name}".lower()
                if key in found:
                    continue
                found[key] = {
                    "id": f"{org.name}/{repo.name}",
                    "name": repo.name.lower(),
                    "title": _app_title(repo),
                    "versions": versions,
                }
    return sorted(found.values(), key=lambda c: c["id"].lower())


class CorpusService:
    """The corpora served by the browser, with their display configurations."""

    def __init__(
        self,
        specs: list[CorpusSpec] | None = None,
        roots: tuple[str, ...] = SEARCH_ROOTS,
    ) -> None:
        self.registry = CorpusRegistry()
        self.roots = roots  # where the library looks for corpora
        self._specs: dict[str, CorpusSpec] = {}
        self._configs: dict[str, DisplayConfig] = {}
        self._books: dict[str, dict[str, str]] = {}
        self._lock = threading.Lock()
        for spec in specs or ():
            self.add(spec)

    def add(self, spec: CorpusSpec) -> None:
        self.registry.register(spec.path, spec.name)
        self._specs[spec.name] = spec

    def spec(self, name: str) -> CorpusSpec:
        return self._specs[name]

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

    def books(self, name: str) -> dict[str, str]:
        """Canonical book ids of a corpus, mapped to its own book names."""
        with self._lock:
            index = self._books.get(name)
        if index is not None:
            return index
        api = self.api(name)
        T = api.T
        names = (
            [
                str(T.sectionFromNode(int(n), lang="en")[0])
                for n in api.F.otype.s(T.sectionTypes[0])
            ]
            if T.sectionTypes
            else []
        )
        index = book_index(names)
        with self._lock:
            self._books[name] = index
        return index

    def align(
        self, source: str, ref: str, target: str, lang: str = "en"
    ) -> dict[str, Any] | None:
        """The passage in `target` that corresponds to `ref` in `source`.

        Books are matched through the book concordance (`1_Samuel` in one
        corpus is `1Sam` in another); chapters and verses by number. When the
        verse (or chapter) does not exist in the target, the nearest enclosing
        section that does is returned, with `exact` false.

        Returns None when the target has no counterpart, e.g. a book it does
        not contain, or a corpus with a different kind of sections.

        Raises
        ------
        ValueError
            If `ref` is not a passage of `source`.
        """
        sApi, tApi = self.api(source), self.api(target)
        sConfig, tConfig = self.config(source), self.config(target)
        node = node_from_section_ref(
            sApi, ref, sConfig.section_sep1, sConfig.section_sep2, lang
        )
        section = sApi.T.sectionFromNode(node, lang="en")
        if not section or not tApi.T.sectionTypes:
            return None

        book = str(section[0])
        canonical = canonical_book(book)
        tBook = self.books(target).get(canonical) if canonical else None
        if tBook is None:
            # corpora with other kinds of sections: same name or nothing
            tBook = book
        parts = (tBook, *section[1 : len(tApi.T.sectionTypes)])
        for k in range(len(parts), 0, -1):
            tNode = tApi.T.nodeFromSection(parts[:k], lang="en")
            if tNode is not None:
                return {
                    "node": int(tNode),
                    "ref": section_ref_for(tApi, tConfig, int(tNode), lang=lang),
                    "exact": k == len(section),
                }
        return None

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
            # the version actually loaded, which may differ from the app config's
            "version": self._specs[name].path.name,
            "doi": provenance.get("doi"),
            "hasAppConfig": find_app_config(self._specs[name].path) is not None,
            "slotType": F.otype.slotType,
            "maxSlot": int(F.otype.maxSlot),
            "maxNode": int(F.otype.maxNode),
            "nodeTypes": types,
            "sectionTypes": list(T.sectionTypes or ()),
            "display": config.to_dict(),
        }
