"""Downloading corpora that are not on this computer yet.

The corpora come from their own releases on GitHub, the same files that
Text-Fabric downloads (`tf.advanced.repo`): a zip attached to a release, either
`complete.zip` (every data folder of the release, from `<org>/` down) or a zip
of one data directory with only its feature files (`flat`). Each download is
pinned: its release, the zip's checksum, and its size.

They are unpacked into the place where Text-Fabric keeps its downloads
(`~/text-fabric-data/github`), in Text-Fabric's layout and with its
`__checkout__.txt` files (release and commit), so that Text-Fabric takes them
for its own downloads and the library finds them like any other corpus.

Unpacking is careful:

*   a zip is checked against its checksum before anything is unpacked;
*   names that would leave the destination (`..`, absolute paths) or belong to
    another organisation are refused;
*   everything is unpacked into a folder next to the destination first, and
    each data folder (a folder with a `__checkout__.txt`) is moved into place
    only when all are unpacked;
*   a data folder that is already there is left as it is: nothing on this
    computer is overwritten.
"""

from __future__ import annotations

import hashlib
import shutil
import urllib.request
import uuid
import zipfile
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath
from typing import Any

from cfabric_browser import __version__
from cfabric_browser.jobs import BackgroundJobs
from cfabric_browser.translations.build import _ssl_context

ROOT = "~/text-fabric-data/github"
CACHE = "~/.cfabric/downloads"
CHECKOUT = "__checkout__.txt"
CHUNK = 1 << 16

# called with a step ("download", "unpack"), the amount done and the total
Progress = Callable[[str, int, int], None]
# fetch a URL into a file, reporting (bytes done, bytes in all)
Fetch = Callable[[str, Path, Callable[[int, int], None]], None]


class DownloadError(Exception):
    """A download that failed or is not what was expected; the message says why."""


@dataclass(frozen=True)
class Extra:
    """A file that the release zip lacks, from the repository at the release,
    e.g. the DSS's `app/config.yaml` (display settings)."""

    path: str  # relative to the repository folder, e.g. app/config.yaml
    url: str
    sha256: str


@dataclass(frozen=True)
class Download:
    """A corpus that can be downloaded."""

    id: str  # org/repo, as in the library
    name: str  # the short name, as on the command line
    title: str
    version: str  # the data version, the folder under tf/
    release: str  # the release tag on GitHub
    commit: str  # the release's commit, for __checkout__.txt
    asset: str  # the zip attached to the release
    size: int  # bytes to download
    unpacked: int  # bytes once unpacked
    sha256: str
    layout: str  # "complete" (folders from <org>/ down) or "flat" (tf/<version> only)
    terms: str  # where its terms of use are
    extras: tuple[Extra, ...] = field(default_factory=tuple)
    # the licence of the data, where it is known (confirmed by the user)
    licence: str | None = None

    @property
    def org(self) -> str:
        return self.id.split("/")[0]

    @property
    def repo(self) -> str:
        return self.id.split("/")[1]

    @property
    def url(self) -> str:
        return f"https://github.com/{self.id}/releases/download/{self.release}/{self.asset}"

    def data_dir(self, root: str | Path = ROOT) -> Path:
        return Path(root).expanduser() / self.org / self.repo / "tf" / self.version


DOWNLOADS: dict[str, Download] = {
    d.id: d
    for d in (
        Download(
            id="ETCBC/bhsa",
            name="bhsa",
            title="BHSA = Biblia Hebraica Stuttgartensia Amstelodamensis",
            version="2021",
            release="v1.8.1",
            commit="b112c161cfd21eae403d51a2733740d8743460e7",
            asset="complete.zip",
            size=33_963_367,
            unpacked=178_427_610,
            sha256="8104fae1151c926cfcfd01f7e8a30a09af8c607546f14482990833b624b73168",
            layout="complete",
            terms="https://github.com/ETCBC/bhsa",
        ),
        Download(
            id="DT-UCPH/sp",
            name="sp",
            title="The Samaritan Pentateuch",
            version="7.2.4",
            release="v7.2.4",
            commit="6da31aa1e02478fb5eab77c6e51259aaa0446c8f",
            asset="complete.zip",
            size=3_860_569,
            unpacked=21_520_139,
            sha256="8d3f2be7ca035d87c1d22a5164a63b43c114b0da437b1f30a99183e9cbf299b8",
            layout="complete",
            terms="https://github.com/DT-UCPH/sp",
            licence="CC BY-NC 4.0",
        ),
        Download(
            id="ETCBC/dss",
            name="dss",
            title="Dead Sea Scrolls",
            version="2.0.1",
            release="v2.0.1",
            commit="47ecc1738fad6e67df7d40a3e84d73e4a125e265",
            asset="tf-2.0.1.zip",
            size=29_981_816,
            unpacked=166_080_661,
            sha256="26a390ed4f089f4a683c0df475a32cdd6932a83ab383d9379ff6799752f90cba",
            layout="flat",
            terms="https://github.com/ETCBC/dss",
            licence="CC BY-NC 4.0",
            extras=(
                Extra(
                    "app/config.yaml",
                    "https://raw.githubusercontent.com/ETCBC/dss/v2.0.1/app/config.yaml",
                    "360bb1e732a2ae5689749d5ba432045a535760004d892030fba9535c1ddfe746",
                ),
            ),
        ),
    )
}


