"""Downloading and unpacking corpora, with small zips made here (no network)."""

from __future__ import annotations

import hashlib
import io
import time
import zipfile
from dataclasses import replace
from pathlib import Path

import pytest
from cfabric_browser.downloads import (
    DOWNLOADS,
    DownloadError,
    Extra,
    fetch,
    find,
    get,
    is_present,
)

CHECKOUT = b"v1.0\ngabc\n"


def make_zip(files: dict[str, bytes]) -> bytes:
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as z:
        for name, data in files.items():
            z.writestr(name, data)
    return buffer.getvalue()


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


# a real corpus (the display test corpus of the core package), so that it can
# also be opened after downloading
FIXTURE = Path(__file__).resolve().parent / "fixtures" / "display_corpus"
COMPLETE = make_zip(
    {
        "Org/corpus/app/config.yaml": (FIXTURE / "app" / "config.yaml").read_bytes(),
        "Org/corpus/app/__checkout__.txt": CHECKOUT,
        **{
            f"Org/corpus/tf/1.0/{f.name}": f.read_bytes()
            for f in sorted((FIXTURE / "tf" / "1.0").glob("*.tf"))
        },
        "Org/corpus/tf/1.0/__checkout__.txt": CHECKOUT,
        "Org/module/tf/1.0/extra.tf": b"@node\n",
        "Org/module/tf/1.0/__checkout__.txt": CHECKOUT,
    }
)
FILES = 2 + len(list((FIXTURE / "tf" / "1.0").glob("*.tf"))) + 3
FLAT = make_zip({"otype.tf": b"@node\n", "book.tf": b"@node\n"})
CONFIG = b"writing: hbo\n"


def corpus(archive: bytes, **changes) -> object:
    fields = {
        "id": "Org/corpus",
        "name": "corpus",
        "version": "1.0",
        "release": "v1.0",
        "commit": "abc",
        "asset": "complete.zip",
        "size": len(archive),
        "unpacked": 10_000,
        "sha256": sha(archive),
    }
    return replace(DOWNLOADS["ETCBC/bhsa"], **{**fields, **changes})


class Fetcher:
    """Serves bytes by URL; can fail the first few times."""

    def __init__(self, files: dict[str, bytes], failures: int = 0) -> None:
        self.files = files
        self.failures = failures
        self.urls: list[str] = []

    def __call__(self, url: str, target: Path, report) -> None:
        self.urls.append(url)
        if self.failures:
            self.failures -= 1
            raise OSError("connection reset")
        data = self.files[url]
        target.write_bytes(data)
        report(len(data), len(data))


def test_find_by_name_or_id() -> None:
    assert find("BHSA") is DOWNLOADS["ETCBC/bhsa"]
    assert find("etcbc/dss") is DOWNLOADS["ETCBC/dss"]
    assert (
        find("sp").url
        == "https://github.com/DT-UCPH/sp/releases/download/v7.2.4/complete.zip"
    )
    assert find("lxx") is None


def test_a_complete_zip_is_unpacked_in_text_fabric_layout(tmp_path: Path) -> None:
    d = corpus(COMPLETE)
    steps: list[tuple[str, int, int]] = []
    data = get(
        d,
        root=tmp_path / "root",
        cache=tmp_path / "cache",
        progress=lambda *s: steps.append(s),
        fetcher=Fetcher({d.url: COMPLETE}),
    )
    assert data == tmp_path / "root" / "Org" / "corpus" / "tf" / "1.0"
    assert is_present(d, tmp_path / "root")
    assert (data / "__checkout__.txt").read_bytes() == CHECKOUT
    assert (tmp_path / "root" / "Org" / "module" / "tf" / "1.0" / "extra.tf").is_file()
    assert ("download", len(COMPLETE), len(COMPLETE)) in steps
    assert steps[-1] == ("unpack", FILES, FILES)
    # nothing left behind: no staging folder, no zip
    assert sorted(p.name for p in (tmp_path / "root").iterdir()) == ["Org"]
    assert list((tmp_path / "cache").iterdir()) == []


def test_folders_already_there_are_left_alone(tmp_path: Path) -> None:
    d = corpus(COMPLETE)
    mine = tmp_path / "root" / "Org" / "module" / "tf" / "1.0"
    mine.mkdir(parents=True)
    (mine / "own.tf").write_text("mine", encoding="utf-8")
    get(
        d,
        root=tmp_path / "root",
        cache=tmp_path / "cache",
        fetcher=Fetcher({d.url: COMPLETE}),
    )
    assert sorted(p.name for p in mine.iterdir()) == ["own.tf"]
    assert is_present(d, tmp_path / "root")


