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

from cfabric_browser import formats
from cfabric_browser.books import book_index, canonical_book
from cfabric_browser.display import (
    DisplayConfig,
    find_app_config,
    node_from_section_ref,
    section_ref_for,
)
from cfabric_browser.display.config import read_app_config
from cfabric_browser.helpers import versionSort
from cfabric_browser.lexicon import Lexicon, has_lexicon
from cfabric_browser.registry import CorpusRegistry
from cfabric_browser.witnesses import BookIndex, format_ref, parse_ref

logger = logging.getLogger(__name__)

SEARCH_ROOTS = ("~/text-fabric-data/github", "~/github")
OVERRIDES = "~/.cfabric/display"
# display settings that come with the browser, for corpora whose app config
# lacks them; local ones (OVERRIDES) win
BUILT_IN = Path(__file__).parent / "display_settings"


@dataclass(frozen=True)
class CorpusSpec:
    """A corpus to serve: its name in the browser and its data directory."""

    name: str
    path: Path

    def to_dict(self) -> dict[str, str]:
        return {"name": self.name, "path": str(self.path)}


def corpus_id(path: Path) -> str | None:
    """`org/repo` of a data directory in Text-Fabric layout (`<org>/<repo>/tf/<version>`),
    e.g. `ETCBC/bhsa`; None for a data directory elsewhere."""
    if path.parent.name != "tf":
        return None
    repo = path.parent.parent
    return f"{repo.parent.name}/{repo.name}"


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


def discover(
    roots: tuple[str, ...] = SEARCH_ROOTS,
    hide: tuple[str, ...] | list[str] = (),
    collapse: bool = True,
    only: tuple[str, ...] | list[str] = (),
) -> list[dict[str, Any]]:
    """Corpora on this computer, in Text-Fabric layout (`<org>/<repo>/tf/<version>`).

    Each entry has the version that opening it would load (`version`), besides
    all versions on disk. A corpus found under several roots is listed once,
    from the first root.

    Parameters
    ----------
    hide: ids (`org/repo`) to leave out, e.g. corpora someone does not use.
    only: if given, the ids (`org/repo`) to consider; all others are left out,
        e.g. for a test with a few corpora.
    collapse: list corpora with the same title (copies and forks of one
        corpus, such as `DT-UCPH/sp` and `saulocantanhede/sp`) once, keeping the
        one with the newest data; the others are named in `copies`.
    """
    hidden = {h.lower() for h in hide}
    allowed = {o.lower() for o in only}
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
                if key in found or key in hidden or (allowed and key not in allowed):
                    continue
                default = _pick_version(repo)
                found[key] = {
                    "id": f"{org.name}/{repo.name}",
                    "name": repo.name.lower(),
                    "title": _app_title(repo),
                    "version": default.name if default else versions[-1],
                    "versions": versions,
                }
    entries = sorted(found.values(), key=lambda c: c["id"].lower())
    if not collapse:
        return entries

    byTitle: dict[str, list[dict[str, Any]]] = {}
    for entry in entries:
        if entry["title"]:
            byTitle.setdefault(entry["title"].strip().lower(), []).append(entry)
    dropped: set[str] = set()
    for group in byTitle.values():
        if len(group) < 2:
            continue
        keep = max(group, key=lambda c: versionSort(c["versions"][-1]))
        keep["copies"] = [c["id"] for c in group if c is not keep]
        dropped.update(c["id"] for c in group if c is not keep)
    return [e for e in entries if e["id"] not in dropped]


SETTINGS = "~/.cfabric/browser.yaml"


def load_settings(path: str | Path = SETTINGS) -> dict[str, Any]:
    """Personal browser settings, e.g. corpora to leave out of the library::

    library:
      hide:
        - CLARIAH/descartes-tf
    """
    file = Path(path).expanduser()
    if not file.is_file():
        return {}
    try:
        settings: dict[str, Any] = read_app_config(file)
        return settings
    except Exception:  # noqa: BLE001 - a broken settings file must not stop the browser
        logger.warning("Could not read browser settings from %s", file)
        return {}


