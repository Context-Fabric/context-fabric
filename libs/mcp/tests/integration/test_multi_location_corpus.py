"""RED contract for composing one MCP corpus from ordered TF locations.

This is the consumer-side half of alexsosn/ugarit-context-parsing#28.
Core cfabric.Fabric already supports ordered multiple locations; these tests
require CorpusManager to preserve that capability without breaking scalar paths.
"""

from pathlib import Path

import pytest

from cfabric_mcp.corpus_manager import CorpusManager


def _feature_only_module(tmp_path: Path) -> Path:
    module = tmp_path / "module"
    module.mkdir()
    (module / "module_label.tf").write_text(
        "@node\n"
        "@valueType=str\n"
        "@description=feature-only composition contract\n"
        "\n"
        "one\n"
        "two\n"
        "three\n"
        "four\n"
        "five\n",
        encoding="utf-8",
    )
    return module


def test_scalar_str_and_path_remain_backward_compatible(mini_corpus_path):
    manager = CorpusManager()

    string_info = manager.load(mini_corpus_path, name="string-path")
    assert string_info.name == "string-path"
    assert manager.get_api("string-path").F.otype.maxSlot == 5

    path_info = manager.load(Path(mini_corpus_path), name="path-object")
    assert path_info.name == "path-object"
    assert manager.get_api("path-object").F.otype.maxSlot == 5


def test_ordered_list_composes_feature_only_second_location(mini_corpus_path, tmp_path):
    module = _feature_only_module(tmp_path)
    manager = CorpusManager()

    info = manager.load([mini_corpus_path, module], name="composed")
    fabric, api = manager.get("composed")

    assert tuple(Path(path).resolve() for path in fabric.locations) == (
        Path(mini_corpus_path).resolve(),
        module.resolve(),
    )
    assert api.F.otype.maxSlot == 5
    assert api.F.otype.maxNode == 8
    assert "module_label" in info.node_features
    assert api.Fs("module_label", warn=False).v(1) == "one"
    assert api.Fs("module_label", warn=False).v(5) == "five"


def test_ordered_tuple_preserves_default_name_from_base_location(mini_corpus_path, tmp_path):
    module = _feature_only_module(tmp_path)
    manager = CorpusManager()

    info = manager.load((Path(mini_corpus_path), module))

    assert info.name == Path(mini_corpus_path).resolve().name
    assert manager.current == info.name
    assert manager.get_api(info.name).Fs("module_label", warn=False).v(3) == "three"


def test_empty_location_iterable_fails_explicitly():
    manager = CorpusManager()

    with pytest.raises(ValueError, match="at least one"):
        manager.load([])


def test_missing_member_location_fails_before_fabric(mini_corpus_path, tmp_path):
    manager = CorpusManager()
    missing = tmp_path / "missing-module"

    with pytest.raises(FileNotFoundError, match="Corpus path not found"):
        manager.load([mini_corpus_path, missing])


def test_non_directory_member_location_fails_before_fabric(mini_corpus_path, tmp_path):
    manager = CorpusManager()
    not_directory = tmp_path / "module.tf"
    not_directory.write_text("not a directory", encoding="utf-8")

    with pytest.raises(NotADirectoryError, match="not a directory"):
        manager.load([mini_corpus_path, not_directory])