def test_a_corpus_that_is_there_is_not_downloaded(tmp_path: Path) -> None:
    d = corpus(COMPLETE)
    data = d.data_dir(tmp_path)
    data.mkdir(parents=True)
    (data / "otype.tf").write_text("@node\n", encoding="utf-8")
    fetcher = Fetcher({})
    assert get(d, root=tmp_path, cache=tmp_path / "cache", fetcher=fetcher) == data
    assert fetcher.urls == []


def test_a_flat_zip_goes_into_its_version_with_extra_files(tmp_path: Path) -> None:
    extra = Extra("app/config.yaml", "https://example.org/config.yaml", sha(CONFIG))
    d = corpus(FLAT, layout="flat", asset="tf-1.0.zip", extras=(extra,))
    data = get(
        d,
        root=tmp_path / "root",
        cache=tmp_path / "cache",
        fetcher=Fetcher({d.url: FLAT, extra.url: CONFIG}),
    )
    assert sorted(p.name for p in data.iterdir()) == [
        "__checkout__.txt",
        "book.tf",
        "otype.tf",
    ]
    assert (data / "__checkout__.txt").read_text(encoding="utf-8") == "v1.0\ngabc\n"
    app = data.parent.parent / "app"
    assert (app / "config.yaml").read_bytes() == CONFIG
    assert (app / "__checkout__.txt").is_file()
    assert list((tmp_path / "cache").iterdir()) == []


def test_a_wrong_checksum_stops_before_unpacking(tmp_path: Path) -> None:
    d = corpus(COMPLETE)
    with pytest.raises(DownloadError, match="not the expected file"):
        get(
            d,
            root=tmp_path / "root",
            cache=tmp_path / "cache",
            fetcher=Fetcher({d.url: FLAT}),
        )
    assert not (tmp_path / "root").exists()
    assert list((tmp_path / "cache").iterdir()) == []


@pytest.mark.parametrize(
    ("name", "message"),
    [
        ("Org/corpus/../../evil.tf", "outside its folder"),
        ("/etc/evil.tf", "outside its folder"),
        ("C:/evil.tf", "outside its folder"),
        ("Other/corpus/tf/1.0/x.tf", "another organisation"),
    ],
)
def test_unsafe_names_are_refused(tmp_path: Path, name: str, message: str) -> None:
    bad = make_zip(
        {
            "Org/corpus/tf/1.0/otype.tf": b"@node\n",
            "Org/corpus/tf/1.0/__checkout__.txt": CHECKOUT,
            name: b"x",
        }
    )
    d = corpus(bad)
    with pytest.raises(DownloadError, match=message):
        get(
            d,
            root=tmp_path / "root",
            cache=tmp_path / "cache",
            fetcher=Fetcher({d.url: bad}),
        )
    assert list((tmp_path / "root").iterdir()) == []


def test_a_flat_zip_with_folders_is_refused(tmp_path: Path) -> None:
    bad = make_zip({"otype.tf": b"@node\n", "sub/x.tf": b"x"})
    d = corpus(bad, layout="flat")
    with pytest.raises(DownloadError, match="a folder, not only files"):
        get(
            d,
            root=tmp_path / "root",
            cache=tmp_path / "cache",
            fetcher=Fetcher({d.url: bad}),
        )


def test_a_zip_much_larger_than_expected_is_refused(tmp_path: Path) -> None:
    d = corpus(COMPLETE, unpacked=10)
    with pytest.raises(DownloadError, match="far more than expected"):
        get(
            d,
            root=tmp_path / "root",
            cache=tmp_path / "cache",
            fetcher=Fetcher({d.url: COMPLETE}),
        )


def test_a_failed_download_is_tried_again_once(tmp_path: Path) -> None:
    d = corpus(COMPLETE)
    fetcher = Fetcher({d.url: COMPLETE}, failures=1)
    get(d, root=tmp_path / "root", cache=tmp_path / "cache", fetcher=fetcher)
    assert len(fetcher.urls) == 2
    with pytest.raises(DownloadError, match="Could not download .*connection reset"):
        get(
            corpus(COMPLETE, version="2.0"),
            root=tmp_path / "root",
            cache=tmp_path / "cache",
            fetcher=Fetcher({}, failures=2),
        )


def test_a_zip_in_the_cache_is_used_again(tmp_path: Path) -> None:
    d = corpus(COMPLETE)
    fetcher = Fetcher({d.url: COMPLETE})
    (archive, _) = fetch(d, cache=tmp_path, fetcher=fetcher)
    fetch(d, cache=tmp_path, fetcher=fetcher)
    assert len(fetcher.urls) == 1
    archive.write_bytes(b"damaged")
    fetch(d, cache=tmp_path, fetcher=fetcher)
    assert len(fetcher.urls) == 2