def find(name: str) -> Download | None:
    """A download by its id (`ETCBC/bhsa`) or short name (`bhsa`), any case."""
    wanted = name.lower()
    for d in DOWNLOADS.values():
        if wanted in (d.id.lower(), d.name):
            return d
    return None


def is_present(download: Download, root: str | Path = ROOT) -> bool:
    """Whether the corpus' data directory is on this computer."""
    data = download.data_dir(root)
    return (data / "otype.tf").is_file() or (data / ".cfm").is_dir()


def _fetch(url: str, target: Path, report: Callable[[int, int], None]) -> None:
    request = urllib.request.Request(
        url, headers={"User-Agent": f"cfabric-browser/{__version__}"}
    )
    with urllib.request.urlopen(
        request, timeout=60, context=_ssl_context()
    ) as response:
        total = int(response.headers.get("Content-Length") or 0)
        done = 0
        with open(target, "wb") as out:
            while chunk := response.read(CHUNK):
                out.write(chunk)
                done += len(chunk)
                report(done, total)


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(1 << 20):
            digest.update(chunk)
    return digest.hexdigest()


def _get(
    url: str,
    target: Path,
    sha256: str,
    fetch: Fetch,
    report: Callable[[int, int], None],
) -> None:
    """Download into `target`, unless a file with the right checksum is there;
    tried twice, then the error is passed on."""
    if target.is_file() and _sha256(target) == sha256:
        return
    partial = target.with_name(target.name + ".part")
    for attempt in (1, 2):
        try:
            fetch(url, partial, report)
            break
        except OSError as e:
            if attempt == 2:
                partial.unlink(missing_ok=True)
                raise DownloadError(f"Could not download {url}: {e}") from e
    found = _sha256(partial)
    if found != sha256:
        partial.unlink(missing_ok=True)
        raise DownloadError(
            f"The download of {url} is not the expected file "
            f"(checksum {found[:12]}…, expected {sha256[:12]}…)"
        )
    partial.replace(target)


def fetch(
    download: Download,
    cache: str | Path = CACHE,
    progress: Progress | None = None,
    fetcher: Fetch = _fetch,
) -> tuple[Path, dict[str, Path]]:
    """The release zip and the extra files, downloaded into `cache` (or taken
    from there when their checksums match)."""
    folder = Path(cache).expanduser()
    folder.mkdir(parents=True, exist_ok=True)
    stem = f"{download.org}-{download.repo}-{download.release}"
    report = (
        (lambda done, total: progress("download", done, total or download.size))
        if progress
        else (lambda done, total: None)
    )
    archive = folder / f"{stem}-{download.asset}"
    _get(download.url, archive, download.sha256, fetcher, report)
    extras = {}
    for extra in download.extras:
        target = folder / f"{stem}-{extra.path.replace('/', '-')}"
        _get(extra.url, target, extra.sha256, fetcher, lambda done, total: None)
        extras[extra.path] = target
    return (archive, extras)


def _checkout(download: Download) -> str:
    return f"{download.release}\ng{download.commit}\n"


