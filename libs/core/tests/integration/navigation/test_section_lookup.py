"""Section lookup must work after selective memory-mapped corpus loading."""

import shutil
from pathlib import Path

import pytest
from cfabric import Fabric


@pytest.mark.parametrize("levels", [1, 2, 3])
@pytest.mark.parametrize("all_features", [False, True])
def test_section_lookup_after_cfm_load(tmp_path, levels, all_features):
    fixture = Path(__file__).parents[2] / "fixtures" / "mini_corpus"
    corpus = tmp_path / "corpus"
    shutil.copytree(fixture, corpus, ignore=shutil.ignore_patterns(".cfm", ".tf"))
    types = ["sentence", "phrase", "word"][:levels]
    features = ["sentence_id", "phrase_id", "word"][:levels]
    (corpus / "otext.tf").write_text(
        "@config\n@fmt:text-orig-full={word}\n"
        f"@sectionTypes={','.join(types)}\n"
        f"@sectionFeatures={','.join(features)}\n",
        encoding="utf-8",
    )
    compiler = Fabric(locations=str(corpus), silent="deep")
    assert compiler.compile()
    cf = Fabric(locations=str(corpus), silent="deep")
    api = cf.loadAll() if all_features else cf.load("word")
    assert cf._loaded_from_cfm
    assert api.T.nodeFromSection(("S1",)) == 8
    if levels > 1:
        assert api.T.nodeFromSection(("S1", 1)) == 6
        assert api.T.nodeFromSection(("S1", 2)) == 7
        assert api.T.nodeFromSection(("S1", 99)) is None
    if levels > 2:
        assert api.T.nodeFromSection(("S1", 1, "hello")) == 1
        assert api.T.nodeFromSection(("S1", 2, "morning")) == 5
        assert api.T.nodeFromSection(("S1", 1, "missing")) is None
    assert api.T.nodeFromSection(("missing",)) is None
