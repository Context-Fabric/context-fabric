from __future__ import annotations

import shutil
from pathlib import Path

import pytest

from cfabric import Fabric


@pytest.fixture
def mini_corpus_path() -> Path:
    return Path(__file__).resolve().parents[1] / "fixtures" / "mini_corpus"


@pytest.fixture
def loaded_api(mini_corpus_path: Path):
    return Fabric(locations=str(mini_corpus_path), silent="deep").loadAll(silent="deep")


LOAD_MODES = ("loadAll", "load", "openMapped")
"""The three ways a corpus reaches the API: all features, selected features,
and a compiled `.cfr` opened through memory mapping."""


@pytest.fixture
def mini_corpus_with_otext(mini_corpus_path: Path, tmp_path: Path):
    """Copy ``mini_corpus`` with a replacement ``otext.tf`` into a fresh directory.

    Returns a factory taking the ``otext`` config lines (after ``@config``) and
    returning the new corpus directory. Compiled caches are never copied, so
    each corpus starts from its `.tf` sources.
    """

    def build(*config_lines: str) -> Path:
        corpus = tmp_path / f"corpus{len(list(tmp_path.iterdir()))}"
        shutil.copytree(
            mini_corpus_path, corpus, ignore=shutil.ignore_patterns(".cf*", ".tf")
        )
        (corpus / "otext.tf").write_text(
            "@config\n@fmt:text-orig-full={word}\n" + "\n".join(config_lines) + "\n",
            encoding="utf-8",
        )
        return corpus

    return build


def open_corpus(locations: list[Path], mode: str, cache_dir: Path, features: str = "word"):
    """Load ``locations`` through one of the `LOAD_MODES`."""
    fabric = Fabric(locations=[str(loc) for loc in locations], silent="deep")
    if mode == "loadAll":
        return fabric.loadAll(silent="deep")
    if mode == "load":
        return fabric.load(features, silent="deep")
    cache = cache_dir / "corpus.cfr"
    assert fabric.compile(str(cache), None, "deep")
    return Fabric(locations=[str(loc) for loc in locations], silent="deep").openMapped(str(cache))