def _merge(base: dict[str, Any], extra: dict[str, Any]) -> dict[str, Any]:
    """Deep merge of two config mappings; `extra` wins."""
    merged = dict(base)
    for key, value in extra.items():
        if isinstance(value, dict) and isinstance(merged.get(key), dict):
            merged[key] = _merge(merged[key], value)
        else:
            merged[key] = value
    return merged


def display_overrides(
    path: Path, overrides: str | Path = OVERRIDES, built_in: Path = BUILT_IN
) -> tuple[dict[str, Any], list[Path]]:
    """Display settings for a corpus, layered over its app config.

    For data in `<org>/<repo>/tf/<version>`, these files are read, later ones
    winning: `<built_in>/<org>/<repo>.yaml`, `<built_in>/<org>/<repo>@<version>.yaml`
    (those that come with the browser), then the same two in `<overrides>`
    (local ones). They use the format of a Text-Fabric app `config.yaml`
    (`typeDisplay`, `dataDisplay`, ...) and need only contain what differs.
    This is the place for settings that a corpus' own app config does not
    (yet) have, without editing downloaded files.
    """
    if path.parent.name != "tf":
        return ({}, [])
    repo = path.parent.parent
    merged: dict[str, Any] = {}
    used: list[Path] = []
    for root in (built_in, Path(overrides).expanduser()):
        base = root / repo.parent.name
        for candidate in (
            base / f"{repo.name}.yaml",
            base / f"{repo.name}@{path.name}.yaml",
        ):
            if candidate.is_file():
                merged = _merge(merged, read_app_config(candidate))
                used.append(candidate)
    return (merged, used)


