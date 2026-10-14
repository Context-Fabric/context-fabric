"""MCP passage lookup with explicitly selected features and a warm CFM cache."""

import shutil
from pathlib import Path

from cfabric import Fabric
from cfabric_mcp import tools
from cfabric_mcp.corpus_manager import corpus_manager


def test_get_passages_after_selective_cfm_load(tmp_path):
    fixture = Path(__file__).parents[1] / "fixtures" / "mini_corpus"
    corpus = tmp_path / "corpus"
    shutil.copytree(fixture, corpus, ignore=shutil.ignore_patterns(".cfm", ".tf"))
    assert Fabric(locations=str(corpus), silent="deep").compile()
    name = "selective-section-regression"
    corpus_manager.load(str(corpus), name=name, features=["word"])
    try:
        result = tools.get_passages([["S1", 1], ["S1", 2], ["S1", 99]], corpus=name)
        assert result["found"] == 2
        assert [p["node"] for p in result["passages"][:2]] == [6, 7]
        assert "hello" in result["passages"][0]["text"]
        assert "morning" in result["passages"][1]["text"]
        assert result["passages"][2]["error"].startswith("Section not found")
    finally:
        corpus_manager.unload(name)