def test_the_catalogue_names_the_versions_the_browser_uses() -> None:
    assert {d.id: (d.version, d.release) for d in DOWNLOADS.values()} == {
        "ETCBC/bhsa": ("2021", "v1.8.1"),
        "DT-UCPH/sp": ("7.2.4", "v7.2.4"),
        "ETCBC/dss": ("2.0.1", "v2.0.1"),
    }


# downloading through the API


@pytest.fixture
def library(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    from cfabric_browser.app import create_app
    from cfabric_browser.corpora import CorpusService
    from cfabric_browser.downloads import DownloadJobs
    from cfabric_browser.workers import SearchPool
    from fastapi.testclient import TestClient

    d = corpus(COMPLETE)
    monkeypatch.setitem(DOWNLOADS, d.id, d)
    root = tmp_path / "root"
    root.mkdir()
    service = CorpusService([], roots=(str(root),))
    jobs = DownloadJobs(
        root, cache=tmp_path / "cache", fetcher=Fetcher({d.url: COMPLETE})
    )
    app = create_app(service, SearchPool([]), manage_pool=False, downloads=jobs)
    with TestClient(app) as client:
        yield (client, jobs)


def finished(client, path: str) -> dict:
    for _ in range(200):
        entry = client.get(path).json()
        if entry["job"] and entry["job"]["state"] != "running":
            return entry
        time.sleep(0.05)
    raise AssertionError("the download did not finish")


def test_download_a_corpus_from_the_library(library) -> None:
    (client, _) = library
    entries = {d["id"]: d for d in client.get("/api/downloads").json()["downloads"]}
    assert entries["Org/corpus"]["present"] is False
    assert entries["ETCBC/bhsa"]["size"] == 33_963_367
    assert {d["licence"] for i, d in entries.items() if i != "Org/corpus"} == {
        "CC BY-NC 4.0"
    }

    started = client.post("/api/downloads/Org/corpus")
    assert started.status_code == 202
    assert started.json()["job"]["state"] == "running"
    entry = finished(client, "/api/downloads/Org/corpus")
    assert entry["job"]["state"] == "done"
    assert entry["job"]["step"] == "prepare"
    assert entry["present"] is True
    # it is open and loaded
    library = client.get("/api/library").json()["corpora"]
    assert [(c["id"], c["open"]) for c in library] == [("Org/corpus", True)]
    assert client.get("/api/corpora/corpus").json()["title"] == "Display test corpus"

    # once it is there, nothing is downloaded again
    again = client.post("/api/downloads/Org/corpus").json()
    assert again["job"]["state"] == "done"


def test_a_failed_download_says_why(library) -> None:
    (client, jobs) = library
    jobs.options["fetcher"] = Fetcher({}, failures=2)
    client.post("/api/downloads/Org/corpus")
    entry = finished(client, "/api/downloads/Org/corpus")
    assert entry["job"]["state"] == "failed"
    assert "connection reset" in entry["job"]["error"]
    assert entry["present"] is False


def test_unknown_or_excluded_corpora_cannot_be_downloaded(tmp_path: Path) -> None:
    from cfabric_browser.app import create_app
    from cfabric_browser.corpora import CorpusService
    from cfabric_browser.workers import SearchPool
    from fastapi.testclient import TestClient

    service = CorpusService([], roots=(str(tmp_path),), only=["ETCBC/bhsa"])
    with TestClient(create_app(service, SearchPool([]), manage_pool=False)) as client:
        assert [d["id"] for d in client.get("/api/downloads").json()["downloads"]] == [
            "ETCBC/bhsa"
        ]
        assert client.post("/api/downloads/ETCBC/dss").status_code == 404
        assert client.post("/api/downloads/ETCBC/lxx").status_code == 404


def test_a_download_can_be_cancelled(library) -> None:
    (client, jobs) = library

    class Slow:
        def __call__(self, url, target, report):
            for _ in range(200):
                report(1, 100)
                time.sleep(0.01)

    jobs.options["fetcher"] = Slow()
    client.post("/api/downloads/Org/corpus")
    time.sleep(0.1)
    client.delete("/api/downloads/Org/corpus")
    entry = finished(client, "/api/downloads/Org/corpus")
    assert entry["job"]["state"] == "cancelled"
    assert entry["present"] is False


def test_the_program_asks_at_the_start() -> None:
    from cfabric_browser.app import create_app
    from cfabric_browser.corpora import CorpusService
    from cfabric_browser.workers import SearchPool
    from fastapi.testclient import TestClient

    def health(**options) -> dict:
        app = create_app(
            CorpusService([]), SearchPool([]), manage_pool=False, **options
        )
        with TestClient(app) as client:
            return client.get("/api/health").json()

    assert health()["askDownloads"] is False
    assert health(ask_downloads=True)["askDownloads"] is True
    # not where corpora can't be opened
    assert health(ask_downloads=True, library=False)["askDownloads"] is False
