"""Downloading: certificates, requests under the browser's name, checksums.

Shared by the corpus downloads (`downloads.py`), the translations
(`translations/`) and the dictionaries (`dictionaries/`).
"""

from __future__ import annotations

import hashlib
import http.client
import ssl
import urllib.request
from collections.abc import Callable
from pathlib import Path

from cfabric_browser import __version__

# where Text-Fabric keeps its downloads, and the library looks first
CORPUS_ROOT = "~/text-fabric-data/github"
AGENT = f"cfabric-browser/{__version__}"
CHUNK = 1 << 16

# reports (bytes done, bytes in all; 0 when unknown)
Report = Callable[[int, int], None]
# fetches a URL into a file, reporting as it goes
Fetch = Callable[[str, Path, Report], None]


class DownloadError(Exception):
    """A download that failed or is not what was expected; the message says why."""


def ssl_context() -> ssl.SSLContext:
    """The system's certificates, or certifi's where it is installed: the
    standalone program on macOS has none of its own."""
    try:
        import certifi
    except ImportError:
        return ssl.create_default_context()
    return ssl.create_default_context(cafile=certifi.where())


def read_url(url: str, agent: str = AGENT) -> bytes:
    """The contents of a URL."""
    request = urllib.request.Request(url, headers={"User-Agent": agent})
    with urllib.request.urlopen(request, timeout=60, context=ssl_context()) as r:
        data: bytes = r.read()
    return data


def fetch_to(url: str, target: Path, report: Report) -> None:
    """Download a URL into a file, in chunks."""
    request = urllib.request.Request(url, headers={"User-Agent": AGENT})
    with urllib.request.urlopen(request, timeout=60, context=ssl_context()) as response:
        total = int(response.headers.get("Content-Length") or 0)
        done = 0
        with open(target, "wb") as out:
            while chunk := response.read(CHUNK):
                out.write(chunk)
                done += len(chunk)
                report(done, total)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(1 << 20):
            digest.update(chunk)
    return digest.hexdigest()


def download_file(
    url: str, target: Path, sha256: str, fetch: Fetch, report: Report
) -> None:
    """Download into `target`, unless a file with the right checksum is there;
    tried twice, then the error is passed on. Nothing is left of a download
    that fails, is cancelled or has another checksum."""
    if target.is_file() and sha256_file(target) == sha256:
        return
    partial = target.with_name(target.name + ".part")
    try:
        for attempt in (1, 2):
            try:
                fetch(url, partial, report)
                break
            # IncompleteRead (a connection cut off) is no OSError
            except (OSError, http.client.HTTPException) as e:
                if attempt == 2:
                    raise DownloadError(f"Could not download {url}: {e}") from e
    except BaseException:
        partial.unlink(missing_ok=True)
        raise
    found = sha256_file(partial)
    if found != sha256:
        partial.unlink(missing_ok=True)
        raise DownloadError(
            f"The download of {url} is not the expected file "
            f"(checksum {found[:12]}…, expected {sha256[:12]}…)"
        )
    partial.replace(target)