def _members(download: Download, archive: zipfile.ZipFile) -> list[zipfile.ZipInfo]:
    """The files of the zip, after checking their names and total size."""
    files = [i for i in archive.infolist() if not i.is_dir()]
    for info in files:
        name = info.filename
        parts = PurePosixPath(name).parts
        if name.startswith(("/", "\\")) or "\\" in name or ":" in name or ".." in parts:
            raise DownloadError(
                f"{download.id}: the zip has a file outside its folder: {name}"
            )
        if download.layout == "complete" and parts[0] != download.org:
            raise DownloadError(
                f"{download.id}: the zip has a file of another organisation: {name}"
            )
        if download.layout == "flat" and len(parts) != 1:
            raise DownloadError(
                f"{download.id}: the zip has a folder, not only files: {name}"
            )
    total = sum(i.file_size for i in files)
    if total > 2 * download.unpacked:
        raise DownloadError(
            f"{download.id}: the zip unpacks to {total} bytes, far more than expected"
        )
    return files


def install(
    download: Download,
    archive: Path,
    extras: dict[str, Path] | None = None,
    root: str | Path = ROOT,
    progress: Progress | None = None,
) -> list[Path]:
    """Unpack a downloaded corpus into `root`; returns the data folders put in
    place (folders already there are left alone and not listed)."""
    base = Path(root).expanduser()
    base.mkdir(parents=True, exist_ok=True)
    staging = base / f".download-{uuid.uuid4().hex[:8]}"
    staging.mkdir()
    try:
        with zipfile.ZipFile(archive) as z:
            files = _members(download, z)
            prefix = (
                PurePosixPath()
                if download.layout == "complete"
                else PurePosixPath(download.org, download.repo, "tf", download.version)
            )
            for i, info in enumerate(files):
                target = staging.joinpath(*(prefix / info.filename).parts)
                target.parent.mkdir(parents=True, exist_ok=True)
                with z.open(info) as source, open(target, "wb") as out:
                    shutil.copyfileobj(source, out, CHUNK)
                if progress is not None:
                    progress("unpack", i + 1, len(files))
        if download.layout == "flat":
            data = staging / download.org / download.repo / "tf" / download.version
            (data / CHECKOUT).write_text(_checkout(download), encoding="utf-8")
        for path, source in (extras or {}).items():
            target = staging / download.org / download.repo / PurePosixPath(path)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
            # the folder of an extra file is a data folder of its own (app/)
            (target.parent / CHECKOUT).write_text(_checkout(download), encoding="utf-8")

        units = sorted(p.parent for p in staging.rglob(CHECKOUT))
        for unit in units:
            if any(other != unit and other in unit.parents for other in units):
                raise DownloadError(
                    f"{download.id}: a data folder inside another: {unit.relative_to(staging)}"
                )
        if not units:
            raise DownloadError(f"{download.id}: the zip has no data folders")
        placed = []
        for unit in units:
            destination = base / unit.relative_to(staging)
            if destination.exists():
                continue
            destination.parent.mkdir(parents=True, exist_ok=True)
            unit.rename(destination)
            placed.append(destination)
    finally:
        shutil.rmtree(staging, ignore_errors=True)
    if not is_present(download, root):
        raise DownloadError(
            f"{download.id}: no data in tf/{download.version} after unpacking"
        )
    return placed


def get(
    download: Download,
    root: str | Path = ROOT,
    cache: str | Path = CACHE,
    progress: Progress | None = None,
    fetcher: Fetch = _fetch,
    keep: bool = False,
) -> Path:
    """Download and unpack a corpus, unless it is there already; returns its
    data directory. The downloaded files are removed afterwards, unless `keep`."""
    if not is_present(download, root):
        (archive, extras) = fetch(
            download, cache=cache, progress=progress, fetcher=fetcher
        )
        install(download, archive, extras, root=root, progress=progress)
        if not keep:
            for path in (archive, *extras.values()):
                path.unlink(missing_ok=True)
    return download.data_dir(root)


class DownloadJobs(BackgroundJobs):
    """Downloads running in the background, one at a time per corpus, with their
    progress for the library to show."""

    def __init__(
        self, root: str | Path = ROOT, cache: str | Path = CACHE, **options: Any
    ) -> None:
        super().__init__(expected=(DownloadError,))
        self.root = root
        self.cache = cache
        self.options = options  # e.g. fetcher=, for tests

    def start(
        self,
        download: Download,
        then: Callable[[Progress], Any] | None = None,
    ) -> dict[str, Any]:
        """Download and unpack a corpus; then `then(progress)`, e.g. open and
        prepare it (the first loading compiles it, which takes minutes)."""

        def work(progress: Progress) -> None:
            get(
                download,
                root=self.root,
                cache=self.cache,
                progress=progress,
                **self.options,
            )
            if then is not None:
                then(progress)

        return self.run(download.id, work)
