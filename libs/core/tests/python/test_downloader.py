"""Corpus downloads and the user cache directory."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

import cfabric


def test_cache_dir_honors_cfabric_cache(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    monkeypatch.setenv("CFABRIC_CACHE", str(tmp_path))
    assert cfabric.get_cache_dir() == tmp_path


def test_cache_dir_defaults_to_the_platform_cache(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("CFABRIC_CACHE", raising=False)
    cache_dir = cfabric.get_cache_dir()
    assert isinstance(cache_dir, Path)
    assert "cfabric" in cache_dir.parts


def test_download_without_the_extra_names_it(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setitem(sys.modules, "huggingface_hub", None)
    with pytest.raises(ImportError, match=r"context-fabric\[download\]"):
        cfabric.download("someone/some-corpus")


def test_unknown_short_names_are_rejected() -> None:
    with pytest.raises(ValueError, match="Unknown corpus"):
        cfabric.download("no-such-corpus")