class CorpusService:
    """The corpora served by the browser, with their display configurations."""

    def __init__(
        self,
        specs: list[CorpusSpec] | None = None,
        roots: tuple[str, ...] = SEARCH_ROOTS,
        overrides: str | Path = OVERRIDES,
        hide: tuple[str, ...] | list[str] = (),
        only: tuple[str, ...] | list[str] = (),
    ) -> None:
        self.registry = CorpusRegistry()
        self.roots = roots  # where the library looks for corpora
        self.overrides = overrides  # local display settings, see display_overrides
        self.hide = tuple(hide)  # library ids not to list
        self.only = tuple(only)  # if given, the only library ids to list and open
        self._specs: dict[str, CorpusSpec] = {}
        self._configs: dict[str, DisplayConfig] = {}
        self._books: dict[str, dict[str, str]] = {}
        self._bookIndexes: dict[str, BookIndex | None] = {}
        self._lexicons: dict[str, Lexicon | None] = {}
        self._translations: dict[str, dict[str, Any] | None] = {}
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
        path = self._specs[name].path
        appConfig = find_app_config(path)
        settings = read_app_config(appConfig) if appConfig else {}
        (extra, used) = display_overrides(path, self.overrides)
        for file in used:
            logger.info("Display settings for %s from %s", name, file)
        config = DisplayConfig.from_api(api, config=_merge(settings, extra))
        config.source = appConfig
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

    def book_index(self, name: str) -> BookIndex | None:
        """The biblical books in a corpus of manuscripts (DSS), if it has them.

        Only for corpora whose own sections are not books: see `witnesses.py`.
        """
        with self._lock:
            if name in self._bookIndexes:
                return self._bookIndexes[name]
        index = None if self.books(name) else BookIndex.build(self.api(name))
        with self._lock:
            self._bookIndexes[name] = index
        return index

    def lexicon(self, name: str) -> Lexicon | None:
        """The lexicon of a corpus with lexeme nodes (BHSA), if it has one."""
        with self._lock:
            if name in self._lexicons:
                return self._lexicons[name]
        lexicon = Lexicon.build(self.api(name))
        with self._lock:
            self._lexicons[name] = lexicon
        return lexicon

    def glosses(self, name: str) -> tuple[Any, str] | None:
        """For a corpus without glosses (the DSS): a lookup of glosses by ETCBC
        lexeme in an open corpus with a lexicon (the BHSA), and its name."""
        if getattr(self.api(name).F, "gloss", None) is not None:
            return None
        for other in self.names():
            if other == name:
                continue
            lexicon = self.lexicon(other)
            if lexicon is not None:
                return (lexicon.gloss, other)
        return None

    def align(
        self,
        source: str,
        ref: str,
        target: str,
        lang: str = "en",
        view: str = "sections",
    ) -> dict[str, Any] | None:
        """The passage in `target` that corresponds to `ref` in `source`.

        Books are matched through the book concordance (`1_Samuel` in one
        corpus is `1Sam` in another); chapters and verses by number. When the
        verse (or chapter) does not exist in the target, the nearest enclosing
        section that does is returned, with `exact` false.

        Returns None when the target has no counterpart, e.g. a book it does
        not contain, or a corpus with a different kind of sections.

        `ref` may also be a book view reference of `source` (`Genesis 1:1` in
        the DSS); with `view="books"` the result is one in the book view of
        `target` (its `node` is None there).

        Raises
        ------
        ValueError
            If `ref` is not a passage of `source`.
        """
        sApi, tApi = self.api(source), self.api(target)
        sConfig, tConfig = self.config(source), self.config(target)
        section: tuple
        sIndex = self.book_index(source)
        parsed = parse_ref(ref) if sIndex else None
        if parsed:
            (book, chapter, verse) = parsed
            section = (book, chapter) if verse is None else parsed
        else:
            node = node_from_section_ref(
                sApi, ref, sConfig.section_sep1, sConfig.section_sep2, lang
            )
            section = sApi.T.sectionFromNode(node, lang="en")
        if not section:
            return None

        book = str(section[0])
        canonical = canonical_book(book)
        if view == "books":
            return self._align_books(target, canonical, section)
        if not tApi.T.sectionTypes:
            return None
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

    def _align_books(
        self, target: str, canonical: str | None, section: tuple
    ) -> dict[str, Any] | None:
        index = self.book_index(target)
        if index is None or canonical is None or canonical not in index.books:
            return None
        if len(section) < 2:
            chapter = next(iter(index.books[canonical]))
            return {"node": None, "ref": format_ref(canonical, chapter), "exact": False}
        chapter = int(section[1])
        if not index.has(canonical, chapter):
            return None
        if len(section) > 2 and index.has(canonical, chapter, int(section[2])):
            ref = format_ref(canonical, chapter, int(section[2]))
            return {"node": None, "ref": ref, "exact": True}
        return {
            "node": None,
            "ref": format_ref(canonical, chapter),
            "exact": len(section) == 2,
        }

    def translation(self, name: str) -> dict[str, Any] | None:
        """What the app config says about a translation (`translation:`, written
        by `cfabric_browser.translations`): licence, source, how to cite it.
        None for a corpus that is not a translation."""
        with self._lock:
            if name in self._translations:
                return self._translations[name]
        found = None
        config = find_app_config(self._specs[name].path)
        if config is not None:
            try:
                found = read_app_config(config).get("translation") or None
            except Exception:  # noqa: BLE001 - a broken config: not a translation
                found = None
        with self._lock:
            self._translations[name] = found
        return found

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
            "id": corpus_id(self._specs[name].path),
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
            # whether the corpus can also be read by biblical book (DSS)
            "bookView": self.book_index(name) is not None,
            # whether the corpus has a lexicon with glosses (BHSA)
            "lexicon": has_lexicon(api),
            # a translation (JPS 1917): licence, source, how to cite it
            "translation": self.translation(name),
            # only the text formats the browser offers (formats.py)
            "display": formats.display(config),
        }
