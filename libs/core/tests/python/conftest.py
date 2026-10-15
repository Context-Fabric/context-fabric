from __future__ import annotations

import os
import shutil
import subprocess
import sys
import threading
import time
from pathlib import Path

import pytest

from cfabric import Fabric
from process_memory import resident_mb

TESTS_DIR = Path(__file__).resolve().parent
"""This directory, importable by `WatchedProcess` scripts."""


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


def write_synthetic_corpus(directory: Path, words: int, words_per_sentence: int = 10) -> Path:
    """Write a corpus of ``words`` words in sentences, each word with a
    ``letter`` (``a`` to ``j`` in turn), large enough for searches that take
    measurable time."""
    directory.mkdir(parents=True)
    sentences = words // words_per_sentence
    header = "@{kind}\n@valueType=str\n@writtenBy=test\n\n"
    (directory / "otype.tf").write_text(
        header.format(kind="node")
        + f"1-{words}\tword\n{words + 1}-{words + sentences}\tsentence\n"
    )
    slots = (
        f"{start}-{start + words_per_sentence - 1}"
        for start in range(1, words + 1, words_per_sentence)
    )
    (directory / "oslots.tf").write_text(
        header.format(kind="edge") + f"{words + 1}\t" + "\n".join(slots) + "\n"
    )
    letters = "abcdefghij"
    (directory / "letter.tf").write_text(
        header.format(kind="node")
        + "\n".join(letters[n % len(letters)] for n in range(words))
        + "\n"
    )
    (directory / "otext.tf").write_text("@config\n@fmt:text-orig-full={letter} \n")
    return directory


class WatchedProcess:
    """A Python child process that is killed when its resident memory passes
    ``rss_limit_mb`` or it runs longer than ``timeout`` seconds, so that a
    search that runs away cannot take the machine down with it. The script
    can import the modules beside this one, such as `process_memory`.
    """

    def __init__(
        self, script: str, *args: str, rss_limit_mb: float = 2048, timeout: float = 120
    ) -> None:
        python_path = os.pathsep.join(
            filter(None, [str(TESTS_DIR), os.environ.get("PYTHONPATH")])
        )
        self.process = subprocess.Popen(
            [sys.executable, "-c", script, *args],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env={**os.environ, "PYTHONPATH": python_path},
        )
        self.killed: str | None = None
        self._limit = rss_limit_mb
        self._deadline = time.monotonic() + timeout
        self._watcher = threading.Thread(target=self._watch, daemon=True)
        self._watcher.start()

    def _watch(self) -> None:
        while self.process.poll() is None:
            rss = resident_mb(self.process.pid)
            if rss > self._limit:
                self.killed = f"resident memory {rss:.0f} MB passed {self._limit} MB"
            elif time.monotonic() > self._deadline:
                self.killed = "timed out"
            if self.killed:
                self.process.kill()
                return
            time.sleep(0.05)

    def readline(self) -> str:
        """The next line the child prints."""
        assert self.process.stdout is not None
        return self.process.stdout.readline().strip()

    def finish(self) -> tuple[str, str]:
        """Wait for the child; its remaining output and its error output."""
        stdout, stderr = self.process.communicate()
        self._watcher.join()
        assert self.killed is None, f"child killed: {self.killed}\n{stderr}"
        return stdout, stderr
