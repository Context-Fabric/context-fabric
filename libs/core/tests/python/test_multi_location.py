"""A corpus composed from ordered locations honors feature precedence.

A later location overrides features of an earlier one in every load mode,
including after compiling the composition, and a compiled cache left behind
by loading the base location alone never stands in for the composition.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from conftest import LOAD_MODES, open_corpus

SECTIONS = ("@sectionFeatures=sentence_id,phrase_id", "@sectionTypes=sentence,phrase")


def _override_module(base: Path, tmp_path: Path) -> Path:
    module = tmp_path / "override"
    module.mkdir()
    word = (base / "word.tf").read_text(encoding="utf-8")
    (module / "word.tf").write_text(word.replace("hello", "OVERRIDDEN"), encoding="utf-8")
    return module


@pytest.mark.parametrize("mode", LOAD_MODES)
def test_later_location_overrides_earlier(mini_corpus_with_otext, tmp_path, mode):
    base = mini_corpus_with_otext(*SECTIONS)
    override = _override_module(base, tmp_path)

    # Leave behind whatever cache loading the base location alone produces.
    assert open_corpus([base], "loadAll", tmp_path).F.word.v(1) == "hello"

    api = open_corpus([base, override], mode, tmp_path)
    assert api.F.word.v(1) == "OVERRIDDEN"
    assert api.T.nodeFromSection(("S1", 1)) == 6
