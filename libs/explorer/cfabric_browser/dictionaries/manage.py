"""Adding and removing dictionaries, from the library or the command line.

A dictionary is one file that we build once (`build.py`) and publish; adding
it downloads that file, checked by its checksum, into
`~/.cfabric/dictionaries/<id>/`. It can also be added from a file on this
computer (`cfabric-browser add bdb FILE`), which is checked by its contents.
The browser reads the file into memory, so it can be removed while it runs.
"""

from __future__ import annotations

import shutil
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from cfabric_browser.dictionaries.store import Dictionary, DictionaryError
from cfabric_browser.downloads import DownloadError, Fetch, _fetch, _get
from cfabric_browser.jobs import BackgroundJobs, Progress

ROOT = "~/.cfabric/dictionaries"


@dataclass(frozen=True)
class Catalogued:
    id: str
    short: str  # the name in its meta (`BDB`)
    title: str
    name: str
    licence: str
    licenceNote: str
    sources: str
    corpus: str  # whose lexemes it links (`ETCBC/bhsa`)
    url: str | None  # None until it is published
    sha256: str | None  # of the published file
    size: int


DICTIONARIES: dict[str, Catalogued] = {
    "bdb": Catalogued(
        id="bdb",
        short="BDB",
        title="Brown–Driver–Briggs",
        name="BDB (Brown–Driver–Briggs, 1906)",
        licence="CC BY-NC 4.0",
        licenceNote=(
            "A Hebrew and English Lexicon of the Old Testament (Oxford, 1906). "
            "The file is CC BY-NC 4.0: BDB's text, from Sefaria, is public domain; "
            "Open Scriptures' ids and short definitions remain CC BY 4.0; the "
            "links derive from the BHSA (CC BY-NC 4.0)."
        ),
        sources="Sefaria; Open Scriptures HebrewLexicon and OSHB (CC BY 4.0)",
        corpus="ETCBC/bhsa",
        url="https://github.com/Context-Fabric/bdb/releases/download/v1.0/bdb.json.gz",
        sha256="d320e12c35f08891bf31edb6ba53a2eb78f5a0b5c8f3f7472b2f504689a6b7e1",
        size=4_554_209,
    ),
}


def target(catalogued: Catalogued, root: str | Path = ROOT) -> Path:
    return Path(root).expanduser() / catalogued.id / f"{catalogued.id}.json.gz"


def is_added(catalogued: Catalogued, root: str | Path = ROOT) -> bool:
    return target(catalogued, root).is_file()


def check(catalogued: Catalogued, path: Path) -> None:
    """Stop with a `DictionaryError` unless `path` is this dictionary."""
    found = Dictionary.load(path)
    if found.meta.get("short") != catalogued.short:
        raise DictionaryError(f"{path} is not {catalogued.name}")


def add(
    catalogued: Catalogued,
    root: str | Path = ROOT,
    progress: Progress | None = None,
    fetcher: Fetch = _fetch,
) -> Path:
    """Download the published file into its place."""
    if catalogued.url is None or catalogued.sha256 is None:
        raise DownloadError(f"{catalogued.name} has not been published yet")
    place = target(catalogued, root)
    place.parent.mkdir(parents=True, exist_ok=True)
    report = (
        (lambda done, total: progress("download", done, total or catalogued.size))
        if progress
        else (lambda done, total: None)
    )
    partial = place.with_name(place.name + ".new")
    _get(catalogued.url, partial, catalogued.sha256, fetcher, report)
    check(catalogued, partial)
    partial.replace(place)
    return place


def add_file(catalogued: Catalogued, path: str | Path, root: str | Path = ROOT) -> Path:
    """Put a file built here, or downloaded by hand, in its place."""
    source = Path(path).expanduser()
    check(catalogued, source)
    place = target(catalogued, root)
    if source.resolve() == place.resolve():
        return place
    place.parent.mkdir(parents=True, exist_ok=True)
    partial = place.with_name(place.name + ".new")
    shutil.copyfile(source, partial)
    partial.replace(place)
    return place


def remove(catalogued: Catalogued, root: str | Path = ROOT) -> bool:
    """Delete the file; False if it wasn't there."""
    place = target(catalogued, root)
    if not place.is_file():
        return False
    place.unlink()
    return True


class DictionaryJobs(BackgroundJobs):
    """Downloads of dictionaries in the background, for the library."""

    def __init__(self, root: str | Path = ROOT, fetcher: Fetch = _fetch) -> None:
        super().__init__(expected=(DownloadError, DictionaryError))
        self.root = root
        self.fetcher = fetcher

    def start(self, catalogued: Catalogued) -> dict[str, Any]:
        return self.run(
            catalogued.id,
            lambda progress: add(catalogued, self.root, progress, self.fetcher),
        )
